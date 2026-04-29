# State: holt

**Project:** holt — Rust statusLine for Claude Code with multi-session orchestration and an ASCII otter pet (Nak)
**Milestone:** v0.1 — Runtime hygiene wedge (the lovable MVP, target 3–4 weekends)
**Last updated:** 2026-04-29 (Phase 4 + 4.1 complete; v0.1 milestone codebase + gap-closure done)

## Project Reference

**Core Value:** Make Claude Code's statusLine never silently fail, never block input, and tell the user — through Nak — exactly what each of their sessions is doing.

**Current focus:** Phase 4 codebase work complete. Milestone v0.1 ready for audit + complete + cleanup. Three maintainer-side handoff items remain (per `RC1-CHECKLIST.md`): `gh repo create Sinderella/holt`, `tools/setup-labels.sh` apply, and the `v0.1.0-rc.1` tag-push that triggers the dist release workflow.

## Current Position

| Field | Value |
|-------|-------|
| Phase | 4 — Distribution + launch (COMPLETE) |
| Plan | both 04-01 and 04-02 landed |
| Status | Phase 4 complete (verifier PARTIAL — gaps are maintainer-only rc.1 tag-push items; reviewer PASS-WITH-WARNINGS; 3/4 fixed, 1 deferred); ready for milestone audit/complete/cleanup |
| Progress | **4 / 4 phases complete** |

```
[x] Phase 1: Schema + supervisor substrate    ✓ 2026-04-28 (verified passed; 28/28 tests; review-fix all critical+warning)
[x] Phase 2: Heartbeat hook (write side)      ✓ 2026-04-28 (verified passed; 51/51 tests; review-fix 11/11 critical+warning)
[x] Phase 3: install-hooks UX                 ✓ 2026-04-28 (verified passed; 80/80 tests; review-fix 6/6 critical+warning; all 6 hard constraints C1..C6 enforced)
[x] Phase 4: Distribution + launch            ✓ 2026-04-29 (verified PARTIAL — codebase complete; 3 maintainer rc.1 tag-push items deferred to RC1-CHECKLIST.md; review-fix 3/4 warnings, 1 deferred + rationale)
[x] Phase 4.1: Gap-closure (bootstrap auto)   ✓ 2026-04-29 (tools/bootstrap-github.sh; RC1-CHECKLIST collapsed from 6 to 4 pre-tag steps; tag-push retained as deliberate human decision)
```

## Performance Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Phases shipped | 4 | **4** |
| v1 requirements covered | 28 / 28 | **27 implemented + 1 deferred to v0.1.x** (CORE-01..10 + HOOK-01..11 + DIST-01,03..07; DIST-02 Homebrew tap deferred per AMENDMENT 2026-04-29) |
| Cold-start overhead (macOS arm64) | < 20ms | **PASS** — `holt --self-bench` p95=0us; `holt --self-bench-hook PreToolUse` p95=5874us (D-15 hook gate, 3× headroom) |
| Cold-start overhead (Linux x86_64) | < 20ms | CI matrix runs both gates on every PR |
| `holt install-hooks` budget | < 500ms | **PASS** — `--dry-run` p95 within budget; 50× concurrent stress + 200× SIGKILL atomicity green |
| Workspace test count | n/a | **80/80 green** at HEAD (`d15951e`) |
| Hard constraints C1..C6 enforced | 6 / 6 | **PASS** — all 6 test-enforced (architecture_dag, cli_dep_boundary, chokepoint_audit, reader_contract, render_path_no_read, install_hooks lock + sigkill) |
| Distribution stack | dist v0.31.0 + binstall | scaffold + 4-target matrix + `--version` parity check + sha256 sidecars + MSRV 1.87 build matrix; Homebrew tap deferred to v0.1.x |

## Accumulated Context

### Key Decisions (from PROJECT.md, locked at roadmap creation)

- **Rust + sync stdlib + threads at v0.1; no async runtime.** Audit transitive `tokio` pull-through.
- **Wrap don't compete at v0.1.** Native rendering is a v0.5+ trigger; the wedge is runtime hygiene.
- **Files-on-disk + hooks (no daemon) through v1.0.** Daemon is a 1.x optimization gated on ≥3 issues reporting ≥10-session lag.
- **`$XDG_RUNTIME_DIR` for heartbeats**, never `~/.claude/` (avoids iCloud/OneDrive sync corruption class).
- **Schemas locked at `schema_version: 1`** for both heartbeat and pet state (`docs/05-schemas.md`).
- **MSRV 1.87 / Edition 2024**, matching `process-wrap` v9.1.0 baseline.

