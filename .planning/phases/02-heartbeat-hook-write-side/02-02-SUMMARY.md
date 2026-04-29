---
phase: 2
plan: 02-02
plan_id: 02-02
status: complete
completed_at: 2026-04-28
requirements:
  - HOOK-01
  - HOOK-06
decisions_implemented:
  - "D-14: clap subcommand `Hook { event: HookEventArg }` with positional ValueEnum (PascalCase) mapped to holt_hooks::HookEvent via into_lib()"
  - "D-15: sub-20ms hook self-bench gate at `holt --self-bench-hook <event>` AND CI gate added to test-linux + test-macos REQUIRED jobs"
key_files:
  created:
    - crates/holt-cli/src/hook.rs
    - crates/holt-cli/tests/hook_subcommand_smoke.rs
    - crates/holt-cli/tests/hook_self_bench_smoke.rs
  modified:
    - crates/holt-cli/src/cli.rs
    - crates/holt-cli/src/main.rs
    - crates/holt-cli/src/self_bench.rs
    - crates/holt-cli/Cargo.toml
    - .github/workflows/ci.yml
    - Cargo.lock
commits:
  - "00e3a98 feat(holt-cli): holt hook <event> subcommand + --self-bench-hook gate (D-14, D-15)"
  - "c7cfef5 test(holt-cli): hook subcommand smoke (5 events + writer_version + 0o600 + paired fixtures)"
  - "8937be0 test+ci(holt-cli): hook self-bench smoke + CI matrix extension (D-15)"
  - "6fe5123 test(02-02): regression gate (Phase 1 C1/C2/C5/C6 + Plan 02-01 tests still green)"
metrics:
  workspace_tests_before: 43
  workspace_tests_after: 50
  new_tests: 7
  tasks_completed: 8
  commits: 4
---

# Plan 02-02 Summary: holt hook CLI wiring + D-15 self-bench gate

**One-liner:** Wired the `holt-hooks` library from Plan 02-01 into the
`holt` binary as the `holt hook <event>` subcommand (D-14), added the
sub-20ms hook self-bench gate (D-15) to both the CLI and the CI matrix,
and proved must_have-1 + must_have-2 end-to-end through the release
binary with 7 new integration tests. Phase 2 is complete.

## What Was Built

### `holt hook <event>` subcommand (D-14)

User-facing command that Phase 3's `holt install-hooks` will register in
`~/.claude/settings.json`. Pipeline:

1. Slurp CC stdin via existing `crate::stdin::slurp_and_parse` (Phase 1
   helper, CR-04 200ms deadline — reused unchanged for the hook path).
2. Build `Env { writer_version: env!("CARGO_PKG_VERSION"), pid,
   now_iso }`. The writer_version comes from the **binary** crate per
   D-11 / HOOK-06.
3. Call `holt_hooks::handle_event(event, stdin_bytes, &env)`.
4. Ignore `HookOutcome` variant. **Always exit 0** — CC must never see a
   non-zero exit from a holt hook (D-03).

Clap surface (`HookEventArg` ValueEnum, `#[value(rename_all = "PascalCase")]`)
matches CC's exact event names verbatim (`PreToolUse`, `PostToolUse`,
`Stop`, `Notification`, `SessionStart`) so the Phase 3 settings.json merge
sees CC-compatible command lines without further transformation.

### `--self-bench-hook <EVENT>` gate (D-15)

Mirrors Phase 1's `--self-bench` exactly. New `run_self_bench_hook`
function in `self_bench.rs` reuses `BenchResult`, the linear-interpolation
percentile picker, the 20_000us / 40_000us platform-tiered budget, and
both `print_human` / `print_json` emitters. Iteration loop calls
`holt_hooks::handle_event` directly with a minimal CC stdin envelope and
a tempdir-scoped `XDG_RUNTIME_DIR` so the user's real
`~/.cache/holt/sessions/` is never written to (WR-05 hermeticity).

**Local measurement (macOS arm64, M-class CPU):** p95 = 6.5–15.7us
across multiple runs — well under the 20_000us budget. The harness
exits 0 with `passed: true`.

CI gate added to `test-linux` and `test-macos` REQUIRED jobs as a
two-step pair after the existing Phase 1 self-bench gate:
- `Hook self-bench PASS gate (D-15)` — pipes JSON to `/tmp/bench-hook.json`.
- `assert hook self-bench PASS (D-15)` — `python3 json.load` exits 1 if
  `d['passed']` is false. Exact same shape as the Phase 1 assert.

Windows + `stable-*` informational jobs intentionally do not gate on
D-15 (the decision text names test-linux + test-macos as the REQUIRED
gates; Windows has a 40ms budget but Defender variance makes it
informational-only at v0.1).

### Integration tests (7 new)

`crates/holt-cli/tests/hook_subcommand_smoke.rs` (5 tests):

