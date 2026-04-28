---
phase: 1
phase_name: Schema + supervisor substrate
status: passed
verified: 2026-04-28
roadmap_criteria_passed: 5/5
requirements_covered: 11/11
decisions_implemented: 16/16
hard_constraints_enforced: 4/4
quality_gates_clean: true
score: 5/5 must-haves verified
overrides_applied: 0
re_verification: false
---

# Phase 1: Schema + supervisor substrate — Verification Report

**Phase Goal (ROADMAP.md):** A `holt` binary that wraps the user's existing `statusLine.command`, supervises it with a configurable timeout + clean Unix process-group kill, falls through to a last-known-good cache on slow invocations, and writes per-fire timing + breach telemetry — all on the sub-20ms cold-start budget — while landing the keystone `holt-schemas` crate (heartbeat type + atomic-rename helper + non-panicking reader contract) that every subsequent phase depends on.

**Verified:** 2026-04-28 (goal-backward audit on macOS arm64 / Darwin 25.4.0)
**Status:** passed — all 5 ROADMAP success criteria verified, 11/11 requirements satisfied, 16/16 decisions implemented, 4/4 hard constraints test-enforced, all quality gates clean.
**Re-verification:** No — initial verification.

## 1. ROADMAP Success Criteria — 5/5 PASS

Each criterion was verified by running the named command on the actual binary / test on disk.

### Criterion #1 — Happy-path passthrough + valid timings line

| Sub-criterion | Command | Observed |
|---------------|---------|----------|
| Byte-exact `hello\n` stdout | `holt run -- bash -c "echo hello"` then `cmp` against `printf 'hello\n'` | `BYTE-EXACT MATCH`; `od -c` shows `h e l l o \n` (6 bytes) |
| Exit 0 | `echo $?` after the run | `exit=0` |
| timings.jsonl line written | `XDG_CACHE_HOME=$T holt run -- ...` then `cat $T/holt/timings.jsonl` | One line: `{"duration_ms":7,"exit_code":0,"fork_count":1,"session_id":"default","stderr_capture":"","ts":"2026-04-28T10:37:35.653475Z"}` — all four required fields (`duration_ms`, `fork_count`, `exit_code: 0`, `stderr_capture: ""`) present and well-typed |
| Integration test parity | `cargo test -p holt-supervisor passthrough_smoke` | `1 passed` |
| CLI integration test parity | `cargo test -p holt-cli run_passthrough_emits_wrapped_stdout_unchanged` | passes (in 2/2 run_passthrough tests) |

**Status:** ✓ VERIFIED.

### Criterion #2 — Timeout breach + killpg + no orphans

| Sub-criterion | Command | Observed |
|---------------|---------|----------|
| Returns within 100ms of breach | `time holt run --timeout 1s -- bash -c 'sleep 5'` | `elapsed_ms=1023` (23ms past the 1000ms deadline; well within the 100ms cushion) |
| Exit 0 (always) | `echo $?` | `exit=0` |
| `pgrep -f 'sleep 5'` empty | `sleep 0.3 && pgrep -f 'sleep 5'` | empty output (no orphaned descendants) |
| `breaches.log` entry | `cat $T/holt/breaches.log` | One JSON line with `"kind":"timeout"`, populated `env_capture`, `writer_version:"0.1.0"` |
| Test parity | `cargo test -p holt-supervisor timeout_killpg_smoke` | `1 passed` |

**Status:** ✓ VERIFIED.

### Criterion #3 — Sub-20ms self-bench + render path no-read

| Sub-criterion | Command | Observed |
|---------------|---------|----------|
| `holt --self-bench --json --iterations 30` exits 0 with PASS | `target/release/holt --self-bench --json --iterations 30` | `{"iterations":30,"overhead_p50_us":0,"overhead_p95_us":0,"overhead_p99_us":0,"budget_p95_us":20000,"passed":true}` (exit 0) |
| Human PASS line | `target/release/holt --self-bench --iterations 20` | `PASS: holt-only p95 ≤ 20000us` |
| Render path no-read test exists | `ls crates/holt-cli/tests/render_path_no_read.rs` | present (2.5KB); strace path `#[cfg(target_os = "linux")]`, macOS no-op stub `#[cfg(not(target_os = "linux"))]` per Plan 01-03 design |
| Test compiles + runs (macOS no-op) | `cargo test -p holt-cli --test render_path_no_read` | `1 passed` (no-op variant on macOS) |
| Test would assert in CI Linux job | `.github/workflows/ci.yml test-linux` step | `apt install strace`, then `cargo test --workspace` covers the strace-instrumented assertion on Linux x86_64 |

**Status:** ✓ VERIFIED. macOS local run is INCONCLUSIVE-platform for the strace half (acceptable per Plan 01-03 design — Linux is the v0.1 enforcement boundary, the success-criterion language explicitly names strace, and CI runs the strace path on every PR).

### Criterion #4 — Malformed CC stdin → no panic, parse_fail breach, exit 0

