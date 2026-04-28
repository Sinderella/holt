# Phase 1: Schema + supervisor substrate - Context

**Gathered:** 2026-04-28
**Status:** Ready for planning
**Mode:** `--auto` (Claude picked recommended defaults from project docs; no interactive Q&A)

<domain>
## Phase Boundary

Land the `holt` binary that wraps the user's existing `statusLine.command`, supervises it under a configurable timeout with clean Unix process-group kill, falls through to a last-known-good cache on slow invocations, and writes per-fire timing + breach telemetry — all on the sub-20ms cold-start budget — while landing the keystone `holt-schemas` crate (heartbeat type + atomic-write helper + non-panicking reader contract `read_heartbeat()`) that every subsequent phase composes on top of.

**In scope:** `holt-schemas` keystone crate, `holt-supervisor` (process-wrap integration, LKG cache, timings.jsonl, breaches.log), `holt-cli` skeleton (`holt run`, `holt --self-bench`, `holt --version`), CI architecture-DAG enforcement, MSRV 1.87 / Edition 2024 toolchain pin.

**Out of scope:** `holt-hooks` crate (Phase 2 owns), `holt install-hooks` subcommand + JSONC handling (Phase 3 owns), `dist` scaffold + Homebrew tap (Phase 4 owns), real `holt-render` and `holt-orchestrator` implementations (passthrough no-op at v0.1; v1.0 owns), all pet/Nak code (v1.0 owns).

</domain>

<decisions>
## Implementation Decisions

### Workspace layout & toolchain

- **D-01:** Single Cargo workspace at repo root with crates under `crates/holt-*/`. Six members declared (even though four ship behavior at v0.1 and two are passthrough placeholders) so the DAG enforced in CI matches the locked architecture from day one.
- **D-02:** **`jiff`** for ISO 8601 timestamps. Reason: modern, well-maintained, no `chrono`-style timezone footguns. Documented at first commit per research/SUMMARY.md §6.
- **D-03:** `anyhow` for application errors at the binary boundary (`holt-cli`, `holt-supervisor` outer surface); `thiserror` only at internal lib boundaries (`holt-schemas` public types). Matches CLAUDE.md Conventions.
- **D-04:** Workspace `[profile.release]` set to `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`, `panic = "abort"`. Required to hit the sub-20ms cold-start budget on macOS arm64 / Linux x86_64 (CORE-09).

### `holt-schemas` keystone API surface

- **D-05:** `Heartbeat` struct uses `#[serde(default)]` on all optional fields and is **not** `deny_unknown_fields` — defensive parse posture is mandatory per PITFALLS.md H5 (CC v2.1.119 stdin-shape regression precedent). `schema_version: u8` is the first declared field.
- **D-06:** `pub fn read_heartbeat(path: &Path) -> Result<Option<Heartbeat>, ReaderError>` — the load-bearing reader contract for C5. Returns `Ok(None)` for: file missing, zero-byte, truncated JSON, unrecognized `schema_version`, missing required fields. Returns `Err` only for I/O errors that are NOT "file missing." **Never panics, never `unwrap()`s.** Exposed for use by Phase 2 round-trip tests and v1.0 orchestrator.
- **D-07:** `pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()>` — same-directory tmp file with PID suffix (`<name>.holt-tmp.<pid>`), `fsync(2)` on the temp fd before `rename(2)`. Hand-rolled per STACK.md §3 ("`atomic-write-file` is the audited fallback if hand-rolled corrupts" — adopt only on ≥2 corruption reports). Helper lives in `holt-schemas` because both `holt-supervisor` (LKG cache) and Phase 2's `holt-hooks` (heartbeat write) need it; placing it here avoids duplication without violating C2.
- **D-08:** Public `Heartbeat` and `LkgEntry` structs marked `#[non_exhaustive]` so `schema_version: 1` stays add-only-compatible and a future `schema_version: 2` reader can degrade gracefully without a major version bump (HOOK-11 / CORE-04).

### `holt-supervisor` wedge composition

