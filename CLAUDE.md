<!-- GSD:project-start source:PROJECT.md -->
## Project

**holt** — a Rust statusLine for Claude Code that wraps the user's existing `statusLine.command`, supervises it under load, exposes a `holt doctor` profiler, and surfaces a multi-session attention queue across all of the user's CC sessions on the same machine. The bar's primary UI is **Nak**, a small ASCII otter (นาก) whose posture, body shape, and trailing companion dots encode session state, peer count, and the current attention queue.

**Core Value:** Make Claude Code's statusLine never silently fail, never block input, and tell the user — through Nak — exactly what each of their sessions is doing.

**Current milestone:** v0.1 — Runtime hygiene wedge (the lovable MVP, target 3–4 weekends). v0.5 (`holt doctor`) and v1.0 (orchestrator + Nak) are future milestones.

**Status:** Design phase complete. No code yet. Phase 1 ready to plan.

**Read in priority order before any work:**
1. `.planning/STATE.md` — current position + open questions for the next phase
2. `.planning/ROADMAP.md` — 4-phase v0.1 milestone with success criteria
3. `.planning/REQUIREMENTS.md` — 28 v1 REQ-IDs with traceability
4. `.planning/PROJECT.md` — north star, constraints, key decisions
5. `.planning/research/SUMMARY.md` — drift since 2026-04-28, hard constraints C1–C6, open questions
6. `docs/02-scope.md` — locked v0.1 IN/OUT scope (authoritative)
7. `docs/05-schemas.md` — heartbeat + pet state v1 schemas (locked)

**Architectural North Star** (from `CONTRIBUTING.md`, in priority order — these win arguments):

1. Don't make Claude Code lag.
2. Be honest with users — no telemetry, no notifications, no emotional manipulation.
3. Stay small — single-binary, sub-20ms cold start.
4. Survive Anthropic's evolution — degrade gracefully or strengthen as APIs grow.
5. Honor the bond — every pet-touching change asks: does this make a long-time user feel more attached, or less?
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

- **Language:** Rust (Edition 2024, MSRV 1.87)
- **Process supervision:** `process-wrap` v9.1.0 (`ProcessGroup::leader()` for Unix setpgid+killpg; `JobObject` for Windows)
- **Distribution:** `dist` v0.31.0 (formerly `cargo-dist`) — config in `dist.toml` / `dist-workspace.toml`, scaffold via `dist init`
- **Install paths:** Homebrew tap (`<user>/holt`, default for macOS), `cargo binstall`, prebuilt GitHub release artifacts
- **JSON:** Strict — `serde_json` with `preserve_order` feature; **NO** `simd-json`/`figment` (pessimization for small inputs)
- **JSONC (settings.json round-trip):** `jsonc-parser` v0.26+ with `cst` feature — **lives ONLY in `holt-cli`, never on render path**
- **CLI:** `clap` 4.5+ (derive macros)
- **Errors:** `anyhow` for application errors; `thiserror` only at internal lib boundaries
- **Filesystem:** `terminal_size` 0.3+ (NOT full `crossterm` for the render path)
- **Color:** `owo-colors` 4.2.3 + `supports-color` 3.x for ANSI/`NO_COLOR`
- **File locking:** `fs2::FileExt::try_lock_exclusive()` for `~/.claude/settings.json` mutation
- **Atomic writes:** Hand-rolled tmp + `fsync(2)` + `rename(2)` (same-directory tmp file mandatory; `atomic-write-file` is the audited fallback if hand-rolled corrupts)
- **No async runtime at v0.1.** Sync stdlib + threads. Audit transitive `tokio` pull-through with `cargo tree -i tokio`.
- **Cold-start budget:** Sub-20ms on macOS arm64 / Linux x86_64; 40ms acceptable on Windows with Defender.

**See:** `.planning/research/STACK.md` for full crate-version table, invocation snippets, and platform-specific gotchas.
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

No codebase yet — conventions will be established as Phase 1 lands. Conventions to enforce from day one:

