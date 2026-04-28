//! D-10 / CORE-03: LKG cache round-trips correctly via
//! `holt_schemas::atomic_write` + `LkgEntry::new` constructor.
//!
//! The supervisor stamps the cache only on `exit == 0`. We verify the file
//! lands at the expected path and deserializes back into an `LkgEntry` that
//! matches the wrapped command's stdout.

#![cfg(unix)]

use std::time::Duration;

use holt_supervisor::{SupervisorOptions, wrap_and_run};
use tempfile::tempdir;

#[test]
fn ok_outcome_writes_readable_lkg_entry() {
    let cache = tempdir().expect("tempdir");
    let opts = SupervisorOptions {
        timeout: Duration::from_secs(5),
        session_id: "test-lkg".into(),
        stdin_bytes: Vec::new(),
        cache_root: cache.path().to_path_buf(),
    };

    let _ = wrap_and_run("bash", &["-c", "echo cached-line"], opts);

    // LKG file should exist at <cache>/lkg/test-lkg.json.
    let lkg_path = cache.path().join("lkg").join("test-lkg.json");
    assert!(lkg_path.exists(), "expected LKG at {lkg_path:?}");

    let bytes = std::fs::read(&lkg_path).expect("read LKG bytes");
    let entry: holt_schemas::LkgEntry =
        serde_json::from_slice(&bytes).expect("LKG must deserialize as LkgEntry");

    assert_eq!(entry.schema_version, holt_schemas::LkgEntry::SCHEMA_VERSION);
    assert_eq!(entry.stdout, "cached-line\n");
    assert_eq!(entry.exit_code, 0);
    assert!(!entry.captured_at.is_empty(), "captured_at should be set");
    assert!(
        entry.duration_ms < 5_000,
        "duration_ms {} should be well under timeout",
        entry.duration_ms
    );
}
