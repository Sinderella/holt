//! `HookEvent` — the five-event subscription set locked at v0.1
//! (`docs/02-scope.md`; CONTEXT.md D-03). PreCompact (CC v2.1.105+) is
//! v1.0 territory and intentionally NOT a variant here.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    Stop,
    Notification,
    SessionStart,
}

impl HookEvent {
    /// Stable string used by clap's value-enum derive in plan 02-02 and by
    /// debugging output. Matches CC's `hook_event_name` field verbatim.
    pub fn as_str(self) -> &'static str {
        match self {
            HookEvent::PreToolUse => "PreToolUse",
            HookEvent::PostToolUse => "PostToolUse",
            HookEvent::Stop => "Stop",
            HookEvent::Notification => "Notification",
            HookEvent::SessionStart => "SessionStart",
        }
    }
}

impl fmt::Display for HookEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
