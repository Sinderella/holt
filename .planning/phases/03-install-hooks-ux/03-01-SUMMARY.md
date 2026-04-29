---
phase: 03-install-hooks-ux
plan: 03-01
plan_id: 03-01
subsystem: holt-cli/install_hooks
tags: [jsonc, fs2, atomic-write, c3, c4, d-01, d-02, d-04, d-07, d-08, d-09, d-10]

dependency_graph:
  requires:
    - holt_schemas::atomic_write (Phase 1 D-07; D-06 reuses unchanged)
    - HookEventArg PascalCase ValueEnum from crates/holt-cli/src/cli.rs (Phase 2)
  provides:
    - install_hooks::merge_settings(input: &str) -> Result<MergeOutput, MergeError>
    - install_hooks::commit(settings_path, pre_merge, merged) -> Result<(), CommitError>
    - install_hooks::acquire_settings_lock(path) -> Result<File, LockError>
    - install_hooks::HOLT_HOOK_ENTRIES + HOLT_HOOK_DETECTION_SUBSTR
  affects:
    - crates/holt-cli/Cargo.toml (jsonc-parser + fs2 + thiserror.workspace deps)
    - crates/holt-cli/src/main.rs (mod install_hooks; declared)

tech-stack:
  added:
    - jsonc-parser = { version = "0.26", features = ["cst"] }   # confined to holt-cli (C4)
    - fs2 = "0.4"                                                # confined to holt-cli (C4)
    - thiserror.workspace = true                                 # workspace dep, first usage in holt-cli
  patterns:
    - "jsonc-parser CST round-trip for in-place JSONC edit (D-02)"
    - "fs2 try_lock_exclusive 4×50ms try-loop = 200ms budget (D-04)"
    - "atomic_write reuse for .holt.bak (D-07) AND merged-file write (D-06)"
    - "Detect-by-substring HOLT_HOOK_DETECTION_SUBSTR for upsert idempotency (D-10)"
    - "is_canonical_entry short-circuit so re-merge is byte-identical (D-08)"

key-files:
  created:
    - crates/holt-cli/src/install_hooks/mod.rs
    - crates/holt-cli/src/install_hooks/entries.rs
    - crates/holt-cli/src/install_hooks/lock.rs
    - crates/holt-cli/src/install_hooks/merge.rs
    - crates/holt-cli/tests/install_hooks_merge_smoke.rs
    - crates/holt-cli/tests/fixtures/settings/README.md
    - crates/holt-cli/tests/fixtures/settings/clean.input.json
    - crates/holt-cli/tests/fixtures/settings/clean.expected.json
    - crates/holt-cli/tests/fixtures/settings/line_comments.input.json
    - crates/holt-cli/tests/fixtures/settings/line_comments.expected.json
    - crates/holt-cli/tests/fixtures/settings/block_comments.input.json
    - crates/holt-cli/tests/fixtures/settings/block_comments.expected.json
    - crates/holt-cli/tests/fixtures/settings/trailing_commas.input.json
    - crates/holt-cli/tests/fixtures/settings/trailing_commas.expected.json
    - crates/holt-cli/tests/fixtures/settings/comments_inside_hooks.input.json
    - crates/holt-cli/tests/fixtures/settings/comments_inside_hooks.expected.json
    - crates/holt-cli/tests/fixtures/settings/user_pretooluse.input.json
    - crates/holt-cli/tests/fixtures/settings/user_pretooluse.expected.json
  modified:
    - crates/holt-cli/Cargo.toml
    - crates/holt-cli/src/main.rs
    - Cargo.lock

decisions:
  - D-01: fixture corpus shipped first — 6 paired scenarios pin merger behavior before any merge code wrote bytes
  - D-02: pure jsonc-parser CST round-trip; no json_comments anywhere (verified by grep)
  - D-03: jsonc-parser confined to crates/holt-cli/Cargo.toml only (verified by grep across siblings)
  - D-04: fs2 try_lock_exclusive 4-attempt × 50ms-sleep loop = 200ms total budget
  - D-05: fs2 confined to crates/holt-cli/Cargo.toml only (verified by grep)
  - D-06: holt_schemas::atomic_write reused unchanged for both .holt.bak and merged-file writes
  - D-07: .holt.bak written BEFORE merged file; single-backup policy (overwrites on each run)
  - D-08: is_canonical_entry short-circuit makes re-merge byte-identical (idempotency invariant)
  - D-09: HOLT_HOOK_ENTRIES is single source of truth for the 5 PascalCase events
  - D-10: HOLT_HOOK_DETECTION_SUBSTR = "holt hook " for env-prefix-tolerant upsert detection

