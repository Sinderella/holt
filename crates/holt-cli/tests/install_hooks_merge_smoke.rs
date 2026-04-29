//! Library-level smoke test for `install_hooks::merge_settings`.
//!
//! Iterates the 6 paired fixtures from Plan 03-01 Task 1 and asserts:
//!   1. byte-identical match against `<scenario>.expected.json`,
//!   2. idempotency invariant — re-merging the output is a byte-identical no-op (D-08),
//!   3. detect-by-substring policy (D-10) prevents duplicates when the user wraps
//!      our entry with an env-var prefix,
//!   4. empty input produces a valid 5-event block.
//!
//! `holt-cli` is a `[[bin]]` crate, so its modules aren't exposed to integration
//! tests as a library. We use `#[path = "..."]` to include the install_hooks
//! sources directly. This is the same trick Phase 1 + 2 used (e.g.,
//! `tests/run_passthrough.rs`). The `dead_code` allow propagates from
//! `install_hooks/mod.rs`'s crate-level `#![allow(...)]` won't apply here
//! because `#[path =]` creates a fresh module tree, so we re-allow at this
//! integration test's root.
//!
//! Plan 03-02 will add binary-level integration tests that drive the full
//! `holt install-hooks` CLI; this test stays at the library boundary so
//! merge logic is falsifiable without subprocess overhead.

#![allow(dead_code, unused_imports)]

use std::fs;
use std::path::PathBuf;

#[path = "../src/install_hooks/entries.rs"]
mod entries;
#[path = "../src/install_hooks/merge.rs"]
mod merge;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/settings")
}

fn read(name: &str) -> String {
    fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

const SCENARIOS: &[&str] = &[
    "clean",
    "line_comments",
    "block_comments",
    "trailing_commas",
    "comments_inside_hooks",
    "user_pretooluse",
];

#[test]
fn fixture_corpus_matches_expected_byte_for_byte() {
    for s in SCENARIOS {
        let input = read(&format!("{s}.input.json"));
        let expected = read(&format!("{s}.expected.json"));
        let out = merge::merge_settings(&input)
            .unwrap_or_else(|e| panic!("merge {s} failed: {e}"));
        assert_eq!(
            out.bytes, expected,
            "fixture `{s}` did not round-trip to expected output\n\
             ---INPUT---\n{input}\n---GOT---\n{}\n---WANT---\n{expected}",
            out.bytes
        );
        assert!(
            out.changed,
            "fixture `{s}` should report changed=true on first merge"
        );
    }
}

#[test]
fn idempotency_re_merge_is_byte_identical_no_op() {
    for s in SCENARIOS {
        let input = read(&format!("{s}.input.json"));
        let first = merge::merge_settings(&input).expect("first merge ok");
        let second = merge::merge_settings(&first.bytes).expect("second merge ok");
        assert_eq!(
            second.bytes, first.bytes,
            "fixture `{s}` re-merge changed bytes (idempotency violated)"
        );
        assert!(
            !second.changed,
            "fixture `{s}` re-merge should report changed=false"
        );
    }
}

#[test]
fn detect_by_substring_does_not_duplicate_when_user_wraps_with_env_prefix() {
    // D-10: user has prefixed our entry with an env-var. Detection by
    // substring means the merger replaces in-place rather than appending a
    // duplicate. The user's env-var prefix is gone after merge (we replace
    // the whole block), but we do NOT have a duplicate. The trade-off is
    // documented in 03-CONTEXT.md D-10 — substring detection accepts the
    // wrapper and replaces; users who want the wrapper preserved must
    // re-add it after `holt install-hooks`.
    let input = r#"{
  "hooks": {
    "PreToolUse": [
      { "matcher": "*", "hooks": [ { "type": "command", "command": "PATH=/opt/bin holt hook PreToolUse" } ] }
    ]
  }
}"#;
    let out = merge::merge_settings(input).expect("merge ok");
    let occurrences = out.bytes.matches("holt hook PreToolUse").count();
    assert_eq!(
        occurrences, 1,
        "expected exactly one PreToolUse entry, got {occurrences}: {}",
        out.bytes
    );
}

#[test]
fn empty_input_produces_full_5_event_block() {
    let out = merge::merge_settings("").expect("merge ok");
    for ev in [
        "PreToolUse",
        "PostToolUse",
        "Stop",
        "Notification",
        "SessionStart",
    ] {
        let pat = format!("\"holt hook {ev}\"");
        assert!(
            out.bytes.contains(&pat),
            "missing event {ev} in output: {}",
            out.bytes
        );
    }
}