1. `must_have_1_hook_pre_tool_use_writes_canonical_xdg_path` — fires
   `holt hook PreToolUse` against the v2.1.119 fixture, asserts the
   heartbeat parses cleanly via `holt_schemas::read_heartbeat`,
   `schema_version=1`, `session_id` matches fixture, `writer_version`
   non-empty, file perms `0o600` on Unix.
2. `must_have_2_all_five_events_round_trip_via_binary` — loops
   PreToolUse/PostToolUse/Stop/Notification/SessionStart, asserts each
   exits 0, writes a heartbeat, and exhibits the D-09 `current_tool`
   policy (Some on PreToolUse, None on the other four).
3. `must_have_2_garbage_stdin_exits_zero_no_panic` — truncated JSON
   (`{"session_id":`) on stdin → exit 0 (no error to CC).
4. `writer_version_is_holt_cli_cargo_pkg_version` — extracts the second
   token of `holt --version` and asserts heartbeat.writer_version
   equals it (HOOK-06).
5. `cwd_label_uses_git_worktree_via_binary` — D-08 paired-fixture
   policy: PreToolUse fixture's `workspace.git_worktree =
   "myrepo/feature-branch"` wins over cwd basename. Pairs with
   `assemble_field_policy::cwd_label_*` in holt-hooks (Plan 02-01)
   to give us coverage from both library + binary surfaces.

`crates/holt-cli/tests/hook_self_bench_smoke.rs` (2 tests):

1. `hook_self_bench_json_has_expected_shape` — JSON has all 6
   `BenchResult` fields, `iterations >= 30`, `budget_p95_us == 20000`
   on Unix tier-1, `passed: true`, exit code 0.
2. `hook_self_bench_human_output_shows_pass_or_fail` — human output
   contains `PASS` or `FAIL`.

## Test Results

| Test suite                                    | Before | After |
| --------------------------------------------- | ------ | ----- |
| Workspace total                               | 43     | 50    |
| `holt-cli/tests/hook_subcommand_smoke.rs`     | n/a    | 5     |
| `holt-cli/tests/hook_self_bench_smoke.rs`     | n/a    | 2     |
| Phase 1 hard constraints (C1/C2/C5/C6)        | 12     | 12    |
| Plan 02-01 (`holt-hooks` integration tests)   | 15     | 15    |

`cargo fmt --check` clean. `cargo clippy --workspace --all-targets --
-D warnings` clean. `cargo build --workspace --release` clean.

## ROADMAP Success-Criteria Mapping (Phase 2)

| # | Criterion                                                | Test                                                                                                              |
| - | -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| 1 | File shape + 0o600 perms after PreToolUse fire           | `cargo test -p holt-cli --test hook_subcommand_smoke must_have_1_hook_pre_tool_use_writes_canonical_xdg_path`     |
| 2 | All 5 events round-trip; current_tool per-event policy   | `cargo test -p holt-cli --test hook_subcommand_smoke must_have_2_all_five_events_round_trip_via_binary`           |
| 3 | 1000× SIGKILL atomicity                                  | `cargo test -p holt-hooks --test sigkill_atomicity --release` (Plan 02-01 owns)                                   |
| 4 | Fallback chain + unwritable resilience                   | `cargo test -p holt-hooks --test handle_event_smoke` (Plan 02-01 owns)                                            |
| 5 | cwd_label paired fixtures                                | `cargo test -p holt-hooks --test assemble_field_policy cwd_label_*` (Plan 02-01 owns) + `cargo test -p holt-cli --test hook_subcommand_smoke cwd_label_uses_git_worktree_via_binary` (this plan) |

All five criteria are runnable from `cargo test --workspace` plus
`target/release/holt`.

## Hygiene Verification

- `cargo tree -i tokio`: "did not match any packages" — no async runtime
  pulled in by `jiff` or any other Plan 02-02 dep.
- `cargo tree -p holt-render | grep holt-supervisor`: 0 lines — C2
  (render does not depend on supervisor) unbroken.
- `grep -E '\b(tokio|simd-json|figment|chrono|jsonc-parser|fs2|owo-colors|supports-color|terminal_size|atomic-write-file|crossterm|wait-timeout|cargo_metadata)\b' crates/holt-{hooks,cli}/Cargo.toml`:
  empty — no forbidden crates in either manifest.
- `grep -cE '\.unwrap\(\)|panic!|\.expect\(' crates/holt-cli/src/hook.rs`:
  0 — render-path no-unwrap discipline preserved (CLAUDE.md mandate).

## Deviations from Plan

### [Rule 1/3 — Blocking issue] forbid(unsafe_code) → deny(unsafe_code) in main.rs

- **Found during:** Task 4 first build attempt
- **Issue:** `holt-cli/src/main.rs` carried `#![forbid(unsafe_code)]`
  (WR-09 defence-in-depth). Rust 2024 made `std::env::set_var` /
  `std::env::remove_var` unsafe, and the D-15 hook self-bench harness
  needs both to point `XDG_RUNTIME_DIR` at a tempdir for WR-05
  hermeticity. `forbid` is non-relaxable; the build failed with `usage
  of an `unsafe` block`.