metrics:
  duration: ~19 minutes
  completed: 2026-04-29
  task_commits: 4
  workspace_test_count_baseline: 51
  workspace_test_count_after: 59
  new_tests: 8 (4 lock unit + 4 merge smoke)
---

# Phase 3 Plan 03-01: JSONC merge library + fs2 lock + commit pipeline — Summary

JSONC-aware merge library inside `holt-cli` for `install-hooks` — fs2-locked
read-merge-write window using jsonc-parser CST round-trip, with `.holt.bak`
backup + `holt_schemas::atomic_write` for the merged file. C3 + C4 hard
constraints implemented and falsifiable; the CLI subcommand wiring lives in
plan 03-02.

## Tasks Completed

| # | Name | Commit | Files | Tests |
| --- | --- | --- | --- | --- |
| 1 | JSONC fixture corpus (6 paired scenarios + README) | `d2dd7ee` | 12 fixtures + README.md | (driven by Task 4) |
| 2 | Cargo.toml deps + module skeleton + entries.rs | `15d88da` | 4 src + Cargo.toml + main.rs | build only |
| 3 | fs2 exclusive lock with 200ms try-loop | `96af781` | install_hooks/lock.rs | 4 unit tests |
| 4 | JSONC merge + smoke test | `d39712f` | merge.rs + tests/install_hooks_merge_smoke.rs | 4 smoke tests |

Total: **8 new tests** (4 lock unit tests + 4 fixture-driven smoke tests).
Workspace serial test count: **51 → 59**.

## Files Created (18) and Modified (3)

**Created:**
- 4 source files under `crates/holt-cli/src/install_hooks/`: `mod.rs`, `entries.rs`, `lock.rs`, `merge.rs`
- 1 integration test: `crates/holt-cli/tests/install_hooks_merge_smoke.rs`
- 12 fixture files (6 paired) + 1 README under `crates/holt-cli/tests/fixtures/settings/`

**Modified:**
- `crates/holt-cli/Cargo.toml` — added `jsonc-parser = { version = "0.26", features = ["cst"] }`, `fs2 = "0.4"`, `thiserror.workspace = true`
- `crates/holt-cli/src/main.rs` — added `mod install_hooks;` (no dispatch arm yet — owned by plan 03-02)
- `Cargo.lock` — pin updates for new deps

## Forbidden-Crate Audit

```
cargo tree -i tokio                            → no match (no async runtime leaked)
grep "jsonc-parser|fs2 =" sibling/Cargo.toml   → empty (C4 boundary holds)
grep forbidden-list crates/holt-cli/Cargo.toml → empty (no tokio, simd-json,
   figment, chrono, owo-colors, supports-color, terminal_size,
   atomic-write-file, crossterm, wait-timeout, cargo_metadata)
```

## Hard-Constraint Preservation

All Phase 1 + 2 hard constraints verified post-Plan 03-01:

| Constraint | Test | Status |
| --- | --- | --- |
| C1 — pipe stdio for supervised processes | `cargo test -p holt-supervisor --test chokepoint_audit` | OK (1/1) |
| C2 — `holt-render` does not depend on `holt-supervisor` | `cargo test --test architecture_dag` | OK (1/1) |
| C5 — reader treats stale-or-corrupt as missing | `cargo test -p holt-schemas --test reader_contract` | OK (9/9) |
| C6 — render path never reads breaches.log/timings.jsonl | `cargo test -p holt-cli --test render_path_no_read` | OK (1/1) |
| Plan 02-01 | `cargo test -p holt-hooks --test sigkill_atomicity` | OK (1/1) |
| Plan 02-02 | `cargo test -p holt-hooks --test handle_event_smoke` | OK (6/6) |

