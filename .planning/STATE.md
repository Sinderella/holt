# State: holt

**Project:** holt — Rust statusLine for Claude Code with multi-session orchestration and an ASCII otter pet (Nak)
**Milestone:** v0.1 — Runtime hygiene wedge (the lovable MVP, target 3–4 weekends)
**Last updated:** 2026-04-28

## Project Reference

**Core Value:** Make Claude Code's statusLine never silently fail, never block input, and tell the user — through Nak — exactly what each of their sessions is doing.

**Current focus:** Phase 1 ready to plan. The keystone `holt-schemas` crate + `holt-supervisor` wedge + `holt-cli` skeleton — the substrate every subsequent phase depends on.

## Current Position

| Field | Value |
|-------|-------|
| Phase | 1 — Schema + supervisor substrate |
| Plan | (none — phase not yet planned) |
| Status | Roadmap approved, awaiting `/gsd-plan-phase 1` |
| Progress | 0 / 4 phases complete |

```
[ ] Phase 1: Schema + supervisor substrate    ← NEXT
[ ] Phase 2: Heartbeat hook (write side)
[ ] Phase 3: install-hooks UX
[ ] Phase 4: Distribution + launch
```

## Performance Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Phases shipped | 4 | 0 |
| v1 requirements covered | 28 / 28 | 28 mapped, 0 implemented |
| Cold-start overhead (macOS arm64) | < 20ms | not yet measured |
| Cold-start overhead (Linux x86_64) | < 20ms | not yet measured |

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

- **Phase 2 start**: JSONC strategy spike. Does `json_comments` (strip-then-parse) compose safely with `jsonc-parser` CST (in-place edit)? Capture the minimal `settings.json` fixture with inline comments that exercises the merge path end-to-end. *(Note: the JSONC concern is actually load-bearing in Phase 3, not Phase 2 — see Phase 3 plan.)*
- **Phase 3 start**: Capture verbatim CC v2.1.119+ stdin JSONs as `tests/fixtures/cc-stdin/v2.1.119.json` (PreToolUse / PostToolUse / Stop) before Phase 2 code starts. *(Note: this is actually a Phase 2 prerequisite; flag at Phase 2 plan time.)*
- **Phase 4 start**: Run `dist init` to generate the canonical `dist.toml` / `dist-workspace.toml` scaffold; do not copy STACK.md snippet wholesale (MEDIUM confidence).

### Todos (carried forward across phases)

- (none yet)

### Blockers

- (none)

## Session Continuity

**Next action:** Run `/gsd-plan-phase 1` to decompose Phase 1 (Schema + supervisor substrate) into 1–3 plans per coarse granularity.

**Files to read on session resume:**
- `/Users/thanats/projects/holt/.planning/ROADMAP.md` — phase structure + success criteria
- `/Users/thanats/projects/holt/.planning/REQUIREMENTS.md` — v1 REQ-IDs with phase mappings
- `/Users/thanats/projects/holt/.planning/PROJECT.md` — north star, constraints, key decisions
- `/Users/thanats/projects/holt/.planning/research/SUMMARY.md` — drift since 2026-04-28, hard constraints, open questions
- `/Users/thanats/projects/holt/docs/02-scope.md` — locked v0.1 IN/OUT
- `/Users/thanats/projects/holt/docs/05-schemas.md` — heartbeat + pet state v1

---

*State initialized: 2026-04-28 after roadmap creation*
