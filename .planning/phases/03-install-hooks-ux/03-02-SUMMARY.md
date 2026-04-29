---
phase: 03-install-hooks-ux
plan: 03-02
plan_id: 03-02
subsystem: holt-cli/install_hooks
tags: [cli, dry-run, print, sigkill, concurrent, c3, c4, d-11, d-12, d-13, d-14, d-15, d-16, d-17]

dependency_graph:
  requires:
    - install_hooks::merge_settings (Plan 03-01)
    - install_hooks::commit (Plan 03-01)
    - install_hooks::acquire_settings_lock (Plan 03-01)
    - install_hooks::HOLT_HOOK_ENTRIES (Plan 03-01)
    - holt_schemas::atomic_write (Phase 1 D-07)
    - tests/architecture_dag.rs BFS pattern (Phase 1)
  provides:
    - holt install-hooks subcommand (6th binary entry point)
    - holt install-hooks --dry-run (D-11 unified diff)
    - holt install-hooks --print (D-12 paste-ready snippet)
    - install_hooks_cmd::run dispatcher
    - install_hooks::diff::unified_diff hand-rolled diff
    - install_hooks::print::pretty_snippet 2-space-indent emitter
    - tests/cli_dep_boundary.rs C4 falsifiability gate
  affects:
    - crates/holt-cli/src/cli.rs (clap surface extended)
    - crates/holt-cli/src/main.rs (dispatcher arm)
    - crates/holt-cli/src/install_hooks/mod.rs (module declarations + relaxed allow)
    - crates/holt-cli/Cargo.toml (libc dev-dep added for SIGKILL test)
    - Cargo.toml (workspace root [[test]] registration)

tech-stack:
  added:
    - libc = "0.2"   # dev-dep only, for libc::kill in install_hooks_sigkill.rs
  patterns:
    - "out-of-process std::process::Command spawn for cross-process fs2 lock contention"
    - "xorshift hand-rolled PRNG (avoids `rand` dev-dep) for SIGKILL random delay"
    - "CARGO_BIN_EXE_holt env var for integration tests calling the cargo-built binary"
    - "HOME=$tempdir override + tempfile::tempdir() for hermetic settings.json tests"
    - "BFS over `cargo metadata --format-version 1` for C4 dep-boundary enforcement"
    - "False-positive-negative guard (assert holt-cli CAN reach forbidden targets) in cli_dep_boundary"

key-files:
  created:
    - crates/holt-cli/src/install_hooks_cmd.rs
    - crates/holt-cli/src/install_hooks/diff.rs
    - crates/holt-cli/src/install_hooks/print.rs
    - crates/holt-cli/tests/install_hooks_smoke.rs
    - crates/holt-cli/tests/install_hooks_concurrent.rs
    - crates/holt-cli/tests/install_hooks_sigkill.rs
    - tests/cli_dep_boundary.rs
  modified:
    - crates/holt-cli/src/cli.rs (added InstallHooks { dry_run, print } variant)
    - crates/holt-cli/src/main.rs (mod install_hooks_cmd; + dispatch arm)
    - crates/holt-cli/src/install_hooks/mod.rs (pub mod diff/print + narrowed allow)
    - crates/holt-cli/src/install_hooks/merge.rs (cosmetic fmt only)
    - crates/holt-cli/tests/install_hooks_merge_smoke.rs (cosmetic fmt only)
    - crates/holt-cli/Cargo.toml (libc dev-dep)
    - Cargo.toml (registered [[test]] cli_dep_boundary)
    - Cargo.lock (libc 0.2 pin)

