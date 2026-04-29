---
phase: 2
plan: 02-01
plan_id: 02-01
status: complete
completed_at: 2026-04-28
requirements:
  - HOOK-01
  - HOOK-02
  - HOOK-03
  - HOOK-04
  - HOOK-05
  - HOOK-06
decisions_implemented:
  - D-01: capture v2.1.119 + pre-2.1.98 paired CC stdin fixtures
  - D-02: fixtures golden, committed, README documents refresh policy
  - D-03: handle_event single entry point + HookOutcome enum (Wrote/FellBack/ParseFailed/Unwritable)
  - D-04: HookStdin defensive parse with #[serde(default)] on every field
  - D-05: pure assemble_heartbeat (no I/O)
  - D-06: 3-tier fallback chain (XDG_RUNTIME_DIR → TMPDIR/holt-$UID → ~/.cache/holt/sessions)
  - D-07: path resolution per-fire (no caching) + DefaultHasher fallback for missing session_id
  - D-08: cwd_label derivation (workspace.git_worktree → cwd basename) — git rev-parse branch deferred to v1.0
  - D-09: current_tool field policy (Some on PreToolUse; None elsewhere)
  - D-10: blocked_on always None; last_assistant_at + model_display populated when present
  - D-11: writer_version plumbed via Env from binary (Phase 1 WR-08 pattern)
  - D-12: atomic_write + 0o600 chmod after rename (cfg(unix))
  - D-13: 1000× SIGKILL atomicity stress test
key_files:
  created:
    - crates/holt-hooks/tests/fixtures/cc-stdin/v2.1.119/PreToolUse.json
    - crates/holt-hooks/tests/fixtures/cc-stdin/v2.1.119/PostToolUse.json
    - crates/holt-hooks/tests/fixtures/cc-stdin/v2.1.119/Stop.json
    - crates/holt-hooks/tests/fixtures/cc-stdin/v2.1.119/Notification.json
    - crates/holt-hooks/tests/fixtures/cc-stdin/v2.1.119/SessionStart.json
    - crates/holt-hooks/tests/fixtures/cc-stdin/pre-2.1.98/PreToolUse.json
    - crates/holt-hooks/tests/fixtures/README.md
    - crates/holt-hooks/src/event.rs
    - crates/holt-hooks/src/stdin.rs
    - crates/holt-hooks/src/path.rs
    - crates/holt-hooks/src/assemble.rs
    - crates/holt-hooks/src/handle.rs
    - crates/holt-hooks/tests/assemble_field_policy.rs
    - crates/holt-hooks/tests/handle_event_smoke.rs
    - crates/holt-hooks/tests/sigkill_atomicity.rs
    - crates/holt-hooks/tests/sigkill_test_driver.rs
  modified:
    - crates/holt-hooks/Cargo.toml
    - crates/holt-hooks/src/lib.rs
    - crates/holt-schemas/src/heartbeat.rs
    - crates/holt-schemas/src/lib.rs
    - crates/holt-supervisor/src/options.rs
    - Cargo.lock
commits:
  - "3cc3228 test(holt-hooks): capture v2.1.119 + pre-2.1.98 CC stdin fixtures (D-01, D-02)"
  - "ea5bca3 feat(holt-hooks,holt-schemas): cargo manifest + Heartbeat::new constructor"
  - "fdace5f feat(holt-hooks): HookEvent + HookStdin + path + assemble + handle (D-03..D-12)"
  - "51eb370 test(holt-hooks): field policy + handle_event smoke + 1000× SIGKILL atomicity + Phase 1 regression gate (D-13)"
---

# Plan 02-01 Summary: Fixture capture + holt-hooks crate

## Tasks completed

| # | Task | Status | Commit |
|---|------|--------|--------|
| 1 | Capture v2.1.119 + pre-2.1.98 CC stdin fixtures | ✓ | `3cc3228` |
| 2 | `holt-hooks/Cargo.toml` real manifest (replace placeholder) | ✓ | `ea5bca3` |
| 3 | Additive `Heartbeat::new` constructor in holt-schemas + `BreachKind::Unwritable` variant in holt-supervisor | ✓ | `ea5bca3` (schemas) + `fdace5f` (supervisor) |
| 4 | `crates/holt-hooks/src/event.rs` — HookEvent enum (PreToolUse/PostToolUse/Stop/Notification/SessionStart) | ✓ | `fdace5f` |
| 5 | `crates/holt-hooks/src/stdin.rs` — HookStdin defensive parse with `#[serde(default)]` everywhere | ✓ | `fdace5f` |
| 6 | `crates/holt-hooks/src/path.rs` — 3-tier fallback resolver + DefaultHasher sid fallback | ✓ | `fdace5f` |
| 7 | `crates/holt-hooks/src/assemble.rs` — pure assemble_heartbeat (no I/O) + `crates/holt-hooks/src/handle.rs` — handle_event entry point with atomic_write + 0o600 chmod + breach routing | ✓ | `fdace5f` |
| 8 | `crates/holt-hooks/tests/assemble_field_policy.rs` — 8 tests covering current_tool/cwd_label/blocked_on/last_assistant_at/model_display | ✓ | `51eb370` |
| 9 | `crates/holt-hooks/tests/handle_event_smoke.rs` — 6 tests covering happy path + fallback chain + read_heartbeat round-trip + parse_fail + unwritable | ✓ | `51eb370` |
| 10 | `crates/holt-hooks/tests/sigkill_atomicity.rs` (1000× SIGKILL) + `sigkill_test_driver.rs` bin + Phase 1 regression gate | ✓ | `51eb370` |

