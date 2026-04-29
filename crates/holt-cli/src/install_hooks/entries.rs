//! D-09: exact bytes of holt's 5 canonical hook-entry blocks.
//!
//! Consumed by `merge.rs` for the in-place CST insertion AND (in plan 03-02)
//! by `--print` for paste-able snippet output. Centralised here so the two
//! consumers cannot drift.

/// One entry per CC hook event in the v0.1 five-event subscription
/// (`docs/02-scope.md`). PreCompact is reserved for v1.0.
pub struct HoltHookEntry {
    /// PascalCase event name; matches CC's event names + the
    /// `holt-cli/src/cli.rs` `HookEventArg` ValueEnum variants.
    pub event: &'static str,
    /// Exactly `holt hook <Event>` (D-09 / D-10). The matcher is always `"*"`
    /// and the entry shape is fixed; the only per-event datum is the command
    /// string, so we keep the struct minimal.
    pub command: &'static str,
}

pub const HOLT_HOOK_ENTRIES: &[HoltHookEntry] = &[
    HoltHookEntry {
        event: "PreToolUse",
        command: "holt hook PreToolUse",
    },
    HoltHookEntry {
        event: "PostToolUse",
        command: "holt hook PostToolUse",
    },
    HoltHookEntry {
        event: "Stop",
        command: "holt hook Stop",
    },
    HoltHookEntry {
        event: "Notification",
        command: "holt hook Notification",
    },
    HoltHookEntry {
        event: "SessionStart",
        command: "holt hook SessionStart",
    },
];

/// Detection sentinel for D-10 substring policy. `merge.rs` treats any
/// existing entry whose `hooks[].command` contains this substring as
/// "holt's" — replaceable. User-defined entries (any other `command`
/// value) are preserved verbatim.
pub const HOLT_HOOK_DETECTION_SUBSTR: &str = "holt hook ";
