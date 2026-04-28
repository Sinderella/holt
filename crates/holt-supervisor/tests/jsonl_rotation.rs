//! D-12 / D-13: jsonl writers rotate at 5MB.
//!
//! Pre-fills `timings.jsonl` with exactly 5MB, then triggers a single append.
//! The rotation must move the original to `timings.jsonl.1` and leave the
//! current file containing only the freshly-appended line.
//!
//! Platform-agnostic: this test runs on Unix and Windows. The supervisor's
//! `bash`-based smoke tests are gated `#[cfg(unix)]`, but the rotation policy
//! is pure `std::fs` and exercises identically everywhere.

use std::fs;

use tempfile::tempdir;

#[test]
fn timings_jsonl_rotates_at_5mb() {
    let cache = tempdir().expect("tempdir");
    let path = cache.path().join("timings.jsonl");
    // Pre-fill with exactly 5MB to force rotation on the next append (the
    // append will push size past `MAX_BYTES`).
    fs::write(&path, vec![b'x'; 5 * 1024 * 1024]).expect("pre-fill timings.jsonl");

    // Append one line. Rotation must trigger BEFORE the append.
    let line = "{\"ts\":\"now\",\"duration_ms\":1}\n";
    holt_supervisor::timings::append_timings(cache.path(), line).expect("append must succeed");

    // <file>.1 should exist with the original 5MB; <file> should contain only
    // the newly-appended line.
    let rotated = cache.path().join("timings.jsonl.1");
    assert!(rotated.exists(), "expected rotated file at {rotated:?}");
    let rotated_size = fs::metadata(&rotated).expect("metadata .1").len();
    assert_eq!(rotated_size, 5 * 1024 * 1024);

    let new_size = fs::metadata(&path).expect("metadata current").len();
    assert_eq!(
        new_size as usize,
        line.len(),
        "current file should contain only the new line, got {new_size} bytes"
    );
}