Newly implemented:

- **C3** (atomic + locked settings.json mutation) — implemented by `lock.rs` (fs2 try_lock_exclusive) + `mod.rs::commit()` (`.holt.bak` written before merged file, both via `holt_schemas::atomic_write`).
- **C4** (JSONC handling lives ONLY in `holt-cli`) — implemented by Cargo.toml dep placement (verified by manual grep; the workspace-root `tests/cli_dep_boundary.rs` test will pin this in CI as part of Plan 03-02).

## Build + Lint Gates

```
cargo build --workspace --release           → clean
cargo fmt --check                            → clean
cargo clippy --workspace --all-targets -- -D warnings → clean
cargo test --workspace -- --test-threads=1  → 59 passed, 0 failed
```

## Deviations from Plan

### API-DRIFT — jsonc-parser 0.26.3 method names

**Found during:** Task 4 (merge.rs implementation)

**Issue:** The plan's `<interfaces>` snippet referenced jsonc-parser CST methods named `object_value_by_name`, `array_value_by_name`, `string_value_by_name`, and `CstContainerNode::as_object`. The actual jsonc-parser 0.26.3 surface (cargo registry source inspected) uses:
- `CstObject::object_value(name)` — not `object_value_by_name`
- `CstObject::array_value(name)` — not `array_value_by_name`
- No `string_value_by_name` exists. Navigate via `obj.get(name).and_then(|prop| prop.value())` and pattern-match the resulting `CstNode::Leaf(CstLeafNode::StringLit(_))`.
- Element walk uses `CstArray::elements() -> Vec<CstNode>`, then pattern-match `CstNode::Container(CstContainerNode::Object(_))` to drill in.
- `CstArray::replace(idx, value)` does NOT exist. To replace an element, navigate to the inner container (e.g., `CstObject`) and call `.replace_with(value)` on it (consumes self).

**Fix:** All call sites in `merge.rs` use the real 0.26.3 verbs. The strategy (CST round-trip + per-event upsert with substring detection) is invariant; only the verb names shifted.

**Documented:** Block comment near top of `merge.rs` headed `API-DRIFT note:` describes each rename so the next jsonc-parser bump (0.27 / 0.32 / etc.) has a clear cross-reference.

### Plan-suggested `replace(idx, value)` replaced by `replace_with` on the inner CstObject

**Found during:** Task 4

**Issue:** The plan's pseudocode used `arr.replace(idx, canonical())` to replace a holt-detected element in place. jsonc-parser 0.26.3 has no `CstArray::replace`. Pattern: drill into `CstNode::Container(CstContainerNode::Object(obj))`, then `obj.replace_with(canonical())` (consumes the object).

**Fix:** `upsert_event` walks `arr.elements()` looking for substring matches; on hit, pattern-matches into the `CstObject` and calls `.replace_with(canonical())`. Same observable behavior; different verb.

### Idempotency: added `is_canonical_entry` short-circuit (Rule 2 — auto-add missing critical functionality)

**Found during:** Task 1 fixture-generation spike

**Issue:** Naive substring-detect-then-`replace_with` is **not** idempotent. On first merge, holt's entry is appended via `append(...)` and rendered with the surrounding 4-space indent. On second merge, the substring detector finds the entry and calls `replace_with()` — which jsonc-parser re-renders with default 2-space indent regardless of surrounding context. Result: byte drift between merge 1 and merge 2, violating D-08.

**Fix:** Added `is_canonical_entry(obj, expected_command) -> bool` that structurally checks whether the existing element already matches `{ "matcher": "*", "hooks": [{ "type": "command", "command": "<expected>" }] }`. If yes, skip — leave it untouched. If no, replace_with (D-10). This makes re-merge byte-identical (D-08 invariant verified by `idempotency_re_merge_is_byte_identical_no_op` test across all 6 fixtures).

**Why this counts as Rule 2:** D-08 is part of the merger's correctness contract per the plan's `<success_criteria>`. Without the short-circuit, idempotency cannot hold and the smoke test would fail. Plan's pseudocode would have shipped a non-idempotent merger.

### Doc-comment indentation flattening (lint compliance)