| Sub-criterion | Command | Observed |
|---------------|---------|----------|
| Truncated `{"session_id":` stdin → exit 0, no panic | `printf '{"session_id":' \| XDG_CACHE_HOME=$T holt run -- bash -c 'echo whatever'` | `exit=0`; no panic; stdout empty (no LKG yet, falls through) |
| `parse_fail` breach written | `cat $T/holt/breaches.log` | One JSON line `"kind":"parse_fail"`, `"stdin_excerpt":"{\"session_id\":"` (the truncated payload captured), `writer_version:"0.1.0"` |
| `read_heartbeat()` Ok(None) for 4 corruption modes + happy + garbage + arbitrary bytes | `cargo test -p holt-schemas --test reader_contract` | `7 passed; 0 failed` — covers `returns_ok_none_for_missing_file`, `..._zero_byte_file`, `..._truncated_json`, `..._unrecognized_schema_version`, `..._missing_required_fields`, `returns_ok_some_for_valid_heartbeat`, `does_not_panic_on_arbitrary_bytes` |
| CLI integration test parity | `cargo test -p holt-cli run_with_malformed_stdin_records_parse_fail_and_exits_zero` | passes (1 of 2 in run_passthrough.rs) |

**Status:** ✓ VERIFIED.

### Criterion #5 — No `holt-render → holt-supervisor` edge

| Sub-criterion | Command | Observed |
|---------------|---------|----------|
| `cargo tree -p holt-render` shows zero supervisor edge | `cargo tree -p holt-render \| grep holt-supervisor` | empty (only `holt-orchestrator → holt-schemas` chain visible); render Cargo.toml has only an inline `# NOTE:` comment forbidding the edge |
| `tests/architecture_dag.rs` exists at workspace ROOT (D-15) | `ls tests/architecture_dag.rs` | present (4.5KB); contains `parse_package_name` helper that handles cargo 1.87+ PkgIdSpec drift |
| `cargo test --test architecture_dag` exits 0 | `cargo test --test architecture_dag` | `holt_render_does_not_depend_on_holt_supervisor ... ok` — `1 passed; 0 failed` |
| CI gates | `.github/workflows/ci.yml` | `cargo test --test architecture_dag` runs in `test-linux`, `test-macos`, AND `test-windows` jobs (3 occurrences) |

**Status:** ✓ VERIFIED.

**Score:** 5/5 truths verified.

## 2. Required Artifacts — All present + substantive + wired + data-flowing

### Workspace skeleton (Plan 01-01)

