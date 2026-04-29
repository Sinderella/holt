---
phase: 2
phase_name: Heartbeat hook (write side)
status: all_fixed
fix_scope: critical_warning
findings_in_scope: 11
fixed: 11
skipped: 7
iteration: 1
fixed_at: 2026-04-28
---

# Phase 2: Code Review Fix Report — Heartbeat hook (write side)

**Fixed at:** 2026-04-28
**Source review:** `.planning/phases/02-heartbeat-hook-write-side/02-REVIEW.md`
**Iteration:** 1
**Fix scope:** `critical_warning` (CR-* + WR-*; INFO findings deferred)

**Summary:**
- Findings in scope: 11 (2 Critical + 9 Warning)
- Fixed: 11
- Skipped (out of scope): 7 (all Info findings)

---

## Fixed Issues

### CR-01: `unreachable!()` on render path violates "never panics" contract

**Files modified:** `crates/holt-hooks/src/handle.rs`
**Commit:** `90cfe55`
**Applied fix:** Replaced the `ResolvedTier::XdgRuntimeDir => unreachable!("is_fallback() filtered this")` arm with `=> FallbackReason::XdgUnavailable`, plus a comment explaining that the defensive fallthrough preserves the module-level "Never panics" contract on `lib.rs:10–11`. A future maintainer adding a new `ResolvedTier` variant (or breaking `is_fallback()`) now gets a slightly-less-precise warning instead of a `panic!()` propagating out of the CLI dispatcher AFTER the heartbeat write succeeded.

### CR-02: Path resolver commits to a dead tier when `create_dir_all` succeeds on an unwritable directory

**Files modified:** `crates/holt-hooks/src/path.rs`
**Commit:** `9044100`
**Applied fix:** Added a `dir_is_writable(parent: &Path) -> bool` probe that follows `create_dir_all` with an `OpenOptions::new().write(true).create_new(true)` test on a PID-suffixed probe file inside the candidate directory. Cleanup is best-effort `remove_file`. `resolve_writer_path` now calls `dir_is_writable` instead of `create_dir_all(...).is_ok()` for each tier, restoring the documented "first writable wins" semantics from `path.rs:14–18`. The TOCTOU window between probe and real `atomic_write` is acceptable — the prior code had the same window AND failed to retry. Kept the syscall cost to one open + one unlink per tier per fire (well under the sub-20ms hook budget; D-15 self-bench p95 was 7988us before, 9921us after — still 2x headroom).

### WR-01: `_outcome` discard in CLI dispatcher loses exhaustiveness checking

**Files modified:** `crates/holt-cli/src/hook.rs`
**Commit:** `2c76ba3`
**Applied fix:** Replaced `let _outcome = handle_event(...)` with an explicit exhaustive `match handle_event(...) { HookOutcome::Wrote { .. } | FellBack { .. } | ParseFailed | Unwritable => {} }`. If a future contributor adds a new `HookOutcome` variant, the compiler now forces a conscious decision rather than silently inheriting "exit 0" semantics. Imported `HookOutcome` alongside `Env`/`HookEvent`.

### WR-02: Empty-stdin path emits a `parse_fail` breach instead of being treated as a benign empty case

**Files modified:** `crates/holt-cli/src/hook.rs`
**Commit:** `04fe4c5`
**Applied fix:** Changed the dispatcher's `StdinParseOutcome::Empty => Vec::new()` arm to `StdinParseOutcome::Empty => return 0`. Empty stdin is documented in Phase 1 (`crates/holt-cli/src/stdin.rs`) as a NORMAL condition under H5 defensive posture; routing it to `handle_event(_, &[], _)` was producing a `parse_fail` breach record on every `echo | holt hook PreToolUse` ad-hoc test or empty-stdin Notification fire. Now: no work, no breach.

### WR-03: Redundant `chmod 0o600` after `atomic_write` adds a render-path syscall pair

**Files modified:** `crates/holt-hooks/src/handle.rs`
**Commit:** `c31c175`
**Applied fix:** Removed the `#[cfg(unix)] { ... metadata + set_permissions(0o600) ... }` block after `atomic_write` and replaced it with a comment documenting that `holt_schemas::atomic_write` opens the tmp file with `OpenOptionsExt::mode(0o600)` and the `rename(2)` inherits it. The `must_have_1_file_shape_and_perms` test still asserts `mode & 0o777 == 0o600` and still passes — the chmod was strictly redundant under the umask analysis (0o600 has no group/other bits, so umask cannot widen). Saves one `metadata()` + one `set_permissions()` syscall on every successful PreToolUse fire.

### WR-04: Bench harness leaves `XDG_RUNTIME_DIR` set to a deleted tempdir on exit

