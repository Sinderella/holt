//! must_have-5 / D-15: 200× SIGKILL atomicity test.
//!
//! Pattern: spawn `holt install-hooks` as a child via std::process::Command,
//! sleep a uniformly-random 0..30ms, then SIGKILL the child via libc::kill.
//! After each iteration, parent reads settings.json and asserts:
//!   (a) parses cleanly via `serde_json::from_str`
//!   (b) parses cleanly via `jsonc_parser::parse_to_ast`
//!   (c) is either the pre-merge state OR the post-merge state — never
//!       half-written.
//!
//! The fsync(2) on the temp fd before rename(2) (Phase 1 atomic_write) is
//! what makes (c) achievable; this test is the falsifiability proof for the
//! C3 atomicity guarantee.
//!
//! `#[cfg(unix)]` because libc::kill + SIGKILL semantics are POSIX.
//! Test budget: <60s wall clock (200 iterations × ~30ms upper bound +
//! parent read-and-check overhead).
//!
//! `#![allow(unsafe_code)]` is scoped to this test file because the
//! libc::kill FFI call is unsafe; the SAFETY comment is at the call site.

#![cfg(unix)]
#![allow(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tempfile::tempdir;

fn holt_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_holt"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/settings")
        .join(name)
}

/// xorshift PRNG — sufficient randomness for 0..30ms scatter; no need to
/// pull `rand` as a dev-dep just for this loop.
fn pseudo_random_delay_ms(seed: u64) -> u64 {
    let mut x = seed.wrapping_add(0x9E3779B97F4A7C15);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x % 31
}

fn parses_with_both(bytes: &str) -> Result<(), String> {
    serde_json::from_str::<serde_json::Value>(bytes).map_err(|e| format!("serde_json: {e}"))?;
    let parse_opts = jsonc_parser::ParseOptions {
        allow_comments: true,
        allow_trailing_commas: true,
        allow_loose_object_property_names: false,
    };
    jsonc_parser::parse_to_ast(bytes, &Default::default(), &parse_opts)
        .map_err(|e| format!("jsonc_parser: {e}"))?;
    Ok(())
}

fn kill_child(child: &mut Child) {
    let pid = child.id() as i32;
    // SAFETY: `pid` is a valid PID owned by this test process (returned by
    // `Child::id()` on a Child we just spawned). SIGKILL is the documented
    // v0.1 atomicity-test signal and mirrors Phase 2's
    // crates/holt-hooks/tests/sigkill_atomicity.rs precedent. No shared
    // state is touched here.
    unsafe {
        libc::kill(pid, libc::SIGKILL);
    }
    let _ = child.wait();
}

#[test]
fn sigkill_200x_never_leaves_half_written_settings() {
    let dir = tempdir().unwrap();
    let home = dir.path().to_path_buf();
    let claude = home.join(".claude");
    fs::create_dir_all(&claude).unwrap();
    let settings = claude.join("settings.json");
    fs::copy(fixture("user_pretooluse.input.json"), &settings).unwrap();

    let pre_bytes = fs::read_to_string(&settings).expect("read pre");
    let mut canonical_post: Option<String> = None;

    let start = Instant::now();
    for i in 0..200u64 {
        // Reset settings to pre state at the start of each iteration so the
        // observable states are exactly {pre, canonical-post}.
        fs::copy(fixture("user_pretooluse.input.json"), &settings).unwrap();
        // Remove any leftover .bak / .holt-tmp from previous iter so they
        // don't accumulate and confuse the orphan-detection invariant.
        let _ = fs::remove_file(home.join(".claude/settings.json.holt.bak"));
        for entry in fs::read_dir(&claude).unwrap().flatten() {
            let n = entry.file_name();
            if n.to_string_lossy().contains(".holt-tmp.") {
                let _ = fs::remove_file(entry.path());
            }
        }

        let mut child = Command::new(holt_binary())
            .env("HOME", &home)
            .arg("install-hooks")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn install-hooks");

        std::thread::sleep(Duration::from_millis(pseudo_random_delay_ms(i)));
        kill_child(&mut child);

        // Read whatever the killed process left behind.
        let observed = fs::read_to_string(&settings).expect("read post-kill");
        parses_with_both(&observed).unwrap_or_else(|e| {
            panic!(
                "iteration {i}: settings.json failed to parse with both engines: {e}\n---FILE---\n{observed}"
            )
        });
        // State must be either pre-merge (the reset above; possibly never
        // touched if SIGKILL hit before the rename) OR a fully-merged post
        // state. Determine the canonical post on the first successful
        // observed-difference, then assert observed ∈ {pre, canonical_post}.
        if canonical_post.is_none() && observed != pre_bytes {
            // Validate this looks like a valid post: contains all 5 holt commands.
            if [
                "PreToolUse",
                "PostToolUse",
                "Stop",
                "Notification",
                "SessionStart",
            ]
            .iter()
            .all(|ev| observed.contains(&format!("\"holt hook {ev}\"")))
            {
                canonical_post = Some(observed.clone());
            }
        }
        if let Some(post) = &canonical_post {
            assert!(
                observed == pre_bytes || observed == *post,
                "iteration {i}: observed state is neither pre nor canonical post.\n---PRE---\n{pre_bytes}\n---POST---\n{post}\n---OBSERVED---\n{observed}"
            );
        } else {
            // Haven't seen a successful merge yet — observed must be pre.
            assert_eq!(
                observed, pre_bytes,
                "iteration {i}: observed state differs from pre but no canonical post seen yet"
            );
        }
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 60,
        "200× SIGKILL stress took {:?}, budget 60s",
        elapsed
    );

    // WR-06: assert final tmp-file state. The 50× concurrent test asserts
    // "no orphan .holt-tmp.<pid>" because every child completed normally —
    // atomic_write's success path removes the tmp file after rename. Under
    // SIGKILL, a child killed BETWEEN tmp-open and rename(2) cannot run
    // its post-rename cleanup, so the tmp file survives until the NEXT
    // `holt install-hooks` invocation observes it.
    //
    // This loop wipes orphans at the START of each iteration (lines ~95-101
    // above), so by iter 200's end there can be at most ONE orphan: the
    // one left behind by the final iteration if its SIGKILL hit before
    // rename. Anything more than one orphan indicates per-iter cleanup
    // is broken or atomic_write is leaking tmp files even on the success
    // path (a real C3 contract regression).
    //
    // The orphan-recovery story for production code: a future hardening
    // (deferred — see WR-06 IN-style follow-up) would sweep stale
    // `.holt-tmp.*` files at the start of `holt install-hooks` so the
    // user never sees them; v0.1 ships without this sweep because the
    // tmp files are 0o600 and cosmetically harmless.
    let orphans: Vec<_> = fs::read_dir(&claude)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".holt-tmp."))
        .collect();
    assert!(
        orphans.len() <= 1,
        "expected ≤1 orphan .holt-tmp.* after 200× SIGKILL (the per-iter \
         cleanup wipes prior leaks; only the final iter's tmp can survive). \
         got {} orphans: {:?}",
        orphans.len(),
        orphans.iter().map(|e| e.file_name()).collect::<Vec<_>>()
    );
}
