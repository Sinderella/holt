//! Integration tests for `handle_event` covering must_have-1 (file shape +
//! 0600 perms), must_have-2 (per-event field policy round-trip), and
//! must_have-4 (fallback chain + unwritable resilience).
//!
//! Each test isolates env state with `tempfile::tempdir()` so the developer's
//! real `~/.cache/holt/`, `$XDG_RUNTIME_DIR`, and `$TMPDIR` are never touched.
//! Env-var manipulation across tests is racy in parallel mode — we serialize
//! these tests via a global Mutex.

use std::sync::Mutex;

use holt_hooks::{Env, HookEvent, HookOutcome, handle_event};
use holt_schemas::read_heartbeat;

// `cargo test` runs integration tests in parallel by default. We mutate
// process-global env vars (XDG_RUNTIME_DIR, TMPDIR, HOME), so any test in
// this file MUST take this lock for its full duration.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env() -> Env {
    Env {
        writer_version: "test-0.0.0",
        pid: std::process::id(),
        now_iso: "2026-04-28T10:00:00Z".to_string(),
    }
}

fn fixture_bytes(rel: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/cc-stdin/{rel}")).expect("fixture exists")
}

#[test]
fn must_have_1_file_shape_and_perms() {
    let _guard = ENV_LOCK.lock().unwrap();
    let xdg = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    // SAFETY: env mutation is serialized by ENV_LOCK; std::env::set_var is
    // safe in single-threaded test scope.
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", xdg.path());
        std::env::set_var("XDG_CACHE_HOME", cache.path());
        std::env::remove_var("TMPDIR");
    }

    let bytes = fixture_bytes("v2.1.119/PreToolUse.json");
    let outcome = handle_event(HookEvent::PreToolUse, &bytes, &env());
    let path = match outcome {
        HookOutcome::Wrote { path, .. } => path,
        other => panic!("expected Wrote, got {other:?}"),
    };

    // File parses via Phase 1 read_heartbeat (must_have-1).
    let hb = read_heartbeat(&path)
        .expect("read_heartbeat returns Result")
        .expect("heartbeat must be Some after handle_event::Wrote");
    assert_eq!(hb.schema_version, 1);
    assert!(!hb.session_id.is_empty());
    assert_eq!(hb.writer_version, "test-0.0.0");
    assert!(!hb.started.is_empty());
    assert!(!hb.updated.is_empty());

    // 0600 perms (Unix only — must_have-1).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "must_have-1: heartbeat file must be 0600 on Unix"
        );
    }
}

#[test]
fn must_have_2_all_five_events_round_trip() {
    let _guard = ENV_LOCK.lock().unwrap();
    let xdg = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", xdg.path());
        std::env::set_var("XDG_CACHE_HOME", cache.path());
        std::env::remove_var("TMPDIR");
    }

    for (event, fixture, expect_tool) in [
        (HookEvent::PreToolUse, "v2.1.119/PreToolUse.json", true),
        (HookEvent::PostToolUse, "v2.1.119/PostToolUse.json", false),
        (HookEvent::Stop, "v2.1.119/Stop.json", false),
        (HookEvent::Notification, "v2.1.119/Notification.json", false),
        (HookEvent::SessionStart, "v2.1.119/SessionStart.json", false),
    ] {
        let bytes = fixture_bytes(fixture);
        let outcome = handle_event(event, &bytes, &env());
        let path = match outcome {
            HookOutcome::Wrote { path, .. } => path,
            other => panic!("event {event:?} expected Wrote, got {other:?}"),
        };
        let hb = read_heartbeat(&path).unwrap().unwrap();
        if expect_tool {
            assert!(
                hb.current_tool.is_some(),
                "{event:?}: D-09 PreToolUse → Some"
            );
        } else {
            assert!(hb.current_tool.is_none(), "{event:?}: D-09 non-Pre → None");
        }
    }
}

