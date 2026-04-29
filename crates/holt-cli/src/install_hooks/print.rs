//! D-12 pretty-printed snippet of just the 5 holt hook entries.
//!
//! 2-space indent. No comments — this is the paste-ready snippet a user
//! drops into their existing `hooks` block. Hand-rolled so we don't rely on
//! serde_json's pretty-printer's whitespace decisions (which depend on the
//! `preserve_order` feature interaction with map-key ordering).

use super::entries::HoltHookEntry;

pub fn pretty_snippet(entries: &[HoltHookEntry]) -> String {
    let mut out = String::new();
    out.push_str("\"hooks\": {\n");
    for (i, e) in entries.iter().enumerate() {
        let comma = if i + 1 < entries.len() { "," } else { "" };
        out.push_str(&format!("  \"{}\": [\n", e.event));
        out.push_str("    {\n");
        out.push_str("      \"matcher\": \"*\",\n");
        out.push_str("      \"hooks\": [\n");
        out.push_str("        {\n");
        out.push_str("          \"type\": \"command\",\n");
        out.push_str(&format!("          \"command\": \"{}\"\n", e.command));
        out.push_str("        }\n");
        out.push_str("      ]\n");
        out.push_str("    }\n");
        out.push_str(&format!("  ]{comma}\n"));
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install_hooks::entries::HOLT_HOOK_ENTRIES;

    #[test]
    fn snippet_contains_all_5_events() {
        let s = pretty_snippet(HOLT_HOOK_ENTRIES);
        for ev in [
            "PreToolUse",
            "PostToolUse",
            "Stop",
            "Notification",
            "SessionStart",
        ] {
            assert!(s.contains(ev), "missing {ev}");
        }
    }

    #[test]
    fn snippet_uses_2_space_indent() {
        let s = pretty_snippet(HOLT_HOOK_ENTRIES);
        // First indented line should start with exactly 2 spaces.
        assert!(s.contains("\n  \"PreToolUse\""), "indent missing: {s}");
    }

    #[test]
    fn snippet_contains_full_pretool_command_string() {
        let s = pretty_snippet(HOLT_HOOK_ENTRIES);
        assert!(
            s.contains("\"command\": \"holt hook PreToolUse\""),
            "missing PreToolUse command: {s}"
        );
    }
}
