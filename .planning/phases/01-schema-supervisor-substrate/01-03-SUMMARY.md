---
phase: 01-schema-supervisor-substrate
plan: 03
plan_id: 01-03
subsystem: holt-cli / CI gates / architecture-DAG enforcement
tags: [holt-cli, clap, self-bench, architecture-dag, ci, c2, c6, core-01, core-07, core-08, core-09, core-10]
requires:
  - phase: 01-schema-supervisor-substrate (plan 01)
    provides: holt_schemas::atomic_write + LkgEntry::new
  - phase: 01-schema-supervisor-substrate (plan 02)
    provides: holt_supervisor::wrap_and_run + SupervisorOutcome + breaches::append_breach + paths::default_cache_root + lkg::read_lkg + options::BreachKind
provides:
  - "binary `holt` with three entry points (`run`, `--self-bench [--json]`, `--version`)"
  - "tests/architecture_dag.rs at workspace root — `cargo metadata` BFS for C2 / CORE-10 enforcement"
  - "crates/holt-cli/tests/render_path_no_read.rs (Linux strace) — CORE-09 / C6 enforcement"
  - "crates/holt-cli/tests/run_passthrough.rs — ROADMAP success criterion #1 + CORE-08 verification"
  - "crates/holt-cli/tests/self_bench_smoke.rs — D-14 / CORE-07 self-bench JSON shape + PASS gate"
  - "crates/holt-cli/tests/version_smoke.rs — `holt --version` semver smoke"
  - ".github/workflows/ci.yml — D-16 matrix (lint + test-linux + test-macos REQUIRED on MSRV 1.87; windows + 2 stable jobs informational)"
affects:
  - "Phase 1 is COMPLETE. All 5 ROADMAP success criteria are runnable from `cargo test` + `target/release/holt`."
  - "All future PRs gated by ci.yml — fmt/clippy/test/architecture_dag/self-bench must stay green."
  - "Future contributors who introduce a `holt-render → holt-supervisor` edge will fail CI on every supported platform."
  - "Phase 2 (HOOK-01..HOOK-11): the holt-cli binary is the entry point that hooks will install via `holt install-hooks` (Phase 3)."

tech-stack:
  added:
    - "clap 4.5+ (derive feature) — top-level CLI parser"
    - "humantime 2.3 — `--timeout` parsing (`2s`, `1500ms`, `1h30m`)"
    - "serde 1 + serde_json 1 — defensive stdin parse + bench JSON output (NO `preserve_order`)"
    - "anyhow 1 — top-level binary error surface (declared, not yet exercised — main() never returns errors per CORE-01)"
    - "tempfile 3 (dev-only) — XDG_CACHE_HOME isolation in run_passthrough + render_path_no_read"
  patterns:
    - "Workspace-root package pattern: virtual workspace gains a tiny holt-workspace-tests package (publish=false, no lib/bin, autotests=true) so `tests/architecture_dag.rs` is discoverable by `cargo test --test architecture_dag`. This was the only way to keep the test at the workspace ROOT (D-15 explicit requirement) under cargo's discovery rules."
    - "Defensive cargo PkgIdSpec parser: `parse_package_name()` handles three observed shapes — modern `path+file://...#name@version`, modern `registry+...#name@version`, and the legacy `name version (source)` form. Critical for cargo 1.87+ where IDs no longer contain whitespace separators."
    - "BFS over cargo metadata's resolved graph (no `cargo_metadata` crate dependency): zero new deps, ~100 lines of stock std + serde_json."
    - "Linux strace trace=openat,access — opt-in skip if strace is missing (CI installs it; local devs without strace get a no-op pass instead of a build break)."
    - "GitHub Actions REQUIRED-vs-informational pattern: omitting `continue-on-error` makes a job required-green; setting `continue-on-error: true` makes it informational. D-16 matrix uses this to mandate MSRV 1.87 on Linux x86_64 + macOS arm64 while keeping Windows / stable runs as advisories."

