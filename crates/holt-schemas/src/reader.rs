//! Non-panicking heartbeat reader (D-06 / C5 / HOOK-11 contract).
//!
//! Returns Ok(None) for every "session unreadable" outcome:
//!   - file does not exist (ENOENT)
//!   - permission denied (mode 0000 from a stale prior install) — WR-02
//!   - target path is a directory — WR-02
//!   - zero-byte file
//!   - truncated / invalid JSON
//!   - unrecognized schema_version
//!   - missing required field (session_id)
//!
//! Returns Err only for I/O errors that are NOT "session unreadable" by the
//! render path's standard. Never panics. Never .unwrap()s. Never .expect()s.

use std::fs;
use std::io;
use std::path::Path;

use crate::error::ReaderError;
use crate::heartbeat::Heartbeat;

pub fn read_heartbeat(path: &Path) -> Result<Option<Heartbeat>, ReaderError> {
    // Step 1: read file. The full "session unreadable from the render path's
    // perspective" set short-circuits to Ok(None):
    //   - NotFound: file missing (the canonical case).
    //   - PermissionDenied: stale install left a 0000-mode file; render path
    //     can't read it so it's effectively missing.
    //   - IsADirectory: someone created a directory at the heartbeat path;
    //     render path can't read it so it's effectively missing.
    // (IsADirectory is a stable ErrorKind since Rust 1.83; MSRV is 1.87.)
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound
            | io::ErrorKind::PermissionDenied
            | io::ErrorKind::IsADirectory => return Ok(None),
            _ => return Err(ReaderError::Io(e)),
        },
    };

    // Step 2: zero-byte file → Ok(None).
    if bytes.is_empty() {
        return Ok(None);
    }

    // Step 3: parse. Any serde error → Ok(None) (truncated, invalid utf-8, missing
    // required field — all map to "session unreadable"; the render path must not fail).
    let hb: Heartbeat = match serde_json::from_slice(&bytes) {
        Ok(h) => h,
        Err(_) => return Ok(None),
    };

    // Step 4: schema_version check. Unrecognized → Ok(None) (forward-compat per H8/H11).
    if hb.schema_version != 1 {
        return Ok(None);
    }

    Ok(Some(hb))
}
