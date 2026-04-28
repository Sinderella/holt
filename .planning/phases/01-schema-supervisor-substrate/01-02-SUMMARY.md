---
phase: 01-schema-supervisor-substrate
plan: 02
plan_id: 01-02
subsystem: holt-supervisor / process supervision wedge
tags: [supervisor, process-wrap, chokepoint, lkg, timings, breaches, killpg, c1, c5, c6]
requires:
  - phase: 01-schema-supervisor-substrate (plan 01)
    provides: holt_schemas::atomic_write + LkgEntry (now consumed via LkgEntry::new)
provides:
  - holt_supervisor::wrap_and_run (the C1 chokepoint — single supervised-spawn site)
  - holt_supervisor::Supervisor::wrap_and_run (namespaced convenience alias)
  - holt_supervisor::SupervisorOptions / SupervisorOutcome / BreachKind
  - holt_supervisor::lkg::{write_lkg, read_lkg} (LKG cache — D-10)
  - holt_supervisor::timings::append_timings (5MB / .1 rotation — D-12)
  - holt_supervisor::breaches::append_breach (5MB / .1 rotation, env allowlist — D-13)
  - holt_supervisor::kill::kill_process_group (killpg + Linux /proc PPID-walk fallback — H3)
  - holt_supervisor::DEFAULT_TIMEOUT (D-11 = 2 seconds)
  - holt_schemas::LkgEntry::new constructor (added to unblock #[non_exhaustive] construction)
affects:
  - "Plan 01-03 (holt-cli + tests/architecture_dag.rs): consumes wrap_and_run; adds the C2 graph audit and the render-path-no-read test that asserts neither timings.jsonl nor breaches.log is opened for reading."
  - "Phase 2 (holt-hooks): no direct edge yet, but the LkgEntry::new constructor is now the canonical way to construct cache entries; hooks use atomic_write directly for heartbeat writes."
  - "v0.5 holt doctor: the only future reader of timings.jsonl + breaches.log."

tech-stack:
  added:
    - "process-wrap = \"=9.1.0\" with features = [\"std\", \"process-group\"] — std is opt-in upstream (default excludes it); enabling process-group keeps the binary small (no JobObject on Unix builds)"
    - "nix 0.27 (workspace pin) — only on cfg(unix); used for killpg + Pid::from_raw + Errno::EPERM matching"
    - "tracing 0.1.44 (transitive via process-wrap default features) — present but unused on the render path"
  patterns:
    - "C1 chokepoint discipline: every supervised spawn must route through wrap_and_run; chokepoint_audit.rs textually counts .wrap(ProcessGroup::leader()) in the crate's src/, filtering Rust line comments so doc-comments don't double-count."
    - "mpsc + std::thread::spawn for child wait deadlines: process-wrap v9.1.0's WrappedChild has no wait_timeout. Pattern: spawn a thread that calls child.wait(), receive on a channel with recv_timeout, kill via killpg on the pre-captured pgid (we own the child's PID = pgid because we spawned as group leader)."
    - "Drain stdout/stderr in dedicated threads BEFORE the wait thread so the child can't block on a full pipe. stdin is also pumped in a thread (broken-pipe is a normal best-effort outcome)."
    - "Telemetry writers (timings.jsonl, breaches.log) share an internal append_jsonl helper that performs the 5MB/.1 rotation at write boundary. Never fsync per line — append-and-lose-on-crash is the right tradeoff for observability data; D-07 fsync-before-rename remains for LKG/heartbeat (durability tier)."
    - "Env allowlist for breach records (D-13): explicit list of safe-to-log keys; secrets pasted into a GitHub issue from a breach record cannot leak."

key-files:
  created:
    - "crates/holt-supervisor/src/lib.rs (public surface + module-level C1/C5/C6 docs)"
    - "crates/holt-supervisor/src/options.rs (SupervisorOptions, SupervisorOutcome, BreachKind, DEFAULT_TIMEOUT)"
    - "crates/holt-supervisor/src/paths.rs (XDG-aware cache root + per-file path helpers)"
    - "crates/holt-supervisor/src/supervisor.rs (THE chokepoint — single .wrap(ProcessGroup::leader()) call site)"
    - "crates/holt-supervisor/src/lkg.rs (write_lkg/read_lkg over holt_schemas::atomic_write)"
    - "crates/holt-supervisor/src/timings.rs (append_timings + shared append_jsonl with 5MB rotation)"
    - "crates/holt-supervisor/src/breaches.rs (BreachRecord schema + ENV_ALLOWLIST + writer_version stamp)"
    - "crates/holt-supervisor/src/kill.rs (killpg + Linux /proc PPID-walk on EPERM)"
    - "crates/holt-supervisor/tests/chokepoint_audit.rs (C1 textual audit)"
    - "crates/holt-supervisor/tests/passthrough_smoke.rs (ROADMAP success criterion #1)"
    - "crates/holt-supervisor/tests/timeout_killpg_smoke.rs (ROADMAP success criterion #2)"
    - "crates/holt-supervisor/tests/lkg_roundtrip.rs (D-10 / CORE-03)"
    - "crates/holt-supervisor/tests/jsonl_rotation.rs (D-12 5MB → .1)"
  modified:
    - "Cargo.toml (workspace.dependencies): process-wrap pin gained features = [\"std\", \"process-group\"]"
    - "crates/holt-supervisor/Cargo.toml: declares holt-schemas, process-wrap, serde, serde_json, jiff, anyhow + cfg(unix) nix + dev tempfile"
    - "crates/holt-schemas/src/lkg.rs: added LkgEntry::new constructor (#[non_exhaustive] forbids struct-literal construction from outside the crate)"

key-decisions:
  - "D-09 single chokepoint: `wrap_and_run` is the only .wrap(ProcessGroup::leader()) call site; Supervisor::wrap_and_run is a thin convenience wrapper that delegates."
  - "D-10 LKG cache schema: written via holt_schemas::atomic_write; refreshed only on exit_code == 0; read returns None on any corruption (mirrors holt_schemas::read_heartbeat C5 posture)."
  - "D-11 default timeout = Duration::from_secs(2); configurable via the public SupervisorOptions (Plan 01-03 wires the CLI flag)."
  - "D-12 + D-13 unified rotation: 5MB cap per file → rename to <existing-extension>.1 at write boundary; rotation is best-effort (rename failure is swallowed); reading these files anywhere except the deferred holt doctor is forbidden by C6."
  - "D-13 breach env_capture is allowlist-only (PATH, HOME, USER, SHELL, TERM, LANG, LC_ALL, XDG_RUNTIME_DIR, TMPDIR, HOLT_LABEL, HOLT_NESTED, HOLT_TRACE, CLAUDE_PROJECT_DIR). writer_version stamps every record from CARGO_PKG_VERSION so v0.5 holt doctor can detect schema drift."
  - "process-wrap v9.1.0's WrappedChild has no wait_timeout — used std::sync::mpsc + std::thread::spawn (no extra dep) per RESEARCH §\"Pitfall: wait_timeout is not on process-wrap's WrappedChild\". child moves into the wait thread; pgid is captured BEFORE the move so we can killpg from the timeout branch."
  - "H3 fallback scope: Linux /proc PPID-walk implemented; macOS libproc::proc_listchildpids fallback deferred to post-v0.1 (no unsafe FFI in Phase 1; trigger to harden = ≥1 macOS sandbox issue)."

patterns-established:
  - "Single-chokepoint enforcement: textual `cargo test` audit beats runtime introspection for C1 — fast, dep-free, fails the build at the same point a developer would notice the symptom."
  - "Telemetry-on-the-render-path: writes only, no fsync per line, rotation at write boundary. C6 enforced by the absence of any read calls in this crate's source."
  - "Defensive read posture (read_lkg): mirrors the C5 reader contract from holt_schemas — Ok(None) for missing/empty/corrupt/version-mismatched files."

requirements-completed: [CORE-02, CORE-03, CORE-04, CORE-05, CORE-06]

metrics:
  start: "2026-04-28T10:00:00Z"
  end: "2026-04-28T10:20:00Z"
  duration: "~20 minutes (executor wall-clock; first cargo run pulled process-wrap v9.1.0 + nix 0.31 transitive + tracing chain)"
  tasks_completed: 2
  files_created: 13
  files_modified: 3
  tests_added: 5
  tests_passing: 5
---

# Phase 1 Plan 02: holt-supervisor Wedge Summary

**One-liner:** D-09 single-chokepoint supervisor (Stdio::piped × 3 BEFORE wrap(ProcessGroup::leader()), mpsc + thread for the deadline), LKG cache via `holt_schemas::atomic_write`, write-only timings.jsonl + breaches.log with 5MB / `.1` rotation, killpg + Linux `/proc` PPID-walk fallback on EPERM — five integration tests passing locally, including the static C1 chokepoint audit and a `sleep 5` / 1s-timeout smoke that confirms no orphans.

## Performance

- **Duration:** ~20 min (executor wall-clock)
- **Started:** 2026-04-28T10:00:00Z
- **Completed:** 2026-04-28T10:20:00Z
- **Tasks completed:** 2
- **Files created:** 13 (8 src + 5 tests)
- **Files modified:** 3 (workspace Cargo.toml, holt-supervisor Cargo.toml, holt-schemas/src/lkg.rs)
- **Tests:** 5/5 passing on macOS arm64
- **LOC:** 714 src + 280 tests

## Accomplishments

- **C1 enforced in code AND tested.** The single `.wrap(ProcessGroup::leader())` call site lives in `crates/holt-supervisor/src/supervisor.rs`, and `tests/chokepoint_audit.rs` textually counts the literal across the crate's `src/` (filtering Rust line comments so doc-comments don't double-count). Adding a second call site fails the build.
- **Stdio::piped × 3 BEFORE wrap.** Inside the `CommandWrap::with_new` closure — text-ordered so a human auditor sees the C1 invariant at a glance. macOS SIGTTIN avoidance documented inline.
- **mpsc + std::thread::spawn for the deadline.** `process-wrap` v9.1.0's `WrappedChild` does not expose `wait_timeout`; we spawn a thread that calls `child.wait()`, receive with `recv_timeout(opts.timeout)`, and `killpg(pgid, SIGKILL)` from the timeout branch. `pgid` is captured before the child moves into the wait thread.
- **5MB / `.1` rotation in the writer.** `timings.jsonl` and `breaches.log` share an internal `append_jsonl` helper that renames the current file to `<ext>.1` at write boundary when the next append would push past 5MB. Tested via a 5MB pre-fill that triggers exactly one rotation.
- **H3 fallback (Linux only).** `kill_process_group` walks `/proc/*/status` for any pid whose `Pgid:` line matches the target pgid and SIGKILLs each match. macOS libproc-based fallback is deferred to post-v0.1 (trigger: ≥1 sandbox issue).
- **LKG cache via `holt_schemas::atomic_write`.** Refreshed only on `exit_code == 0`; written to `<cache_root>/lkg/<session_id>.json` with the same fsync-before-rename discipline Plan 01-01 established.

