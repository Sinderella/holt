//! CR-03 regression: when SupervisorOptions::stdin_bytes is empty, the
//! supervisor must still close the child's stdin pipe. Otherwise a wrapped
//! script that does `read_to_end(stdin)` (cat, jq, ccstatusline) will block
//! forever waiting for EOF and synthesize a guaranteed timeout breach.
//!
//! Test shape: wrap `cat` with empty stdin and a 2s timeout. If stdin is
//! closed correctly, `cat` finishes (with no output) well under 1s. If the
//! pipe is left open, we'd hit the 2s deadline and observe a Breach.

#![cfg(unix)]

use std::time::{Duration, Instant};

use holt_supervisor::{SupervisorOptions, SupervisorOutcome, wrap_and_run};
use tempfile::tempdir;

#[test]
fn empty_stdin_does_not_hang_stdin_reading_child() {
    let cache = tempdir().expect("tempdir");
    let opts = SupervisorOptions {
        timeout: Duration::from_secs(2),
        session_id: "test-empty-stdin".into(),
        stdin_bytes: Vec::new(),
        cache_root: cache.path().to_path_buf(),
        writer_version: "test-0.0.0",
    };

    let started = Instant::now();
    let outcome = wrap_and_run("bash", &["-c", "cat"], opts);
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_millis(1500),
        "expected `cat` with empty stdin to finish quickly; took {elapsed:?} \
         — stdin pipe is probably being left open (CR-03)"
    );
    match outcome {
        SupervisorOutcome::Ok { stdout, exit_code, .. } => {
            assert_eq!(stdout, "");
            assert_eq!(exit_code, 0);
        }
        other => panic!(
            "expected Ok (clean EOF on empty stdin), got {other:?} — \
             child stdin probably not being closed (CR-03)"
        ),
    }
}
