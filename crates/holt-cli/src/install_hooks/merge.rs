//! D-02: pure jsonc-parser CST round-trip. NO `json_comments` strip-then-parse
//! compose path — that loses byte positions on round-trip and contradicts D-02.
//!
//! Strategy:
//! 1. Parse the input as CST (preserves comments + key order + trailing commas).
//! 2. Walk to the top-level `hooks` object (create if absent — appended AFTER
//!    existing keys per D-08 key-order preservation).
//! 3. For each event in [`HOLT_HOOK_ENTRIES`]: if `hooks.<event>` is absent,
//!    create a new array containing holt's canonical block; if present, scan
//!    the array for any element whose `hooks[].command` contains
//!    [`HOLT_HOOK_DETECTION_SUBSTR`] (`"holt hook "`). If the element is
//!    already byte-equivalent to holt's canonical block, leave it untouched
//!    (idempotency invariant — D-08). If it differs, replace in-place (D-10).
//!    If no match, append holt's canonical block (D-08 co-existence).
//! 4. Emit the CST as bytes. Return [`MergeOutput`] with `changed = bytes != input`.
//!
//! Idempotency invariant (D-08): re-merging the output yields `changed == false`
//! and bytes byte-identical to the first merge's output. The
//! `is_canonical_entry` check is what makes this hold — without it, the second
//! merge would call `replace_with` on an entry that already has the right
//! command string, and jsonc-parser's CST `replace_with` re-renders with default
//! indent (2 spaces, regardless of the surrounding indent), which causes byte
//! drift.

// API-DRIFT note: the plan's snippet referenced `object_value_by_name`,
// `array_value_by_name`, `string_value_by_name`, and `as_object` (on
// `CstContainerNode`). Inspecting jsonc-parser 0.26.3 source
// (~/.cargo/registry/src/.../jsonc-parser-0.26.3/src/cst/mod.rs) reveals the
// real surface uses `object_value`, `array_value`, and `get` returning
// `Option<CstObjectProp>`. There is no `string_value_by_name`; navigate via
// `get(name).and_then(|prop| prop.value())` and pattern-match the resulting
// `CstNode::Leaf(CstLeafNode::StringLit(_))`. The strategy (CST round-trip +
// per-event upsert with substring detection) is invariant; only the verb
// names shifted.

use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{
    CstContainerNode, CstInputValue, CstLeafNode, CstNode, CstObject, CstRootNode,
};

use super::entries::{HOLT_HOOK_DETECTION_SUBSTR, HOLT_HOOK_ENTRIES, HoltHookEntry};

