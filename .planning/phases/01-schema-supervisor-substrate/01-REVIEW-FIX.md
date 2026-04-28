---
phase: 1
phase_name: Schema + supervisor substrate
status: all_fixed
fix_scope: critical_warning
findings_in_scope: 16
fixed: 16
skipped: 0
iteration: 1
fixed_at: 2026-04-28
---

# Phase 1 — Code Review Fix Report

**Source review:** `.planning/phases/01-schema-supervisor-substrate/01-REVIEW.md` (24 findings: 5 CR / 11 WR / 8 IN)
**Fix scope:** `critical_warning` — all 5 CRITICAL + 11 WARNING.
**Result:** all 16 in-scope findings fixed; 8 INFO findings deferred per scope.

**Pre-fix verification baseline (must remain green):** 23/23 tests, 5/5 ROADMAP must-haves, 4/4 hard constraints (C1/C2/C5/C6).

**Post-fix verification:** 28/28 tests (gained 5 regression tests from CR-01, CR-03, CR-05, WR-02 ×2), 5/5 ROADMAP must-haves still green, 4/4 hard constraints still enforced. `cargo build --workspace --release`, `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` all clean.

---

## Fixed findings

### CR-01 — `serde_json` `preserve_order` feature missing
- **Files changed:** `Cargo.toml`, `crates/holt-schemas/tests/preserve_order_smoke.rs` (new), `Cargo.lock`
- **Fix:** added `features = ["preserve_order"]` to the workspace `serde_json` dep; added a smoke test that round-trips `{"b":1,"a":2,"zzz":3,"alpha":4}` and asserts byte-identical output as a regression guard.
- **Commit:** `ac18923`
- **Test:** `cargo test -p holt-schemas --test preserve_order_smoke` → 1 passed.

### CR-02 — `holt run` re-serialised CC stdin via `Value::to_string()`
- **Files changed:** `crates/holt-cli/src/stdin.rs`, `crates/holt-cli/src/run.rs`
- **Fix:** `StdinParseOutcome::Ok` and `::ParseFail` now carry `raw: Vec<u8>` of the original bytes. `run.rs` forwards `raw` directly. The defensive parse remains a *check*, not a transform. `value` and `raw` (in ParseFail) are explicitly `#[allow(dead_code)]`-marked for Phase 2 hook consumption.
- **Commit:** `55ee017`
- **Test:** `cargo test -p holt-cli --test run_passthrough` → 2 passed (byte-exact `b"hello\n"` + parse_fail).

### CR-03 — supervised child stdin never closed when `stdin_bytes` empty
- **Files changed:** `crates/holt-supervisor/src/supervisor.rs`, `crates/holt-supervisor/tests/empty_stdin_closes_child_pipe.rs` (new)
- **Fix:** always `take()` the child's stdin handle inside the `if let Some(...)`. When `stdin_bytes` is non-empty we spawn the writer thread; otherwise the handle drops at end of scope, closing the pipe and giving the child immediate EOF. Added regression test that wraps `cat` with empty stdin and a 2s deadline; the child must finish well under 1.5s as `Ok`, otherwise the pipe is being left open.
- **Commit:** `23e32f3`
- **Test:** `cargo test -p holt-supervisor --test empty_stdin_closes_child_pipe` → 1 passed (`finished in 0.04s`, well under the 1.5s threshold). chokepoint_audit + passthrough_smoke also still pass.

### CR-04 — `slurp_and_parse` blocks on stdin with no timeout
- **Files changed:** `crates/holt-cli/src/stdin.rs`
- **Fix:** wrap `read_to_end` in `mpsc + thread + recv_timeout(200ms)`. On timeout, fall through to `StdinParseOutcome::Empty` so the caller degrades to LKG / empty stdout without breaching. The render-path budget is now bounded — no slow CC stdin can hang the whole shim. Worker thread leaks on stdin-never-eof but dies with the short-lived statusLine process; promoting to a distinct breach kind ("stdin_timeout") deferred to v0.5 doctor.
- **Commit:** `c12855d`
- **Test:** `cargo test -p holt-cli --test run_passthrough` → 2 passed (still byte-exact + still parse_fail).

