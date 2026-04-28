//! HOOK-11 / C5 contract test: read_heartbeat returns Ok(None) for every
//! "session unreadable" outcome — never Err, never panics.
//!
//! Source: CONTEXT.md D-06; ROADMAP success criterion #4.
//! Seven cases — six Ok(None), one Ok(Some).

use holt_schemas::{Heartbeat, read_heartbeat};
use std::fs;
use tempfile::tempdir;

#[test]
fn returns_ok_none_for_missing_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");
    let result = read_heartbeat(&path);
    assert!(
        matches!(result, Ok(None)),
        "expected Ok(None) for missing file, got {result:?}"
    );
}

#[test]
fn returns_ok_none_for_zero_byte_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.json");
    fs::write(&path, b"").unwrap();
    let result = read_heartbeat(&path);
    assert!(
        matches!(result, Ok(None)),
        "expected Ok(None) for zero-byte file, got {result:?}"
    );
}

#[test]
fn returns_ok_none_for_truncated_json() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("trunc.json");
    // Missing closing brace: serde will return EOF-while-parsing.
    fs::write(&path, br#"{"schema_version":1,"session_id":"abc","#).unwrap();
    let result = read_heartbeat(&path);
    assert!(
        matches!(result, Ok(None)),
        "expected Ok(None) for truncated json, got {result:?}"
    );
}

#[test]
fn returns_ok_none_for_unrecognized_schema_version() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("future.json");
    fs::write(&path, br#"{"schema_version":99,"session_id":"abc"}"#).unwrap();
    let result = read_heartbeat(&path);
    assert!(
        matches!(result, Ok(None)),
        "expected Ok(None) for schema_version=99, got {result:?}"
    );
}

#[test]
fn returns_ok_none_for_missing_required_fields() {
    // session_id is required; everything else has #[serde(default)] per D-05.
    let dir = tempdir().unwrap();
    let path = dir.path().join("partial.json");
    fs::write(&path, br#"{"schema_version":1}"#).unwrap(); // no session_id
    let result = read_heartbeat(&path);
    assert!(
        matches!(result, Ok(None)),
        "expected Ok(None) for missing session_id, got {result:?}"
    );
}

#[test]
fn returns_ok_some_for_valid_heartbeat() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("valid.json");
    fs::write(&path, br#"{"schema_version":1,"session_id":"abc"}"#).unwrap();
    let hb: Heartbeat = read_heartbeat(&path)
        .expect("io error in test should not happen")
        .expect("expected Some(Heartbeat) for valid input");
    assert_eq!(hb.session_id, "abc");
    assert_eq!(hb.schema_version, 1);
}

/// Sanity: parsing arbitrary garbage bytes never panics.
/// PITFALLS H5 motivates this — CC may ship a payload shape we never imagined.
#[test]
fn does_not_panic_on_arbitrary_bytes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("garbage.json");
    fs::write(&path, [0xff_u8; 4096]).unwrap();
    let result = read_heartbeat(&path);
    assert!(matches!(result, Ok(None)));
}
