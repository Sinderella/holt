# `tests/fixtures/settings/` — JSONC merge fixture corpus (Phase 3 D-01)

Six paired (`<scenario>.input.json`, `<scenario>.expected.json`) fixtures
that pin the byte-level behavior of `holt-cli`'s `install_hooks::merge_settings`.
The corpus exists per Phase 3 decision **D-01** so the merger is falsifiable
from line one — without these, the comment-preservation invariant is
unfalsifiable.

The companion test that drives this corpus end-to-end is
`crates/holt-cli/tests/install_hooks_merge_smoke.rs` (added in plan 03-01
Task 4). For each pair, the test asserts:

1. `merge_settings(read(input.json)) == read(expected.json)` byte-for-byte.
2. `merge_settings(merge_settings(input).bytes).changed == false` and
   `bytes` are byte-identical to the first merge's output (idempotency
   invariant — D-08).

If any expected output drifts in a future jsonc-parser release, the byte
diff will surface immediately; **regenerate** by re-capturing
`merge_settings`'s output (mark with an `// API-DRIFT:` note in the
SUMMARY) rather than weakening the assertions.

## Scenarios

| Scenario | What it pins | Decision |
| --- | --- | --- |
| `clean` | Strict-JSON input, no comments. New `hooks` block appended after existing keys. | D-09 entry shape |
| `line_comments` | `// ...` line comments preserved verbatim at the same byte position. | D-02 CST round-trip |
| `block_comments` | `/* ... */` block comments preserved including inline ones. | D-02 CST round-trip |
| `trailing_commas` | User's trailing commas tolerated and preserved on round-trip. | D-02 CST round-trip |
| `comments_inside_hooks` | User's pre-existing `Stop` entry is preserved as element [0]; holt's entry is appended as [1]; the `// user's prior hooks` line comment is preserved at the same relative position. | D-08 co-existence |
| `user_pretooluse` | User's `{ "matcher": "Bash", ... }` PreToolUse entry preserved as [0]; holt's `{ "matcher": "*", ... }` entry appended as [1]. | D-08 co-existence |

## Canonical hook entry shape (D-09)

Every `expected.json` contains the literal command strings
`holt hook PreToolUse`, `holt hook PostToolUse`, `holt hook Stop`,
`holt hook Notification`, `holt hook SessionStart` — exactly five events,
PascalCase, matching the v0.1 subscription set in `docs/02-scope.md`.
PreCompact is reserved for v1.0.

## Detect-by-substring policy (D-10)

The merger walks each `hooks.<event>[]` array and treats any element
whose `hooks[].command` contains `"holt hook "` (the
`HOLT_HOOK_DETECTION_SUBSTR` const in `entries.rs`) as "holt's" —
replaceable on next install. Entries with any other `command` value
are preserved verbatim. The `comments_inside_hooks` and
`user_pretooluse` fixtures verify this co-existence end-to-end.

## Forbidden patterns

These fixtures are intentionally **not** strict-JSON-parser-clean:
several use trailing commas, line comments, or block comments. They
must NOT be parsed by `serde_json::from_str` in any test — only by
`jsonc_parser::cst::CstRootNode::parse` with comments + trailing
commas allowed. Anything that round-trips this corpus through plain
`serde_json` and re-emits will violate D-02 byte-identity.

## Adding a new fixture

1. Drop a new `<name>.input.json` here.
2. Run the smoke test once with `--include-ignored` (or hand-run
   `merge_settings`) to capture the expected output.
3. Inspect the captured output for sanity (comments preserved? user
   entries co-existing with holt's? key order matches expectation?)
4. Save as `<name>.expected.json`.
5. Add `"<name>"` to the `SCENARIOS` slice in
   `tests/install_hooks_merge_smoke.rs`.

The corpus is not exhaustive — it pins the behaviors named in D-08,
D-09, and D-10 of `.planning/phases/03-install-hooks-ux/03-CONTEXT.md`.
Other JSONC quirks (Unicode escapes, BOM, mixed indentation widths)
are jsonc-parser's responsibility and tested upstream.