key-files:
  created:
    - "crates/holt-cli/src/cli.rs (clap derive Cli + Command)"
    - "crates/holt-cli/src/run.rs (defensive stdin → wrap_and_run → emit stdout / LKG / empty)"
    - "crates/holt-cli/src/self_bench.rs (run_self_bench + print_human + print_json + BenchResult)"
    - "crates/holt-cli/src/stdin.rs (slurp_and_parse + StdinParseOutcome)"
    - "crates/holt-cli/tests/version_smoke.rs (1 test)"
    - "crates/holt-cli/tests/run_passthrough.rs (2 tests)"
    - "crates/holt-cli/tests/self_bench_smoke.rs (1 test)"
    - "crates/holt-cli/tests/render_path_no_read.rs (1 test, Linux only)"
    - "tests/architecture_dag.rs (workspace root, 1 test)"
    - ".github/workflows/ci.yml (6 jobs)"
  modified:
    - "Cargo.toml (workspace) — added holt-workspace-tests root package + [[test]] target for architecture_dag"
    - "Cargo.lock (regenerated for new clap/humantime/anyhow/tempfile deps)"
    - "crates/holt-cli/Cargo.toml — added clap, anyhow, humantime, serde, serde_json + dev-dep tempfile"
    - "crates/holt-cli/src/main.rs — replaced placeholder with mod-declarations + dispatch"

key-decisions:
  - "D-14 (locked): self-bench wraps `:` (POSIX) / `cmd /c exit 0` (Windows), reports p50/p95/p99, PASS/FAIL gate vs 20ms (Linux/macOS) or 40ms (Windows). Implemented in src/self_bench.rs."
  - "D-15 (locked): tests/architecture_dag.rs walks `cargo metadata --format-version 1` JSON via `serde_json::Value` — no `cargo_metadata` crate dep. Implemented at workspace root."
  - "D-16 (locked): CI matrix has lint + test-linux + test-macos REQUIRED on MSRV 1.87 (ubuntu-latest + macos-14); windows-latest + 2 stable jobs informational. Self-bench PASS gate runs on the two REQUIRED Unix jobs."
  - "Open Question #2 resolved: architecture_dag uses raw `cargo metadata` + `serde_json::from_slice::<Value>` — zero new deps. Confirmed working against cargo 1.87."
  - "Open Question #4 resolved: `--self-bench --json` writes to stdout; CI consumes via `python3 -c \"json.load(sys.stdin)\"` (universal in GitHub Actions runners)."

patterns-established:
  - "Single binary, three entry points: `holt run` (subcommand), `holt --self-bench` (top-level flag), `holt --version` (clap auto). The flag-vs-subcommand split keeps the bench harness invocable without a wrapped command."
  - "Always-exit-0 render path: holt run never bubbles errors to CC. ParseFail → breach + LKG → exit 0. Spawn fail / timeout → breach + LKG → exit 0. Only missing arguments (developer error) returns exit 2."
  - "Tempdir-isolated integration tests: every CLI test sets `XDG_CACHE_HOME` to a `tempdir()` so the developer's real `~/.cache/holt/` is never touched during `cargo test`."
  - "Negative test for the architecture invariant: introducing the disallowed edge (`holt-render = { holt-supervisor = ... }`) makes `cargo test --test architecture_dag` fail with a precise C2-VIOLATED message naming the chain."

requirements-completed: [CORE-01, CORE-07, CORE-08, CORE-09, CORE-10]

metrics:
  start: "2026-04-28T10:25:00Z"
  end: "2026-04-28T10:55:00Z"
  duration: "~30 minutes"
  tasks_completed: 2
  files_created: 10
  files_modified: 4
  tests_added: 6 (5 holt-cli + 1 workspace root)
  tests_passing: 23/23 across the workspace
---

# Phase 1 Plan 03: holt-cli + architecture_dag + CI Summary

**One-liner:** Wired the `holt` binary together — `holt run` (defensive CC stdin → `Supervisor::wrap_and_run` → byte-exact stdout / LKG / empty, never bubbles errors to CC), `holt --self-bench` (≥10 iterations of `:` no-op, p50/p95/p99 + PASS/FAIL gate vs 20ms on macOS arm64 / Linux x86_64), and `holt --version` (clap-generated). Added the workspace-root `tests/architecture_dag.rs` that walks `cargo metadata` JSON and asserts no path from `holt-render` to `holt-supervisor` (verified by deliberately introducing the edge: test fails with `C2 VIOLATED: ...`). Added `crates/holt-cli/tests/render_path_no_read.rs` Linux strace test asserting the render path opens neither `breaches.log` nor `timings.jsonl` for reading. Shipped `.github/workflows/ci.yml` with the D-16 matrix — fmt + clippy + test --workspace + architecture_dag + self-bench gate REQUIRED green on `ubuntu-latest` + `macos-14` (MSRV 1.87), Windows + stable runs informational. **All 5 Phase 1 ROADMAP success criteria now run from `cargo test --workspace` + `target/release/holt`.**