## Task Commits

Each task was committed atomically:

1. **Task 1: Cargo.toml + types + chokepoint + telemetry primitives** — `e4e5014` (feat)
2. **Task 2: Integration tests — chokepoint audit, passthrough, timeout, LKG, rotation** — `0f681eb` (test)

**Plan metadata:** *(this SUMMARY commit, see git log after this file lands)* (docs)

## Files Created/Modified

### Created (8 src + 5 tests)

- `crates/holt-supervisor/src/lib.rs` — public surface; module-level C1/C5/C6 doc-comment.
- `crates/holt-supervisor/src/options.rs` — `SupervisorOptions`, `SupervisorOutcome`, `BreachKind`, `DEFAULT_TIMEOUT`.
- `crates/holt-supervisor/src/paths.rs` — XDG-aware cache root + per-file path helpers.
- `crates/holt-supervisor/src/supervisor.rs` — the C1 chokepoint; the only `.wrap(ProcessGroup::leader())` call site; mpsc + thread deadline; LKG refresh on `exit == 0`; timings + breach writes.
- `crates/holt-supervisor/src/lkg.rs` — `write_lkg`/`read_lkg` over `holt_schemas::atomic_write`.
- `crates/holt-supervisor/src/timings.rs` — `append_timings` + shared internal `append_jsonl` with 5MB rotation; constants `MAX_BYTES`.
- `crates/holt-supervisor/src/breaches.rs` — D-13 record schema (`BreachRecord`), `ENV_ALLOWLIST`, `STDIN_EXCERPT_CAP` (2KB), `STDERR_EXCERPT_CAP` (4KB), `writer_version` stamp.
- `crates/holt-supervisor/src/kill.rs` — `killpg` + Linux `/proc` PPID-walk fallback on `EPERM`.
- `crates/holt-supervisor/tests/chokepoint_audit.rs` — C1 textual audit (1 test).
- `crates/holt-supervisor/tests/passthrough_smoke.rs` — `bash -c "echo hello"` → Ok + valid timings.jsonl line (1 test, Unix-gated).
- `crates/holt-supervisor/tests/timeout_killpg_smoke.rs` — `bash -c 'sleep 5'` + 1s timeout → Breach{Timeout} within 1.5s, no orphans, breaches.log entry (1 test, Unix-gated).
- `crates/holt-supervisor/tests/lkg_roundtrip.rs` — Ok outcome → valid `LkgEntry` round-trip (1 test, Unix-gated).
- `crates/holt-supervisor/tests/jsonl_rotation.rs` — 5MB pre-fill → rename to `.1` on next append (1 test, platform-agnostic).

