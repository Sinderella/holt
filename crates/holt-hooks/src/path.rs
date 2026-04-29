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
///
/// Each tier is probed for actual writability (CR-02): `create_dir_all`
/// returns `Ok(())` when the directory already exists regardless of the
/// caller's write permission, so a tier where the parent dir exists with the
/// wrong owner / 0o700 / read-only-mount would silently win and the next
/// `atomic_write` would fail with EACCES — burning a tier that the next-tier
/// fallback would have served. We probe by creating a same-dir file with
/// `create_new`; if that fails, the tier is treated as not-writable and we
/// move on. There IS a TOCTOU window between the probe and the real write,
/// but it's strictly an improvement over the prior code which had the same
/// window AND failed to retry on the false-positive path.
pub fn resolve_writer_path(stdin: &HookStdin) -> Option<ResolvedPath> {
    let sid = session_id_or_hash(stdin);
    let filename = format!("{sid}.json");

    if let Some(parent) = tier_xdg_runtime_dir() {
        if dir_is_writable(&parent) {
            return Some(ResolvedPath {
                path: parent.join(&filename),
                tier: ResolvedTier::XdgRuntimeDir,
            });
        }
    }

    if let Some(parent) = tier_tmpdir() {
        if dir_is_writable(&parent) {
            return Some(ResolvedPath {
                path: parent.join(&filename),
                tier: ResolvedTier::TmpDir,
            });
        }
    }

    let cache_parent = default_cache_root().join("sessions");
    if dir_is_writable(&cache_parent) {
        return Some(ResolvedPath {
            path: cache_parent.join(&filename),
            tier: ResolvedTier::Cache,
        });
    }

    None
}

/// CR-02: probe whether `parent` is actually writable by the current process.
///
/// `std::fs::create_dir_all(parent)` returns `Ok(())` if the directory
/// already exists, regardless of perms — so a stale `~/.cache/holt/sessions`
/// owned by root with 0o700 (e.g., user once ran `sudo holt`) would let the
/// resolver pick that tier and then EACCES on the real `atomic_write`. We
/// follow the create_dir_all with a `create_new` probe file: if THAT
/// succeeds, the dir is writable; otherwise treat the tier as not-writable
/// so the next-tier fallback gets a chance.
///
/// The probe filename is process-id-suffixed to avoid colliding with another
/// holt instance's probe (or our own retry on a flaky NFS mount). Cleanup is
/// best-effort — leaving a stale probe file behind is acceptable; we'll
/// `create_new` a new one with a different PID next time.
fn dir_is_writable(parent: &Path) -> bool {
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    let probe = parent.join(format!(".holt-probe.{}", std::process::id()));
    let writable = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .is_ok();
    let _ = std::fs::remove_file(&probe); // best-effort cleanup
    writable
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