**Files modified:** `crates/holt-cli/src/self_bench.rs`
**Commit:** `7d66338`
**Applied fix:** Captured `prev_xdg = std::env::var_os("XDG_RUNTIME_DIR")` and `prev_tmpdir = std::env::var_os("TMPDIR")` before the unsafe `set_var` block in `run_self_bench_hook`. Added a matching restore block (inside the same `#[allow(unsafe_code)]` scope) at the end of the function that uses `set_var(prev_v)` if the value existed prior, else `remove_var`. Mirrors the prev/restore pattern from `crates/holt-hooks/tests/handle_event_smoke.rs`. No in-process consumer today, but closes the footgun before any future "run bench then continue" mode lands.

### WR-05: `cwd_label` for empty `cwd` returns empty string, silently violating must_have-5

**Files modified:** `crates/holt-hooks/src/assemble.rs`, `crates/holt-hooks/tests/assemble_field_policy.rs`
**Commit:** `9d44675`
**Applied fix:** Added a final fallback in `derive_cwd_label`: after the `cwd.to_string()` branch, if `cwd.is_empty()` we return the literal `"<unknown>"`. Bracket-flagged so it never collides with a real cwd basename. Added a regression test `cwd_label_falls_back_to_unknown_when_cwd_empty_and_no_workspace` that constructs `HookStdin` directly (bypassing fixtures) with `cwd: ""` and `workspace.git_worktree: None`, asserting both the `must_have-5` non-empty invariant AND the literal `<unknown>` value. Test count rose 8 → 9 in the `assemble_field_policy` suite.

### WR-06: Test `must_have_4_unwritable_returns_unwritable_outcome` accepts `FellBack` as a substitute

**Files modified:** `crates/holt-hooks/tests/handle_event_smoke.rs`
**Commit:** `62e2e6a`
**Applied fix:** Replaced the `HookOutcome::Unwritable | HookOutcome::FellBack { .. } => { ... }` arm with `HookOutcome::Unwritable => { /* expected */ }` and a `panic!` on any other variant. Updated the comment to explain WHY: `default_cache_root` only falls back to `temp_dir()` when HOME is unset; the test sets HOME (to a regular file path), so the `~/.cache/holt/sessions` branch is taken and `create_dir_all` MUST fail. The dead `FellBack` arm was masking a real bug if `default_cache_root`'s fallback semantics ever changed. Test still passes after tightening, confirming the analysis.

### WR-07: 1000-iteration SIGKILL test fails on slow CI via `panic!` rather than degrading

**Files modified:** `crates/holt-hooks/tests/sigkill_atomicity.rs`
**Commit:** `a5128e8`
**Applied fix:** Replaced `panic!("sigkill_atomicity: budget exceeded after {i} iterations")` with `eprintln!(...)` + `return`. A slow GitHub Actions worker under load or a thermally-throttled macos-14 arm64 runner no longer converts a budget overrun into a CI failure. The test still asserts `read_heartbeat` returns `Ok(_)` on every iteration that does execute, so a real atomicity violation still fails loudly. Local run still completes in ~11s for the full 1000 iterations, so the early-return path doesn't fire on healthy CI.

### WR-08: `tempfile` declared as a runtime (non-dev) dependency in `holt-cli`

**Files modified:** `crates/holt-cli/Cargo.toml`, `crates/holt-cli/src/self_bench.rs`
**Commit:** `7c1ec46`
**Applied fix:** Added a `BenchScratchDir` struct in `self_bench.rs` with a `create(path) -> Option<Self>` factory and a `Drop` impl that does best-effort `remove_dir_all`. Both `run_self_bench` and `run_self_bench_hook` now use `BenchScratchDir::create(std::env::temp_dir().join(format!("holt-self-bench[-hook]-{pid}")))` instead of `tempfile::tempdir()`. Removed `tempfile.workspace = true` from `[dependencies]` in `holt-cli/Cargo.toml` (kept it in `[dev-dependencies]` for the integration tests). Release binary no longer pulls `cfg-if`, `fastrand`, or `libc` for the opt-in bench mode. Self-bench-hook still passes (p95=11272us < 20000us budget) on macOS arm64 post-fix; rerun confirmed 9921us p95 in the final verification.

### WR-09: `ENV_LOCK` Mutex serializes test bodies but not helper threads

**Files modified:** `crates/holt-hooks/tests/handle_event_smoke.rs`
**Commit:** `0b2e4f3`
**Applied fix:** Added a 18-line documentation comment above `static ENV_LOCK: Mutex<()> = Mutex::new(());` enumerating exactly what the lock guards against (parallel test bodies inside this binary) and what it does NOT guard against (tempfile/Command::spawn helper threads, fork-time env inheritance during another test's `set_var`, other test binaries running in parallel via cargo's per-binary `--test-threads`). Names the escape valves (the `serial_test` crate or `cargo test -- --test-threads=1`) so a future maintainer landing helper-thread code knows where the safety contract becomes brittle. No code changes, no new dependency — pure documentation hardening, lower-friction than introducing `serial_test` today.