decisions:
  - D-11: hand-rolled common-prefix + common-suffix unified diff (no `similar` crate)
  - D-12: hand-rolled 2-space-indent JSON snippet emitter (no serde_json pretty)
  - D-13: clap-derived help; auto-rendered at 15 lines (well under 40-line cap)
  - D-14: out-of-process std::process::Command spawn for cross-process fs2 lock
  - D-15: libc::kill (not nix) for SIGKILL — keeps holt-cli dev-deps off nix
  - D-16: clap conflicts_with for --dry-run / --print mutual exclusion
  - D-17: <500ms release / <800ms debug budget (measured ~10-20ms warm)
  - C4 enforced by tests/cli_dep_boundary.rs (jsonc-parser + fs2 only in holt-cli)
  - Exit codes: 0=success, 1=merge/commit fail, 2=lock timeout, 3=io/parse pre-mutation
  - Argument order for jsonc_parser::parse_to_ast is (text, &CollectOptions, &ParseOptions) — plan snippets had it swapped; tests reflect the real 0.26.3 signature

metrics:
  duration: ~30 minutes
  completed: 2026-04-29
  task_commits: 3
  workspace_test_count_baseline: 59
  workspace_test_count_after: 75
  new_tests: 16 (7 smoke + 3 diff unit + 3 print unit + 1 concurrent + 1 sigkill + 1 cli_dep_boundary)
---

# Phase 3 Plan 03-02: install-hooks UX (CLI + escape hatches + stress + boundary) — Summary

The 6th binary entry point — `holt install-hooks` — is wired alongside Plan
03-01's merge library. Two escape hatches (`--dry-run` for unified diff,
`--print` for paste-ready snippet) ship with `conflicts_with` mutual-
exclusion. Two C3 falsifiability proofs (50× concurrent and 200× SIGKILL
atomicity) prove the lock + atomic-write contract under load. The C4
boundary test at workspace root pins `jsonc-parser` and `fs2` to
`crates/holt-cli/Cargo.toml` only — adding either dep to a sibling crate
will fail `cargo test --test cli_dep_boundary` in CI.

## Tasks Completed

| # | Name | Commit | Files | Tests |
| --- | --- | --- | --- | --- |
| 1 | clap subcommand + dispatcher + diff + print + smoke | `2da1c54` | 9 (4 src + 1 test + cli/main/mod/merge fmt) | 7 smoke + 3 diff unit + 3 print unit |
| 2 | 50× concurrent + 200× SIGKILL atomicity | `0e010fe` | 4 (2 tests + Cargo.toml + Cargo.lock) | 1 concurrent + 1 sigkill |
| 3 | tests/cli_dep_boundary.rs + workspace registration | `ae4141a` | 2 (1 test + workspace Cargo.toml) | 1 cli_dep_boundary |

Total: **16 new tests**. Workspace test count: **59 → 75**.

## Files Created (7) and Modified (8)

**Created (7):**
- `crates/holt-cli/src/install_hooks_cmd.rs` — D-16 dispatcher; lock → read → merge → (dry-run | print | commit)
- `crates/holt-cli/src/install_hooks/diff.rs` — D-11 hand-rolled unified diff
- `crates/holt-cli/src/install_hooks/print.rs` — D-12 hand-rolled 2-space-indent JSON snippet emitter
- `crates/holt-cli/tests/install_hooks_smoke.rs` — must_have-1, -2, -3 + D-13 + D-16 + D-17
- `crates/holt-cli/tests/install_hooks_concurrent.rs` — D-14 / must_have-4
- `crates/holt-cli/tests/install_hooks_sigkill.rs` — D-15 / must_have-5
- `tests/cli_dep_boundary.rs` — C4 / D-03 / D-05 workspace-root BFS gate

**Modified (8):**
- `crates/holt-cli/src/cli.rs` — added `InstallHooks { #[arg(long, conflicts_with="print")] dry_run, #[arg(long)] print }` variant
- `crates/holt-cli/src/main.rs` — declared `mod install_hooks_cmd;` + dispatch arm
- `crates/holt-cli/src/install_hooks/mod.rs` — `pub mod diff;` + `pub mod print;`; relaxed `#![allow(dead_code, unused_imports)]` to a narrower `#![allow(dead_code)]` + a localised `#[allow(unused_imports)]` on the detection-substr re-export
- `crates/holt-cli/src/install_hooks/merge.rs` — fmt-only cosmetic changes
- `crates/holt-cli/tests/install_hooks_merge_smoke.rs` — fmt-only cosmetic changes
- `crates/holt-cli/Cargo.toml` — `libc = "0.2"` added to `[dev-dependencies]`
- `Cargo.toml` (workspace root) — registered `[[test]] cli_dep_boundary`
- `Cargo.lock` — libc 0.2 pin

