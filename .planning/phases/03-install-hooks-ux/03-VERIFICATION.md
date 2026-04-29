---
phase: 3
phase_name: install-hooks UX
status: passed
verified: 2026-04-28
roadmap_criteria_passed: 5/5
requirements_covered: 4/4
decisions_implemented: 17/17
hard_constraints_enforced: 6/6 (C1..C6)
quality_gates_clean: true
score: 5/5 must-haves verified
overrides_applied: 0
---

# Phase 3: install-hooks UX Verification Report

**Phase Goal:** A `holt install-hooks` subcommand that idempotently merges holt's hook entries into the user's `~/.claude/settings.json` without corrupting any pre-existing hooks, without losing JSONC comments or key order, without racing concurrent editors, and with a `--dry-run` / `--print` escape hatch for users who'd rather paste manually.

**Verified:** 2026-04-28
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (5 ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Clean settings.json + statusLine preserved + .holt.bak byte-equal | VERIFIED | `cargo test -p holt-cli --test install_hooks_smoke must_have_1_clean_fixture_byte_equal_and_bak_present` → 1 passed, 0 failed in 0.35s suite |
| 2 | JSONC line comments + key order preserved + user PreToolUse co-exists | VERIFIED | `cargo test -p holt-cli --test install_hooks_smoke must_have_2_line_comments_and_key_order_preserved` → passed; user_pretooluse fixture preserves `{"matcher":"Bash","command":"user-script"}` as element [0] alongside holt's `matcher:"*"` as [1] |
| 3a | `--dry-run` no mutation + diff printed | VERIFIED | `must_have_3_dry_run_does_not_mutate_and_prints_diff` → passed; manual run with HOME=$tempdir confirmed mtime preserved + unified diff with `--- `, `+++ `, `+`-prefix lines emitted; no `.holt.bak` created |
| 3b | `--print` no mutation + JSON snippet | VERIFIED | `must_have_3_print_does_not_mutate_and_emits_snippet` → passed; manual run confirmed `"command": "holt hook PreToolUse"` substring + 2-space indent; mtime unchanged |
| 3c | `--dry-run --print` mutually exclusive | VERIFIED | `dry_run_and_print_are_mutually_exclusive` → passed; manual run exits 2 with `error: the argument '--dry-run' cannot be used with '--print'` |
| 4 | 50× concurrent stress | VERIFIED | `cargo test -p holt-cli --test install_hooks_concurrent --release` → 1 passed in 0.66s (budget 30s); post-stress file parses with both serde_json + jsonc-parser via --dry-run proxy; each holt command appears exactly once; user's Bash matcher entry survives; no .holt-tmp.* orphans |
| 5 | 200× SIGKILL atomicity | VERIFIED | `cargo test -p holt-cli --test install_hooks_sigkill --release` → 1 passed in 3.97s (budget 60s); every iteration's settings.json parses with both `serde_json::from_str` and `jsonc_parser::parse_to_ast`; observed state ∈ {pre-merge, canonical-post} |

**Score:** 5/5 truths verified (must_have-1, must_have-2, must_have-3 [3a+3b+3c], must_have-4, must_have-5).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/holt-cli/src/install_hooks/mod.rs` | Public surface (commit, MergeOutput re-exports) + CommitError | VERIFIED | 96 lines; exposes `commit`, `acquire_settings_lock`, `merge_settings`, `HOLT_HOOK_ENTRIES`; `pub mod diff;` + `pub mod print;` declared |
| `crates/holt-cli/src/install_hooks/entries.rs` | HOLT_HOOK_ENTRIES + HOLT_HOOK_DETECTION_SUBSTR | VERIFIED | 47 lines; 5 PascalCase events with `holt hook <Event>` commands |
| `crates/holt-cli/src/install_hooks/lock.rs` | fs2 try_lock_exclusive 200ms loop | VERIFIED | 175 lines; MAX_ATTEMPTS=4, SLEEP_BETWEEN_ATTEMPTS=50ms, TOTAL_BUDGET_MS=200; 4 unit tests pass (acquires_when_free, second_attempt_after_drop_succeeds, timeout_when_held_by_other_handle, error_message_contains_keywords) |
| `crates/holt-cli/src/install_hooks/merge.rs` | JSONC CST round-trip + idempotent upsert | VERIFIED | jsonc_parser::cst::CstRootNode + per-event upsert with substring detection + is_canonical_entry idempotency short-circuit |
| `crates/holt-cli/src/install_hooks/diff.rs` | Hand-rolled unified diff | VERIFIED | 4.4KB; common-prefix + common-suffix algorithm; 3 unit tests pass |
| `crates/holt-cli/src/install_hooks/print.rs` | 2-space JSON snippet emitter | VERIFIED | 2.1KB; 3 unit tests pass |
| `crates/holt-cli/src/install_hooks_cmd.rs` | Run-mode dispatcher | VERIFIED | 156 lines; lock → read → merge → (dry-run | print | commit) pipeline; exit codes 0/1/2/3 documented |
| `crates/holt-cli/src/cli.rs` | InstallHooks { dry_run, print } variant | VERIFIED | Lines 71-79: `InstallHooks { #[arg(long, conflicts_with = "print")] dry_run: bool, #[arg(long)] print: bool }` |
| `crates/holt-cli/src/main.rs` | Dispatch arm for InstallHooks | VERIFIED | Lines 59-61: `Some(cli::Command::InstallHooks { dry_run, print }) => install_hooks_cmd::run(dry_run, print)` |
| `crates/holt-cli/Cargo.toml` | jsonc-parser, fs2, thiserror, libc dev | VERIFIED | jsonc-parser = { version = "0.26", features = ["cst"] }; fs2 = "0.4"; thiserror.workspace = true; libc = "0.2" in [dev-dependencies] |
| `crates/holt-cli/tests/fixtures/settings/` | 6 paired JSONC fixtures + README | VERIFIED | 12 .json files (6 input + 6 expected) + README.md present; clean / line_comments / block_comments / trailing_commas / comments_inside_hooks / user_pretooluse |
| `crates/holt-cli/tests/install_hooks_smoke.rs` | 7 must_have-1/2/3 + D-13/D-16/D-17 tests | VERIFIED | 7 tests; all pass; covers must_have-1, must_have-2, must_have-3a/b/c, D-17 budget, D-13 help |
| `crates/holt-cli/tests/install_hooks_merge_smoke.rs` | 4 library merge tests | VERIFIED | 4 tests pass: fixture_corpus_matches_expected_byte_for_byte, idempotency_re_merge_is_byte_identical_no_op, detect_by_substring_does_not_duplicate_when_user_wraps_with_env_prefix, empty_input_produces_full_5_event_block |
| `crates/holt-cli/tests/install_hooks_concurrent.rs` | 50× concurrent stress test | VERIFIED | `#![cfg(unix)]`; passes in 0.66s |
| `crates/holt-cli/tests/install_hooks_sigkill.rs` | 200× SIGKILL atomicity test | VERIFIED | `#![cfg(unix)]` + `#![allow(unsafe_code)]`; libc::kill SIGKILL; uses both serde_json + jsonc_parser parsers; passes in 3.97s |
| `tests/cli_dep_boundary.rs` | Workspace-root C4 BFS gate | VERIFIED | 149 lines; BFS from each non-cli workspace crate; false-positive-negative guard asserts holt-cli CAN reach jsonc-parser+fs2 |
| `Cargo.toml` (workspace root) | [[test]] cli_dep_boundary registration | VERIFIED | Lines 52-54: `name = "cli_dep_boundary"`, path = "tests/cli_dep_boundary.rs" |

### Key Link Verification

| From | To | Via | Status | Details |
|------|------|-----|--------|---------|
| install_hooks_cmd.rs | install_hooks::{lock,merge,mod}.rs | use crate::install_hooks::{...} | WIRED | Lines 28-33 of install_hooks_cmd.rs import acquire_settings_lock, commit, merge_settings, HOLT_HOOK_ENTRIES |
| main.rs | install_hooks_cmd.rs | match arm `Command::InstallHooks => install_hooks_cmd::run` | WIRED | Lines 59-61 of main.rs dispatch arm present |
| install_hooks/lock.rs | fs2::FileExt::try_lock_exclusive | use fs2::FileExt | WIRED | Line 18; called at line 78 |
| install_hooks/mod.rs::commit | holt_schemas::atomic_write | function call | WIRED | Lines 84, 88: `holt_schemas::atomic_write(&bak_path, ...)` then `holt_schemas::atomic_write(settings_path, ...)` |
| install_hooks/merge.rs | jsonc_parser::cst | CstRootNode::parse + Display round-trip | WIRED | Lines 36-39 import; `CstRootNode::parse` at L68; `root.to_string()` at L95 |
| install_hooks/merge.rs | install_hooks/entries.rs | use super::entries::{HOLT_HOOK_DETECTION_SUBSTR, HOLT_HOOK_ENTRIES, HoltHookEntry} | WIRED | Line 41 |
| tests/cli_dep_boundary.rs | cargo metadata --format-version 1 | std::process::Command | WIRED | Lines 53-57 shell out + JSON parse |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|---------------------|--------|
| `holt install-hooks` (default) | merged.bytes | merge_settings(input from fs::read_to_string) | Yes — `cat $HOME/.claude/settings.json` post-run shows merged 5 events | FLOWING |
| `holt install-hooks --print` | snippet | pretty_snippet(HOLT_HOOK_ENTRIES) const | Yes — verified by manual run; emits 56 lines of paste-ready JSON | FLOWING |
| `holt install-hooks --dry-run` | diff string | unified_diff(input_bytes, merged.bytes, ...) | Yes — verified by manual run against clean fixture; emits unified-diff format with `---`, `+++`, `@@`, `+` markers | FLOWING |
| `.holt.bak` backup | pre_merge bytes | input (fs::read_to_string before merge) | Yes — must_have_1 test asserts `bak_bytes == pre_bytes` byte-equal | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `holt install-hooks --help` exits 0 | `target/release/holt install-hooks --help` | exit=0; 15 lines (D-13 cap is 40); mentions --dry-run, --print, .holt.bak | PASS |
| `holt install-hooks --dry-run --print` exits non-zero | same | exit=2; clap usage error names both flags | PASS |
| `holt install-hooks --dry-run` against tempdir clean fixture | HOME=$tempdir on `{ "statusLine": ... }` | exit=0; mtime unchanged; 61-line unified diff with `+`-prefix appended hooks block | PASS |
| `holt install-hooks --print` against tempdir clean fixture | HOME=$tempdir | exit=0; mtime unchanged; 56-line "hooks": { ... } snippet | PASS |
| `holt install-hooks` end-to-end <500ms in release | HOME=$tempdir; release binary | 66ms warm wall clock | PASS (D-17 budget) |
| `cargo build --workspace --release` exit 0 | same | Finished `release` in 0.04s (cached) | PASS |
| `cargo test --workspace` exit 0 + count | same | 75 passed, 0 failed, 0 ignored — exactly matches SUMMARY claim | PASS |
| `cargo clippy --all-targets -- -D warnings` exit 0 | same | Finished dev profile clean; no warnings | PASS |
| `cargo fmt --check` exit 0 | same | exit=0 (no diff) | PASS |
| `cargo tree -i tokio` empty | same | "package ID specification `tokio` did not match any packages" — empty as required | PASS |
| `cargo tree -p holt-render \| grep holt-supervisor` empty | same | exit=1 (grep no match) — C2 still intact | PASS |
| C1 chokepoint_audit | `cargo test -p holt-supervisor --test chokepoint_audit` | 1 passed | PASS |
| C2 architecture_dag | `cargo test --test architecture_dag` | 1 passed | PASS |
| C3 + C4 (new) | install_hooks_concurrent + sigkill + cli_dep_boundary | 3 tests passed | PASS |
| C5 reader_contract | `cargo test -p holt-schemas --test reader_contract` | 9 passed | PASS |
| C6 render_path_no_read | `cargo test -p holt-cli --test render_path_no_read` | 1 passed | PASS |
| Forbidden-crate audit on each Cargo.toml | grep across crates/*/Cargo.toml | only holt-cli has jsonc-parser+fs2; no tokio/simd-json/figment/chrono/owo-colors/supports-color/terminal_size/atomic-write-file/crossterm/wait-timeout/cargo_metadata anywhere | PASS |
| `cli_dep_boundary` false-positive-negative guard | source review of tests/cli_dep_boundary.rs L94-107 | guard asserts holt-cli CAN reach jsonc-parser+fs2 (would fail if dep stripped); main contract iterates 5 non-cli crates × 2 forbidden targets | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| HOOK-07 | 03-01, 03-02 | `holt install-hooks` reads ~/.claude/settings.json, merges holt's hook entries non-destructively, writes back atomically with .holt.bak backup | SATISFIED | install_hooks_cmd.rs default path: lock → read → merge_settings → commit (writes .holt.bak THEN settings.json via atomic_write); must_have_1 test verifies byte-equal .holt.bak |
| HOOK-08 | 03-02 | --dry-run prints diff, --print emits JSON snippet; both exit 0 without mutating | SATISFIED | install_hooks_cmd.rs L51-78: print and dry-run branches both early-return without invoking commit; must_have_3_dry_run_* + must_have_3_print_* tests assert mtime unchanged |
| HOOK-09 | 03-01, 03-02 | acquires fs2::FileExt::try_lock_exclusive() for entire RMW window; fsync(2) before rename(2) | SATISFIED | lock.rs uses fs2::FileExt::try_lock_exclusive; commit() routes through holt_schemas::atomic_write (Phase 1 D-07 hand-rolled tmp+fsync+rename); 50× concurrent + 200× SIGKILL tests prove the contract |
| HOOK-10 | 03-01, 03-02 | round-trips JSONC (preserves user comments and key order); uses jsonc-parser v0.26+ CST in holt-cli only | SATISFIED | merge.rs uses jsonc_parser::cst::CstRootNode; cli_dep_boundary test enforces the holt-cli-only confinement at workspace level |

All 4 requirements (HOOK-07..HOOK-10) are SATISFIED. No orphaned requirements detected for Phase 3.

### CONTEXT Decisions Implementation (D-01..D-17)

| # | Decision | Status | Evidence |
|---|---------|--------|----------|
| D-01 | ≥6 paired JSONC fixtures | IMPLEMENTED | 12 .json files in fixtures/settings/ (6 input + 6 expected) + README.md |
| D-02 | Pure jsonc-parser CST round-trip; no json_comments | IMPLEMENTED | merge.rs uses CstRootNode::parse + Display; grep `json_comments` in merge.rs returns empty |
| D-03 | jsonc-parser confined to holt-cli/Cargo.toml only | IMPLEMENTED | Forbidden-crate audit + cli_dep_boundary test prove confinement |
| D-04 | fs2 200ms try-loop (4× 50ms) | IMPLEMENTED | lock.rs MAX_ATTEMPTS=4, SLEEP_BETWEEN_ATTEMPTS=50ms, TOTAL_BUDGET_MS=200 |
| D-05 | fs2 confined to holt-cli/Cargo.toml only | IMPLEMENTED | Same audit + cli_dep_boundary test |
| D-06 | Reuse holt_schemas::atomic_write for the merged-file write | IMPLEMENTED | mod.rs::commit() calls holt_schemas::atomic_write twice (.bak + merged) |
| D-07 | .holt.bak single-backup policy (NOT .bak) | IMPLEMENTED | mod.rs::commit constructs `<settings>.holt.bak`; help text mentions ".holt.bak"; must_have_1 test asserts bak path |
| D-08 | Idempotency = co-exist not replace | IMPLEMENTED | merge.rs upsert_event() with is_canonical_entry short-circuit; idempotency_re_merge_is_byte_identical_no_op test passes |
| D-09 | Hook entry shape exact bytes (matcher="*", type="command", command="holt hook <Event>") | IMPLEMENTED | entries.rs HOLT_HOOK_ENTRIES const; 5 PascalCase events; merge.rs upsert_event builds canonical block |
| D-10 | Detect-by-command-substring "holt hook " | IMPLEMENTED | entries.rs HOLT_HOOK_DETECTION_SUBSTR = "holt hook "; merge.rs uses substring detection; detect_by_substring test asserts env-prefix variant collapses to 1 |
| D-11 | --dry-run unified diff (hand-rolled, no `similar`) | IMPLEMENTED | install_hooks/diff.rs (4.4KB hand-rolled common-prefix/suffix); no `similar` in Cargo.toml |
| D-12 | --print pretty 2-space-indent snippet | IMPLEMENTED | install_hooks/print.rs; manually verified output uses 2-space indent at top level |
| D-13 | --help UX ≤40 lines + mentions modes + .holt.bak | IMPLEMENTED | clap-derived; 15 lines (well under 40); help_text_mentions_dry_run_print_and_holt_bak_in_at_most_40_lines test passes |
| D-14 | 50× concurrent stress test, <30s | IMPLEMENTED | install_hooks_concurrent.rs `#[cfg(unix)]`; 50 spawns; passes in 0.66s |
| D-15 | 200× SIGKILL atomicity test, <60s | IMPLEMENTED | install_hooks_sigkill.rs uses libc::kill with SIGKILL; xorshift PRNG for delay; passes in 3.97s |
| D-16 | clap subcommand with conflicts_with | IMPLEMENTED | cli.rs InstallHooks variant with `#[arg(long, conflicts_with = "print")] dry_run`; manually verified mutual exclusion |
| D-17 | <500ms cold-start budget for install-hooks | IMPLEMENTED | d17_completes_within_500ms_in_release_or_800ms_in_debug test passes; manual measurement: 66ms warm release |

All 17 decisions implemented.

### Hard-Constraint Preservation (C1..C6 + new C3, C4)

| Constraint | Status | Test |
|-----------|--------|------|
| C1 — Pipe stdio for supervised processes | PRESERVED | `cargo test -p holt-supervisor --test chokepoint_audit` → 1 passed |
| C2 — holt-render does NOT depend on holt-supervisor | PRESERVED | `cargo test --test architecture_dag` → 1 passed; `cargo tree -p holt-render \| grep holt-supervisor` → empty |
| C3 — fs2 lock + fsync-before-rename + .holt.bak + PID-suffix tmp | NEWLY ENFORCED | 50× concurrent + 200× SIGKILL tests prove falsifiability; both pass |
| C4 — JSONC + fs2 deps live ONLY in holt-cli | NEWLY ENFORCED | `cargo test --test cli_dep_boundary` → 1 passed; BFS from 5 non-cli crates × 2 forbidden targets all return false |
| C5 — Reader treats stale-or-corrupt as missing | PRESERVED | `cargo test -p holt-schemas --test reader_contract` → 9 passed |
| C6 — Render path never reads breaches.log/timings.jsonl | PRESERVED | `cargo test -p holt-cli --test render_path_no_read` → 1 passed |

### Anti-Patterns Found

None. Code review on all created/modified files reveals no TODOs/FIXMEs/placeholders/stubs/empty handlers/console.log-only impls.

Specific spot-checks performed:
- `grep -E "TODO|FIXME|XXX|HACK|PLACEHOLDER|placeholder|coming soon" crates/holt-cli/src/install_hooks/*.rs crates/holt-cli/src/install_hooks_cmd.rs` → no matches in production code
- `grep -E "unimplemented!|todo!" crates/holt-cli/src/install_hooks/*.rs` → no matches (the placeholder `unimplemented!()` from Task 2 of plan 03-01 was filled in by Tasks 3 and 4 of the same plan)
- `unwrap()` audit on render-path: install-hooks is NOT on render path (D-17 explicitly relaxes the budget); user-invoked subcommand correctly uses Result<i32> propagation

### Deviations Documented

The two SUMMARY files document API-DRIFT findings (jsonc-parser 0.26.3 method names + parse_to_ast argument order) that were correctly handled at execution time. These were resolved in code, not deferred. Verified by:
- merge.rs successfully calls `object_value`, `array_value`, `get` (the real 0.26.3 surface)
- install_hooks_sigkill.rs calls `parse_to_ast(bytes, &Default::default(), &parse_opts)` — args in correct order
- All 75 tests pass

### Quality Gates

| Gate | Status |
|------|--------|
| `cargo build --workspace --release` exit 0 | PASS |
| `cargo test --workspace` exit 0; count = 75 (matches SUMMARY claim) | PASS |
| `cargo clippy --all-targets -- -D warnings` exit 0 | PASS |
| `cargo fmt --check` exit 0 | PASS |
| `cargo tree -i tokio` empty | PASS |
| Forbidden-crate audit (per Cargo.toml) — only jsonc-parser + fs2 newly allowed in holt-cli | PASS |

### Human Verification Required

None. All 5 ROADMAP success criteria have runnable, observable test commands; the CLI behaviors were spot-checked via direct release-binary invocation (--help, --dry-run, --print, --dry-run --print mutual exclusion, default mutation, D-17 budget); both the 50× concurrent and 200× SIGKILL atomicity stress tests run in CI to prove the C3 contract; the workspace-root cli_dep_boundary test mechanically prove the C4 confinement.

The phase is fully verifiable programmatically and the verification was completed end-to-end.

---

## Gaps Summary

No gaps. All 5 ROADMAP success criteria, 4 requirements (HOOK-07..HOOK-10), 17 CONTEXT decisions (D-01..D-17), and 6 hard constraints (C1..C6, including the two newly-enforced C3 + C4) are implemented and falsifiable in CI. The phase goal — "A `holt install-hooks` subcommand that idempotently merges holt's hook entries into the user's `~/.claude/settings.json` without corrupting any pre-existing hooks, without losing JSONC comments or key order, without racing concurrent editors, and with a `--dry-run` / `--print` escape hatch" — is verifiably achieved in the codebase.

---

## VERIFICATION COMPLETE

**Status:** PASSED
**Score:** 5/5 must-haves verified (5/5 ROADMAP criteria; 4/4 requirements; 17/17 decisions; 6/6 hard constraints)
**Workspace tests:** 75 passed, 0 failed (exactly matches SUMMARY claim of 75)
**Quality gates:** all clean (build / test / clippy / fmt / cargo-tree -i tokio / forbidden-crate audit)

Phase 3 is complete and ready for the next phase. Recommended next action per `/gsd-next` is Phase 4: Distribution + launch.

---

*Verified: 2026-04-28*
*Verifier: Claude (gsd-verifier)*
