//! `holt install-hooks` library — JSONC-aware merge of holt's hook entries
//! into `~/.claude/settings.json`.
//!
//! Phase 3 hard constraints:
//!   - C3: fs2 exclusive lock + fsync-before-rename + PID-suffix tmp + `.holt.bak` backup.
//!   - C4: `jsonc-parser` + `fs2` deps live ONLY in this crate (verified by
//!     `tests/cli_dep_boundary.rs` at workspace root in plan 03-02).
//!
//! Public surface consumed by the (forthcoming, plan 03-02)
//! `crates/holt-cli/src/install_hooks_cmd.rs` CLI dispatcher:
//!   - [`merge_settings`] — pure JSONC CST round-trip.
//!   - [`commit`] — `.holt.bak` backup + atomic write of merged bytes.
//!   - [`acquire_settings_lock`] — fs2 exclusive lock with 200ms try-loop.
//!   - [`HOLT_HOOK_ENTRIES`] / [`HOLT_HOOK_DETECTION_SUBSTR`] —
//!     single source of truth for the 5 canonical hook entries (D-09 / D-10).
//!
//! Single-backup policy (D-07): `~/.claude/settings.json.holt.bak` is overwritten
//! on every successful run. NOT a versioned backup chain — long-lived user files
//! shouldn't accumulate cruft. NOT named `.bak` (vim's namespace per
//! research/SUMMARY.md §3 C3).

// Wired into the binary in plan 03-02 — until then, the public-but-unused
// re-exports below are intentional (they form the stable surface for the
// CLI dispatcher). `dead_code` covers everything except the re-exports
// themselves; `unused_imports` covers those.
#![allow(dead_code, unused_imports)]

mod entries;
mod lock;
mod merge;

pub use entries::{HOLT_HOOK_DETECTION_SUBSTR, HOLT_HOOK_ENTRIES, HoltHookEntry};
pub use lock::{LockError, acquire_settings_lock};
pub use merge::{MergeError, MergeOutput, merge_settings};

use std::path::Path;

/// Errors at the commit stage (after merge succeeded, before the .bak + atomic_write).
#[derive(Debug, thiserror::Error)]
pub enum CommitError {
    #[error("failed to write .holt.bak backup at {path}: {source}")]
    BackupWrite {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to atomically write merged settings.json at {path}: {source}")]
    AtomicWrite {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Backup pre-merge bytes to `<settings_path>.holt.bak`, then atomically write merged bytes
/// to `<settings_path>` via [`holt_schemas::atomic_write`] (D-06 reuses Phase 1's PID-suffix
/// tmp + fsync-before-rename helper unchanged).
///
/// Order is load-bearing: the `.holt.bak` is written BEFORE the merged file so that a crash
/// between the two writes leaves the user with a recoverable backup.
pub fn commit(
    settings_path: &Path,
    pre_merge: &[u8],
    merged: &MergeOutput,
) -> Result<(), CommitError> {
    let bak_path = {
        let mut s = settings_path.as_os_str().to_owned();
        s.push(".holt.bak");
        std::path::PathBuf::from(s)
    };
    holt_schemas::atomic_write(&bak_path, pre_merge).map_err(|e| CommitError::BackupWrite {
        path: bak_path.display().to_string(),
        source: e,
    })?;
    holt_schemas::atomic_write(settings_path, merged.bytes.as_bytes()).map_err(|e| {
        CommitError::AtomicWrite {
            path: settings_path.display().to_string(),
            source: e,
        }
    })?;
    Ok(())
}