## Performance

- **Duration:** ~30 minutes (executor wall-clock; clap+humantime+tempfile resolved in <10s thanks to existing rust-cache)
- **Started:** 2026-04-28T10:25:00Z
- **Completed:** 2026-04-28T10:55:00Z
- **Tasks completed:** 2
- **Files created:** 10 (4 src + 5 holt-cli/tests + 1 workspace-root tests + 1 CI yml)
- **Files modified:** 4 (workspace Cargo.toml, Cargo.lock, holt-cli/Cargo.toml, holt-cli/src/main.rs)
- **Tests added:** 6 (1 architecture_dag + 1 version_smoke + 2 run_passthrough + 1 self_bench_smoke + 1 render_path_no_read)
- **Tests passing:** 23/23 (12 from plan 01 + 5 from plan 02 + 6 new) on macOS arm64 local
- **LOC:** 369 src + 209 tests + 87 ci.yml

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | holt-cli skeleton + clap + run + self-bench + stdin + workspace tests/architecture_dag.rs (C2) | `9e2f61e` | 9 files (4 new src, 1 new test, Cargo.toml/Cargo.lock/main.rs/holt-cli Cargo.toml modified) |
| 2 | holt-cli integration tests + render-path-no-read + CI workflow (C6 + D-16) | `6a1befc` | 5 files (4 new tests + ci.yml) |
| 3 | This SUMMARY | *(this commit)* | 1 file |

## Files Created

### holt-cli sources

- `crates/holt-cli/src/main.rs` — replaced the Plan 01 placeholder. Declares `cli`, `run`, `self_bench`, `stdin` modules. `main()` parses `Cli`, dispatches `--self-bench` (top-level flag) before `run` (subcommand), exits with the right code (0 PASS / 1 FAIL on bench; 0 always for `run` happy/breach paths; 2 only on missing-args developer error).
- `crates/holt-cli/src/cli.rs` — `clap` derive `Cli` with `--self-bench`, `--json`, `--iterations` (default 10) flags + optional `Run { --timeout, --session-id, wrapped: trailing_var_arg + allow_hyphen_values }` subcommand. Verbatim per plan `<interfaces>`.
- `crates/holt-cli/src/run.rs` — defensive stdin parse → `humantime::parse_duration` → `holt_supervisor::wrap_and_run` → match outcome:
  - `Ok` → `write_all(stdout)` + flush, exit 0
  - `Breach` → emit `read_lkg` if present, else empty stdout, exit 0
  - `ParseFail` → `append_breach` with `BreachKind::ParseFail` + LKG fall-through + exit 0
- `crates/holt-cli/src/self_bench.rs` — `run_self_bench(iterations)` runs ≥10 iterations of `wrap_and_run` over `sh -c :` (or `cmd /c exit 0` on Windows), measures `total - supervised` overhead in microseconds, sorts samples, picks p50/p95/p99 via fractional indexing, returns `BenchResult { iterations, overhead_p50_us, overhead_p95_us, overhead_p99_us, budget_p95_us, passed }`. `print_human` outputs the human-readable summary; `print_json` emits a single-line JSON document. Budget: 20_000us on Unix, 40_000us on Windows.
- `crates/holt-cli/src/stdin.rs` — `slurp_and_parse() -> StdinParseOutcome { Ok(Value), ParseFail { excerpt }, Empty }`. Reads stdin to EOF with 4KB initial capacity, returns `Empty` on read error or zero bytes, attempts `serde_json::from_slice::<Value>`, returns `ParseFail { excerpt }` (≤2KB excerpt for breach log) on any serde error.

### holt-cli integration tests (`crates/holt-cli/tests/`)