### Modified (3)

- `Cargo.toml` (workspace) — `process-wrap` dep gained `features = ["std", "process-group"]`. `std` is opt-in upstream; without it, `process_wrap::std::*` does not exist.
- `crates/holt-supervisor/Cargo.toml` — declares `holt-schemas`, `process-wrap.workspace`, `serde.workspace`, `serde_json.workspace`, `jiff.workspace`, `anyhow.workspace`; `cfg(unix)` adds `nix.workspace`; dev-deps add `tempfile.workspace`.
- `crates/holt-schemas/src/lkg.rs` — added `LkgEntry::new(stdout, exit_code, captured_at, duration_ms)` constructor (see Deviations § Rule 3).

## Decisions Made

| ID | Decision | Where it landed |
|----|----------|-----------------|
| D-09 | Single chokepoint API `wrap_and_run` | `crates/holt-supervisor/src/supervisor.rs` (sole `.wrap(ProcessGroup::leader())`) |
| D-10 | LKG cache file at `<cache_root>/lkg/<sid>.json`, schema_version-tagged | `paths.rs::lkg_path` + `lkg.rs::write_lkg/read_lkg` |
| D-11 | Default timeout 2s; killpg on breach; H3 fallback | `options.rs::DEFAULT_TIMEOUT`, `supervisor.rs` (timeout branch), `kill.rs` |
| D-12 | timings.jsonl 5MB / `.1` rotation, write-only | `timings.rs::append_timings` + `append_jsonl` |
| D-13 | breaches.log JSONL with allowlist + size-capped excerpts | `breaches.rs::BreachRecord` + `ENV_ALLOWLIST` |
| (new) | `Supervisor` marker struct + `Supervisor::wrap_and_run` namespaced alias | Honors the interface contract in 01-02-PLAN.md (`pub use supervisor::{Supervisor, SupervisorOutcome};`) without breaking the single-chokepoint discipline (the alias delegates to the free function). |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] `LkgEntry` is `#[non_exhaustive]`; struct-literal construction from `holt-supervisor` would not compile**