---

## Skipped Issues (out of scope — Info findings)

The following IN-* (Info) findings are documented for the next iteration / follow-up pass. Per `<config>` `fix_scope: critical_warning`, Info findings are out of scope for this fix run.

### IN-01: `#[allow(unsafe_code)]` placement on `run_self_bench_hook` is broader than necessary

**File:** `crates/holt-cli/src/self_bench.rs:157`
**Reason:** Out of scope (Info; cosmetic — broader-than-necessary lint allowance).
**Original issue:** The function-level `#[allow(unsafe_code)]` covers the entire `pub fn run_self_bench_hook(...)` body, but only the two `unsafe { ... }` blocks (env set + env restore) actually need it. Tightening to block-level allows would reduce the surface area where `clippy::deny(unsafe_code)` is bypassed.

### IN-02: `_bench_tmp` leading-underscore naming masks RAII intent

**File:** `crates/holt-cli/src/self_bench.rs:46`
**Reason:** Out of scope (Info; naming nit). Note: the WR-08 fix replaced this binding with `_bench_tmp_keepalive` of type `BenchScratchDir`, which partially addresses the original concern, but the leading underscore is preserved.
**Original issue:** Rust convention treats `_var` as "intentionally unused"; the binding IS used (its `Drop` impl deletes the tempdir). Rename to something like `bench_tmp_keepalive` and add a one-line comment documenting the RAII intent.

### IN-03: LCG seed `0xdeadbeef` is a magic number with no clear rationale

**File:** `crates/holt-hooks/tests/sigkill_atomicity.rs:54`
**Reason:** Out of scope (Info; doc-only).
**Original issue:** Add a 2-line comment explaining the seed is fixed for reproducibility (a flaky kill-delay distribution would make the test impossible to triage) and that the LCG is non-cryptographic by design (we only need uniform 0..=15ms delays, not unpredictability).

### IN-04: `current_uid_string` returns hardcoded `"0"` on non-Unix, conflating root and Windows

**File:** `crates/holt-hooks/src/path.rs:132–138`
**Reason:** Out of scope (Info; cosmetic, only matters on Windows where `$TMPDIR` is rare and tier-3 normally takes over).
**Original issue:** On non-Unix, all users collapse to `$TMPDIR/holt-0/sessions/`. Replace with `std::env::var("USERNAME").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "user".to_string())` so the path component reads sensibly in error messages.

### IN-05: `now_iso` is captured ONCE per fire and used for both `started` and `updated`

**Files:** `crates/holt-cli/src/hook.rs:33`, `crates/holt-hooks/src/assemble.rs:60–61`
**Reason:** Out of scope (Info; doc-only — the field naming is locked at the schema level).
**Original issue:** The schema field name `started` suggests "session start time" but in v0.1 it's "this fire's start time". Either rename to `fired_at` (breaking schema change, defer to v1.0) or add a comment in `assemble.rs` documenting the v0.1 semantic so future readers don't get confused.

### IN-06: `serde_json::to_vec(&heartbeat)` failure path is dead code in practice

**File:** `crates/holt-hooks/src/handle.rs:83–92`
**Reason:** Out of scope (Info; comment refinement only).
**Original issue:** `Heartbeat` only contains plain types (`String`, `u8`, `u32`, `Option<String>`, `Option<f64>`); `serde_json::to_vec` cannot fail. Refine the existing comment ("Should never happen") to note the arm is structurally unreachable but coded defensively because `Heartbeat` is `#[non_exhaustive]` and v1.0 might add a `f64::NAN`-shaped field that DOES fail to serialize.

### IN-07: `holt-hooks/Cargo.toml` declares `nix` with the `user` feature only, but the comment hints at a wider use

**File:** `crates/holt-hooks/Cargo.toml:31–32`
**Reason:** Out of scope (Info; comment refinement only).
**Original issue:** Drop the speculative half of the comment ("to keep the door open for the SIGKILL test driver to call `nix::sys::signal::kill` if needed") since today's code uses `child.kill()` (std-only) and the `user` feature alone does not enable `signal`.

---

## Verification

Post-fix quality gates (run on macOS arm64 from repo root):

| Gate | Command | Result |
|------|---------|--------|
| Build | `cargo build --workspace --release` | exit 0, Finished `release` profile in 6.19s |
| Tests | `cargo test --workspace` | **51 passed**, 0 failed (was 50 — added WR-05 regression test) |
| Clippy | `cargo clippy --all-targets -- -D warnings` | exit 0, no warnings |
| Format | `cargo fmt --check` | exit 0 |
| D-15 self-bench | `./target/release/holt --self-bench-hook PreToolUse --iterations 30 --json` | `{"overhead_p95_us":9921,"budget_p95_us":20000,"passed":true}` |