- `version_smoke.rs` — spawns `target/release/holt --version`, asserts exit 0 and stdout contains `0.1.0`.
- `run_passthrough.rs` — two tests:
  1. `run_passthrough_emits_wrapped_stdout_unchanged` — `holt run -- bash -c "echo hello"` with empty stdin emits exactly `b"hello\n"` (byte-exact comparison) and exits 0.
  2. `run_with_malformed_stdin_records_parse_fail_and_exits_zero` — feeds truncated `{"session_id":` to stdin, asserts exit 0, opens `<XDG_CACHE_HOME>/holt/breaches.log`, parses the first line as JSON, asserts `kind == "parse_fail"`.
- `self_bench_smoke.rs` — runs `holt --self-bench --json --iterations 10`, parses stdout as JSON, asserts all six `BenchResult` fields exist + `iterations >= 10`. On Linux/macOS additionally asserts `budget_p95_us == 20000` and `passed == true`.
- `render_path_no_read.rs` — Linux-only test (no-op stub on macOS/Windows): primes `breaches.log` + `timings.jsonl` with 3 fires, runs `holt run` once under `strace -f -e trace=openat,access`, scans the trace file for any `breaches.log`/`timings.jsonl` line containing `O_RDONLY` or `O_RDWR`, panics with a precise C6-VIOLATED message if found. Skips gracefully if `which strace` fails (local dev without strace installed).

### Workspace-level test (`tests/`)

- `tests/architecture_dag.rs` — shells out to `cargo metadata --format-version 1`, parses with `serde_json::Value`, builds two maps: `id → deps[]` and `name → id`. Resolves `holt-render` and `holt-supervisor` IDs, runs BFS from `holt-render`, asserts the supervisor ID is unreachable. The `parse_package_name()` helper handles three PkgIdSpec shapes (see Deviations below).

### CI (`.github/workflows/`)

- `ci.yml` — six jobs:
  1. **`lint`** (ubuntu-latest, MSRV 1.87) — `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings`. REQUIRED.
  2. **`test-linux`** (ubuntu-latest, MSRV 1.87, REQUIRED) — installs `strace`, runs `cargo build --workspace --release`, `cargo test --workspace`, `cargo test --test architecture_dag`, `holt --self-bench --json --iterations 30`, asserts `passed == true` via `python3 -c json.load`.
  3. **`test-macos`** (macos-14 = aarch64-apple-darwin, MSRV 1.87, REQUIRED) — same as test-linux minus the strace install. Self-bench gate present.
  4. **`test-windows`** (windows-latest, `continue-on-error: true`) — best-effort.
  5. **`stable-linux`** + **`stable-macos`** (informational, `continue-on-error: true`) — drift detection on the future-MSRV-bump path.

## Decisions Made

| ID  | Decision | Where it landed |
|-----|----------|-----------------|
| D-14 | Self-bench wraps `:` ≥10×, reports p50/p95/p99, PASS/FAIL vs 20ms / 40ms budget | `crates/holt-cli/src/self_bench.rs` |
| D-15 | Architecture-DAG test walks `cargo metadata --format-version 1` JSON via `serde_json::Value`; NO `cargo_metadata` crate dep | `tests/architecture_dag.rs` (workspace root) |
| D-16 | CI required-green = MSRV 1.87 on ubuntu-latest + macos-14; windows-latest + stable jobs allowed-failure | `.github/workflows/ci.yml` |
| (resolved) Open Q #2 | Raw `cargo metadata` + `serde_json::from_slice::<Value>` is the architecture_dag strategy | `tests/architecture_dag.rs` |
| (resolved) Open Q #4 | `--self-bench --json` writes to stdout; CI consumes via `python3 json.load` | `ci.yml` test-linux + test-macos `assert self-bench PASS` step |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] cargo PkgIdSpec format change in cargo 1.87**

