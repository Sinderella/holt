//! `handle_event` — the single public entry point per CONTEXT.md D-03.
//!
//! Pipeline:
//!   1. Parse CC stdin via `stdin::parse` (D-04). Failure → `HookOutcome::ParseFailed`,
//!      breach to `breaches.log` (best-effort; if breaches.log is also unwritable
//!      we exit 0 silently per D-06 tier 4 spirit), return.
//!   2. Resolve writer path via `path::resolve_writer_path` (D-06/D-07).
//!      All three tiers failed → `HookOutcome::Unwritable`, breach + return.
//!   3. Assemble heartbeat via `assemble::assemble_heartbeat` (D-05).
//!   4. Serialize to JSON, write atomically via `holt_schemas::atomic_write`
//!      (D-12). Set 0o600 perms on the final path (Unix only, defence-in-depth).
//!   5. Emit a one-line stderr warning if a fallback tier was used (D-06 tier 2/3
//!      and ROADMAP criterion #4 require this).
//!
//! Never panics. Never bubbles errors. The caller (`holt-cli`) ignores the
//! returned `HookOutcome` and exits 0 unconditionally.

use std::path::PathBuf;

use crate::assemble::{Env, assemble_heartbeat};
use crate::event::HookEvent;
use crate::path::{ResolvedTier, resolve_writer_path};
use crate::stdin::parse;

use holt_supervisor::breaches::append_breach;
use holt_supervisor::options::BreachKind;
use holt_supervisor::paths::default_cache_root;

/// What `handle_event` did. The CLI dispatcher logs it (when given a debug
/// flag in plan 02-02) but ALWAYS exits 0 from CC's perspective regardless
/// of the variant.
#[derive(Debug)]
pub enum HookOutcome {
    /// Heartbeat written successfully at the canonical XDG path.
    Wrote { path: PathBuf, bytes: usize },
    /// Heartbeat written successfully but a fallback tier was used.
    /// `path` is the file actually written; `reason` documents why tier 1 failed.
    FellBack {
        path: PathBuf,
        reason: FallbackReason,
    },
    /// CC stdin did not parse as JSON. A breach record was attempted; nothing
    /// was written to the heartbeat path.
    ParseFailed,
    /// All three D-06 tiers failed to provide a writable parent dir. A breach
    /// record was attempted (best-effort); nothing was written to a heartbeat
    /// path. The hook still exits 0 from CC's perspective.
    Unwritable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    /// `$XDG_RUNTIME_DIR` was unset / empty / not writable; resolved via `$TMPDIR`.
    XdgUnavailable,
    /// `$XDG_RUNTIME_DIR` AND `$TMPDIR` both unavailable; resolved via `~/.cache/holt/`.
    XdgAndTmpUnavailable,
}

/// Public entry point per D-03. Pipeline documented at module top.
pub fn handle_event(event: HookEvent, stdin_bytes: &[u8], env: &Env) -> HookOutcome {
    // Step 1: parse stdin.
    let parsed = match parse(stdin_bytes) {
        Some(p) => p,
        None => {
            best_effort_breach(BreachKind::ParseFail, stdin_bytes, env);
            return HookOutcome::ParseFailed;
        }
    };

    // Step 2: resolve writer path.
    let resolved = match resolve_writer_path(&parsed) {
        Some(r) => r,
        None => {
            best_effort_breach(BreachKind::Unwritable, stdin_bytes, env);
            return HookOutcome::Unwritable;
        }
    };

    // Step 3: assemble heartbeat (pure).
    let heartbeat = assemble_heartbeat(event, &parsed, env);

    // Step 4: serialize + atomic_write.
    let bytes = match serde_json::to_vec(&heartbeat) {
        Ok(b) => b,
        Err(_) => {
            // Should never happen: Heartbeat is plain Serialize-able. If it
            // does (e.g., non-UTF8 in a String field somehow), treat as a
            // breach but do NOT panic.
            best_effort_breach(BreachKind::Unwritable, stdin_bytes, env);
            return HookOutcome::Unwritable;
        }
    };

    if holt_schemas::atomic_write(&resolved.path, &bytes).is_err() {
        best_effort_breach(BreachKind::Unwritable, stdin_bytes, env);
        return HookOutcome::Unwritable;
    }

    // D-12: set 0o600 perms after rename. atomic_write opens the tmp with
    // mode 0o600 on Unix, so the rename inherits it; this explicit chmod is
    // defence-in-depth and matches the success-criterion language ("0600
    // permissions" verified via stat).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&resolved.path) {
            let mut perm = meta.permissions();
            perm.set_mode(0o600);
            let _ = std::fs::set_permissions(&resolved.path, perm);
        }
    }

    // Step 5: warn on fallback (criterion #4: "emits a single one-line stderr
    // warning naming the fallback path").
    if resolved.tier.is_fallback() {
        eprintln!(
            "holt-hooks: $XDG_RUNTIME_DIR unavailable; wrote heartbeat to {} (tier: {})",
            resolved.path.display(),
            resolved.tier.as_str()
        );
        let reason = match resolved.tier {
            ResolvedTier::TmpDir => FallbackReason::XdgUnavailable,
            ResolvedTier::Cache => FallbackReason::XdgAndTmpUnavailable,
            // CR-01: Defensive fallthrough. `is_fallback()` filters
            // `XdgRuntimeDir` today, but a future maintainer adding a new
            // `ResolvedTier` variant must update both that helper AND this
            // match. Per the module-level "Never panics" contract on lib.rs,
            // we degrade to the broadest known fallback reason rather than
            // panic on the render path. A panic here would propagate to the
            // CLI dispatcher AFTER a successful heartbeat write, turning a
            // cosmetic stderr-warning step into a CC-visible non-zero exit.
            ResolvedTier::XdgRuntimeDir => FallbackReason::XdgUnavailable,
        };
        return HookOutcome::FellBack {
            path: resolved.path,
            reason,
        };
    }

    HookOutcome::Wrote {
        path: resolved.path,
        bytes: bytes.len(),
    }
}

/// Try to write a breach record to `<default_cache_root>/breaches.log`. If
/// THAT path is also unwritable, swallow silently (D-06 tier 4 spirit: CC
/// must never see a hook error).
fn best_effort_breach(kind: BreachKind, stdin_bytes: &[u8], env: &Env) {
    let cache_root = default_cache_root();
    let _ = append_breach(
        &cache_root,
        kind,
        stdin_bytes,
        &[],
        None,
        env.writer_version,
    );
}