#[derive(Debug, thiserror::Error)]
pub enum MergeError {
    #[error("settings.json is not valid JSONC: {0}")]
    Parse(String),
    #[error("settings.json root is not a JSON object (got {got})")]
    NotAnObject { got: &'static str },
}

pub struct MergeOutput {
    /// Post-merge bytes (UTF-8). Pass to `holt_schemas::atomic_write`.
    pub bytes: String,
    /// True if any byte changed vs the input. False = no-op (idempotent re-run).
    pub changed: bool,
}

pub fn merge_settings(input: &str) -> Result<MergeOutput, MergeError> {
    // Empty file = treat as `{}` (newly-created settings.json from lock acquire).
    let normalized = if input.trim().is_empty() { "{}" } else { input };

    let opts = ParseOptions {
        allow_comments: true,
        allow_trailing_commas: true,
        allow_loose_object_property_names: false,
    };
    let root =
        CstRootNode::parse(normalized, &opts).map_err(|e| MergeError::Parse(e.to_string()))?;

    // CstRootNode → CstObject for the top-level value.
    let root_obj = root.object_value().ok_or(MergeError::NotAnObject {
        got: "non-object root",
    })?;

    // Walk-or-create `hooks` object.
    let hooks_obj = match root_obj.object_value("hooks") {
        Some(obj) => obj,
        None => {
            // Append a new `hooks: {}` property AFTER existing keys (preserves key order).
            let prop = root_obj.append("hooks", CstInputValue::Object(Vec::new()));
            prop.value()
                .and_then(|v| match v {
                    CstNode::Container(CstContainerNode::Object(o)) => Some(o),
                    _ => None,
                })
                .expect("just-appended hooks value is an object")
        }
    };

    // For each event: ensure exactly one holt entry exists in `hooks.<event>`.
    for entry in HOLT_HOOK_ENTRIES {
        upsert_event(&hooks_obj, entry);
    }

    let bytes = root.to_string();
    let changed = bytes != input;
    Ok(MergeOutput { bytes, changed })
}

fn upsert_event(hooks_obj: &CstObject, entry: &HoltHookEntry) {
    let canonical = || canonical_entry_value(entry.command);

    match hooks_obj.array_value(entry.event) {
        None => {
            // No existing array — create with one element (holt's).
            hooks_obj.append(
                entry.event,
                CstInputValue::Array(vec![canonical()]),
            );
        }
        Some(arr) => {
            let mut handled = false;
            for elem in arr.elements() {
                if !element_command_contains(&elem, HOLT_HOOK_DETECTION_SUBSTR) {
                    continue;
                }
                let CstNode::Container(CstContainerNode::Object(obj)) = elem else {
                    continue;
                };
                // D-08: if the existing entry is already canonical, leave it untouched.
                // This is what makes re-merging byte-identical (idempotency).
                if is_canonical_entry(&obj, entry.command) {
                    handled = true;
                    break;
                }
                // D-10: substring match but content differs — replace in-place.
                obj.replace_with(canonical());
                handled = true;
                break;
            }
            if !handled {
                // D-08: co-exist, do not clobber. Append after user's entries.
                arr.append(canonical());
            }
        }
    }
}

/// D-09 canonical block:
///   `{ "matcher": "*", "hooks": [ { "type": "command", "command": "<command>" } ] }`
fn canonical_entry_value(command: &str) -> CstInputValue {
    CstInputValue::Object(vec![
        ("matcher".to_string(), CstInputValue::String("*".to_string())),
        (
            "hooks".to_string(),
            CstInputValue::Array(vec![CstInputValue::Object(vec![
                (
                    "type".to_string(),
                    CstInputValue::String("command".to_string()),
                ),
                (
                    "command".to_string(),
                    CstInputValue::String(command.to_string()),
                ),
            ])]),
        ),
    ])
}

/// True if `elem` is an object with a `hooks` array whose first element's
/// `command` field is a string containing `needle`. Anything else returns
/// false (defensive — user-malformed entries are NOT mistakenly classified
/// as ours).
fn element_command_contains(elem: &CstNode, needle: &str) -> bool {
    let CstNode::Container(CstContainerNode::Object(obj)) = elem else {
        return false;
    };
    let Some(hooks_arr) = obj.array_value("hooks") else {
        return false;
    };
    for inner in hooks_arr.elements() {
        let CstNode::Container(CstContainerNode::Object(inner_obj)) = inner else {
            continue;
        };
        let Some(cmd) = string_property(&inner_obj, "command") else {
            continue;
        };
        if cmd.contains(needle) {
            return true;
        }
    }
    false
}

/// True iff `obj` is structurally `{ "matcher": "*", "hooks": [ { "type": "command", "command": "<expected>" } ] }`.
/// Used to short-circuit replace_with on idempotent re-merge so the second
/// merge is byte-identical to the first.
fn is_canonical_entry(obj: &CstObject, expected_command: &str) -> bool {
    if string_property(obj, "matcher").as_deref() != Some("*") {
        return false;
    }
    let Some(hooks_arr) = obj.array_value("hooks") else {
        return false;
    };
    let elems = hooks_arr.elements();
    if elems.len() != 1 {
        return false;
    }
    let CstNode::Container(CstContainerNode::Object(inner)) = &elems[0] else {
        return false;
    };
    if string_property(inner, "type").as_deref() != Some("command") {
        return false;
    }
    string_property(inner, "command").as_deref() == Some(expected_command)
}

/// Read a JSON-string property's decoded value as a Rust String, returning
/// `None` if absent or non-string. Strips the surrounding `"` quotes from
/// `raw_value()`; the canonical command/matcher strings have no escapes
/// and no embedded quotes, so trim_matches('"') is faithful here.
fn string_property(obj: &CstObject, name: &str) -> Option<String> {
    let prop = obj.get(name)?;
    let value = prop.value()?;
    let CstNode::Leaf(CstLeafNode::StringLit(s)) = value else {
        return None;
    };
    let raw = s.raw_value();
    Some(raw.trim_matches('"').to_string())
}