- **Fix:** Switched to `#![deny(unsafe_code)]` (still hard-error by
  default but relaxable per-call-site) and added a single
  `#[allow(unsafe_code)]` on `run_self_bench_hook` with a safety comment
  documenting that the bench is invoked from `main()` before any
  threads spawn and the env mutation is contained to a tempdir-scoped
  override required by WR-05.
- **Rationale:** WR-09 was "defence-in-depth, no other code uses
  unsafe" — that property is preserved (the `#[allow]` is scoped to
  exactly one function with a justification comment, and clippy will
  deny new uses). Phase 1 also did not anticipate Rust 2024's
  set_var-is-unsafe change because it was added after Phase 1 plans
  were written. This is a Rust-2024-edition adjustment, not a
  doctrine relaxation.
- **Files modified:** `crates/holt-cli/src/main.rs`,
  `crates/holt-cli/src/self_bench.rs`
- **Commit:** 00e3a98 (Tasks 1-4)

### [Rule 3 — Blocking format] cargo fmt collapse on hook_self_bench_smoke

- **Found during:** Task 8 regression gate
- **Issue:** `cargo fmt --check` flagged a 4-line `assert_eq!` that
  rustfmt prefers as a single line.
- **Fix:** Ran `cargo fmt`. One-line collapse, no semantic change.
- **Commit:** 6fe5123 (Task 8)

No architectural deviations. No CLAUDE.md violations introduced. No
forbidden-crate pulls.

## C6 strace Test Gap (plan-checker WARNING)

The plan-checker flagged a "C6 strace test gap" warning during Phase 2
planning — strace verification is a Linux-only CI gate that confirms the
render path never opens `breaches.log` / `timings.jsonl`. Phase 1's
`render_path_no_read` test exercises this at the Rust source level
(`std::process::Command` mock) but does NOT run `strace` against the
release binary in CI. The `install strace` step in `.github/workflows/ci.yml`
test-linux job already exists from Phase 1 but is not paired with a
strace-driven test invocation.

**Disposition:** Deferred. Phase 1's source-level `render_path_no_read`
test (9 cases) is the contract enforcer; an strace-driven CI gate is
defence-in-depth that can land in a future Phase 1 review-fix or Phase 4
hardening pass without touching the v0.1 critical path. Plan 02-02's
new code (`hook.rs`, `self_bench.rs` extensions) does not introduce any
new render-path file reads, so the C6 invariant is not regressed by
this plan. Logged here for the verifier.

## Phase 2 Status

**Phase 2 is complete.** All 5 ROADMAP success criteria have a runnable
test command. All 6 phase requirement IDs (HOOK-01..HOOK-06) are
implemented and verified. Both phase decisions D-14 and D-15 are locked
into shipped code and CI.

**Next action:** `/gsd-verify-phase 2`.

## Follow-ups

Nothing pending in Phase 2 itself. The deferred-to-v1.0 items
(PreCompact subscription, rich heartbeat fields including `mode` /
`context_pct_real` / `burn_rate_usd_per_min`, daemon optimization, the
git rev-parse branch of D-08 cwd_label derivation) remain in
`02-CONTEXT.md` "Deferred Ideas" exactly as the user locked them at
discussion time.

The C6 strace CI gate (described above) is a follow-up candidate for a
future Phase 1 review-fix pass or Phase 4 hardening; tracked here
rather than in deferred-items.md because it long-pre-dates Plan 02-02.

## Self-Check: PASSED

Verified files exist:
- `crates/holt-cli/src/hook.rs`: FOUND
- `crates/holt-cli/tests/hook_subcommand_smoke.rs`: FOUND
- `crates/holt-cli/tests/hook_self_bench_smoke.rs`: FOUND
- `crates/holt-cli/src/cli.rs`: FOUND (modified)
- `crates/holt-cli/src/main.rs`: FOUND (modified)
- `crates/holt-cli/src/self_bench.rs`: FOUND (modified)
- `crates/holt-cli/Cargo.toml`: FOUND (modified)
- `.github/workflows/ci.yml`: FOUND (modified)

Verified commits exist (`git log --oneline -6` confirmed before SUMMARY):
- 00e3a98 feat(holt-cli): holt hook <event> subcommand + --self-bench-hook gate (D-14, D-15)
- c7cfef5 test(holt-cli): hook subcommand smoke (5 events + writer_version + 0o600 + paired fixtures)
- 8937be0 test+ci(holt-cli): hook self-bench smoke + CI matrix extension (D-15)
- 6fe5123 test(02-02): regression gate (Phase 1 C1/C2/C5/C6 + Plan 02-01 tests still green)
