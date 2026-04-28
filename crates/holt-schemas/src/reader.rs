//! Non-panicking heartbeat reader (D-06 / C5 / HOOK-11 contract).
//!
//! Returns Ok(None) for every "session unreadable" outcome:
//!   - file does not exist (ENOENT)
//!   - zero-byte file
//!   - truncated / invalid JSON
//!   - unrecognized schema_version
//!   - missing required field (session_id)
//!
//! Returns Err only for I/O errors that are NOT "file missing".
//! Never panics. Never .unwrap()s. Never .expect()s.

use std::fs;
use std::io;
use std::path::Path;

use crate::error::ReaderError;
use crate::heartbeat::Heartbeat;

pub fn read_heartbeat(path: &Path) -> Result<Option<Heartbeat>, ReaderError> {
    // Step 1: read file. ENOENT → Ok(None) (the "missing" case, not an error).
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ReaderError::Io(e)),
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
