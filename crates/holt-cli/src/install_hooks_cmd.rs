//! D-16 dispatcher for the `holt install-hooks` subcommand.
//!
//! Pipeline:
//!   1. Resolve settings.json path: `$HOME/.claude/settings.json` (HOME via
//!      `std::env::var`).
//!   2. If `--print`: build the snippet from [`HOLT_HOOK_ENTRIES`] and emit
//!      to stdout. Exit 0.
//!   3. If `--dry-run`: read settings.json (no lock), call [`merge_settings`],
//!      emit unified diff vs the input. Exit 0. Do NOT write `.holt.bak`.
//!   4. Default: acquire fs2 lock, read settings.json bytes, call
//!      [`merge_settings`], call [`commit`] (writes `.holt.bak` then
//!      atomic_writes the merged bytes). Exit 0.
//!
//! Errors print to stderr and return non-zero exit codes. Note this is
//! distinct from `holt hook` (Phase 2) which always exits 0 — `holt
//! install-hooks` is user-invoked, not a CC hook, so non-zero exits are
//! appropriate here.
//!
//! Exit codes:
//!   0 = success (or idempotent no-op, or --dry-run / --print success)
//!   1 = merge or commit failed (settings.json was NOT modified — atomic invariant)
//!   2 = lock acquisition timed out (D-04 user-facing message printed to stderr)
//!   3 = io / parse error before any mutation (or argument validation error)

use std::io::Write;
use std::path::PathBuf;

use crate::install_hooks::diff::unified_diff;
use crate::install_hooks::print::pretty_snippet;
use crate::install_hooks::{
    CommitError, HOLT_HOOK_ENTRIES, LockError, MergeError, acquire_settings_lock, commit,
    merge_settings,
};

pub fn run(dry_run: bool, print: bool) -> i32 {
    // clap's `conflicts_with` should already prevent both being true, but
    // defend at the dispatcher boundary in case the caller is ever changed.
    if dry_run && print {
        eprintln!("holt install-hooks: --dry-run and --print are mutually exclusive");
        return 3;
    }

    let path = match settings_path() {
        Some(p) => p,
        None => {
            eprintln!("holt install-hooks: $HOME is not set; cannot locate settings.json");
            return 3;
        }
    };

    if print {
        let snippet = pretty_snippet(HOLT_HOOK_ENTRIES);
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(snippet.as_bytes());
        return 0;
    }

    if dry_run {
        let input = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                eprintln!("holt install-hooks: read {}: {e}", path.display());
                return 3;
            }
        };
        let merged = match merge_settings(&input) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("holt install-hooks: merge: {e}");
                return 1;
            }
        };
        let diff = unified_diff(&input, &merged.bytes, "a/settings.json", "b/settings.json");
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(diff.as_bytes());
        return 0;
    }

    // Default: lock + read + merge + commit.
    let lock_handle = match acquire_settings_lock(&path) {
        Ok(f) => f,
        Err(LockError::Timeout { path, budget_ms }) => {
            eprintln!(
                "holt install-hooks: could not acquire exclusive lock on {path} after {budget_ms}ms: another holt install-hooks is running (or settings.json is locked by another editor)"
            );
            return 2;
        }
        Err(LockError::Io { path, source }) => {
            eprintln!("holt install-hooks: open {path}: {source}");
            return 3;
        }
    };

    // `acquire_settings_lock` opened settings.json with create(true), so the
    // file is guaranteed to exist on disk. Re-read its bytes (do NOT use the
    // locked File handle's read because the rename in commit() will swap the
    // underlying inode — keep `lock_handle` alive only for serialisation).
    let input = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("holt install-hooks: read {}: {e}", path.display());
            drop(lock_handle);
            return 3;
        }
    };

    let merged = match merge_settings(&input) {
        Ok(m) => m,
        Err(MergeError::Parse(msg)) => {
            eprintln!(
                "holt install-hooks: settings.json is not valid JSONC: {msg}\n\
                 hint: settings.json was not modified."
            );
            drop(lock_handle);
            return 1;
        }
        Err(MergeError::CstShape) => {
            // WR-02: jsonc-parser API contract violated (logically unreachable
            // on 0.26.x). Route to the same clean stderr-with-hint exit that
            // `MergeError::Parse` uses so the user sees a useful message
            // instead of an `expect()`-panic abort.
            eprintln!(
                "holt install-hooks: internal jsonc-parser CST shape error.\n\
                 hint: settings.json was not modified. Please file a bug at\n\
                 https://github.com/Sinderella/holt/issues with the contents of\n\
                 ~/.claude/settings.json (redact secrets)."
            );
            drop(lock_handle);
            return 1;
        }
        Err(e) => {
            eprintln!("holt install-hooks: merge: {e}");
            drop(lock_handle);
            return 1;
        }
    };

    if !merged.changed {
        // Idempotent re-run: nothing to do. Skip the .holt.bak refresh too —
        // a no-op invocation should not bump backup mtime.
        drop(lock_handle);
        return 0;
    }

    if let Err(e) = commit(&path, input.as_bytes(), &merged) {
        match e {
            CommitError::BackupWrite { path, source } => {
                eprintln!("holt install-hooks: backup write {path}: {source}");
            }
            CommitError::AtomicWrite { path, source } => {
                eprintln!(
                    "holt install-hooks: atomic write {path}: {source}\n\
                     hint: settings.json may have been left in pre-merge state."
                );
            }
        }
        drop(lock_handle);
        return 1;
    }

    drop(lock_handle);
    0
}

fn settings_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".claude").join("settings.json"))
}