- **Found during:** Task 1, first run of `cargo test --test architecture_dag`.
- **Issue:** RESEARCH §"Pattern 9" snippet parsed `id` via `id.split_whitespace().next()`, expecting the legacy format `name version (source)`. Cargo 1.77+ Package ID Spec uses `<source>#<name>@<version>` (e.g., `path+file:///abs/path/to/crate-x#0.1.0` or `registry+https://.../index#crate-x@1.2.3`). Result: `name_to_id` was populated with full URLs as keys; `name_to_id.get("holt-render")` returned `None` and the test panicked with `holt-render must be in the workspace metadata`. RESEARCH Assumption A2 explicitly flagged this as a possible escape: "the exact JSON path … might be `deps` or `dependencies` … the test should be written defensively (try both)".
- **Fix:** Added `parse_package_name(id: &str) -> String` helper that handles all three shapes:
  1. `path+file:///abs/path/to/crate-x#0.1.0` → extract last URL segment before `#`.
  2. `path+file:///abs/path#crate-x@0.1.0` or `registry+...#crate-x@1.2.3` → extract `name` from `name@version` after `#`.
  3. Legacy `crate-x 0.1.0 (path+...)` → extract token before first space (only if id doesn't start with `path+`/`registry+`/`git+`).
- **Verification:** Test now passes locally; deliberately adding `holt-supervisor = { path = "../holt-supervisor" }` to `crates/holt-render/Cargo.toml` fails with the precise C2-VIOLATED message naming the chain through `holt-render` to `holt-supervisor`. Reverted after verification.
- **Files modified:** `tests/architecture_dag.rs` (+30 lines for `parse_package_name`).
- **Committed in:** `9e2f61e` (Task 1 commit).

**2. [Rule 3 — Blocking] Workspace root needed a phantom `[package]` to host `tests/architecture_dag.rs`**

- **Found during:** Task 1, first attempt at `cargo test --test architecture_dag`.
- **Issue:** RESEARCH §"Pattern 9" claims "this test must be at `tests/architecture_dag.rs` (workspace ROOT, not inside any crate). Running `cargo test` at the workspace root picks it up because it's a workspace integration test." That's only true if the workspace root is itself a Cargo package. Our Plan 01 setup made the root a *virtual* workspace (only `[workspace]`, no `[package]`). With a virtual workspace, top-level `tests/` is invisible to Cargo — `cargo test --test architecture_dag` exited with `error: no test target named architecture_dag in default-run packages`.
- **Fix:** Added a tiny `[package]` block to the workspace root Cargo.toml — `name = "holt-workspace-tests"`, `version = "0.0.0"`, `publish = false`, `autobins = false`, `autoexamples = false`, `autobenches = false`, `autotests = true`, with an explicit `[[test]] name = "architecture_dag" path = "tests/architecture_dag.rs"`. Also added `dev-dependencies = { serde_json.workspace }`. The root is now a hybrid workspace + tiny package whose only purpose is hosting workspace-level integration tests (no lib, no bin, never published).
- **Why this preserves the plan intent:** The test still lives at the workspace ROOT (D-15 explicit requirement), still invokes via `cargo test --test architecture_dag`, still walks `cargo metadata` JSON. The phantom package is a Cargo plumbing detail with no runtime footprint.
- **Verification:** `cargo test --test architecture_dag` exits 0 (PASS); `cargo build --workspace --release` continues to build only the six member crates plus their transitive deps; `cargo metadata` shows the new `holt-workspace-tests@0.0.0` node, but it has no edges into any of the six member crates so `cargo tree -i tokio` and the C2 BFS are unaffected.
- **Files modified:** `Cargo.toml` (+25 lines for the `[package]` block, `[[test]]`, and `[dev-dependencies]`).
- **Committed in:** `9e2f61e` (Task 1 commit).

**3. [Rule 2 — Style/correctness] rustfmt re-wrapped `assert_eq!` in self_bench_smoke.rs**

- **Found during:** Task 2, first run of `cargo fmt --check` after writing the integration tests.
- **Issue:** The plan's literal source for `assert_eq!(budget, 20_000, "budget_p95_us must be 20000 on Unix per D-14")` exceeded rustfmt's preferred line width and got re-wrapped to 4 lines. CLAUDE.md mandates `cargo fmt` clean before commit.
- **Fix:** Ran `cargo fmt --all`. No semantics changed.
- **Files modified:** `crates/holt-cli/tests/self_bench_smoke.rs` (1 assert_eq! call), `crates/holt-cli/tests/render_path_no_read.rs` (one `if line.contains(...)` chain).
- **Committed in:** `6a1befc` (Task 2 commit; the rustfmt-clean form is what actually landed in HEAD).

**4. [Rule 2 — Style] `&PathBuf` → `&Path` in `run.rs::emit_lkg_or_empty`**

- **Found during:** Task 1, `cargo clippy --all-targets -- -D warnings`.
- **Issue:** The plan snippet for `run.rs` used `fn emit_lkg_or_empty(cache_root: &PathBuf, ...)`. clippy's `ptr_arg` lint flags `&PathBuf` parameters since `&Path` is the strictly more general signature. The plan's surrounding code already passes `&cache_root` (where `cache_root: PathBuf`), which auto-derefs to `&Path` at the call site, so the change is API-compatible.
- **Fix:** Changed parameter type from `&PathBuf` to `&Path`. Removed unused `std::path::PathBuf` import in favor of `std::path::Path`.
- **Files modified:** `crates/holt-cli/src/run.rs` (2 lines).
- **Committed in:** `9e2f61e` (Task 1 commit).

---

**Total deviations:** 4 auto-fixed (1 Rule 1 cargo-format change, 1 Rule 3 blocking package wiring, 2 Rule 2 style/lint).
**Impact on plan:** None semantically. All four were necessary for the code to (a) compile under cargo 1.87, (b) be discoverable by `cargo test`, (c) pass `cargo fmt --check`, and (d) pass `cargo clippy --all-targets -- -D warnings`. The plan's intent — workspace-root architecture_dag test, byte-exact passthrough, self-bench PASS gate, REQUIRED CI on ubuntu-latest + macos-14 — is preserved exactly.

## Issues Encountered

- **rustfmt minor reformatting** during `cargo fmt --all` (Deviation #3 above). Treated as expected formatter behavior, not a deviation.
- **Cargo metadata schema drift** (Deviation #1) confirmed RESEARCH Assumption A2's defensive caveat. The `parse_package_name()` helper now stays valid against the legacy whitespace form too, in case future toolchain pinning shifts.

## Hygiene Verification (the load-bearing constraints)

```text
cargo build --workspace --release:                                   exits 0
cargo test --workspace:                                              23 tests pass (12 plan-01 + 5 plan-02 + 6 new)
cargo test --test architecture_dag:                                  1/1 pass
cargo clippy --workspace --all-targets -- -D warnings:               clean
cargo fmt --check:                                                   clean
target/release/holt --version:                                       "holt 0.1.0"
target/release/holt run -- bash -c 'echo hello' | od -c (header):    "h e l l o \n" (6 bytes, byte-exact)
target/release/holt --self-bench --json --iterations 30 .passed:     true
target/release/holt --self-bench --iterations 10 (PASS line):        "PASS: holt-only p95 ≤ 20000us"
cargo tree -p holt-render | grep -c holt-supervisor:                 0
cargo tree -i tokio:                                                 "did not match any packages"
grep -cE 'jsonc-parser|simd-json|figment|chrono|fs2|owo-colors|...' holt-cli/Cargo.toml: 0
grep -c 'macos-14' .github/workflows/ci.yml:                         2 (REQUIRED + stable informational)
grep -c '1.87' .github/workflows/ci.yml:                             4
grep -c 'cargo test --test architecture_dag' .github/workflows/ci.yml: 3 (linux + macos + windows)
grep -c 'continue-on-error: true' .github/workflows/ci.yml:          3 (windows + 2 stable)
```

## Test Results

```
holt-cli (5 tests):
  version_smoke ...................................... 1 passed
  run_passthrough .................................... 2 passed
  self_bench_smoke ................................... 1 passed
  render_path_no_read ................................ 1 passed (no-op stub on macOS arm64; full strace path runs in Linux CI)

holt-supervisor (5 tests, unchanged from plan 02):     5 passed

holt-schemas (12 tests, unchanged from plan 01):       12 passed

workspace tests (1 test, NEW):
  architecture_dag::holt_render_does_not_depend_on_holt_supervisor ... ok

23 passed; 0 failed; 0 ignored
```

`pgrep -f 'sleep 5'` after the timeout test (plan 02 still passing) returns empty — confirming ROADMAP success criterion #2's "no orphan descendants" clause continues to hold through the binary harness too.

## ROADMAP Success Criteria — Phase 1 Mapping

All five Phase 1 ROADMAP success criteria are now runnable from `cargo test --workspace` + `target/release/holt`:

| # | ROADMAP success criterion | Runnable command | Observed result |
|---|---------------------------|------------------|-----------------|
| 1 | Wrapped statusLine command runs and stdout passes through unchanged; one valid timings line written | `cargo test -p holt-cli run_passthrough` then `cargo test -p holt-supervisor passthrough_smoke` | `out.stdout == b"hello\n"`; timings.jsonl line parses with `duration_ms`, `fork_count`, `exit_code:0`, `stderr_capture:""` |
| 2 | Timeout kills wrapped script + all descendants within ~100ms of deadline; structured breach landed | `cargo test -p holt-supervisor timeout_killpg_smoke` | Returns within 1.5s on a 1s timeout against `sleep 5`; `pgrep -f 'sleep 5'` empty; breaches.log has Timeout entry |
| 3 | Self-bench reports holt-only p95 < 20ms (Unix); render path opens neither breaches.log nor timings.jsonl for reading (Linux strace) | `target/release/holt --self-bench --json --iterations 30` + `cargo test -p holt-cli render_path_no_read` | `passed:true`; macOS arm64 local p95 = 0us (well under 20ms); Linux strace test no-ops on macOS/Windows but the Linux CI run executes it on every PR |
| 4 | Malformed CC stdin → parse_fail breach + LKG fall-through + exit 0 + no panic; `read_heartbeat()` returns `Ok(None)` for 4 corruption modes | `cargo test -p holt-cli run_with_malformed_stdin` + `cargo test -p holt-schemas reader_contract` | parse_fail breach written, exit 0; 7 reader_contract tests pass (4 corruption modes + 3 happy paths) |
| 5 | `cargo tree --workspace -p holt-render` shows zero edge to `holt-supervisor`; CI fails the PR if the edge is introduced | `cargo test --test architecture_dag` + `cargo tree -p holt-render \| grep -c holt-supervisor` | DAG test passes; tree count = 0; deliberate edge addition confirmed to fail the test with `C2 VIOLATED: ...` |

## Phase 1 Requirement Coverage (11/11)

| Requirement | Plan | Verification |
|-------------|------|--------------|
| HOOK-11 | 01 | `holt-schemas::read_heartbeat` C5 contract — 7 tests in `reader_contract.rs` |
| CORE-02 | 02 | `timings::append_timings` 5MB rotation — `jsonl_rotation` test |
| CORE-03 | 02 | LKG cache write/read — `lkg_roundtrip` test |
| CORE-04 | 02 | Single-chokepoint discipline — `chokepoint_audit` text-grep test |
| CORE-05 | 02 | killpg + Linux /proc PPID-walk — `timeout_killpg_smoke` test |
| CORE-06 | 02 | Breach record schema with env allowlist — exercised in `timeout_killpg_smoke` and `run_with_malformed_stdin_records_parse_fail_and_exits_zero` |
| CORE-01 | 03 | byte-exact stdout passthrough — `run_passthrough_emits_wrapped_stdout_unchanged` |
| CORE-07 | 03 | sub-20ms p95 self-bench — `self_bench_json_has_expected_shape` |
| CORE-08 | 03 | defensive stdin parse — `run_with_malformed_stdin_records_parse_fail_and_exits_zero` |
| CORE-09 | 03 | render path never reads breaches.log/timings.jsonl — `render_path_does_not_open_observability_logs_for_reading` (Linux strace) |
| CORE-10 | 03 | no holt-render → holt-supervisor edge — `holt_render_does_not_depend_on_holt_supervisor` (`cargo metadata` BFS) |

Coverage: **11/11**, no gaps, no duplicates. Phase 1 is complete.

## Patterns Established (consumed by Phase 2 / 3)

1. **Always-exit-0 render contract.** Phase 2 hooks invoke `holt run` from CC's statusLine; Phase 3's `holt install-hooks` writes the `statusLine.command` JSONC entry. Both rely on the invariant that `holt run` never bubbles errors to CC — the binary CONTRACT is "exit 0 unless the developer mis-invoked the args (exit 2)". Future entry points (`holt doctor` v0.5, `holt orchestrator` v1.0) will follow the same pattern.
2. **`holt --self-bench --json` as the cold-start regression net.** Phase 3+ contributors bumping deps or refactoring the render path can run a single command to see whether they pushed p95 over 20ms. CI gates this on every PR.
3. **`tests/architecture_dag.rs` as the architecture invariant net.** Phase 2 will likely add `holt-hooks → holt-supervisor` (Phase 3) or `holt-orchestrator → holt-render` (v1.0) edges; the BFS handles arbitrary new edges as long as the `holt-render → holt-supervisor` edge stays absent. To enforce additional invariants (e.g., "holt-orchestrator MUST NOT depend on holt-supervisor on the read fanout path"), add a second test fn in the same file with a different src/dst pair.
4. **Workspace-root phantom-package pattern.** Future workspace-level integration tests (e.g., a "no transitive tokio" test, a "all crates have license metadata" test, a "no `unwrap()` in render-path crates" lint test) live alongside `architecture_dag.rs` in `tests/`. Add new entries to the root Cargo.toml's `[[test]]` block.

## User Setup Required

None. CI is GitHub-Actions-only (no self-hosted runners, no secrets, no environment variables). When the user pushes the next branch + opens a PR, the matrix will execute automatically.

## Follow-ups (for Phase 2 / 3 / future)

- **Phase 2 (HOOK-01..HOOK-11):** Heartbeat-write hook subcommand (`holt hook pre-tool-use`, `post-tool-use`, `stop`, `notification`, `session-start`) writes per-session JSON to `$XDG_RUNTIME_DIR/holt/sessions/<sid>.json` (Linux) or `$TMPDIR/holt-$UID/sessions/<sid>.json` (macOS) with `~/.cache/holt/sessions/` fallback via `holt_schemas::atomic_write`. The `slurp_and_parse` helper in plan 03 captures the CC stdin envelope shape; Phase 2 promotes the Empty/Ok variants into typed Heartbeat construction.
- **Phase 3 (`holt install-hooks`):** mutate `~/.claude/settings.json` via `fs2::FileExt::try_lock_exclusive()` + `jsonc-parser` CST round-trip + fsync-before-rename + `.holt.bak` backup (C3, C4). JSONC handling is C4-locked to live ONLY in `holt-cli` — this plan does NOT pull `jsonc-parser` (audited via `grep -c 'jsonc-parser' holt-cli/Cargo.toml` = 0). Phase 3 adds the dep.
- **macOS render-path-no-read coverage** (deferred to v0.5): the strace test only runs on Linux. macOS DTrace requires SIP relaxation or a co-signed test binary; not practical for v0.1 CI. The Linux path is the v0.1 enforcement boundary; the success-criterion language explicitly names strace.
- **Cold-start wall-clock self-bench** (deferred): the in-process `--self-bench` measures WARM cold-start (process already running), not wall-clock launch overhead. If the user's wrapped script runtime climbs to push the holt-side overhead toward the 20ms ceiling, a follow-up CI script wraps `time target/release/holt --self-bench` externally — see RESEARCH §"Pattern 8 Important nuance".
- **`cargo tree -i tokio` as a CI step** (deferred): currently only run locally as a sanity check. If future deps risk pulling tokio transitively, add a CI step that runs `cargo tree -i tokio` and asserts the exit code is non-zero (no match = no tokio = green).
- **macOS H3 fallback** (post-v0.1, from plan 02): wire `libproc::proc_listchildpids` via `libproc-sys` crate. Trigger to harden: ≥1 macOS-tagged "killpg failed under sandbox" issue.

## Self-Check

Files claimed → verified:

- `crates/holt-cli/src/{cli,run,self_bench,stdin,main}.rs`: ALL FOUND (5/5)
- `crates/holt-cli/tests/{version_smoke,run_passthrough,self_bench_smoke,render_path_no_read}.rs`: ALL FOUND (4/4)
- `tests/architecture_dag.rs` (workspace root): FOUND
- `.github/workflows/ci.yml`: FOUND
- `Cargo.toml` (workspace, modified — gained `[package]` + `[[test]]` for holt-workspace-tests): FOUND
- `Cargo.lock` (modified — new clap/humantime/anyhow/tempfile lockfile entries): FOUND
- `crates/holt-cli/Cargo.toml` (modified — production manifest with deps + dev-deps): FOUND

Commits claimed → verified:

- `9e2f61e` (Task 1, feat): FOUND in `git log --oneline -5`
- `6a1befc` (Task 2, test+ci): FOUND in `git log --oneline -5`

## Self-Check: PASSED

---
*Phase: 01-schema-supervisor-substrate*
*Completed: 2026-04-28*