### Hard Constraints (from research/SUMMARY.md §3 — must hold across the codebase)

- **C1**: Always pipe stdin/stdout/stderr (`Stdio::piped()` × 3) before `wrap(ProcessGroup::leader())`. Never inherit parent TTY (SIGTTIN avoidance). Lands in Phase 1 as the single chokepoint for all supervised process spawning.
- **C2**: `holt-render` MUST NOT depend on `holt-supervisor`. Enforced in CI via `cargo tree`. v0.1 has no real `holt-render` (passthrough only) but the CI rule is set in Phase 1.
- **C3**: `~/.claude/settings.json` mutation requires `fs2::FileExt::try_lock_exclusive()` + fsync-before-rename + PID-suffix tmp file. Lands in Phase 3.
- **C4**: JSONC handling lives ONLY in `holt-cli`, never on the render path. Lands in Phase 3.
- **C5**: Reader treats stale-or-corrupt heartbeat as missing — never `unwrap()`, never panics. Lands in Phase 1 as the `holt-schemas::read_heartbeat()` API contract.
- **C6**: The render path never reads `breaches.log` or `timings.jsonl`. Lands in Phase 1; documented in `CONTRIBUTING.md`.

### Open Questions (from research/SUMMARY.md §5 — resolve at phase-start)

- ~~**Phase 2 start**: JSONC strategy spike.~~ ✓ Resolved in Phase 3 — `jsonc-parser` 0.26.3 CST in-place edit with `is_canonical_entry` byte-equal short-circuit confirmed safe; `json_comments` strip-then-parse not used (single library is sufficient).
- ~~**Phase 3 start**: Capture verbatim CC v2.1.119+ stdin JSONs.~~ ✓ Resolved in Phase 2 — `crates/holt-hooks/tests/fixtures/cc-stdin/v2.1.119/` corpus committed with refresh-procedure README.
- ~~**Phase 4 start**: Run `dist init` to generate the canonical scaffold.~~ ✓ Resolved in Phase 4 Plan 04-01 — `dist init --yes` produced `dist-workspace.toml` + `release.yml`; customizations applied as separate atomic commits per D-02..D-04. Re-add path documented in `release.yml` route-finder block.

### Todos (carried forward across phases)

- (none yet)

### Blockers

- (none)

## Session Continuity

**Next action:** Milestone v0.1 lifecycle — gsd-audit-milestone → gsd-complete-milestone → gsd-cleanup. After lifecycle: maintainer follows `RC1-CHECKLIST.md` (gh repo create → setup-labels.sh apply → tag v0.1.0-rc.1 push → verify release artifacts).

**Files to read on session resume:**
- `/Users/thanats/projects/holt/.planning/phases/04-distribution-launch/RC1-CHECKLIST.md` — maintainer rc.1 tag-push handoff (153 LOC; pre-tag, tag-push, post-tag verification, rollback)
- `/Users/thanats/projects/holt/.planning/phases/04-distribution-launch/04-VERIFICATION.md` — verifier PARTIAL verdict + 3 maintainer-side handoff items
- `/Users/thanats/projects/holt/.planning/phases/04-distribution-launch/04-REVIEW-FIX.md` — fix-pass result (3/4 fixed, WR-02 deferred + rationale)
- `/Users/thanats/projects/holt/.planning/phases/04-distribution-launch/04-CONTEXT.md` — Phase 4 decisions D-01..D-16 (with AMENDMENT BANNER for Homebrew tap deferral)
- `/Users/thanats/projects/holt/.planning/REQUIREMENTS.md` — final state of all 28 v1 requirements
- `/Users/thanats/projects/holt/dist-workspace.toml` — distribution config
- `/Users/thanats/projects/holt/.github/workflows/release.yml` — dist-generated release workflow + hand-edits (Windows continue-on-error, `v*` trigger, D-14 parity check)
- `/Users/thanats/projects/holt/README.md` — launch README (demo gif + 2-command install + platform tier + first-run + label routing)

---

*State initialized: 2026-04-28 after roadmap creation*