- **D-09:** Single chokepoint API: `Supervisor::wrap_and_run(cmd, opts) -> SupervisorOutcome`. **All** supervised process spawning goes through this function. Internally calls `cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())` **before** `wrap(ProcessGroup::leader())` — C1 enforced at the only spawn site, no other code path may call `process-wrap` directly. Documented as such in `crates/holt-supervisor/src/lib.rs` doc-comment with the SIGTTIN-on-macOS rationale inlined.
- **D-10:** LKG cache schema = single JSON file at `~/.cache/holt/lkg/<session_id>.json`, schema_version-tagged: `{schema_version: 1, stdout: String, exit_code: i32, captured_at: ISO8601, duration_ms: u64}`. Render path reads ONLY the `stdout` field on cache hit; remaining fields are observability for `holt doctor` later. Atomic-write via D-07.
- **D-11:** Default timeout = **2 seconds**. Rationale: Claude Code's default statusLine `refreshInterval` is 5s; 2s is well under the next refresh boundary while leaving headroom for the user's wrapped script (see PROJECT.md hard constraint: "the user's 18ms"). Configurable via `holt run --timeout <N>` (parsed as `humantime::Duration`). On breach: `nix::sys::signal::killpg(pgid, SIGKILL)` → record breach → render LKG (or empty stdout if no LKG yet) → exit 0.
- **D-12:** `timings.jsonl` rotation: append-only JSONL with **5MB size cap**, single `.1` rotation on first overflow at write-time (no compression, no multi-tier rollover). Rotation happens **only inside the writer**, never on the render path (C6 enforced — render path opens this file zero times).
- **D-13:** `breaches.log` = JSONL with **5MB / `.1`** rotation, identical policy to D-12. One JSON object per breach: `{ts: ISO8601, kind: "timeout" | "parse_fail" | "spawn_fail", env_capture: {PATH, HOME, …allowlist}, stdin_excerpt: String (≤2KB), stderr_excerpt: String (≤4KB), exit_code: Option<i32>}`. Render path never reads this (C6).

### Self-bench + CI architecture-DAG enforcement