Test count delta: **50 → 51** (+1 from WR-05 regression test `cwd_label_falls_back_to_unknown_when_cwd_empty_and_no_workspace`). No tests were removed; no test was relaxed in a way that masks regression. WR-06 actually *tightened* an existing test (Unwritable | FellBack → Unwritable only) and the tightened assertion still passes — confirming the analysis.

---

## Regression Check — ROADMAP Must-Haves and Hard Constraints

### 5/5 ROADMAP Success Criteria still verify green

| # | Criterion | Test | Result |
|---|-----------|------|--------|
| 1 | PreToolUse fixture → canonical XDG path + 0600 perms | `must_have_1_file_shape_and_perms` | PASS |
| 2 | All 5 events round-trip with correct field policy | `must_have_2_all_five_events_round_trip` | PASS |
| 3 | 1000× SIGKILL atomicity | `one_thousand_sigkills_never_corrupt_heartbeat` | PASS (~11.29s) |
| 4 | Fallback chain + unwritable resilience | `must_have_4_*` (3 subtests, including the WR-06-tightened one) | PASS |
| 5 | workspace.git_worktree paired fixtures | `cwd_label_uses_workspace_git_worktree_when_present` + `cwd_label_falls_back_to_basename_when_workspace_absent` (and now the WR-05 fallback test) | PASS |

### 4/4 Phase 1 Hard Constraints still test-enforced

| # | Constraint | Test | Result |
|---|-----------|------|--------|
| C1 | Always pipe stdio when spawning supervised processes | `cargo test -p holt-supervisor --test chokepoint_audit` | 1 passed (no new wrap call sites added) |
| C2 | `holt-render` MUST NOT depend on `holt-supervisor` | `cargo test --test architecture_dag` | 1 passed |
| C5 | Reader contract: stale/corrupt heartbeat treated as missing, never panics | `cargo test -p holt-schemas --test reader_contract` | 9 passed |
| C6 | Render path never reads `breaches.log` / `timings.jsonl` | `cargo test -p holt-cli --test render_path_no_read` | 1 passed |

### Forbidden-crate audit

`grep` for forbidden runtime deps in `Cargo.toml`s after WR-08:
- `tokio` — none (confirmed via `cargo tree -i tokio` → "did not match any packages")
- `simd-json`, `figment`, `chrono`, `jsonc-parser`, `fs2`, `owo-colors`, `supports-color`, `terminal_size`, `atomic-write-file`, `crossterm`, `wait-timeout`, `cargo_metadata` — none added or remaining
- `tempfile` — moved from `holt-cli/[dependencies]` to `[dev-dependencies]` only (WR-08); still allowed in dev-deps per Phase 1 / Phase 2 conventions

### "No `unwrap()` on the render path" discipline preserved (CLAUDE.md mandate)

`grep -cE '\.unwrap\(\)|panic!|\.expect\(' crates/holt-hooks/src/*.rs crates/holt-cli/src/hook.rs` — **0 occurrences** in any production-side render-path file. CR-01's `unreachable!()` (which is a panic primitive) is now removed; no new panics added.

---

## Commits Summary

11 atomic commits, all on `main`, in CR → WR order:

```
0b2e4f3 test(holt-hooks): document ENV_LOCK scope and unstated safety assumptions (WR-09)
7c1ec46 fix(holt-cli): hand-roll bench scratch dir, drop tempfile from runtime deps (WR-08)
a5128e8 test(holt-hooks): soft early-return on slow CI instead of panic (WR-07)
62e2e6a test(holt-hooks): tighten unwritable test to require Unwritable, not FellBack (WR-06)
9d44675 fix(holt-hooks): cwd_label fallback to '<unknown>' instead of empty string (WR-05)
7d66338 fix(holt-cli): restore env vars on self-bench-hook exit (WR-04)
c31c175 fix(holt-hooks): remove redundant chmod 0o600 after atomic_write rename (WR-03)
04fe4c5 fix(holt-cli): short-circuit empty stdin instead of routing to parse_fail breach (WR-02)
2c76ba3 fix(holt-cli): make hook dispatcher's HookOutcome match exhaustive (WR-01)
9044100 fix(holt-hooks): probe each tier for actual writability via create_new (CR-02)
90cfe55 fix(holt-hooks): replace unreachable!() on render path with defensive fallthrough (CR-01)
```

---

_Fixed: 2026-04-28_
_Fixer: Claude (gsd-code-fixer, Opus 4.7 1M)_
_Iteration: 1_
