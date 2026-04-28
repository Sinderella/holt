//! ROADMAP success criterion #1: wrapping `bash -c "echo hello"` returns
//! `Ok` with `stdout = "hello\n"`, exit_code 0, and writes a valid
//! `timings.jsonl` entry containing the CORE-02 fields.

#![cfg(unix)]

use std::time::Duration;

use holt_supervisor::{SupervisorOptions, SupervisorOutcome, wrap_and_run};
use tempfile::tempdir;

#[test]
fn echo_hello_returns_ok_and_writes_timings() {
    let cache = tempdir().expect("tempdir");
    let opts = SupervisorOptions {
        timeout: Duration::from_secs(5),
        session_id: "test-passthrough".into(),
        stdin_bytes: Vec::new(),
        cache_root: cache.path().to_path_buf(),
        writer_version: "test-0.0.0",
    };

    let outcome = wrap_and_run("bash", &["-c", "echo hello"], opts);
    match outcome {
        SupervisorOutcome::Ok {
            stdout, exit_code, ..
        } => {
            assert_eq!(stdout, "hello\n");
            assert_eq!(exit_code, 0);
        }
        other => panic!("expected Ok, got {other:?}"),
    }

    // timings.jsonl must exist and contain exactly one valid JSON line with
    // the CORE-02 schema.
    let timings = cache.path().join("timings.jsonl");
    let body = std::fs::read_to_string(&timings).expect("timings.jsonl must exist");
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "expected 1 timings line, got {lines:?}");

    let parsed: serde_json::Value =
        serde_json::from_str(lines[0]).expect("timings line must be valid JSON");
    assert_eq!(parsed["exit_code"], 0);
    assert!(parsed["duration_ms"].is_number());
    assert_eq!(parsed["fork_count"], 1);
    assert!(parsed["stderr_capture"].is_string());
    assert_eq!(parsed["session_id"], "test-passthrough");
    assert!(parsed["ts"].is_string());
}