## ROADMAP Success-Criteria Mapping (verified end-to-end)

| # | Success criterion | Owning plan | Owning test | Status |
|---|---|---|---|---|
| 1 | clean fixture → expected merge + .holt.bak byte-equal | 03-02 | `cargo test -p holt-cli --test install_hooks_smoke must_have_1_clean_fixture_byte_equal_and_bak_present` | OK |
| 2 | line comments + key order preserved | 03-02 | `cargo test -p holt-cli --test install_hooks_smoke must_have_2_line_comments_and_key_order_preserved` | OK |
| 3 | --dry-run + --print exit 0, no mutation | 03-02 | `cargo test -p holt-cli --test install_hooks_smoke must_have_3_*` (2 tests) + `dry_run_and_print_are_mutually_exclusive` | OK |
| 4 | 50× concurrent stress, idempotent | 03-02 | `cargo test -p holt-cli --test install_hooks_concurrent --release concurrent_50x_idempotent_no_torn_writes` | OK (0.67s wall clock, budget 30s) |
| 5 | 200× SIGKILL atomicity | 03-02 | `cargo test -p holt-cli --test install_hooks_sigkill --release sigkill_200x_never_leaves_half_written_settings` | OK (4.16s wall clock, budget 60s) |

## C3 + C4 Falsifiability Evidence

**C3 (lock + fsync-before-rename + .holt.bak):**
- 50× concurrent test asserts: post-stress file parses with both serde_json + jsonc-parser; each holt command appears exactly once; user's pre-existing entry survives; no `.holt-tmp.<pid>` orphans. Removing the fs2 lock would cause concurrent merges to interleave and produce duplicate or malformed entries — the test would fail at criterion (b).
- 200× SIGKILL test asserts: every observed read parses with both engines AND state ∈ {pre-merge, canonical-post}. Removing fsync-before-rename would allow the rename(2) to expose a partial write under power-loss-equivalent SIGKILL conditions — the test would fail at criterion (c).

