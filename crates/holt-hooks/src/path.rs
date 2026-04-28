//! Heartbeat writer path resolution (D-06 three-tier fallback chain; D-07
//! re-evaluation per fire).
//!
//! Tier order:
//!   1. `$XDG_RUNTIME_DIR/holt/sessions/<sid>.json` (Linux convention).
//!   2. `$TMPDIR/holt-$UID/sessions/<sid>.json` (macOS convention; `$TMPDIR`
//!      defaults to `/var/folders/...` on macOS, so this tier nearly always
//!      resolves on Mac).
//!   3. `<default_cache_root>/sessions/<sid>.json` (universal fallback —
//!      `~/.cache/holt/sessions/<sid>.json` in normal use, `$TMPDIR/holt-<pid>/
//!      sessions/<sid>.json` if HOME is also unset per WR-01 in
//!      `holt-supervisor::paths::default_cache_root`).
//!
//! "First writable wins" — we attempt to create the parent directory at each
//! tier; the first tier where `create_dir_all` succeeds (or where the dir
//! already exists with write perms) is the chosen path. If ALL THREE tiers
//! fail to provide a writable parent dir, `resolve_writer_path` returns
//! `None`; the caller (`handle.rs`) routes that to `HookOutcome::Unwritable`.
//!
//! D-07: each call re-evaluates the chain from scratch. We do NOT cache. A
//! freshly mounted `$XDG_RUNTIME_DIR` on a system that didn't have one when
//! holt started (e.g., systemd-logind logged in mid-session) gets picked up
//! on the very next hook fire.
//!
//! Filename shape: `<sid>.json` where `<sid>` is `stdin.session_id` if
//! non-empty, else a deterministic 16-char lowercase hex hash of
//! `(stdin.cwd, stdin.transcript_path)` using `std::hash::DefaultHasher`. The
//! hash policy ensures two different sessions without a stdin-provided
//! `session_id` never clobber each other's heartbeat. We accept that
//! `DefaultHasher` is not cryptographic — collision risk for a per-machine
//! session count of <10 sessions is negligible.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use holt_supervisor::paths::default_cache_root;

use crate::stdin::HookStdin;

/// Resolved writer path along with which tier won — exposed for breach
/// records and the one-line stderr warning when a fallback fires (criterion
/// #4 explicitly requires the warning name the fallback path).
#[derive(Debug, Clone)]
pub struct ResolvedPath {
    pub path: PathBuf,
    pub tier: ResolvedTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTier {
    /// Tier 1: `$XDG_RUNTIME_DIR/holt/sessions/`.
    XdgRuntimeDir,
    /// Tier 2: `$TMPDIR/holt-$UID/sessions/`.
    TmpDir,
    /// Tier 3: `<default_cache_root>/sessions/` (`~/.cache/holt/sessions/`).
    Cache,
}

impl ResolvedTier {
    pub fn is_fallback(self) -> bool {
        !matches!(self, ResolvedTier::XdgRuntimeDir)
    }
    pub fn as_str(self) -> &'static str {
        match self {
            ResolvedTier::XdgRuntimeDir => "xdg_runtime_dir",
            ResolvedTier::TmpDir => "tmpdir",
            ResolvedTier::Cache => "cache",
        }
    }
}

/// Resolve the heartbeat writer path per D-06. Returns `None` if all three
/// tiers fail to provide a writable parent directory.
pub fn resolve_writer_path(stdin: &HookStdin) -> Option<ResolvedPath> {
    let sid = session_id_or_hash(stdin);
    let filename = format!("{sid}.json");

    if let Some(parent) = tier_xdg_runtime_dir() {
        if std::fs::create_dir_all(&parent).is_ok() {
            return Some(ResolvedPath {
                path: parent.join(&filename),
                tier: ResolvedTier::XdgRuntimeDir,
            });
        }
    }

    if let Some(parent) = tier_tmpdir() {
        if std::fs::create_dir_all(&parent).is_ok() {
            return Some(ResolvedPath {
                path: parent.join(&filename),
                tier: ResolvedTier::TmpDir,
            });
        }
    }

    let cache_parent = default_cache_root().join("sessions");
    if std::fs::create_dir_all(&cache_parent).is_ok() {
        return Some(ResolvedPath {
            path: cache_parent.join(&filename),
            tier: ResolvedTier::Cache,
        });
    }

    None
}

fn tier_xdg_runtime_dir() -> Option<PathBuf> {
    match std::env::var("XDG_RUNTIME_DIR") {
        Ok(v) if !v.is_empty() => Some(Path::new(&v).join("holt").join("sessions")),
        _ => None,
    }
}

fn tier_tmpdir() -> Option<PathBuf> {
    match std::env::var("TMPDIR") {
        Ok(v) if !v.is_empty() => {
            let uid = current_uid_string();
            Some(Path::new(&v).join(format!("holt-{uid}")).join("sessions"))
        }
        _ => None,
    }
}

#[cfg(unix)]
fn current_uid_string() -> String {
    // nix::unistd::Uid::current() is a safe wrapper around getuid(2). We do
    // NOT use unsafe directly — `#![forbid(unsafe_code)]` at the crate root.
    nix::unistd::Uid::current().as_raw().to_string()
}

#[cfg(not(unix))]
fn current_uid_string() -> String {
    // Windows / non-Unix: $TMPDIR is rare; the tier-3 fallback handles us.
    // We still build a stable path component so the directory shape doesn't
    // change between platforms in tests that mock $TMPDIR.
    "0".to_string()
}

/// Filename shape (per CONTEXT.md "Decisions to lock into plan tasks"):
/// use `stdin.session_id` if present and non-empty; else a deterministic
/// 16-char lowercase hex hash of `(stdin.cwd, stdin.transcript_path)` using
/// `std::hash::DefaultHasher` (no new dep). Used so two different sessions
/// without a stdin-provided session_id never clobber each other's heartbeat.
pub fn session_id_or_hash(stdin: &HookStdin) -> String {
    if !stdin.session_id.is_empty() {
        return stdin.session_id.clone();
    }
    let mut h = DefaultHasher::new();
    stdin.cwd.hash(&mut h);
    stdin.transcript_path.hash(&mut h);
    format!("{:016x}", h.finish())
}