### CR-05 — atomic-write tmp file leaks on `write_all` / `sync_all` failure
- **Files changed:** `crates/holt-schemas/src/writer.rs`, `crates/holt-schemas/tests/atomic_write_smoke.rs`
- **Fix:** wrap `open → write_all → sync_all → drop → rename` in an immediately-invoked closure so `let _ = fs::remove_file(&tmp)` runs unconditionally on `Err`. Previously cleanup only ran when rename failed, so a write/fsync error left an orphan that poisoned subsequent calls' `create_new(true)` with EEXIST. Added `no_orphan_tmp_when_inner_pipeline_errors` regression that targets a read-only parent dir, forces the open to fail, and asserts zero `*.holt-tmp.*` entries afterwards.
- **Commit:** `0ee03ff`
- **Test:** `cargo test -p holt-schemas --test atomic_write_smoke` → 6 passed (was 5; gained 1 regression).

### WR-01 — `default_cache_root` falls back to `.` when HOME unset
- **Files changed:** `crates/holt-supervisor/src/paths.rs`
- **Fix:** check `HOME` then `USERPROFILE`, and on both empty fall back to `std::env::temp_dir().join(format!("holt-{}", std::process::id()))`. Avoids polluting whatever working directory CC was started in (often a git repo) with `breaches.log` / `timings.jsonl` / `lkg/`.
- **Commit:** `976f3c6`
- **Test:** workspace builds + tests pass; no dedicated test (env mutation across tests is racy and the verify-doc path didn't ask for one).

### WR-02 — `read_heartbeat` returns Err for PermissionDenied / IsADirectory
- **Files changed:** `crates/holt-schemas/src/reader.rs`, `crates/holt-schemas/tests/reader_contract.rs`
- **Fix:** broadened the soft-fail set to include `io::ErrorKind::PermissionDenied` and `io::ErrorKind::IsADirectory` (stable on `ErrorKind` since Rust 1.83; well within MSRV 1.87). Both collapse to `Ok(None)` per C5. Added `returns_ok_none_for_permission_denied` (Unix, with a `is_root()` guard for some CI containers where mode 0000 doesn't deny root) and `returns_ok_none_for_directory_at_path` regression tests.
- **Commit:** `2803c07`
- **Test:** `cargo test -p holt-schemas --test reader_contract` → 9 passed (was 7).
- **Status:** fixed: requires human verification — the `is_root()` skip path is only exercised on CI containers running as root; locally on macOS arm64 the PermissionDenied branch fired and the assertion held. Worth a re-run on the Linux REQUIRED CI job.

### WR-03 — TOCTOU in jsonl rotation (multi-process race)
- **Files changed:** `crates/holt-supervisor/src/timings.rs` (doc-only)
- **Fix:** documented as a known v0.1 limitation in the `append_jsonl` rustdoc — explains the race, the accepted tradeoff (worst case is `.1` data loss; live file always carries the most recent ~5MB), why the obvious `flock(2)` fix is blocked (`fs2` is forbidden, hand-rolling would need `unsafe`), and the trigger to harden (≥1 user report, OR Phase 4 introduces a hooks-managed lock-file convention we can reuse). The reviewer explicitly said "acceptable for v0.1 if explicitly accepted as a known limitation".
- **Commit:** `2a3e411`
- **Test:** workspace test run still green; no behavioral change.

### WR-04 — TOCTOU PID-reuse race in Linux `ppid_walk_kill`
- **Files changed:** `crates/holt-supervisor/src/kill.rs`
- **Fix:** added `reverify_pgid()` that re-reads `/proc/<pid>/stat` immediately before `kill(2)` and re-parses the `pgrp` field (5th whitespace field, accounting for parens around `comm`). Only SIGKILL if it still matches. Window shrinks from milliseconds (full /proc walk) to microseconds (one syscall). Not airtight (the race can still happen between re-read and kill) but significantly mitigates.
- **Commit:** `88ce18d`
- **Test:** `cargo test -p holt-supervisor --test timeout_killpg_smoke` → 1 passed (1.21s; same as baseline). `ppid_walk_kill` is Linux-only and only fires on `EPERM`, so on macOS the path isn't directly exercised — the change is verified-by-build on macOS and verified-by-test on the Linux REQUIRED CI job.
- **Status:** fixed: requires human verification on the Linux EPERM-fallback path; macOS local run only covers the happy path.

### WR-05 — self-bench writes to user's real `~/.cache/holt/`
- **Files changed:** `crates/holt-cli/src/self_bench.rs`, `crates/holt-cli/Cargo.toml`
- **Fix:** `run_self_bench` now creates a `tempfile::tempdir()` for `cache_root`, with a `std::env::temp_dir().join(format!("holt-self-bench-{pid}"))` fallback if tempdir fails. Promoted `tempfile` from dev-dependencies to runtime dependencies for `holt-cli` (already in `Cargo.lock`; only used by the opt-in `--self-bench` mode, never the live render path). CI no longer pollutes the user's real telemetry stream with synthetic samples.
- **Commit:** `e26ef5d`
- **Test:** `cargo test -p holt-cli --test self_bench_smoke` → 1 passed; release-binary `--self-bench --json --iterations 30` returns `{"iterations":30,"overhead_p95_us":0,"passed":true}` (no pollution path, exit 0).

### WR-06 — self-bench `pick(0.95)` returns max sample on len=10
- **Files changed:** `crates/holt-cli/src/self_bench.rs`
- **Fix:** replaced `((len-1)*frac).round()` indexing with a linear-interpolation percentile: `pos = (n-1)*frac; lo = floor(pos); hi = lo+1; weight = pos - lo; lo_v + (hi_v - lo_v) * weight`. p95 and p99 are now meaningful even on small N (no longer collapsed to the maximum). Defensive empty-vec branch avoids panics on degenerate input.
- **Commit:** `4706744`
- **Test:** `cargo test -p holt-cli --test self_bench_smoke` → 1 passed; bench still reports `passed: true` on the macOS arm64 dev box.

### WR-07 — stdout-drain thread orphaned on timeout path
- **Files changed:** `crates/holt-supervisor/src/supervisor.rs`
- **Fix:** added `let _ = stdout_thread.and_then(|t| t.join().ok());` at the top of the timeout arm. Previously the JoinHandle was dropped (detached) and the thread ran until `read_to_end` on the SIGKILL'd stdout pipe finished; on a long-running CC session with hundreds of breaches, threads + their `Vec<u8>` buffers leaked.
- **Commit:** `5e44ffe`
- **Test:** `cargo test -p holt-supervisor --test timeout_killpg_smoke` → 1 passed.

### WR-08 — `breaches.rs` `writer_version` wrong crate
- **Files changed:** `crates/holt-supervisor/src/options.rs`, `crates/holt-supervisor/src/breaches.rs`, `crates/holt-supervisor/src/supervisor.rs`, `crates/holt-cli/src/run.rs`, `crates/holt-cli/src/self_bench.rs`, plus all 4 supervisor integration tests.
- **Fix:** added `SupervisorOptions::writer_version: &'static str`. `holt-cli` now passes its own `env!("CARGO_PKG_VERSION")` from `main.rs`/`run.rs`/`self_bench.rs`. `breaches::append_breach` takes `writer_version` as a parameter rather than expanding `env!` from inside the supervisor crate. `with_defaults()` retains the supervisor-crate-version fallback for tests. All four supervisor tests gained `writer_version: "test-0.0.0"`.
- **Commit:** `5a74cf3`
- **Test:** full workspace test suite still green; the `timeout_killpg_smoke` test specifically verifies `writer_version` is a JSON string (still passes).

### WR-09 — four crates lacked `#![forbid(unsafe_code)]`
- **Files changed:** `crates/holt-cli/src/main.rs`, `crates/holt-render/src/lib.rs`, `crates/holt-orchestrator/src/lib.rs`, `crates/holt-hooks/src/lib.rs`
- **Fix:** added `#![forbid(unsafe_code)]` to each. Schemas and supervisor already had it.
- **Commit:** `f87616d`
- **Test:** `cargo build --workspace` passes (no unsafe blocks anywhere; the forbid is satisfied trivially).

### WR-10 — self-bench bench command happens to not read stdin (CR-03 mask)
- **Files changed:** `crates/holt-cli/src/self_bench.rs`
- **Fix:** rolled into the WR-05 commit (`e26ef5d`) — the inline comment now explicitly says "`:` is chosen specifically because it does not read stdin" and explains that this used to mask CR-03 (now fixed) but swapping in a stdin-reading bench command without re-checking remains a foot-gun.
- **Commit:** `e26ef5d` (shared with WR-05)
- **Test:** N/A — comment-only.

### WR-11 — Cargo `resolver = "2"` with edition 2024 (which defaults to 3)
- **Files changed:** `Cargo.toml`
- **Fix:** removed the explicit `resolver = "2"` line and added an explanatory comment. Edition 2024 implies `resolver = "3"` (stabilized in 1.84; MSRV is 1.87 so comfortably supported). We now get MSRV-aware feature unification by default.
- **Commit:** `68cbf8a`
- **Test:** `cargo build --workspace`, `cargo test --workspace`, `cargo clippy`, `cargo fmt --check` all clean.

### Style follow-up
- **Files changed:** `crates/holt-schemas/tests/atomic_write_smoke.rs`, `crates/holt-supervisor/tests/empty_stdin_closes_child_pipe.rs`
- **Fix:** bare `cargo fmt` follow-up over the CR-03 + CR-05 regression tests. Formatting-only adjustments so `cargo fmt --check` stays green.
- **Commit:** `8a5cee5`
- **Test:** `cargo fmt --check` exit 0.

---

## Skipped / deferred findings

All 8 INFO findings are deferred per the explicit fix scope (`critical_warning`):

- **IN-01** — `Supervisor` ZST is dead-code shim. Cosmetic; v1.0 may use it for namespacing once orchestrator wires up.
- **IN-02** — `holt-schemas/src/lib.rs` re-exports `error::ReaderError` AND keeps `pub mod error;`. Cosmetic public-surface narrowing.
- **IN-03** — `status.code().unwrap_or(-1)` uses `-1` as magic sentinel. Document or widen to `Option<i32>` end-to-end. Defer to v0.5 doctor (it'll need to parse this).
- **IN-04** — `default_cache_root` doesn't honor `LOCALAPPDATA` on Windows. Phase 1 explicitly says Windows is best-effort; defer to a follow-up.
- **IN-05** — `Heartbeat.mode` accepts any `Option<String>`. Defer to Phase 2 when `mode` becomes load-bearing for pet-state transitions.
- **IN-06** — `Map<String, Value>` for `env_capture` is heavy; cosmetic.
- **IN-07** — mirror C2 doc-comment in `holt-orchestrator/src/lib.rs`. Touched WR-09 file but kept fix tightly scoped to `#![forbid(unsafe_code)]` to honor "do NOT fix INFO findings"; defer to a follow-up.
- **IN-08** — `lkg_roundtrip.rs` test asserts on file existence, not outcome shape. Cosmetic test-quality polish; defer.

No CR/WR findings were skipped.

---

## Verification (post-fix)

| Gate | Command | Result |
|------|---------|--------|
| Release build | `cargo build --workspace --release` | exit 0 |
| Test suite | `cargo test --workspace` | 28 passed; 0 failed (was 23 baseline → +5 regression tests) |
| Clippy | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Format | `cargo fmt --check` | exit 0 |
| Forbidden crates | grep across all `Cargo.toml` + `Cargo.lock` | no `tokio`, `simd-json`, `figment`, `chrono`, `jsonc-parser`, `fs2`, `owo-colors`, `supports-color`, `terminal_size`, `atomic-write-file`, `crossterm`, `wait-timeout`, or `cargo_metadata` |
| Self-bench (release binary) | `target/release/holt --self-bench --json --iterations 30` | `{"iterations":30,"overhead_p95_us":0,"passed":true}` exit 0 |

## Regression check — 5/5 ROADMAP must-haves still green

| # | Test | Result |
|---|------|--------|
| 1 | `cargo test -p holt-cli --test run_passthrough` (CORE-01 happy path + CORE-08 parse_fail) | 2 passed |
| 2 | `cargo test -p holt-supervisor --test timeout_killpg_smoke` (timeout breach + killpg + no orphans) | 1 passed (1.21s) |
| 3 | `cargo test -p holt-cli --test self_bench_smoke` + `cargo test -p holt-cli --test render_path_no_read` (sub-20ms p95 + render-path no-read) | 2 passed |
| 4 | `cargo test -p holt-cli --test run_passthrough run_with_malformed_stdin_records_parse_fail_and_exits_zero` (malformed CC stdin → no panic, parse_fail breach, exit 0) | 1 passed (covered in #1) |
| 5 | `cargo test --test architecture_dag` (no `holt-render → holt-supervisor` edge) | 1 passed |

## Hard constraints — 4/4 still enforced

| Constraint | Test | Status |
|------------|------|--------|
| C1 — exactly one `.wrap(ProcessGroup::leader())` site | `chokepoint_audit::only_one_wrap_call_site_in_supervisor_crate` | passing |
| C2 — no `holt-render → holt-supervisor` edge | `architecture_dag::holt_render_does_not_depend_on_holt_supervisor` | passing |
| C5 — reader contract Ok(None) for corruption | `reader_contract` (now 9 cases — was 7; gained PermissionDenied + IsADirectory regressions per WR-02) | passing |
| C6 — render path never reads `breaches.log` / `timings.jsonl` | `render_path_no_read::render_path_does_not_open_observability_logs_for_reading` | passing |

---

_Fixed: 2026-04-28_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