- **D-14:** `holt --self-bench` wraps the no-op `:` shell builtin (or `cmd /c exit 0` on Windows) ≥10 iterations, reports `holt-only` render-path overhead at p50 / p95 / p99, plus a single `PASS` / `FAIL` line vs the 20ms p95 budget on macOS arm64 / Linux x86_64 (40ms on Windows). **Exits non-zero on FAIL** so CI can gate the sub-20ms invariant. Output format: human-readable text by default; `--self-bench --json` for machine consumption (CORE-09).
- **D-15:** `tests/architecture_dag.rs` walks `cargo metadata --format-version 1` resolved-graph JSON (NOT shells out to `cargo tree`). Asserts no path from `holt-render` package node to `holt-supervisor` package node in the resolved dependency graph (C2). Test added to default `cargo test` invocation so every PR exercises it locally and in CI.
- **D-16:** CI matrix at v0.1: MSRV 1.87 on `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin` are **required green**; stable rust on the same two targets is informational. `x86_64-pc-windows-msvc` is allowed-failure (best-effort tier per docs/02-scope.md). `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are required gates per CLAUDE.md Conventions.

### Claude's Discretion

The planner has flexibility on these — they're below the architecture-decision waterline:

- Module layout inside each crate (`mod.rs` vs `lib.rs` plus split files; planner picks based on test ergonomics).
- Exact `clap` derive shape for top-level vs subcommand structure beyond the locked entry points (`run`, `--self-bench`, `--version`).
- Internal symbol naming inside `holt-supervisor` (e.g., `SupervisorOptions` builder vs struct-with-defaults).
- Specific layout under `~/.cache/holt/` beyond `lkg/`, `timings.jsonl`, `breaches.log` (e.g., subdirectory for rotation backups).
- Whether `humantime` or hand-rolled parser handles `--timeout` arg.
- Test fixture layout (`tests/fixtures/` vs `crates/<name>/tests/data/`).
- Whether `nix` or `libc` provides the `killpg` binding (planner picks based on transitive-dep audit).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents (gsd-phase-researcher, gsd-planner) MUST read these before research and planning.**

### Project anchors (locked, authoritative)

- `.planning/PROJECT.md` — north star, hard constraints C1–C6, key locked decisions (Rust + sync + threads at v0.1, no async runtime).
- `.planning/REQUIREMENTS.md` — v1 REQ-IDs CORE-01 through CORE-10 + HOOK-11 mapped to Phase 1.
- `.planning/ROADMAP.md` §"Phase 1: Schema + supervisor substrate" — five success criteria (the verification checklist).
- `.planning/STATE.md` — current position, accumulated context, blocker list (currently empty).
- `CONTRIBUTING.md` — Architectural North Star priority order (these win arguments).
- `CLAUDE.md` — project conventions, technology stack table, six-crate architecture, hard constraints C1–C6 inline.

### Research substrate (locked at roadmap creation)

- `.planning/research/SUMMARY.md` §2 (drift since 2026-04-28: process-wrap v9.1.0, dist v0.31.0), §3 (hard constraints C1–C6 with citations), §4 ("Phase 1 — Schema + Supervisor substrate" sub-section), §6 (jiff vs chrono), §7 (confidence assessment).
- `.planning/research/STACK.md` — crate-version table, `process-wrap` v9.1.0 invocation snippets, `Stdio::piped()` × 3 example, hand-rolled atomic-write reference, MSRV 1.87 / Edition 2024 rationale.
- `.planning/research/ARCHITECTURE.md` — six-crate workspace DAG, dataflow diagrams, failure-mode topology, 12-phase build order P0–P12 (Phase 1 = P0 + P1).
- `.planning/research/PITFALLS.md` — H1 (settings.json corruption — informs C3 for Phase 3, mention only here), H2 (ext4 atomic-rename — informs D-07), H3 (macOS setpgid + SIGTTIN — informs C1 / D-09), H5 (defensive serde — informs D-05), H9 (read-storm under telemetry — informs C6 / D-12 / D-13).

### Locked design docs

- `docs/02-scope.md` — v0.1 IN/OUT tables, platform tier statement (Unix tier-1 / Windows best-effort), trigger criteria for native rendering.
- `docs/05-schemas.md` — heartbeat + pet state v1 schemas, `schema_version: 1` lock (HOOK-11).

### External (read on demand only — do not pre-fetch)

- [process-wrap v9.1.0 docs.rs](https://docs.rs/process-wrap/9.1.0/process_wrap/) — `ProcessGroup::leader()` API, `Stdio::piped()` ordering requirement.
- [Apple APFS Features — rename atomicity](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/Features/Features.html) — `rename(2)` atomicity guarantees on macOS for D-07 rationale.
- [LWN: fsync-before-rename](https://lwn.net/Articles/789600/) — ext4 `data=writeback` delayed-alloc rationale for D-07's mandatory fsync.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **None.** This is a greenfield Rust crate. The repo currently contains only `.planning/`, `docs/`, `CLAUDE.md`, `CONTRIBUTING.md`, `README.md`. Phase 1 establishes every reusable asset in the project.

### Established Patterns

- **None on disk yet.** This phase establishes the patterns enforced from day one: `cargo fmt && cargo clippy --all-targets -- -D warnings` clean before commit, no `unwrap()` on the render path, all supervised spawns through one chokepoint, atomic writes via D-07, schema-version-tagged JSON.

### Integration Points

- **Workspace `Cargo.toml`** at repo root will declare the six-member workspace and pin `rust-version = "1.87"` and `edition = "2024"`.
- **`crates/holt-schemas/`** is the keystone — every other v0.1 crate `holt-schemas = { path = "../holt-schemas" }`.
- **`crates/holt-cli/`** is the only `[[bin]]` target; produces the single `holt` binary.
- **CI workflow** (`.github/workflows/ci.yml`) added in Phase 1 to gate clippy + fmt + the architecture-DAG test + matrix builds.

</code_context>

<specifics>
## Specific Ideas

- **Cold-start budget is load-bearing, not aspirational.** The planner must structure the binary entry point so `holt run` can fall straight to the supervisor in `<2ms` of holt-only overhead on the wrapped path; a `--self-bench` regression closes the PR (D-14).
- **Pin `process-wrap = "9.1.0"` exactly.** Research/SUMMARY.md §2 flagged the docs/02-scope.md v6.0.0 reference as stale; v9.x carries the MSRV 1.87 baseline.
- **The chokepoint matters more than the API surface.** The single `Supervisor::wrap_and_run` is the C1 enforcement boundary — every alternative spawning path is an audit hazard. Document this in module-level rustdoc.
- **Phase 1 establishes the C2 CI rule even though `holt-render` is empty at v0.1.** The architecture_dag test goes in now so Phase 5 / v1.0 can't accidentally introduce the disallowed edge later (cheaper than fixing it post-hoc).

</specifics>

<deferred>
## Deferred Ideas

- **`atomic-write-file` crate adoption** — STACK.md §3 audited fallback; adopt ONLY if ≥2 corrupted-heartbeat reports post-v0.1 launch. Track via a backlog issue, not Phase 1 scope.
- **Daemon optimization for >8 sessions** — gated on ≥3 user reports of ≥10-session lag. Architectural decision lives in PROJECT.md key decisions; do not pre-optimize.
- **Real `holt-render` and `holt-orchestrator` implementations** — v1.0 owns. Phase 1 declares the crates as workspace members but leaves them as `pub fn placeholder() {}` with the C2 invariant tested.
- **`holt-hooks` crate, `holt hook <event>` subcommand, `$XDG_RUNTIME_DIR/holt/sessions/` writer** — Phase 2 owns; HOOK-11's reader contract (this phase) is the only Phase 2 dependency landing here.
- **`holt install-hooks` subcommand, JSONC round-trip, `~/.claude/settings.json` mutation** — Phase 3 owns; C3, C4, H1, H12 land there.
- **`dist init`, Homebrew tap, `cargo binstall` metadata, README asciinema** — Phase 4 owns.
- **Plan-mode color flip, effort/thinking pill, stuck-loop detector** — v1.0 IN per research/SUMMARY.md §2 trigger fires; Phase 1 only ensures heartbeat schema can carry the relevant fields without a schema bump.
- **`PreCompact` hook subscription** — v1.0; not part of the v0.1 hook list (Phase 2 covers the v0.1 five-event subscription only).

</deferred>

---

*Phase: 01-schema-supervisor-substrate*
*Context gathered: 2026-04-28 (--auto mode; recommended defaults grounded in PROJECT.md, REQUIREMENTS.md, research/SUMMARY.md, research/STACK.md, research/ARCHITECTURE.md, research/PITFALLS.md, docs/02-scope.md, docs/05-schemas.md)*
