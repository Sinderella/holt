//! ROADMAP success criterion #1 + CORE-08 verification:
//!   - happy path: `holt run -- bash -c "echo hello"` writes exactly "hello\n", exits 0
//!   - malformed CC stdin: parse_fail breach written, no panic, exits 0

use std::io::Write;
use std::process::{Command, Stdio};

use tempfile::tempdir;

#[cfg(unix)]
#[test]
fn run_passthrough_emits_wrapped_stdout_unchanged() {
    let exe = env!("CARGO_BIN_EXE_holt");
    let cache = tempdir().unwrap();

    let mut child = Command::new(exe)
        .args(["run", "--", "bash", "-c", "echo hello"])
        .env("XDG_CACHE_HOME", cache.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn holt");

    // CC normally pipes a JSON envelope; for this test, give it an empty stdin so
    // the defensive parse hits the Empty branch.
    drop(child.stdin.take());

    let out = child.wait_with_output().expect("wait holt");
    assert!(
        out.status.success(),
        "holt run exited non-zero: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // CRITICAL: exactly "hello\n" — no chrome.
    assert_eq!(out.stdout, b"hello\n");
}

#[cfg(unix)]
#[test]
fn run_with_malformed_stdin_records_parse_fail_and_exits_zero() {
    let exe = env!("CARGO_BIN_EXE_holt");
    let cache = tempdir().unwrap();

    let mut child = Command::new(exe)
        .args(["run", "--", "bash", "-c", "echo whatever"])
        .env("XDG_CACHE_HOME", cache.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn holt");

    // Truncated JSON — parse will fail. CORE-08 says: capture as parse_fail breach,
    // fall through to LKG (or empty), exit 0.
    if let Some(mut s) = child.stdin.take() {
        let _ = s.write_all(br#"{"session_id":"#);
    }

    let out = child.wait_with_output().expect("wait holt");
    assert!(
        out.status.success(),
        "holt run with malformed stdin must exit 0; got status={:?} stderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );

    // breaches.log must contain a parse_fail entry. Path is <cache>/holt/breaches.log.
    let breaches = cache.path().join("holt").join("breaches.log");
    assert!(breaches.exists(), "expected breaches.log at {breaches:?}");
    let body = std::fs::read_to_string(&breaches).unwrap();
    let line = body.lines().next().expect("at least one breach line");
    let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
    assert_eq!(parsed["kind"], "parse_fail");
}