#[test]
fn must_have_4_fallback_to_cache_when_xdg_and_tmpdir_unset() {
    let _guard = ENV_LOCK.lock().unwrap();
    // Force tier-3: XDG and TMPDIR both empty. HOME points at a fresh tempdir
    // so default_cache_root computes a writable HOME/.cache/holt path.
    let home = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", "");
        std::env::set_var("TMPDIR", "");
        std::env::remove_var("XDG_CACHE_HOME");
        std::env::set_var("HOME", home.path());
    }

    let bytes = fixture_bytes("v2.1.119/PreToolUse.json");
    let outcome = handle_event(HookEvent::PreToolUse, &bytes, &env());
    match outcome {
        HookOutcome::FellBack { path, .. } => {
            let s = path.to_string_lossy();
            assert!(
                s.contains(".cache") && s.contains("holt") && s.contains("sessions"),
                "must_have-4: tier-3 fallback path must be inside ~/.cache/holt/sessions/, got {s}"
            );
        }
        other => panic!("expected FellBack, got {other:?}"),
    }
}

#[test]
fn must_have_4_unwritable_returns_unwritable_outcome() {
    let _guard = ENV_LOCK.lock().unwrap();
    // WR-06: tighten the assertion. The previous test accepted FellBack as
    // a substitute for Unwritable, citing a hypothetical default_cache_root
    // last-resort to temp_dir — but reading
    // crates/holt-supervisor/src/paths.rs::default_cache_root, the temp_dir
    // last-resort branch fires ONLY when HOME (and USERPROFILE) is unset.
    // We DO set HOME (to a regular file path), so the
    // PathBuf::from(h).join('.cache').join('holt') branch is taken, and
    // create_dir_all under a regular file MUST fail. Therefore the only
    // legitimate outcome is Unwritable; FellBack would mask a real bug.
    let blocker = tempfile::NamedTempFile::new().unwrap(); // a regular file, not a dir
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", "");
        std::env::set_var("TMPDIR", "");
        std::env::remove_var("XDG_CACHE_HOME");
        std::env::set_var("HOME", blocker.path()); // HOME is a file → can't mkdir under it
    }

    let bytes = fixture_bytes("v2.1.119/PreToolUse.json");
    let outcome = handle_event(HookEvent::PreToolUse, &bytes, &env());
    match outcome {
        HookOutcome::Unwritable => { /* expected */ }
        other => panic!(
            "WR-06: expected HookOutcome::Unwritable when HOME points at a regular file \
             (default_cache_root must fail to mkdir under it), got {other:?}"
        ),
    }
}

#[test]
fn must_have_4_warning_emitted_on_fallback_via_stderr() {
    // We can't easily capture eprintln from a test process; instead we assert
    // the FallbackReason is set when tier 1 fails, which is the trigger for
    // the warning emission in handle.rs.
    let _guard = ENV_LOCK.lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", "");
        std::env::set_var("TMPDIR", "");
        std::env::remove_var("XDG_CACHE_HOME");
        std::env::set_var("HOME", home.path());
    }

    let bytes = fixture_bytes("v2.1.119/PreToolUse.json");
    let outcome = handle_event(HookEvent::PreToolUse, &bytes, &env());
    assert!(
        matches!(
            outcome,
            HookOutcome::FellBack {
                reason: holt_hooks::FallbackReason::XdgAndTmpUnavailable,
                ..
            }
        ),
        "expected FellBack with XdgAndTmpUnavailable reason"
    );
}

#[test]
fn parse_failed_on_garbage_stdin() {
    let _guard = ENV_LOCK.lock().unwrap();
    let xdg = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", xdg.path());
        std::env::set_var("XDG_CACHE_HOME", cache.path());
        std::env::remove_var("TMPDIR");
    }

    let bytes = b"{\"session_id\":"; // truncated — same shape as Phase 1 CORE-08 test
    let outcome = handle_event(HookEvent::PreToolUse, bytes, &env());
    assert!(
        matches!(outcome, HookOutcome::ParseFailed),
        "expected ParseFailed on truncated JSON, got {outcome:?}"
    );
}
