//! Defensive CC stdin envelope parse (D-04).
//!
//! `HookStdin` mirrors the v2.1.119 CC stdin shape but with `#[serde(default)]`
//! on every field. New fields land silently per PITFALLS.md H5 (the v2.1.119
//! `effort.level: "xhigh"` regression is the named precedent).
//!
//! Parse failure surfaces as `None` from `parse(bytes)`; the caller
//! (`handle_event`) is responsible for routing the original bytes through
//! `holt_supervisor::breaches::append_breach` with `BreachKind::ParseFail`.

use serde::Deserialize;

/// Defensive CC stdin envelope. Every field is `#[serde(default)]` per D-04 —
/// a missing or differently-typed field falls back to its default rather than
/// failing the whole parse. Unknown fields are silently accepted (NO
/// `deny_unknown_fields`) so future CC stdin shape additions never break the
/// hook write path.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct HookStdin {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub transcript_path: String,
    #[serde(default)]
    pub hook_event_name: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub workspace: HookWorkspace,
    #[serde(default)]
    pub model: HookModel,
    #[serde(default)]
    pub last_assistant_at: Option<String>,
    #[serde(default)]
    pub permission_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HookWorkspace {
    /// CC v2.1.98+ first-class field per `docs/05-schemas.md` and
    /// `research/SUMMARY.md` "6 new features" #2. D-08 cwd_label tier-1.
    #[serde(default)]
    pub git_worktree: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HookModel {
    #[serde(default)]
    pub display_name: Option<String>,
}

/// Try to parse `bytes` as `HookStdin`. Returns `Some(parsed)` on success,
/// `None` on any serde error or empty input. Callers route the failure case
/// through `holt_supervisor::breaches::append_breach` with `BreachKind::ParseFail`.
pub fn parse(bytes: &[u8]) -> Option<HookStdin> {
    if bytes.is_empty() {
        return None;
    }
    serde_json::from_slice(bytes).ok()
}
