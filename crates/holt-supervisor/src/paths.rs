//! Cache path resolution.
//!
//! Defaults to `$XDG_CACHE_HOME/holt` if set, else `$HOME/.cache/holt`. Tests
//! inject a tempdir into `SupervisorOptions::cache_root` and never touch these
//! defaults.

use std::path::{Path, PathBuf};

/// `~/.cache/holt/` (or `$XDG_CACHE_HOME/holt` if set + non-empty).
pub fn default_cache_root() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("holt");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".cache").join("holt")
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
