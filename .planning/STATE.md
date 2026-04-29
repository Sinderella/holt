# State: holt

**Project:** holt — Rust statusLine for Claude Code with multi-session orchestration and an ASCII otter pet (Nak)
**Milestone:** v0.1 — Runtime hygiene wedge (the lovable MVP, target 3–4 weekends)
**Last updated:** 2026-04-28 (Phase 3 complete)

## Project Reference

**Core Value:** Make Claude Code's statusLine never silently fail, never block input, and tell the user — through Nak — exactly what each of their sessions is doing.

**Current focus:** Phase 4 ready to plan. Distribution + launch — `dist` v0.31.0 scaffold (Linux x64, macOS x64+arm64, Windows x64 best-effort), Homebrew tap, `cargo binstall` metadata, README with asciinema/gif demo, CONTRIBUTING.md issue labels.

## Current Position

| Field | Value |
|-------|-------|
| Phase | 4 — Distribution + launch |
| Plan | (none — phase not yet planned) |
| Status | Phase 3 complete (verified passed; 6/6 critical+warning review findings fixed; 5 info deferred); awaiting `/gsd-plan-phase 4` |
| Progress | 3 / 4 phases complete |

```
[x] Phase 1: Schema + supervisor substrate    ✓ 2026-04-28 (verified passed; 28/28 tests; review-fix all critical+warning)
[x] Phase 2: Heartbeat hook (write side)      ✓ 2026-04-28 (verified passed; 51/51 tests; review-fix 11/11 critical+warning)
[x] Phase 3: install-hooks UX                 ✓ 2026-04-28 (verified passed; 80/80 tests; review-fix 6/6 critical+warning; all 6 hard constraints C1..C6 enforced)
[ ] Phase 4: Distribution + launch            ← NEXT — `dist init` open question (don't copy STACK.md snippet wholesale)
```

## Performance Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Phases shipped | 4 | 3 |
| v1 requirements covered | 28 / 28 | 28 mapped, 21 implemented (CORE-01..10 + HOOK-01..06 + HOOK-07..11) |
| Cold-start overhead (macOS arm64) | < 20ms | **PASS** — `holt --self-bench` p95=0us; `holt --self-bench-hook PreToolUse` p95=9921us (D-15 hook gate, 2x headroom) |
| Cold-start overhead (Linux x86_64) | < 20ms | CI matrix runs both gates on every PR |
| `holt install-hooks` budget | < 500ms | **PASS** — `--dry-run` p95 within budget; 50× concurrent stress + 200× SIGKILL atomicity green |

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
- ~~**Phase 3 start**: Capture verbatim CC v2.1.119+ stdin JSONs.~~ ✓ Resolved in Phase 2 — `crates/holt-hooks/tests/fixtures/cc-stdin/v2.1.119/` corpus (PreToolUse / PostToolUse / Stop / Notification / SessionStart) committed with refresh-procedure README.
- **Phase 4 start**: Run `dist init` to generate the canonical `dist.toml` / `dist-workspace.toml` scaffold; do not copy STACK.md snippet wholesale (MEDIUM confidence).

### Todos (carried forward across phases)

- (none yet)

### Blockers

- (none)

## Session Continuity

**Next action:** Run `/gsd-plan-phase 4` to decompose Phase 4 (Distribution + launch) into plans. **Open Question to resolve at Phase 4 start: dist scaffold** — run `dist init` to generate the canonical `dist.toml` / `dist-workspace.toml` rather than copy STACK.md snippet wholesale (MEDIUM confidence per research/SUMMARY.md §5).

**Files to read on session resume:**
- `/Users/thanats/projects/holt/.planning/ROADMAP.md` — phase structure + success criteria (Phase 4 detailed)
- `/Users/thanats/projects/holt/.planning/REQUIREMENTS.md` — v1 REQ-IDs with phase mappings (DIST-01..07 → Phase 4)
- `/Users/thanats/projects/holt/.planning/PROJECT.md` — north star, constraints, key decisions
- `/Users/thanats/projects/holt/.planning/research/SUMMARY.md` — drift since 2026-04-28, hard constraints, open questions
- `/Users/thanats/projects/holt/.planning/research/STACK.md` — distribution stack (`dist` v0.31.0, Homebrew tap config, `cargo binstall` metadata)
- `/Users/thanats/projects/holt/.planning/phases/03-install-hooks-ux/` — Phase 3 artifacts (CONTEXT, plans, SUMMARYs, VERIFICATION, REVIEW, REVIEW-FIX)
- `/Users/thanats/projects/holt/Cargo.toml` — workspace manifest, MSRV 1.87, edition 2024
- `/Users/thanats/projects/holt/.github/workflows/ci.yml` — existing CI matrix that Phase 4 will extend with release workflow
- `/Users/thanats/projects/holt/docs/02-scope.md` — locked v0.1 IN/OUT
- `/Users/thanats/projects/holt/docs/05-schemas.md` — heartbeat + pet state v1

---

*State initialized: 2026-04-28 after roadmap creation*
