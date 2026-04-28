//! Pure heartbeat field derivation (D-05).
//!
//! `assemble_heartbeat` takes the parsed CC stdin envelope plus environment
//! context and returns a fully populated `Heartbeat`. It performs NO I/O —
//! the writer in `handle.rs` calls `holt_schemas::atomic_write` separately.
//! This separation lets us unit-test field derivation (especially `cwd_label`,
//! D-08) without touching disk.

use std::path::Path;

use holt_schemas::Heartbeat;

use crate::event::HookEvent;
use crate::stdin::HookStdin;

/// Environment context populated by the CLI dispatcher (plan 02-02). The
/// `writer_version` field is plumbed from the holt-cli binary's
/// `env!("CARGO_PKG_VERSION")` per D-11; passing `&'static str` keeps it
/// allocation-free.
#[derive(Debug, Clone)]
pub struct Env {
    /// holt-binary version stamped into the heartbeat's `writer_version` field.
    /// In production: `env!("CARGO_PKG_VERSION")` from `holt-cli/src/main.rs`.
    /// In tests: `"test-0.0.0"`.
    pub writer_version: &'static str,
    /// `std::process::id()` at hook fire time.
    pub pid: u32,
    /// ISO 8601 timestamp captured at the start of `handle_event` — used for
    /// both `Heartbeat.started` and `Heartbeat.updated` (PreToolUse fires
    /// frequently; `updated` always advances per criterion #2).
    pub now_iso: String,
}

/// D-05 pure assembly: build a `Heartbeat` from a parsed CC stdin envelope and
/// `Env`. NO disk I/O. Field policies:
///
/// - `current_tool` (D-09): `Some(stdin.tool_name)` on `PreToolUse`; `None` on
///   every other event. `tool_name` is already an `Option<String>` from
///   defensive parse; if absent on PreToolUse we still emit `None`.
/// - `blocked_on` (D-10): always `None` at v0.1 (reserved for v1.0).
/// - `last_assistant_at` (D-10): from `stdin.last_assistant_at`.
/// - `model_display` (D-10): from `stdin.model.display_name`.
/// - `mode`, `context_pct_real`, `burn_rate_usd_per_min`: deferred to v1.0;
///   set to `None` per CONTEXT.md "Deferred Ideas" — Rich heartbeat fields.
/// - `cwd_label` (D-08): see `derive_cwd_label`.
pub fn assemble_heartbeat(event: HookEvent, stdin: &HookStdin, env: &Env) -> Heartbeat {
    let current_tool = match event {
        HookEvent::PreToolUse => stdin.tool_name.clone(),
        HookEvent::PostToolUse
        | HookEvent::Stop
        | HookEvent::Notification
        | HookEvent::SessionStart => None,
    };

    let cwd_label = derive_cwd_label(&stdin.workspace.git_worktree, &stdin.cwd);

    Heartbeat::new(
        stdin.session_id.clone(),
        env.pid,
        env.now_iso.clone(),
        env.now_iso.clone(),
        stdin.cwd.clone(),
        cwd_label,
        None, // mode — deferred to v1.0
        current_tool,
        None, // blocked_on — D-10 always None at v0.1
        None, // context_pct_real — deferred to v1.0
        None, // burn_rate_usd_per_min — deferred to v1.0
        stdin.last_assistant_at.clone(),
        stdin.model.display_name.clone(),
        env.writer_version.to_string(),
    )
}

/// D-08 cwd_label derivation:
/// 1. If `workspace.git_worktree` is `Some(non_empty)`, use it verbatim.
/// 2. Else use the basename of `cwd` if it has one (handles `/` edge case).
/// 3. Else fall back to `cwd` itself (covers empty-string / current-dir cases).
///
/// Note on D-08 simplification: the CONTEXT.md branch 2 mentions a `git
/// rev-parse` heuristic for `<repo>/<branch>` derivation when cwd contains
/// `.git`. The CONTEXT itself qualifies it as "best-effort; fallback to cwd
/// basename only" — and shelling out to `git` from the render path violates
/// the sub-20ms budget (D-15). We implement only the basename fallback here.
/// The git-rev-parse branch becomes a v1.0 enhancement when the orchestrator
/// can call it on a non-render-path background thread.
///
/// Exposed as `pub` for unit testing — the integration test in
/// `tests/assemble_field_policy.rs` exercises both branches via paired fixtures.
pub fn derive_cwd_label(git_worktree: &Option<String>, cwd: &str) -> String {
    if let Some(label) = git_worktree {
        if !label.is_empty() {
            return label.clone();
        }
    }
    if let Some(base) = Path::new(cwd).file_name().and_then(|s| s.to_str()) {
        if !base.is_empty() {
            return base.to_string();
        }
    }
    cwd.to_string()
}
