//! D-07 smoke test: atomic_write writes via same-dir tmp, fsyncs, renames atomically.

use holt_schemas::atomic_write;
use std::fs;
use tempfile::tempdir;

#[test]
fn writes_target_file_with_expected_contents() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("payload.json");
    atomic_write(&target, b"{\"hello\":\"world\"}").expect("atomic_write should succeed");
    let read_back = fs::read(&target).unwrap();
    assert_eq!(read_back, b"{\"hello\":\"world\"}");
}

#[test]
fn leaves_no_orphan_tmp_file_on_success() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("payload.json");
    atomic_write(&target, b"x").expect("atomic_write should succeed");

    // After successful rename, the only file in the dir should be the target.
    let entries: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    assert_eq!(
        entries.len(),
        1,
        "found extra files after atomic_write: {entries:?}"
    );
    assert_eq!(entries[0], "payload.json");

    // Specifically — no .holt-tmp.<pid> sibling.
    for name in &entries {
        assert!(
            !name.contains(".holt-tmp."),
            "orphan tmp file left behind: {name}"
        );
    }
}

#[test]
fn overwrites_existing_target() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("payload.json");
    fs::write(&target, b"old contents").unwrap();
    atomic_write(&target, b"new contents").expect("atomic_write should succeed");
    let read_back = fs::read(&target).unwrap();
    assert_eq!(read_back, b"new contents");
}

#[cfg(unix)]
#[test]
fn unix_perms_are_0600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let target = dir.path().join("private.json");
    atomic_write(&target, b"secret").expect("atomic_write should succeed");
    let meta = fs::metadata(&target).unwrap();
    let mode = meta.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "expected 0600 perms, got {mode:o}");
}

#[test]
fn errors_on_invalid_target_no_parent() {
    // Empty path has no parent — should return InvalidInput.
    let result = atomic_write(std::path::Path::new(""), b"x");
    assert!(result.is_err(), "expected error for empty path");
}
