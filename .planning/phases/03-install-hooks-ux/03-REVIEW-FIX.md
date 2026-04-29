---
phase: 3
phase_name: install-hooks UX
status: all_fixed
fix_scope: critical_warning
findings_in_scope: 6
fixed: 6
skipped: 0
iteration: 1
fixed_at: 2026-04-28
---

# Phase 3: Code Review Fix Report

**Fixed at:** 2026-04-28
**Source review:** `.planning/phases/03-install-hooks-ux/03-REVIEW.md`
**Iteration:** 1
**Pre-fix verification:** PASSED — 75/75 tests, 5/5 ROADMAP must-haves, 6/6 hard constraints (C1..C6). Fix-pass MUST NOT regress.

**Summary:**
- Findings in scope: 6 (all WARNING; 0 CRITICAL; 5 INFO deferred)
- Fixed: 6
- Skipped: 0
- Test count after fixes: 80/80 passing (added 3 regression tests: WR-01, WR-03×2)
- Quality gates after fixes: build clean, clippy `-D warnings` clean, `cargo fmt --check` clean
- Hard constraints after fixes: 6/6 still green (C1 chokepoint_audit, C2 architecture_dag, C3+C4 install_hooks_concurrent + install_hooks_sigkill + cli_dep_boundary, C5 reader_contract, C6 render_path_no_read)

## Fixed Issues

### WR-01: `acquire_settings_lock` creates settings.json before lock contention is resolved

**Files modified:** `crates/holt-cli/src/install_hooks/lock.rs`
**Commit:** `2cc94be`
**Applied fix:** Gated `OpenOptions::create(true)` on a `path.exists()` probe so the fresh-system + lock-timeout case no longer leaves a zero-byte settings.json on disk. Race-tolerant: `create(true)` (without `create_new(true)`) is idempotent if another process creates the file between probe and our open. Added `fresh_path_creates_zero_byte_file_only_on_success` regression test. Updated rustdoc on `acquire_settings_lock` with a "WR-01 contract" paragraph naming the no-create-on-timeout invariant. All 5 lock unit tests pass; the 50× concurrent and 200× SIGKILL stress tests continue to pass.

### WR-02: `merge.rs::merge_settings` panics if `append("hooks", Object(...))` returns a non-object

**Files modified:** `crates/holt-cli/src/install_hooks/merge.rs`, `crates/holt-cli/src/install_hooks_cmd.rs`
**Commit:** `f08912d`
**Applied fix:** Replaced `expect("just-appended hooks value is an object")` with a typed `MergeError::CstShape` variant. The dispatcher in `install_hooks_cmd.rs` routes `CstShape` to a clean stderr-with-hint exit (matching the `MergeError::Parse` pattern) and includes a "please file a bug" pointer because this branch indicates a jsonc-parser API contract violation, not user input. Logically unreachable on jsonc-parser 0.26.x today; the typed error future-proofs against minor-version bumps that could change the CST shape contract.

### WR-03: `string_property` uses `trim_matches('"')` instead of decoding the JSON string literal

**Files modified:** `crates/holt-cli/src/install_hooks/merge.rs`
**Commit:** `91a5869`
**Applied fix:** Switched from `raw_value().trim_matches('"').to_string()` to `CstStringLit::decoded_value().ok()` — jsonc-parser 0.26.3's proper JSON-string unescape. Decode errors map to `None` per the defensive-parse posture (Phase 1+2 precedent: never panic on user-shaped input; just refuse to treat the entry as canonical/holt-owned). Added two regression tests: (a) `wr03_escaped_canonical_command_is_recognized_as_canonical` — escape-laden canonical command (e.g., `"holt hook PreToolUse"` decodes to `"holt hook PreToolUse"`) is recognised as canonical and not replaced (D-08 byte-equal idempotency holds); (b) `wr03_escaped_substring_match_still_replaced` — escape-laden non-canonical entry with a substring match still replaces in-place (D-10 detect-by-substring works). Both tests pass on the new decoded path. Updated rustdoc on `string_property` documenting WR-03 and the defensive-parse choice.

### WR-04: `--dry-run` reads settings.json without holding the fs2 lock

**Files modified:** `crates/holt-cli/src/cli.rs`, `crates/holt-cli/src/install_hooks_cmd.rs`
**Commit:** `9631d65`
**Applied fix:** Per the review's recommended cheaper option, documented the lock-free read contract at the user-facing surface rather than acquiring a 200ms shared lock for negligible benefit on a sub-second concurrent window. Updates: (a) clap help text in `cli.rs` for `InstallHooks` and the `--dry-run` / `--print` flag docs explicitly note that dry-run is "a preview — if another `holt install-hooks` is concurrently running, the diff may show changes that are already applied by the time you read it"; (b) `install_hooks_cmd.rs` module doc adds a "WR-04" section explaining the rationale and an explicit "WR-05" cross-reference paragraph naming the panic-abort lock-release contract (companion to WR-05's lock.rs doc).

### WR-05: Lock release on panic depends solely on stack unwinding

**Files modified:** `crates/holt-cli/src/install_hooks/lock.rs`
**Commit:** `de513b7`
**Applied fix:** Added an explicit "Lock release semantics (WR-05 contract)" section to `lock.rs`'s module-level rustdoc (top of file, first thing maintainers see). Names both release paths (1. normal `Drop` via dispatcher's explicit `drop(lock_handle)` on every return path; 2. panic/abort via OS reaping the file descriptor at process exit), documents the OS-reap fallback, and warns future maintainers about `panic = "unwind"` switch + `catch_unwind` wrapper considerations. The companion module doc in `install_hooks_cmd.rs` (WR-04 commit) cross-references this section. Doc-only — no behavior change.

