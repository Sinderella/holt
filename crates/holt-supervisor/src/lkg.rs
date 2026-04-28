//! Last-known-good cache reader/writer (D-10).
//!
//! On every Ok outcome the supervisor stamps a fresh `LkgEntry` to
//! `<cache_root>/lkg/<session_id>.json` via [`holt_schemas::atomic_write`] —
//! the same fsync-before-rename helper Phase 2 hooks will use for heartbeat
//! durability. The render path reads only the `stdout` field on cache hit;
//! everything else is observability for future `holt doctor` (v0.5).

use std::path::Path;

use holt_schemas::{LkgEntry, atomic_write};

use crate::paths::lkg_path;

/// Persist `entry` atomically to `<cache_root>/lkg/<session_id>.json`.
///
/// Creates the `lkg/` subdirectory if missing. Returns the underlying I/O
/// error on failure; supervisor callers ignore the result (LKG write must
/// never fail the render path).
pub fn write_lkg(cache_root: &Path, session_id: &str, entry: &LkgEntry) -> std::io::Result<()> {
    let path = lkg_path(cache_root, session_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec(entry)
        .map_err(|e| std::io::Error::other(format!("lkg serialize: {e}")))?;
    atomic_write(&path, &bytes)
}

/// Best-effort read. Returns `None` for: missing file, empty file, malformed
/// JSON, or unrecognized `schema_version`. Never panics, never bubbles serde
/// errors — same C5 posture as `holt_schemas::read_heartbeat`.
pub fn read_lkg(cache_root: &Path, session_id: &str) -> Option<LkgEntry> {
    let path = lkg_path(cache_root, session_id);
    let bytes = std::fs::read(&path).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let entry: LkgEntry = serde_json::from_slice(&bytes).ok()?;
    if entry.schema_version != LkgEntry::SCHEMA_VERSION {
        return None;
    }
    Some(entry)
}