**C4 (JSONC handling lives only in holt-cli):**
- `tests/cli_dep_boundary.rs` BFS-walks `cargo metadata --format-version 1` from each of 5 non-cli workspace crates and asserts no path exists to `jsonc-parser` or `fs2`. False-positive-negative guard: also asserts holt-cli CAN reach both targets (so a regression dropping the dep doesn't silently pass).

## Hard-Constraint Preservation (C1, C2, C5, C6 + Plan 02 + Plan 03-01)

All Phase 1 + Phase 2 + Plan 03-01 hard constraints verified post-Plan 03-02:

| Constraint | Test | Status |
| --- | --- | --- |
| C1 — pipe stdio for supervised processes | `cargo test -p holt-supervisor --test chokepoint_audit` | OK (1/1) |
| C2 — `holt-render` does not depend on `holt-supervisor` | `cargo test --test architecture_dag` | OK (1/1) |
| C3 — atomic + locked settings.json mutation | `install_hooks_concurrent` + `install_hooks_sigkill` (this plan) | OK (1/1 each) |
| C4 — JSONC handling lives ONLY in holt-cli | `cargo test --test cli_dep_boundary` (this plan) | OK (1/1) |
| C5 — reader treats stale-or-corrupt as missing | `cargo test -p holt-schemas --test reader_contract` | OK (9/9) |
| C6 — render path never reads breaches.log/timings.jsonl | `cargo test -p holt-cli --test render_path_no_read` | OK (1/1) |
| Plan 02-01 SIGKILL atomicity for hooks | `cargo test -p holt-hooks --test sigkill_atomicity` | OK (1/1) |
| Plan 02-02 hook event handling | `cargo test -p holt-hooks --test handle_event_smoke` | OK (6/6) |
| Plan 03-01 merge library | `cargo test -p holt-cli --test install_hooks_merge_smoke` | OK (4/4) |
| Plan 03-01 fs2 lock | (unit tests in `install_hooks::lock`) | OK (4/4) |

## Forbidden-Crate Audit

```
$ cargo tree -i tokio
error: package ID specification `tokio` did not match any packages
   → no async runtime leaked

$ cargo tree -p holt-render | grep -c holt-supervisor
0
   → C2 still intact

$ for c in holt-schemas holt-supervisor holt-hooks holt-orchestrator holt-render; do
    grep '\b(jsonc-parser|fs2)\b' crates/$c/Cargo.toml
  done
   → empty (C4 boundary holds)

$ for c in holt-{schemas,supervisor,hooks,orchestrator,render,cli}; do
    grep -E '\b(tokio|simd-json|figment|chrono|owo-colors|supports-color|terminal_size|atomic-write-file|crossterm|wait-timeout|cargo_metadata)\b' crates/$c/Cargo.toml
  done
   → empty (forbidden list clean)
```

`libc = "0.2"` is the ONLY new dep in this plan. It is dev-only (under
`[dev-dependencies]` on `holt-cli`), already in the transitive graph via
`tempfile`/`fs2`, and is NOT on the forbidden list. Zero impact on the
release binary surface.

## Build + Lint Gates

```
cargo build --workspace --release           → clean
cargo fmt --check                            → clean
cargo clippy --workspace --all-targets -- -D warnings → clean
cargo test --workspace                      → 75 passed, 0 failed
target/release/holt install-hooks --help    → 15 lines (D-13 ≤40 cap)
holt install-hooks (warm release run)       → ~10-20ms wall clock
holt install-hooks --dry-run (warm)         → <10ms wall clock
```

The 7th smoke test (`d17_completes_within_500ms_in_release_or_800ms_in_debug`)
runs in debug-build mode under cargo test default and consistently lands
under the 800ms ceiling.

## Deviations from Plan

### API-DRIFT — jsonc_parser::parse_to_ast argument order

**Found during:** Task 2 (install_hooks_concurrent.rs first compile)

**Issue:** The plan's snippet for both `install_hooks_concurrent.rs` and
`install_hooks_sigkill.rs` invokes `jsonc_parser::parse_to_ast(bytes, &opts, &Default::default())`
where `opts` is a `&ParseOptions`. The actual jsonc-parser 0.26.3 signature
(verified at `~/.cargo/registry/src/.../jsonc-parser-0.26.3/src/parse_to_ast.rs:215`)
is:

```rust
pub fn parse_to_ast<'a>(
    text: &'a str,
    collect_options: &CollectOptions,
    parse_options: &ParseOptions,
) -> Result<ParseResult<'a>, ParseError>
```

So the call must be `jsonc_parser::parse_to_ast(bytes, &Default::default(), &parse_opts)`
with `&CollectOptions` second and `&ParseOptions` third. Plan-DRIFT note
documented in both new test files.

**Fix:** Reordered both call sites to match the real 0.26.3 signature. The
`ParseOptions` shape (`allow_comments=true, allow_trailing_commas=true,
allow_loose_object_property_names=false`) is unchanged from Plan 03-01's
merge.rs.

### Module-level `#![allow(dead_code, unused_imports)]` narrowed (Rule 1 — bug fix)

**Found during:** Task 1 (after wiring the dispatcher, the previous
crate-level allow was masking a real "unused re-export" warning on the
HOLT_HOOK_DETECTION_SUBSTR + HoltHookEntry symbols, both of which are
consumed only via `super::entries::*` from inside the install_hooks/
modules — they are NOT consumed through the `pub use` from
install_hooks_cmd.rs).

**Issue:** Plan 03-01 added `#![allow(dead_code, unused_imports)]` at
`install_hooks/mod.rs` to suppress warnings while the dispatcher was
deferred. With Plan 03-02 wiring the dispatcher, the broad allow would
mask any future regression.

**Fix:** Narrowed the module-level allow to `#![allow(dead_code)]` (kept
because `LockError::Io::path` and `CommitError::*::path` are used only
via `Display` impls on the `thiserror` derive — rustc may not always
count those as reads). Added a localised `#[allow(unused_imports)]` on
the detection-substr re-export only, with a comment explaining the
public-surface intent.

**Why this counts as Rule 1:** the broad allow masks the difference
between "this is a documented public API" and "this re-export has no
downstream consumer." Narrowing it makes the lint compass honest going
forward.

### `&PathBuf` → `&Path` in test helper (clippy::ptr_arg)

**Found during:** Task 1 first clippy pass

**Issue:** `fn write_settings(home: &PathBuf, name: &str) -> PathBuf` triggered
clippy's `ptr_arg` lint.

**Fix:** Changed signature to `fn write_settings(home: &Path, name: &str) -> PathBuf`
and added `Path` to the `use std::path::{Path, PathBuf}` import. All
callsites already pass `&dir.path().to_path_buf()` which auto-derefs to
`&Path`.

### Dispatcher placed at `crates/holt-cli/src/install_hooks_cmd.rs` (sibling to `install_hooks/`)

**Per plan:** the plan's must_have artifacts list shows
`crates/holt-cli/src/install_hooks_cmd.rs` as a sibling file alongside
the existing `install_hooks/` module directory. This was followed
exactly — it is NOT a deviation, but worth flagging since the alternative
(nesting under `install_hooks/cmd.rs`) would have been equally valid.
The plan's `<key_links>` block references `install_hooks_cmd::run` so
the path was load-bearing.

## Authentication Gates

None encountered.

## Pre-Existing Test Flake (NOT introduced by this plan)

Plan 03-01's SUMMARY noted that `hook_self_bench_smoke::hook_self_bench_json_has_expected_shape`
is environmentally flaky under heavy parallel cargo test load (p95 ~21–36ms vs
20ms budget). This plan's `cargo test --workspace` run (without `--test-threads=1`)
DID pass — 75 tests passed, 0 failed — likely because Plan 03-02 increases the
total test count to 75 but does not add any tests competing for the same
self-bench-style timing budget. The flake remains a pre-existing environmental
sensitivity in the existing self-bench harness; out of scope for this plan.

If CI surfaces it as a blocker for Phase 3 verification, a separate fix-plan
can add `--test-threads=1` to the CI invocation. Logged here for visibility.

## Threat Flags

No new threats introduced. Plan 03-01 already flagged `settings.json mutation`
under `crates/holt-cli/src/install_hooks/mod.rs::commit`. Plan 03-02 wires
that surface to a CLI subcommand and adds two stress tests that exercise
it under contention; the mitigations (fs2 lock, .holt.bak backup,
fsync-before-rename) are unchanged from Plan 03-01.

## Self-Check: PASSED

- [x] All 7 created files exist on disk and are git-tracked.
- [x] All 8 modified files are git-tracked and committed.
- [x] All 3 task commits exist in git log: `2da1c54`, `0e010fe`, `ae4141a`.
- [x] `cargo build --workspace --release` clean.
- [x] `cargo fmt --check` clean.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [x] `cargo test --workspace` reports 75 passing, 0 failed (vs 59 baseline = +16 new).
- [x] `cargo test --test cli_dep_boundary` passes.
- [x] `cargo test --test architecture_dag` still passes (C2 untouched).
- [x] Hard constraints C1, C2, C3, C4, C5, C6 all green.
- [x] `cargo tree -i tokio` empty; no async runtime leaked.
- [x] Forbidden-crate grep across all workspace crates returns empty.
- [x] `cargo tree -p holt-render | grep holt-supervisor` empty.
- [x] HOOK-07, HOOK-08, HOOK-09, HOOK-10 all implemented + tested.
- [x] `holt install-hooks --help` 15 lines (≤40-line cap), mentions --dry-run, --print, .holt.bak.

## Phase 3 Status

Phase 3 is complete. All 5 ROADMAP success criteria have runnable test
commands binding the release binary to expected behavior. C3 + C4 hard
constraints are empirically falsifiable in CI alongside Phase 1's C2
gate. The next action is `/gsd-verify-phase 3`.
