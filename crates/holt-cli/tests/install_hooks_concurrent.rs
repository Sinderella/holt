//! must_have-4 / D-14: 50× concurrent invocations against a shared settings.json.
//!
//! `#[cfg(unix)]` because the test relies on POSIX flock(2) semantics via fs2.
//! Windows lock semantics differ (LockFileEx is mandatory), so the test is
//! skipped on Windows; install-hooks itself works on Windows but the
//! contention pattern is platform-specific.
//!
//! Strategy: spawn 50 separate `holt install-hooks` processes via
//! `std::process::Command` (out-of-process, NOT in-process threads, so the
//! fs2 lock is genuinely cross-process). Each is invoked against the same
//! `HOME=$tempdir` so they all target the same settings.json. After all
//! children exit, the parent asserts:
//!   (a) The final file parses cleanly via BOTH `serde_json::from_str` AND
//!       `jsonc_parser::parse_to_ast` (the file is valid JSONC).
//!   (b) Each of the 5 holt commands appears EXACTLY ONCE — concurrent
//!       invocations did not introduce duplicates.
//!   (c) The user's pre-existing `Bash`-matcher PreToolUse entry survives.
//!   (d) No `.holt-tmp.<pid>` files remain in `~/.claude/` — the temp-file
//!       cleanup in atomic_write held up under contention.
//!
//! Test budget: <30s wall clock. Allowed exit codes per child: 0 (success)
//! or 2 (lock-timeout). Codes 1 and 3 are NOT acceptable — they indicate
//! a real merge / IO failure that the test must surface.

#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use tempfile::tempdir;

fn holt_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_holt"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/settings")
        .join(name)
}

#[test]
fn concurrent_50x_idempotent_no_torn_writes() {
    let dir = tempdir().unwrap();
    let home = Arc::new(dir.path().to_path_buf());
    let claude = home.join(".claude");
    fs::create_dir_all(&claude).unwrap();
    // Use the user_pretooluse fixture so we can also assert the user's
    // pre-existing PreToolUse entry survives the stress.
    let settings = claude.join("settings.json");
    fs::copy(fixture("user_pretooluse.input.json"), &settings).unwrap();

    let start = Instant::now();
    let mut handles = vec![];
    for _ in 0..50 {
        let home = Arc::clone(&home);
        let bin = holt_binary();
        handles.push(thread::spawn(move || {
            // out-of-process Command spawn so each invocation is a real
            // separate process whose fs2 lock is genuinely cross-process.
            let out = Command::new(bin)
                .env("HOME", home.as_path())
                .arg("install-hooks")
                .output()
                .expect("spawn install-hooks");
            // Allowed: 0 (success) or 2 (lock-timeout). 1 / 3 indicate a
            // real merge / IO failure — those must fail the test.
            let code = out.status.code().unwrap_or(-1);
            assert!(
                code == 0 || code == 2,
                "unexpected exit code {code}; stderr: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }));
    }
    for h in handles {
        h.join().expect("worker thread");
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 30,
        "50× stress took {:?}, budget 30s",
        elapsed
    );

    // (a) Final file parses cleanly with BOTH parsers.
    let post = fs::read_to_string(&settings).expect("read final settings.json");
    let _serde: serde_json::Value = serde_json::from_str(&post)
        .unwrap_or_else(|e| panic!("serde_json failed on final file: {e}\n---FILE---\n{post}"));
    // jsonc-parser is a runtime dep of `holt-cli` (per Plan 03-01) and is
    // visible to this integration test through the same crate. Parse
    // directly to enforce the JSONC contract.
    let parse_opts = jsonc_parser::ParseOptions {
        allow_comments: true,
        allow_trailing_commas: true,
        allow_loose_object_property_names: false,
    };
    jsonc_parser::parse_to_ast(&post, &Default::default(), &parse_opts)
        .unwrap_or_else(|e| panic!("jsonc_parser failed on final file: {e}\n---FILE---\n{post}"));

    // (b) Each holt command appears exactly once.
    for ev in [
        "PreToolUse",
        "PostToolUse",
        "Stop",
        "Notification",
        "SessionStart",
    ] {
        let needle = format!("\"holt hook {ev}\"");
        let count = post.matches(&needle).count();
        assert_eq!(
            count, 1,
            "expected exactly 1 occurrence of `{needle}`, got {count} in:\n{post}"
        );
    }

    // (c) User's pre-existing PreToolUse entry survived.
    assert!(
        post.contains("\"matcher\": \"Bash\"") && post.contains("\"command\": \"user-script\""),
        "user's PreToolUse entry was clobbered:\n{post}"
    );

    // (d) No orphan `.holt-tmp.*` files. atomic_write should have cleaned
    // these up, even under contention.
    let orphans: Vec<_> = fs::read_dir(&claude)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".holt-tmp."))
        .collect();
    assert!(
        orphans.is_empty(),
        "orphan .holt-tmp.* files: {:?}",
        orphans.iter().map(|e| e.file_name()).collect::<Vec<_>>()
    );
}
