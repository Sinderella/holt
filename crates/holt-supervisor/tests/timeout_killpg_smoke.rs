//! ROADMAP success criterion #2: wrapping `bash -c 'sleep 5'` with a 1s
//! timeout
//!   1. returns `Breach { kind: Timeout }` within ~500ms of the deadline;
//!   2. leaves no orphaned `bash` / `sleep 5` PIDs (verified via `pgrep -f`);
//!   3. writes one `breaches.log` JSON entry with the D-13 schema.

#![cfg(unix)]

use std::process::Command;
use std::time::{Duration, Instant};

use holt_supervisor::{BreachKind, SupervisorOptions, SupervisorOutcome, wrap_and_run};
use tempfile::tempdir;

#[test]
fn timeout_breach_kills_descendants_and_writes_breach_log() {
    let cache = tempdir().expect("tempdir");
    let opts = SupervisorOptions {
        timeout: Duration::from_secs(1),
        session_id: "test-killpg".into(),
        stdin_bytes: Vec::new(),
        cache_root: cache.path().to_path_buf(),
    };

    let started = Instant::now();
    let outcome = wrap_and_run("bash", &["-c", "sleep 5"], opts);
    let elapsed = started.elapsed();

    // (1) Breach within a tight window of the 1s deadline.
    match outcome {
        SupervisorOutcome::Breach {
            kind: BreachKind::Timeout,
            ..
        } => {}
        other => panic!("expected Breach{{Timeout}}, got {other:?}"),
    }
    assert!(
        elapsed >= Duration::from_secs(1),
        "breach fired too early: {elapsed:?}"
    );
    assert!(
        elapsed <= Duration::from_millis(1500),
        "breach + cleanup took too long: {elapsed:?}"
    );

    // (2) No orphaned `sleep 5` descendants. Sleep grace then `pgrep`.
    std::thread::sleep(Duration::from_millis(150));
    let pgrep = Command::new("pgrep").args(["-f", "sleep 5"]).output();
    if let Ok(out) = pgrep {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let our_pid = std::process::id().to_string();
        let stragglers: Vec<&str> = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .filter(|p| *p != our_pid)
            .collect();
        assert!(
            stragglers.is_empty(),
            "orphaned descendants survived killpg: {stragglers:?}"
        );
    }
    // If `pgrep` is missing on the test image, the SupervisorOutcome assertion
    // above is the primary signal — accept the test still passes.

    // (3) breaches.log gained exactly one valid entry with the D-13 schema.
    let breaches_path = cache.path().join("breaches.log");
    let body = std::fs::read_to_string(&breaches_path).expect("breaches.log must exist");
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "expected 1 breach entry, got {lines:?}");

    let parsed: serde_json::Value =
        serde_json::from_str(lines[0]).expect("breach line must be valid JSON");
    assert_eq!(parsed["kind"], "timeout");
    assert!(parsed["env_capture"].is_object());
    assert!(parsed["stdin_excerpt"].is_string());
    assert!(parsed["stderr_excerpt"].is_string());
    assert!(parsed["writer_version"].is_string());
    assert!(parsed["ts"].is_string());
}
