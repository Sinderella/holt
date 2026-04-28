//! Cache path resolution.
//!
//! Defaults to `$XDG_CACHE_HOME/holt` if set, else `$HOME/.cache/holt`. Tests
//! inject a tempdir into `SupervisorOptions::cache_root` and never touch these
//! defaults.

use std::path::{Path, PathBuf};

/// `~/.cache/holt/` (or `$XDG_CACHE_HOME/holt` if set + non-empty).
///
/// WR-01: when both `XDG_CACHE_HOME` and `HOME` are unset/empty (rare but real
/// — minimal Docker images, FreeBSD jails, some sandbox profiles), we fall
/// back to a per-uid subdirectory of the system temp dir rather than the
/// current working directory. Falling back to "." would silently pollute the
/// repo a CC session was started in with `holt/breaches.log` and friends.
/// Windows path of last resort honors `USERPROFILE` before the temp fallback.
pub fn default_cache_root() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("holt");
        }
    }
    let home = std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
        .filter(|h| !h.is_empty());
    match home {
        Some(h) => PathBuf::from(h).join(".cache").join("holt"),
        None => std::env::temp_dir().join(format!("holt-{}", std::process::id())),
    }
}

/// `<cache_root>/lkg/<session_id>.json` — D-10 file layout.
pub fn lkg_path(cache_root: &Path, session_id: &str) -> PathBuf {
    cache_root.join("lkg").join(format!("{session_id}.json"))
}

/// `<cache_root>/timings.jsonl` — D-12 telemetry stream.
pub fn timings_path(cache_root: &Path) -> PathBuf {
    cache_root.join("timings.jsonl")
}

/// `<cache_root>/breaches.log` — D-13 breach record stream (JSONL).
pub fn breaches_path(cache_root: &Path) -> PathBuf {
    cache_root.join("breaches.log")
}