- **Found during:** Task 1 (writing `supervisor.rs`)
- **Issue:** Plan 01-01 marked `LkgEntry` `#[non_exhaustive]` per D-08 to keep schema bumps add-only-compatible. But `#[non_exhaustive]` on a struct forbids construction with `LkgEntry { ... }` syntax from outside the defining crate. The plan's snippet for `supervisor.rs` does exactly that. Without a constructor, `cargo build -p holt-supervisor` fails with `E0639: cannot create non-exhaustive struct using struct expression`.
- **Fix:** Added `pub fn LkgEntry::new(stdout, exit_code, captured_at, duration_ms) -> Self` to `crates/holt-schemas/src/lkg.rs`. Constructor stamps `schema_version = SCHEMA_VERSION` automatically and is documented as the canonical construction path.
- **Files modified:** `crates/holt-schemas/src/lkg.rs` (+15 lines, no behavior change to the existing struct).
- **Verification:** `cargo build --workspace --release` clean; `cargo test -p holt-schemas` (12 tests) still passes; `lkg_roundtrip.rs` round-trips through `serde_json::from_slice` and confirms the constructor sets the right schema_version.
- **Committed in:** `e4e5014` (Task 1 commit).

**2. [Rule 3 — Blocking] `process-wrap` `std` feature is opt-in upstream**