- **`cargo fmt` and `cargo clippy --all-targets -- -D warnings` clean** before any commit (per `CONTRIBUTING.md`).
- **Tests for runtime-affecting changes.** Sprites/text don't need tests; behavior does.
- **One concept per PR.** Mixed-concern PRs get split.
- **No `unwrap()` on the render path.** Heartbeat reader contract (`holt-schemas::read_heartbeat`) returns `Result<Option<Heartbeat>, _>` — corrupt or missing = `Ok(None)`, never panic.
- **Always pipe stdio when spawning supervised processes** (hard constraint C1 — see Architecture).
- **`holt-render` MUST NOT depend on `holt-supervisor`** (hard constraint C2 — enforced in CI via `cargo tree`).
- **JSONC-tolerant parsing lives only in `holt-cli`** (hard constraint C4).
- **Render path never reads `breaches.log` / `timings.jsonl`** (hard constraint C6 — write-only outputs).
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:research/ARCHITECTURE.md -->
## Architecture

**Six-crate workspace** (one published binary):

```
holt-schemas   ←  every other crate (keystone)
       │
       ├── holt-supervisor   (process-wrap, LKG cache, timings.jsonl, breaches.log)
       │       └── holt-cli  (binary; subcommand dispatch)
       │
       ├── holt-hooks        (heartbeat-write hook subcommand)
       │       └── holt-cli
       │
       └── holt-orchestrator (cross-session reader, attention queue) [v1.0]
               └── holt-render (sprite assembly, peer dots, color flip) [v1.0]
                       └── holt-cli
```

**Hard constraint:** `holt-render` MUST NOT depend on `holt-supervisor`. Enforced in CI via `cargo tree --workspace -p holt-render` + `tests/architecture_dag.rs`.

**Files-on-disk + hooks (no daemon):**

- **Heartbeat:** Hooks (PreToolUse / PostToolUse / Stop / Notification / SessionStart / PreCompact-at-v1.0) write per-session JSON to `$XDG_RUNTIME_DIR/holt/sessions/<sid>.json` (Linux) or `$TMPDIR/holt-$UID/sessions/<sid>.json` (macOS), with `~/.cache/holt/sessions/` fallback. Single writer per file → no locking. Atomic write = same-dir tmp + `fsync(2)` + `rename(2)`.
- **Reader:** Every fire of every session's statusLine binary reads all heartbeat files. Treats `mtime > 2 × refreshInterval` as stale. Treats unparseable as missing.
- **Pet state:** `~/.local/state/holt/pet/<name>.json` — written off the render path; render reads heartbeat, never pet state.
- **Settings.json:** `~/.claude/settings.json` — mutated only by `holt install-hooks` with `fs2` exclusive lock, JSONC round-trip via `jsonc-parser` CST, fsync-before-rename, `.holt.bak` backup.

**Render-path budget (sub-20ms):** holt-only overhead ~1.5ms; remaining 18ms+ for the user's wrapped script and (at v1.0) ~5ms cross-session fanout across ≤8 sessions.

**Hard Constraints from research/SUMMARY.md §3 — must hold across the codebase:**

- **C1 — Always pipe stdio when spawning supervised processes.** `Stdio::piped()` × 3 before `wrap(ProcessGroup::leader())`. macOS SIGTTIN avoidance.
- **C2 — `holt-render` MUST NOT depend on `holt-supervisor`.** Enforced in CI.
- **C3 — `~/.claude/settings.json` mutation:** `fs2::FileExt::try_lock_exclusive()` + fsync-before-rename + PID-suffix tmp + `.holt.bak` (never `.bak` — vim's territory).
- **C4 — JSONC handling lives ONLY in `holt-cli`.** Never in `holt-schemas`, `holt-hooks`, `holt-orchestrator`, or `holt-render`.
- **C5 — Reader treats stale-or-corrupt heartbeat as missing.** Never `unwrap()` deserialization on the render path.
- **C6 — Render path never reads `breaches.log` or `timings.jsonl`.** Reading on the render path creates a storm: measuring slowdowns causes slowdowns as the log grows.

**See:** `.planning/research/ARCHITECTURE.md` for full crate-DAG diagrams, dataflow, failure-mode topology, and 12-phase build order P0–P12.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, or `.github/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.

**Project mode:** YOLO + coarse + parallel + all quality gates on (research / plan-check / verifier). Per-phase research is enabled — phase planning starts with `/gsd-discuss-phase <N>` to surface the open questions before `/gsd-plan-phase <N>`.

**Next action:** `/gsd-discuss-phase 1` (or skip discussion: `/gsd-plan-phase 1`).
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` — do not edit manually.
<!-- GSD:profile-end -->