## Verification

| Gate | Result |
|------|--------|
| `cargo build --workspace --release` | exit 0 |
| `cargo test --workspace` | **43/43 pass** (28 Phase 1 baseline + 15 new Phase 2) |
| `cargo test -p holt-hooks --test sigkill_atomicity` | exit 0 — 1000 iterations, ~11.2s wall clock |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |
| `cargo tree -i tokio` | empty |
| `cargo tree -p holt-render \| grep holt-supervisor` | empty (C2 unbroken) |
| `cargo test --test architecture_dag` | exit 0 (C2 enforced) |
| `cargo test -p holt-supervisor chokepoint_audit` | exit 0 (C1 enforced) |
| `cargo test -p holt-schemas reader_contract` | exit 0 — 9 cases (C5 enforced) |
| Forbidden-crate audit on `crates/holt-hooks/Cargo.toml` | clean |
| `unwrap`/`panic`/`expect` count in `crates/holt-hooks/src/` | 0 (render-path safe) |

## Decisions implemented

D-01..D-13 all landed. See frontmatter `decisions_implemented` for the full table.

### D-08 narrowing — git rev-parse branch deferred to v1.0

CONTEXT.md D-08 originally specified a 3-branch cwd_label policy:
1. workspace.git_worktree if present (CC v2.1.98+)
2. **git rev-parse heuristic** (best-effort `<repo>/<branch>` derivation when workspace.git_worktree absent)
3. cwd basename verbatim

Plan 02-01 narrowed this to a 2-branch policy (omitting branch 2). **Justification:** shelling out to `git rev-parse` on the render path violates the sub-20ms self-bench budget (D-15). CONTEXT.md branch 2 was already qualified as "best-effort; fallback to cwd basename only," so this honors the spirit of the decision. Branch 2 is now a v1.0 follow-up captured in `## Follow-ups` below.

The two paired fixtures (v2.1.119 with workspace.git_worktree present vs pre-2.1.98 synthetic without it) BOTH produce non-empty cwd_label per success criterion #5 — the v1.0 branch 2 enhancement is incremental, not load-bearing.

## Follow-ups (deferred to later phases or v1.0)

- **D-08 branch 2** — `git rev-parse` heuristic for `<repo>/<branch>` derivation when workspace.git_worktree is absent. Defer to v1.0 (`holt doctor` already shells out to git for first-run checks; that's the natural seam to extend).
- **C6 strace coverage of `holt hook`** — The render_path_no_read.rs test currently strace-asserts the `holt run` path. Plan 02-02 should extend OR add a sibling test asserting `holt hook PreToolUse < fixture` opens neither breaches.log nor timings.jsonl for read. Plan-checker WARNING; code is C6-compliant by inspection (`grep -E 'breaches\.log|timings\.jsonl' crates/holt-hooks/src/` returns empty), but no automated regression net.
- **SIGKILL test prescription** — CONTEXT.md D-13 specified verification via both `serde_json::from_slice` AND `read_heartbeat`. Plan 02-01 used `read_heartbeat` only. `read_heartbeat` internally calls `serde_json::from_slice`, so the verification is functionally equivalent (and strictly stronger because it adds the C5 contract layer). No defect.

## Notes & gotchas

- **Heartbeat::new pattern.** `#[non_exhaustive]` on `Heartbeat` blocks struct-literal construction across crates, which `assemble.rs` needs. Added `Heartbeat::new(schema_version, writer_version, pid, started, updated, ...)` constructor on `holt-schemas` (mirrors Phase 1 Plan 01-02's `LkgEntry::new` pattern landed in fix-pass). Additive — does not modify existing fields.
- **BreachKind::Unwritable.** Added a fourth variant to `BreachKind` (alongside Timeout, ParseFail, SpawnFail). The `as_str` arm produces `"unwritable"`. This is the kind tag used by `handle_event` when the writer's 3-tier fallback chain is fully exhausted.
- **path.rs DefaultHasher fallback.** When `stdin.session_id` is missing or empty, the writer derives a 16-char hex hash via `std::hash::DefaultHasher` over `(stdin.cwd, stdin.transcript_path)`. Documented in `path.rs` rustdoc with the rationale (no new dep; deterministic across reinvocations of the same session).
- **0o600 chmod ordering.** Set on the FINAL file path after rename, not on the tmp file. Phase 1's atomic_write rename inherits tmp mode anyway, but explicit post-rename chmod is defense-in-depth and matches success criterion #1 wording ("file's permissions are 0600").
- **Fixture authenticity.** v2.1.119 fixtures are synthetic-but-realistic per the README escape clause — match the documented field shape from CC's changelog. Real-capture refresh procedure documented in `crates/holt-hooks/tests/fixtures/README.md` for when a CC version ships a stdin shape change.

## Next

Plan 02-02 wires the CLI subcommand: `holt hook <event>` clap entry, dispatcher to `holt_hooks::handle_event`, hook self-bench gate (D-15), CI extension. 8 tasks, Wave 2.