- **Found during:** Task 1 (first `cargo build -p holt-supervisor`)
- **Issue:** Workspace pin was `process-wrap = "=9.1.0"` with default features. `process_wrap::std::{CommandWrap, ProcessGroup}` is gated behind the `std` feature, which is NOT in the default set. First build error: `unresolved import process_wrap::std`.
- **Fix:** Updated workspace dependency to `process-wrap = { version = "=9.1.0", features = ["std", "process-group"] }`. Kept `process-group` explicit even though it's a default; dropped the unused `kill-on-drop`, `process-session`, `creation-flags`, `tracing`, `job-object` defaults to minimize transitive deps. (Tracing still lands transitively because the upstream re-exports it; future audit if it grows.)
- **Files modified:** `Cargo.toml` (workspace.dependencies).
- **Verification:** `cargo build --workspace --release` clean; `cargo tree -i tokio` returns "no matches" (no async runtime introduced).
- **Committed in:** `e4e5014` (Task 1 commit).

**3. [Rule 2 — Style/correctness] Plan-snippet `Supervisor` struct + impl block**

- **Found during:** Task 1 (matching the interfaces table)
- **Issue:** The PLAN frontmatter `key_links` and the interfaces snippet both say `pub use supervisor::{Supervisor, SupervisorOutcome};`, but the plan body's `src/lib.rs` snippet only re-exported `wrap_and_run` (the free function), not `Supervisor`. To honor the interfaces contract without breaking the single-chokepoint discipline, I added a zero-sized `pub struct Supervisor;` with `impl Supervisor { pub fn wrap_and_run(...) -> SupervisorOutcome { wrap_and_run(...) } }` — a thin convenience that delegates.
- **Fix:** `Supervisor::wrap_and_run` is the namespaced alias; `wrap_and_run` (the free function) remains the actual chokepoint. There is still EXACTLY ONE `.wrap(ProcessGroup::leader())` call site (in the free function); the impl method calls through.
- **Files modified:** `crates/holt-supervisor/src/supervisor.rs`, `crates/holt-supervisor/src/lib.rs`.
- **Verification:** `chokepoint_audit.rs` passes (single call site); `pub fn wrap_and_run` matches twice (free function + impl method) — but the impl method does NOT contain the wrap call; it delegates. `grep -c '\.wrap(ProcessGroup::leader())' crates/holt-supervisor/src/*.rs` returns `1`.
- **Committed in:** `e4e5014` (Task 1 commit).