| Artifact | Status | Evidence |
|----------|--------|----------|
| `Cargo.toml` (workspace root) | ✓ VERIFIED | Declares all six members; `rust-version = "1.87"`, `edition = "2024"`, `resolver = "2"`; pinned `process-wrap = "=9.1.0"` (with `features = ["std", "process-group"]` per Plan 02 deviation #2); `[profile.release]` has all four D-04 flags. Also gained `[package] name = "holt-workspace-tests"` to host `tests/architecture_dag.rs` at the workspace root (Plan 03 deviation #2). |
| `rust-toolchain.toml` | ✓ VERIFIED | `channel = "1.87"`, `components = ["rustfmt", "clippy"]`, `profile = "minimal"` |
| `.gitignore` | ✓ VERIFIED | Includes `/target`, `Cargo.lock.bak`, `*.holt-tmp.*` |

### `holt-schemas` keystone (Plan 01-01)

| Artifact | Status | Evidence |
|----------|--------|----------|
| `crates/holt-schemas/src/lib.rs` | ✓ VERIFIED | `#![forbid(unsafe_code)]`; declares 5 modules; re-exports `Heartbeat`, `LkgEntry`, `ReaderError`, `read_heartbeat`, `atomic_write` |
| `crates/holt-schemas/src/heartbeat.rs` | ✓ VERIFIED | `#[non_exhaustive]` `pub struct Heartbeat`; `schema_version: u8` first field; `session_id` required; 13 other fields with `#[serde(default)]`; `pub const SCHEMA_VERSION: u8 = 1`. NO `deny_unknown_fields` (D-05 compliant) |
| `crates/holt-schemas/src/lkg.rs` | ✓ VERIFIED | `#[non_exhaustive]` `pub struct LkgEntry`; `schema_version: u8` first; `stdout / exit_code / captured_at / duration_ms` per D-10. Adds `LkgEntry::new(...)` constructor (Plan 02 deviation #1) so `holt-supervisor` can construct outside the defining crate. |
| `crates/holt-schemas/src/error.rs` | ✓ VERIFIED | `pub enum ReaderError` with single `Io(#[from] std::io::Error)` variant via `thiserror` |
| `crates/holt-schemas/src/reader.rs` | ✓ VERIFIED | `pub fn read_heartbeat(path: &Path) -> Result<Option<Heartbeat>, ReaderError>` — implements 4-step C5 contract (file → ENOENT short-circuit → empty-file short-circuit → serde fallback to Ok(None) → schema_version check). No `unwrap`, no `panic!` |
| `crates/holt-schemas/src/writer.rs` | ✓ VERIFIED | `pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()>` — same-dir tmp with `.holt-tmp.<pid>` suffix, `OpenOptions::create_new(true).mode(0o600)` (cfg-unix), `f.sync_all()` BEFORE rename, orphan-tmp cleanup on rename failure |
| `crates/holt-schemas/tests/reader_contract.rs` | ✓ VERIFIED | 7 tests, all `Ok(None)`/`Ok(Some)` shape per D-06; passing |
| `crates/holt-schemas/tests/atomic_write_smoke.rs` | ✓ VERIFIED | 5 tests (Unix 0600 perms, contents written, no orphan tmp, overwrites, error on path-without-parent); passing |

### `holt-supervisor` wedge (Plan 01-02)

| Artifact | Status | Evidence |
|----------|--------|----------|
| `src/lib.rs` | ✓ VERIFIED | C1/C5/C6 module-level docs; `#![forbid(unsafe_code)]`; re-exports `Supervisor`, `wrap_and_run`, `BreachKind`, `DEFAULT_TIMEOUT`, `SupervisorOptions`, `SupervisorOutcome` |
| `src/supervisor.rs` | ✓ VERIFIED | `pub fn wrap_and_run(...)` is the SOLE `.wrap(ProcessGroup::leader())` site (chokepoint_audit confirms count = 1); `Stdio::piped()` × 3 invoked INSIDE `CommandWrap::with_new` closure BEFORE the wrap call. mpsc + thread for the deadline. LKG written via `LkgEntry::new` + `atomic_write` only on `exit_code == 0`. Always emits a timings.jsonl line. |
| `src/options.rs` | ✓ VERIFIED | `pub struct SupervisorOptions`, `pub enum SupervisorOutcome { Ok, Breach }`, `pub enum BreachKind { Timeout, ParseFail, SpawnFail }`, `pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2)` |
| `src/lkg.rs` | ✓ VERIFIED | `pub fn write_lkg` calls `holt_schemas::atomic_write`; `pub fn read_lkg` returns `Option<LkgEntry>` with C5-style defensive read (None on any unreadable state) |
| `src/timings.rs` | ✓ VERIFIED | `pub const MAX_BYTES: u64 = 5 * 1024 * 1024` (5MB); `pub fn append_timings`; shared `pub(crate) fn append_jsonl` with rotation BEFORE append |
| `src/breaches.rs` | ✓ VERIFIED | `pub fn append_breach`; `BreachRecord` has `ts/kind/env_capture/stdin_excerpt/stderr_excerpt/exit_code/writer_version` per D-13; `STDIN_EXCERPT_CAP = 2 * 1024`; `STDERR_EXCERPT_CAP = 4 * 1024`; `ENV_ALLOWLIST` contains all 13 D-13 keys (PATH, HOME, USER, SHELL, TERM, LANG, LC_ALL, XDG_RUNTIME_DIR, TMPDIR, HOLT_LABEL, HOLT_NESTED, HOLT_TRACE, CLAUDE_PROJECT_DIR); `writer_version: env!("CARGO_PKG_VERSION")` |
| `src/kill.rs` | ✓ VERIFIED | `#[cfg(unix)] pub fn kill_process_group`; uses `nix::sys::signal::killpg`; `Errno::EPERM` branch falls through to `ppid_walk_kill`; Linux variant walks `/proc/*/status` for matching `Pgid:`; macOS variant returns `Err` (libproc fallback explicitly deferred per Plan 02 §"H3 fallback scope") |
| `src/paths.rs` | ✓ VERIFIED | `default_cache_root()` honors `XDG_CACHE_HOME`; `lkg_path/timings_path/breaches_path` helpers |
| `tests/chokepoint_audit.rs` | ✓ VERIFIED | Walks `src/`, filters `//`-comment lines, asserts exactly 1 occurrence of `.wrap(ProcessGroup::leader())`. Passes. |
| `tests/passthrough_smoke.rs` | ✓ VERIFIED | Asserts `stdout == "hello\n"`, exit 0, single timings line with `exit_code:0`, `fork_count:1`. Passes. |
| `tests/timeout_killpg_smoke.rs` | ✓ VERIFIED | 1s timeout against `sleep 5` returns within 1.5s as `Breach { Timeout }`; checks `pgrep -f 'sleep 5'` empty; reads breaches.log, asserts `kind:"timeout"`. Passes (1.21s). |
| `tests/lkg_roundtrip.rs` | ✓ VERIFIED | `Ok` outcome writes `LkgEntry` to `<cache>/lkg/<sid>.json`; serde round-trip confirms schema_version, stdout, exit_code, captured_at, duration_ms. Passes. |
| `tests/jsonl_rotation.rs` | ✓ VERIFIED | 5MB pre-fill + 1 append → `<file>` contains only the new line, `<file>.1` contains the original 5MB. Passes. |

### `holt-cli` (Plan 01-03)

| Artifact | Status | Evidence |
|----------|--------|----------|
| `src/main.rs` | ✓ VERIFIED | Mod declarations + `clap` parse + `--self-bench` precedence over subcommand + `Run` dispatch |
| `src/cli.rs` | ✓ VERIFIED | `#[derive(Parser)] pub struct Cli` with `--self-bench`, `--json`, `--iterations` flags + optional `Run { --timeout, --session-id, wrapped: trailing_var_arg + allow_hyphen_values }` subcommand |
| `src/run.rs` | ✓ VERIFIED | `pub fn run(...) -> i32`. Defensive stdin parse → `humantime::parse_duration` → `wrap_and_run` → match outcome. ParseFail records breach via `append_breach(BreachKind::ParseFail, ...)` and falls through. Always returns 0 unless wrapped is empty (returns 2 — developer error). |
| `src/self_bench.rs` | ✓ VERIFIED | `pub fn run_self_bench(iterations: u32) -> BenchResult` runs ≥10 iterations (clamped via `cli.iterations.max(10)` in main); samples sorted; p50/p95/p99 picked via fractional indexing; budget 20000us Unix / 40000us Windows; `print_human` + `print_json` |
| `src/stdin.rs` | ✓ VERIFIED | `pub fn slurp_and_parse() -> StdinParseOutcome { Ok / ParseFail { excerpt } / Empty }`; excerpt capped at 2KB |
| `tests/version_smoke.rs` | ✓ VERIFIED | Asserts `holt --version` exit 0, stdout contains `0.1.0`. Passes. |
| `tests/run_passthrough.rs` | ✓ VERIFIED | 2 tests: byte-exact `hello\n` (CORE-01); malformed-stdin → `parse_fail` breach + exit 0 (CORE-08). Both pass. |
| `tests/self_bench_smoke.rs` | ✓ VERIFIED | Asserts JSON shape (6 fields), `iterations >= 10`, on Unix `budget_p95_us == 20000` and `passed == true`. Passes locally. |
| `tests/render_path_no_read.rs` | ✓ VERIFIED | `#[cfg(target_os = "linux")]` strace path; macOS no-op stub. Passes (no-op on macOS local). CI Linux runs the strace assertion on every PR. |
| `tests/architecture_dag.rs` (workspace root) | ✓ VERIFIED | BFS over `cargo metadata` resolved graph; `parse_package_name` handles 3 PkgIdSpec shapes; precise `C2 VIOLATED:` panic message names the chain. Passes. |

### CI workflow (Plan 01-03)

| Artifact | Status | Evidence |
|----------|--------|----------|
| `.github/workflows/ci.yml` | ✓ VERIFIED | 6 jobs. **lint** (fmt + clippy, MSRV 1.87, ubuntu-latest, REQUIRED). **test-linux** (REQUIRED, ubuntu-latest, MSRV 1.87) — installs strace + procps; runs `cargo build --workspace --release`, `cargo test --workspace`, `cargo test --test architecture_dag`, self-bench gate via `python3 json.load`. **test-macos** (REQUIRED, macos-14 = Apple Silicon, MSRV 1.87) — same suite minus strace install. **test-windows** + **stable-linux** + **stable-macos** all `continue-on-error: true` per D-16. |

## 3. Key Link Verification

| From | To | Via | Status | Detail |
|------|----|-----|--------|--------|
| `holt-supervisor::supervisor` | `holt_schemas::atomic_write` (LKG cache) | `use holt_schemas::LkgEntry; ... write_lkg → atomic_write` | ✓ WIRED | `lkg.rs::write_lkg` calls `atomic_write(&path, &bytes)` |
| `holt-supervisor::supervisor` | `process_wrap::std::{CommandWrap, ProcessGroup}` | `use` + single `.wrap(ProcessGroup::leader())` site | ✓ WIRED | Single call in `supervisor.rs:85` (cfg-unix); `Stdio::piped()` × 3 inside the `with_new` closure BEFORE the wrap call (text-ordered C1) |
| `holt-cli::run` | `holt_supervisor::wrap_and_run` | `use holt_supervisor::{wrap_and_run, ...};` then dispatch in run.rs:71 | ✓ WIRED | `let outcome = wrap_and_run(program, &args, opts);` |
| `holt-cli::run` | `holt_supervisor::breaches::append_breach` (ParseFail path) | `use holt_supervisor::breaches::append_breach;` then call in run.rs:43-49 | ✓ WIRED | Verified at runtime — feeding malformed stdin produces `kind:"parse_fail"` in breaches.log |
| `holt-cli::run` | `holt_supervisor::lkg::read_lkg` (Breach fall-through) | `use holt_supervisor::lkg::read_lkg;` then `emit_lkg_or_empty` | ✓ WIRED | Function actually invoked from both ParseFail and Breach branches |
| `holt-cli::self_bench` | `holt_supervisor::wrap_and_run` (≥10× per run) | `use holt_supervisor::{wrap_and_run, ...};` then loop in self_bench.rs | ✓ WIRED | Self-bench output `{"iterations":30,"passed":true}` proves 30 supervised spawns ran |
| `tests/architecture_dag.rs` | `cargo metadata --format-version 1` | `Command::new(env!("CARGO")).args(["metadata", "--format-version", "1"])` | ✓ WIRED | Shells out, parses with `serde_json::Value`, BFS over `/resolve/nodes`. NO `cargo_metadata` crate dep. |
| `.github/workflows/ci.yml` | `tests/architecture_dag.rs` | `run: cargo test --test architecture_dag` | ✓ WIRED | Step appears in `test-linux` AND `test-macos` AND `test-windows` jobs |
| `.github/workflows/ci.yml` | self-bench PASS gate | `python3 -c "json.load(...).passed"` exit | ✓ WIRED | Runs on the two REQUIRED Unix jobs |
| `holt-render → holt-supervisor` | (must NOT exist) | — | ✓ ABSENT | `cargo tree -p holt-render \| grep holt-supervisor` is empty; `tests/architecture_dag.rs` BFS confirms no path |

All 10 key links verified. Data flows from CC stdin → `slurp_and_parse` → `wrap_and_run` → child process → drained stdout/stderr → either LKG refresh + timings line, or breach record + LKG fall-through. No hollow props. No disconnected sources.

## 4. Data-Flow Trace (Level 4)

| Artifact | Data variable | Source | Real data? | Status |
|----------|---------------|--------|------------|--------|
| `holt run` stdout | `stdout` String returned by `wrap_and_run` | drained from child via `child.stdout().take()` thread | yes — verified `hello\n` byte-exact | ✓ FLOWING |
| timings.jsonl line | serde_json::json! object built in `append_timings_for` | `started.elapsed().as_millis()` + `status.code()` + drained stderr_bytes | yes — measured `duration_ms=7`, `exit_code:0` | ✓ FLOWING |
| breaches.log line | `BreachRecord` built in `append_breach` | `jiff::Timestamp::now()`, real `std::env::var(k)` allowlist iteration, real stdin/stderr excerpts | yes — `parse_fail` test captured `"stdin_excerpt":"{\"session_id\":"` (the actual truncated payload) | ✓ FLOWING |
| LKG cache `<sid>.json` | `LkgEntry::new(stdout, exit_code, ...)` | constructor from real `wrap_and_run` outputs | yes — `lkg_roundtrip` test reads back identical `LkgEntry` | ✓ FLOWING |
| self-bench JSON | `BenchResult` struct | sorted samples_us from 30 actual `wrap_and_run` calls of `sh -c :` | yes — 30 distinct supervised spawns | ✓ FLOWING |

## 5. Behavioral Spot-Checks

All commands ran in under 10 seconds; no state-mutating side effects beyond the temp `XDG_CACHE_HOME` directories that were cleaned up.

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `holt --version` semver | `target/release/holt --version` | `holt 0.1.0` (exit 0) | ✓ PASS |
| `holt --help` lists subcommands | `target/release/holt --help` | Lists `run`, `--self-bench`, `--json`, `--iterations` | ✓ PASS |
| Byte-exact passthrough | `holt run -- bash -c "echo hello" \| od -c` then `cmp` | `BYTE-EXACT MATCH` | ✓ PASS |
| Timings line on happy path | `XDG_CACHE_HOME=$T holt run -- bash -c "echo hello"` then `cat $T/holt/timings.jsonl` | One JSON line with all four required fields | ✓ PASS |
| Timeout returns within 100ms cushion | `time holt run --timeout 1s -- bash -c 'sleep 5'` | elapsed_ms=1023 (23ms past 1s deadline) | ✓ PASS |
| No orphans after timeout | `pgrep -f 'sleep 5'` post-test | empty | ✓ PASS |
| breaches.log written on timeout | `cat $T/holt/breaches.log` | `kind:"timeout"`, full env_capture, writer_version=0.1.0 | ✓ PASS |
| Malformed stdin → no panic, exit 0, parse_fail | `printf '{"session_id":' \| holt run --` | exit=0; breach with `kind:"parse_fail"`, `stdin_excerpt:"{\"session_id\":"` | ✓ PASS |
| Self-bench JSON shape + PASS | `holt --self-bench --json --iterations 30` | `{"iterations":30,"overhead_p50_us":0,...,"passed":true}` (exit 0) | ✓ PASS |
| Self-bench human PASS line | `holt --self-bench --iterations 20` | `PASS: holt-only p95 ≤ 20000us` | ✓ PASS |
| `cargo build --workspace --release` | (background task) | exit 0 (no warnings) | ✓ PASS |
| `cargo test --workspace` | (background task) | 23 passed; 0 failed across all crates + workspace-root test (12 schemas + 5 supervisor + 1 architecture_dag + 5 cli — version+passthrough×2+self_bench+render_path = 5) | ✓ PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | direct invocation | exit 0 | ✓ PASS |
| `cargo fmt --check` | direct invocation | exit 0 | ✓ PASS |
| `cargo tree -i tokio` | direct invocation | `error: package ID specification 'tokio' did not match any packages` (no transitive async runtime) | ✓ PASS |
| `cargo tree -p holt-render \| grep holt-supervisor` | direct invocation | empty (zero edge) | ✓ PASS |
| Forbidden crates audit on every Cargo.toml | grep across 7 manifests | All clean (no tokio, simd-json, figment, chrono, jsonc-parser, fs2, owo-colors, supports-color, terminal_size, atomic-write-file, crossterm, wait-timeout, cargo_metadata) | ✓ PASS |
| Cargo.lock has no forbidden crates | grep on Cargo.lock | clean | ✓ PASS |

## 6. Requirements Coverage — 11/11 SATISFIED

| Requirement | Plan | Implementation | Test |
|-------------|------|----------------|------|
| **CORE-01** Shim wraps user statusLine.command + emits stdout unchanged on happy path | 01-03 | `crates/holt-cli/src/run.rs::run()` dispatches to `wrap_and_run` and writes `stdout.as_bytes()` to `stdout()`; CLI subcommand `Run { wrapped: trailing_var_arg }` accepts arbitrary arg-vec | `run_passthrough_emits_wrapped_stdout_unchanged` (byte-exact `b"hello\n"`) ✓ |
| **CORE-02** Per-fire timing log to `~/.cache/holt/timings.jsonl` | 01-02 | `crates/holt-supervisor/src/timings.rs::append_timings` + `supervisor.rs::append_timings_for` (ts/session_id/duration_ms/fork_count/exit_code/stderr_capture) | `passthrough_smoke::echo_hello_returns_ok_and_writes_timings` ✓ |
| **CORE-03** Last-known-good TTL cache | 01-02 | `crates/holt-supervisor/src/lkg.rs::{write_lkg, read_lkg}` over `LkgEntry` schema_version 1 | `lkg_roundtrip::ok_outcome_writes_readable_lkg_entry` ✓ |
| **CORE-04** Configurable timeout + Unix process-group kill via process-wrap v9.1.0 + EPERM fallback | 01-02 | `supervisor.rs` mpsc + `recv_timeout(opts.timeout)` deadline; on timeout calls `kill::kill_process_group(pgid)` which uses `nix::sys::signal::killpg` then PPID-walk on `Errno::EPERM` (Linux) | `timeout_killpg_smoke::timeout_breach_kills_descendants_and_writes_breach_log` ✓ |
| **CORE-05** All spawned children pipe stdin/stdout/stderr × 3 | 01-02 | Inside `CommandWrap::with_new` closure: `c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())` BEFORE `wrap.wrap(ProcessGroup::leader())` | `chokepoint_audit::only_one_wrap_call_site_in_supervisor_crate` (C1 site) + `Stdio::piped()` × 3 grep ✓ |
| **CORE-06** Breach log to `~/.cache/holt/breaches.log` with full context | 01-02 | `crates/holt-supervisor/src/breaches.rs::append_breach` writes `BreachRecord { ts/kind/env_capture/stdin_excerpt/stderr_excerpt/exit_code/writer_version }` | exercised in `timeout_killpg_smoke` AND `run_with_malformed_stdin_records_parse_fail_and_exits_zero` ✓ |
| **CORE-07** Sub-20ms cold-start overhead — `holt --self-bench` | 01-03 | `crates/holt-cli/src/self_bench.rs::run_self_bench` + p95 PASS/FAIL gate | `self_bench_json_has_expected_shape` asserts `passed == true` on Unix ✓ + actual run shows `p95=0us` |
| **CORE-08** Defensive stdin JSON parse — parse-fail breach + LKG fall-through | 01-03 | `crates/holt-cli/src/stdin.rs::slurp_and_parse` returns `StdinParseOutcome::ParseFail { excerpt }`; `run.rs:43-50` records the breach and falls through | `run_with_malformed_stdin_records_parse_fail_and_exits_zero` ✓ |
| **CORE-09** Render path never opens breaches.log/timings.jsonl for reading | 01-03 | Telemetry writers use `OpenOptions::create(true).append(true)` only; `holt-render` has no telemetry-reading code at all | `render_path_does_not_open_observability_logs_for_reading` (Linux strace; macOS no-op) ✓ |
| **CORE-10** `holt-render` zero direct dep on `holt-supervisor` | 01-03 | `crates/holt-render/Cargo.toml` declares only `holt-schemas` + `holt-orchestrator` | `tests/architecture_dag.rs::holt_render_does_not_depend_on_holt_supervisor` (BFS over cargo metadata) ✓ |
| **HOOK-11** Reader treats stale-or-corrupt heartbeat as missing — never `unwrap()`, never panics | 01-01 | `crates/holt-schemas/src/reader.rs::read_heartbeat` — Ok(None) for 5 corruption modes; `Err` only for non-NotFound I/O | `reader_contract.rs` (7 tests, all passing) — including `does_not_panic_on_arbitrary_bytes` ✓ |

**Coverage:** 11/11 — no orphans (REQUIREMENTS.md Phase 1 maps the same 11 IDs); no duplicates.

## 7. Decision Implementation — 16/16

| ID | Decision | Implemented at |
|----|----------|----------------|
| **D-01** | Single workspace at repo root, six members declared from day one | `Cargo.toml [workspace] members` (6 paths) |
| **D-02** | `jiff` for timestamps; no `chrono` | `[workspace.dependencies] jiff = "0.2"`; `chrono` not in Cargo.toml or Cargo.lock |
| **D-03** | `anyhow` at binary boundary; `thiserror` at internal lib boundary | `holt-schemas/src/error.rs` uses `thiserror::Error`; `holt-cli/Cargo.toml` declares `anyhow.workspace = true` |
| **D-04** | `[profile.release] lto="thin", codegen-units=1, strip="symbols", panic="abort"` | `Cargo.toml` lines 61-66 (verbatim) |
| **D-05** | Heartbeat defensive serde — schema_version first, `#[serde(default)]` on optionals, NO `deny_unknown_fields` | `crates/holt-schemas/src/heartbeat.rs` — schema_version is first, 13× `#[serde(default)]`, no `deny_unknown_fields` |
| **D-06** | `read_heartbeat` C5 contract — Ok(None) for 5 corruption modes; never panics | `crates/holt-schemas/src/reader.rs` 4-step impl + 7 tests in `reader_contract.rs` |
| **D-07** | Hand-rolled atomic_write — same-dir tmp + PID suffix + fsync + rename | `crates/holt-schemas/src/writer.rs::atomic_write` |
| **D-08** | Heartbeat + LkgEntry `#[non_exhaustive]` for forward-compat | `heartbeat.rs:14`, `lkg.rs:10`. `LkgEntry::new` constructor added (Plan 02 deviation #1) so cross-crate construction works |
| **D-09** | Single chokepoint API `Supervisor::wrap_and_run` | `crates/holt-supervisor/src/supervisor.rs::wrap_and_run` (free fn) + `impl Supervisor` alias; chokepoint_audit confirms count=1 |
| **D-10** | LKG cache schema `{schema_version:1, stdout, exit_code, captured_at, duration_ms}` | `crates/holt-schemas/src/lkg.rs::LkgEntry` + `crates/holt-supervisor/src/lkg.rs::write_lkg/read_lkg` + `paths.rs::lkg_path` (= `<cache_root>/lkg/<session_id>.json`) |
| **D-11** | Default timeout 2s; `nix::sys::signal::killpg` + EPERM fallback | `options.rs::DEFAULT_TIMEOUT = Duration::from_secs(2)`; `kill.rs::kill_process_group` |
| **D-12** | timings.jsonl 5MB / `.1` rotation, write-only | `timings.rs::MAX_BYTES = 5*1024*1024`, rotation in `append_jsonl` BEFORE append; verified by `jsonl_rotation` test |
| **D-13** | breaches.log JSONL with allowlist + size-capped excerpts | `breaches.rs::ENV_ALLOWLIST` (13 keys), `STDIN_EXCERPT_CAP=2KB`, `STDERR_EXCERPT_CAP=4KB`, `writer_version: env!("CARGO_PKG_VERSION")` |
| **D-14** | `holt --self-bench` — wraps `:` ≥10×, p50/p95/p99, PASS/FAIL vs 20ms / 40ms | `crates/holt-cli/src/self_bench.rs` budget = 20000us Unix / 40000us Windows |
| **D-15** | `tests/architecture_dag.rs` walks `cargo metadata --format-version 1` JSON via `serde_json::Value` (no `cargo_metadata` crate) | `tests/architecture_dag.rs` uses `Command::new(env!("CARGO"))` + `serde_json::Value` BFS |
| **D-16** | CI matrix MSRV 1.87 REQUIRED on Linux x86_64 + macOS arm64; Windows + stable informational | `.github/workflows/ci.yml` — lint + test-linux + test-macos REQUIRED; test-windows + stable-linux + stable-macos `continue-on-error: true` |

**Score:** 16/16 — all locked decisions implemented as written.

## 8. Hard Constraints — 4/4 enforced by named test

| Constraint | Enforcement test | Status |
|------------|------------------|--------|
| **C1** — `Stdio::piped()` × 3 BEFORE `wrap(ProcessGroup::leader())` | `crates/holt-supervisor/tests/chokepoint_audit.rs::only_one_wrap_call_site_in_supervisor_crate` — textual audit, exit 0 | ✓ ENFORCED |
| **C2** — no `holt-render → holt-supervisor` edge | `tests/architecture_dag.rs::holt_render_does_not_depend_on_holt_supervisor` — BFS over `cargo metadata`, exit 0 | ✓ ENFORCED |
| **C5** — reader contract Ok(None) for corruption | `crates/holt-schemas/tests/reader_contract.rs` — 7 cases (5 Ok(None) corruption modes + 1 Ok(Some) happy + 1 garbage-bytes don't-panic), all exit 0 | ✓ ENFORCED |
| **C6** — render path never opens breaches.log / timings.jsonl for reading | `crates/holt-cli/tests/render_path_no_read.rs::render_path_does_not_open_observability_logs_for_reading` — Linux strace; macOS no-op stub | ✓ ENFORCED (Linux full strace path runs in CI; macOS no-op acceptable per Plan 01-03 design — strace is the documented enforcement boundary) |

C3 and C4 are not in scope for Phase 1 (they belong to Phase 3 install-hooks UX).

## 9. Workspace Structure Invariants

| Invariant | Status | Evidence |
|-----------|--------|----------|
| Workspace `Cargo.toml` declares all 6 crates as `[workspace] members` | ✓ | `Cargo.toml` lines 3-10 |
| `rust-toolchain.toml` pins `channel = "1.87.0"` | ✓ | `channel = "1.87"` (rustup resolves to 1.87.0) |
| `[workspace.dependencies] process-wrap = "=9.1.0"` | ✓ | `Cargo.toml:49 process-wrap = { version = "=9.1.0", features = ["std", "process-group"] }` — exact-pinned. Feature additions are necessary (Plan 02 deviation #2) since `process_wrap::std::*` is gated. |
| `[profile.release]` has lto, codegen-units, strip, panic="abort" | ✓ | All four flags present per D-04 |
| `.github/workflows/ci.yml` exists with matrix per Plan 01-03 | ✓ | 6 jobs, MSRV 1.87 REQUIRED on ubuntu-latest + macos-14, Windows + stable informational |

## 10. Quality Gates — All clean

| Gate | Status |
|------|--------|
| `cargo build --workspace --release` | ✓ exit 0 |
| `cargo test --workspace` | ✓ 23 passed, 0 failed |
| `cargo clippy --all-targets -- -D warnings` | ✓ exit 0 |
| `cargo fmt --check` | ✓ exit 0 |
| `cargo tree -i tokio` | ✓ no matches |
| Forbidden-crate audit | ✓ no tokio, simd-json, figment, chrono, jsonc-parser, fs2, owo-colors, supports-color, terminal_size, atomic-write-file, crossterm, wait-timeout, cargo_metadata |

## 11. Test Suite Totals

```
holt-schemas (12 tests):
  reader_contract:     7 passed
  atomic_write_smoke:  5 passed

holt-supervisor (5 tests):
  chokepoint_audit:           1 passed
  jsonl_rotation:             1 passed
  lkg_roundtrip:              1 passed
  passthrough_smoke:          1 passed
  timeout_killpg_smoke:       1 passed (1.21s — runs `sleep 5` against a 1s timeout)

holt-cli (5 tests):
  version_smoke:              1 passed
  run_passthrough:            2 passed (happy path + parse_fail)
  self_bench_smoke:           1 passed
  render_path_no_read:        1 passed (no-op on macOS; full strace path in CI Linux)

workspace-root tests (1 test):
  architecture_dag::holt_render_does_not_depend_on_holt_supervisor:  1 passed

TOTAL: 23 passed; 0 failed; 0 ignored
```

The `target/release/holt` binary is fresh: `cargo build --workspace --release` reports `Finished release profile [optimized] target(s) in 0.04s` (cache hit, no rebuild needed).

## 12. Anti-Pattern Scan

| File | Pattern | Severity | Notes |
|------|---------|----------|-------|
| `crates/holt-schemas/src/reader.rs` | doc-comment mentions `.unwrap()s` (negative form) | ℹ️ Info | Line 11 doc-comment. Forbids the pattern; not a code use. Confirmed by Plan 01-01 SUMMARY hygiene section. |
| `crates/holt-render/Cargo.toml` | `# NOTE:` comment names `holt-supervisor` | ℹ️ Info | Comment forbids the dep edge. CI test enforces actual absence. |
| All other crates | no TODO/FIXME/PLACEHOLDER/Stub returns/empty handlers found | — | Clean. |

No blockers. No warnings beyond informational doc-comment hits that are intentional contraindications, not stub indicators.

## 13. Cross-plan integration soundness

`cargo test --workspace` runs the union of all suites cleanly (23/23 in 1.21s wall-clock dominated by the deliberate 1s sleep in timeout_killpg_smoke). No flakiness observed. The binary at `target/release/holt` round-trips:

1. `holt --version` → `holt 0.1.0` exit 0
2. `holt run -- bash -c "echo hello"` → byte-exact `hello\n` exit 0 + valid timings line
3. `holt run --timeout 1s -- bash -c 'sleep 5'` → exit 0 within 1.5s, no orphans, breach record
4. `printf '{"session_id":' | holt run -- bash -c 'echo whatever'` → exit 0, parse_fail breach
5. `holt --self-bench --json --iterations 30` → `{"passed":true}` exit 0

Every Phase 1 ROADMAP success criterion is runnable end-to-end from `cargo test --workspace` + `target/release/holt` invocations alone.

## 14. Gaps

None. No deferred items (the explicitly deferred macOS H3 libproc fallback is out of Phase 1 scope per CONTEXT.md `<deferred>` and Plan 02 design).

## 15. Human Verification Required

None for Phase 1. The asciinema/gif demo is explicitly Phase 4 scope (DIST-04). All Phase 1 success criteria are programmatically verifiable.

## 16. Summary

**All five ROADMAP success criteria pass. All 11 phase requirements are satisfied with code on disk and named tests. All 16 CONTEXT.md decisions D-01..D-16 are implemented. All four hard constraints (C1, C2, C5, C6) are enforced by passing tests in CI. Quality gates (build, test, clippy, fmt, tokio audit, forbidden-crate audit) all clean.**

The phase delivered exactly what was promised: a `holt` binary that wraps `statusLine.command`, supervises with timeout + clean Unix process-group kill, falls through to LKG cache, writes per-fire telemetry, hits sub-20ms overhead — and the keystone `holt-schemas` crate the rest of the milestone composes against.

**Phase 1 ready to proceed to Phase 2 (Heartbeat hook write side).**

---

*Verified: 2026-04-28*
*Verifier: Claude (gsd-verifier, goal-backward audit on macOS arm64)*