**Found during:** Task 4 (clippy)

**Issue:** clippy's `doc_overindented_list_items` lint flagged the nested `a./b.` sub-list inside `merge.rs`'s strategy doc-comment (continuation lines indented to col 9, lint wants col 4 max).

**Fix:** Flattened the nested list into prose under step 3.

### `holt install-hooks` placeholder modules in mod.rs marked `#![allow(dead_code, unused_imports)]`

**Found during:** Task 2 (cargo build warnings)

**Issue:** `mod.rs` re-exports `MergeError`, `MergeOutput`, `LockError`, `acquire_settings_lock` etc. for plan 03-02's CLI dispatcher to consume. Until 03-02 wires them, the `pub use` re-exports trigger `unused_imports` warnings.

**Fix:** `#![allow(dead_code, unused_imports)]` at `mod.rs`'s root, with a comment explaining the wiring is deferred to plan 03-02. This is a defensive crate-level allow; clippy stays green elsewhere.

## Authentication Gates

None encountered.

## Pre-Existing Test Flake (NOT introduced by this plan)

`hook_self_bench_smoke::hook_self_bench_json_has_expected_shape` is a p95-timing-budget test that fails under heavy parallel cargo test load (p95 ~21–36ms vs 20ms budget). Verified pre-existing by checking out `587c0b7` (the commit before this plan) and running the test under load — same failure mode. Passes cleanly under `--test-threads=1`. This is environmental sensitivity in the existing self-bench harness, not a regression. Out of scope for Plan 03-01 (logged here for visibility; if it surfaces as a blocker for CI, Plan 03-02 or a separate fix-plan could add `--test-threads=1` to CI invocation).

## Threat Flags

| Flag | File | Description |
| --- | --- | --- |
| threat_flag: settings.json mutation | crates/holt-cli/src/install_hooks/mod.rs::commit | This plan introduces the FIRST holt code path that writes `~/.claude/settings.json`. Mitigations in place: fs2 exclusive lock (D-04), `.holt.bak` backup before merge (D-07), `holt_schemas::atomic_write` for both writes (fsync-before-rename + same-dir tmp + cleanup-on-error from Phase 1 CR-05). Plan 03-02 adds the CLI surface and the 50× concurrency / SIGKILL atomicity tests that exercise this surface end-to-end. |

No new threats outside the threat model. The `install-hooks` surface is plan 03-02's job to wire to the user.

## ROADMAP Success-Criteria Mapping (handed off to plan 03-02)

This plan delivers **library-level** correctness for criteria #1, #2 of the Phase 3 ROADMAP. CLI-surface verification (criteria #3, #4, #5) is plan 03-02's remit:

| Criterion | Owned by | Status |
| --- | --- | --- |
| #1 settings.json contains 5 holt entries after install | 03-01 lib + 03-02 CLI | lib green; CLI in 03-02 |
| #2 user-defined PreToolUse entry coexists with holt's | 03-01 lib + 03-02 CLI | lib green; CLI in 03-02 |
| #3 50× concurrent invocation = serial-equivalent state | 03-02 only | TODO |
| #4 SIGKILL mid-write leaves valid pre-merge or post-merge | 03-02 only | TODO |
| #5 `--dry-run` + `--print` + `--help` UX | 03-02 only | TODO |

## Self-Check: PASSED

- [x] All 18 created files exist on disk and are git-tracked.
- [x] All 4 task commits exist in git log: `d2dd7ee`, `15d88da`, `96af781`, `d39712f`.
- [x] `cargo build --workspace --release` clean.
- [x] `cargo fmt --check` clean.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [x] `cargo test --workspace -- --test-threads=1` reports 59 passing, 0 failed.
- [x] Hard constraints C1, C2, C5, C6 all green; C3 + C4 newly implemented.
- [x] `cargo tree -i tokio` empty; no async runtime leaked.
- [x] Forbidden-crate grep across all sibling crates returns empty for `jsonc-parser` and `fs2`.
- [x] Plan deferred items: 50× concurrency test + SIGKILL atomicity + `--dry-run` / `--print` / `--help` are 03-02's remit (intentional split per plan).