**4. [Rule 1 — Bug, clippy] `needless_borrow` in `finalize_spawn_fail`**

- **Found during:** Task 1 (first `cargo clippy --all-targets -- -D warnings`)
- **Issue:** `append_timings_for(&opts, ...)` was passed an `&SupervisorOptions` reference, but the parameter already had an `&` and clippy flagged the immediate-deref pattern (`-D clippy::needless-borrow`).
- **Fix:** Changed call site to `append_timings_for(opts, ...)` (the function's first parameter is already `&SupervisorOptions`).
- **Files modified:** `crates/holt-supervisor/src/supervisor.rs` (one line).
- **Committed in:** `e4e5014` (Task 1 commit).

---

**Total deviations:** 4 auto-fixed (1 Rule 1 clippy, 1 Rule 2 style, 2 Rule 3 blocking).
**Impact on plan:** All four were necessary for the code to compile or pass the workspace gates. None affect behavior; none expand scope. The `LkgEntry::new` addition (Rule 3 #1) is a tiny, additive API surface in `holt-schemas` — Phase 2 hooks will use the same constructor when they instantiate `LkgEntry` for round-trip tests.

## Issues Encountered

- **rustfmt reformatted hand-written code** during `cargo fmt --all`. Formatter normalized `&stderr_bytes[..stderr_bytes.len().min(4096)]).into_owned()` line wrapping, the `nix` re-export ordering (`Signal::SIGKILL` before `kill`), and `Stdio::piped()` argument layout. All changes were stylistic; no semantics changed and all tests still pass. Treated as expected formatter behavior, not a deviation.
- **Plan-snippet line `let _ = MAX_BYTES;` in `breaches.rs`** would have emitted a clippy `let_unit_value` or `path_statements` lint and was unnecessary — the `MAX_BYTES` cap is enforced inside `append_jsonl` (the helper called by `append_breach`), so re-mentioning it in `breaches.rs` adds no enforcement. Dropped from the implementation. (This was a snippet-level redundancy, not a formal deviation — the surrounding logic already covers the policy.)

## Hygiene Verification (the load-bearing constraints)

```
.wrap(ProcessGroup::leader()) call sites in src/:                   1   ← C1 enforced
Stdio::piped() in supervisor.rs (≥3 expected):                      3   ← C1 ordering
Stdio::inherit() across src/ (must be 0):                           0   ← C1 negative
pub fn wrap_and_run in supervisor.rs:                               2   ← free fn + Supervisor impl alias
pub struct SupervisorOptions / pub enum SupervisorOutcome:          1 / 1
pub fn append_breach / append_timings / write_lkg / kill_process_group: 1 / 1 / 1 / 1
killpg occurrences in kill.rs (≥1):                                 9
EPERM occurrences in kill.rs (≥1):                                  5
cargo tree -i tokio:                                                no matches
cargo fmt --check:                                                  clean
cargo clippy --workspace --all-targets -- -D warnings:              clean
cargo build --workspace --release:                                  exits 0
cargo test -p holt-supervisor:                                      5/5 pass
```

## Test Results

```
running 1 test  (chokepoint_audit)
test only_one_wrap_call_site_in_supervisor_crate ... ok

running 1 test  (jsonl_rotation)
test timings_jsonl_rotates_at_5mb ... ok

running 1 test  (lkg_roundtrip)
test ok_outcome_writes_readable_lkg_entry ... ok

running 1 test  (passthrough_smoke)
test echo_hello_returns_ok_and_writes_timings ... ok

running 1 test  (timeout_killpg_smoke)
test timeout_breach_kills_descendants_and_writes_breach_log ... ok

5 passed; 0 failed; 0 ignored
```

`pgrep -f 'sleep 5'` returns empty after the timeout test — confirming ROADMAP success criterion #2's "no orphan descendants" clause.

## Patterns Established (consumed by plan 03)

1. **`holt_supervisor::wrap_and_run` is THE supervised-spawn API.** Plan 03's `holt run` subcommand and `--self-bench` harness call it directly; `chokepoint_audit.rs` will continue to fail any second wrap call site introduced anywhere in the crate.
2. **Telemetry write-only invariant.** `timings.jsonl` and `breaches.log` are opened with `OpenOptions::create(true).append(true)` only. Plan 03's `tests/render_path_no_read.rs` will instrument `holt run` to assert neither file is opened for reading on the render path. C6 enforced from this commit forward.
3. **`SupervisorOptions::with_defaults(session_id, cache_root)`** gives plan 03 a clean construction path for the CLI; `--timeout` (humantime parse) and `--session-id` (clap arg) just override the defaults.
4. **mpsc + thread deadline pattern.** Future supervised work (e.g., v0.5 `holt doctor`'s synthetic statusLine harness) should reuse this pattern via the same `wrap_and_run` chokepoint, NOT add another spawn site.

## User Setup Required

None - this plan introduces no external services, no environment variables, no daemon configuration. The supervisor is library code consumed by `holt-cli` (plan 03).

## Follow-ups (for plan 03 / future)

- **Plan 01-03 (`holt-cli` + architecture_dag + CI):** wire `holt run --timeout` (humantime parse) and `holt run` stdin pumping into `SupervisorOptions::stdin_bytes`; add `tests/architecture_dag.rs` walking `cargo metadata` to enforce C2 (`holt-render` ↛ `holt-supervisor`); add `tests/render_path_no_read.rs` instrumenting `holt run` to assert neither breaches.log nor timings.jsonl is opened for reading; add `cargo tree -i tokio` and `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` as required CI gates.
- **macOS H3 fallback** (post-v0.1): wire `libproc::proc_listchildpids` (`libproc-sys` crate) for the macOS sandbox case where `killpg` returns EPERM. Trigger to harden: ≥1 macOS-tagged "killpg failed under sandbox" issue.
- **Windows JobObject children-on-parent-exit race** (deferred): JobObject auto-reaps when the parent exits, but a wedged Windows child may linger between Breach{Timeout} and parent exit. Trigger to harden: ≥1 Windows-tagged "leftover process" report.
- **`tracing` transitive dep audit:** `process-wrap` v9.1.0's default features re-export `tracing` even when only `["std", "process-group"]` is requested. We don't use it on the render path; if the cold-start budget tightens, audit whether dropping `tracing` upstream is feasible.
- **`v0.5 holt doctor`:** the only future reader of `timings.jsonl` and `breaches.log`; will need a JSONC-tolerant ndjson parser and the `writer_version` field to detect schema drift (already stamped on every breach record).

## Self-Check

Files claimed → verified:

- `crates/holt-supervisor/src/{lib,options,paths,supervisor,lkg,timings,breaches,kill}.rs`: ALL FOUND (8/8)
- `crates/holt-supervisor/tests/{chokepoint_audit,passthrough_smoke,timeout_killpg_smoke,lkg_roundtrip,jsonl_rotation}.rs`: ALL FOUND (5/5)
- `crates/holt-supervisor/Cargo.toml`: FOUND (modified)
- `Cargo.toml` (workspace): FOUND (modified — `process-wrap` features added)
- `crates/holt-schemas/src/lkg.rs`: FOUND (modified — `LkgEntry::new` added)

Commits claimed → verified:
- `e4e5014` (Task 1, feat): FOUND in `git log --oneline -5`
- `0f681eb` (Task 2, test): FOUND in `git log --oneline -5`

## Self-Check: PASSED

---
*Phase: 01-schema-supervisor-substrate*
*Completed: 2026-04-28*