### WR-06: SIGKILL test does not assert clean tmp-file state at the end

**Files modified:** `crates/holt-cli/tests/install_hooks_sigkill.rs`
**Commit:** `4191831`
**Applied fix:** Added a final `orphans.len() <= 1` assertion after the 200-iter loop. Bound is ≤1 (not zero, like the concurrent test) because per-iter cleanup wipes prior leaks but the FINAL iter's tmp file (if SIGKILL hit between tmp-open and rename) survives until the next invocation. Anything >1 indicates per-iter cleanup is broken OR atomic_write is leaking on the success path (a real C3 contract regression). Comment block documents the upper bound's reasoning and notes that production-side orphan recovery (sweep stale `.holt-tmp.*` at the start of each `holt install-hooks` invocation) is deferred — v0.1 ships without the sweep because tmp files are 0o600 and cosmetically harmless. Test passes (4.02s release wall clock, well under 60s budget).

## Skipped Issues

None. All 6 in-scope warnings were fixed.

## Deferred (not in fix_scope)

The 5 INFO findings from REVIEW.md were deferred per the `critical_warning` fix scope. They are documented here for traceability:

- **IN-01** — `MergeError::NotAnObject { got }` field always passes `"non-object root"`. Cosmetic; defer.
- **IN-02** — `pretty_snippet` does not escape `command` strings for JSON safety. Today's static `HOLT_HOOK_ENTRIES` is known-safe ASCII; defer until a future plan registers user-derived entries.
- **IN-03** — `extract_quoted_keys` test helper may misclassify values-followed-by-colon. Test passes today; defer.
- **IN-04** — `tests/cli_dep_boundary.rs::parse_package_name` duplicated from `architecture_dag.rs`. Deliberate; no action.
- **IN-05** — `pseudo_random_delay_ms` returns `0..=30` (inclusive), so ~6 of 200 iterations land on 0ms ("killed before any work"). Optional bias to `1..=30`; defer.

## Verification (post-fix)

| Gate | Pre-fix | Post-fix | Notes |
|------|---------|----------|-------|
| `cargo build --workspace --release` | clean | clean | 6.90s |
| `cargo test --workspace --release` | 75 passed | **80 passed** | +3 regression tests (1 lock + 2 merge); +2 from earlier delta unaccounted for in raw count but no failures |
| `cargo clippy --all-targets -- -D warnings` | clean | clean | no warnings |
| `cargo fmt --check` | clean | clean | no diffs |
| `cargo tree -i tokio` | empty | empty | no async runtime introduced |
| Forbidden-crate audit | clean | clean | no new deps; jsonc-parser + fs2 still confined to holt-cli |

## Regression Check (against verified phase output)

| ROADMAP Must-Have | Pre-fix | Post-fix |
|-------------------|---------|----------|
| 1 — Clean settings.json + statusLine preserved + .holt.bak byte-equal | VERIFIED | PRESERVED |
| 2 — JSONC line comments + key order preserved + user PreToolUse co-exists | VERIFIED | PRESERVED |
| 3 — `--dry-run` no mutation, `--print` no mutation, mutual exclusion | VERIFIED | PRESERVED |
| 4 — 50× concurrent stress | VERIFIED (0.66s) | PRESERVED |
| 5 — 200× SIGKILL atomicity | VERIFIED (3.97s) | PRESERVED (4.02s, plus new ≤1 orphan assertion) |

| Hard Constraint | Pre-fix | Post-fix | Test |
|-----------------|---------|----------|------|
| C1 — Pipe stdio for supervised processes | green | green | `holt-supervisor::chokepoint_audit` 1 passed |
| C2 — holt-render does NOT depend on holt-supervisor | green | green | `architecture_dag` 1 passed; `cargo tree` empty |
| C3 — fs2 lock + fsync-before-rename + .holt.bak + PID-suffix tmp | green | green | install_hooks_concurrent + install_hooks_sigkill (200 iters in 4.02s) |
| C4 — JSONC + fs2 deps live ONLY in holt-cli | green | green | `cli_dep_boundary` 1 passed |
| C5 — Reader treats stale-or-corrupt as missing | green | green | `holt-schemas::reader_contract` 9 passed |
| C6 — Render path never reads breaches.log/timings.jsonl | green | green | `holt-cli::render_path_no_read` 1 passed |

| Phase 2 contract | Pre-fix | Post-fix |
|------------------|---------|----------|
| `holt-hooks::sigkill_atomicity` (200× SIGKILL on hook write) | green | green (1 passed in 11.05s) |

No regressions detected. All 5 ROADMAP must-haves preserved; all 6 hard constraints preserved; Phase 2 SIGKILL atomicity contract preserved.

## Commits (in order, all on a separate worktree branch)

| # | Hash | Subject |
|---|------|---------|
| 1 | `2cc94be` | fix(holt-cli): gate create(true) on path.exists() in acquire_settings_lock (WR-01) |
| 2 | `f08912d` | fix(holt-cli): replace merge.rs expect() with typed MergeError::CstShape (WR-02) |
| 3 | `91a5869` | fix(holt-cli): decode JSON-string escapes in string_property (WR-03) |
| 4 | `9631d65` | fix(holt-cli): document --dry-run/--print lock-free contract (WR-04) |
| 5 | `de513b7` | fix(holt-cli): document panic-release contract in lock.rs module doc (WR-05) |
| 6 | `4191831` | test(holt-cli): assert final tmp-file state in SIGKILL stress test (WR-06) |

---

_Fixed: 2026-04-28_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
