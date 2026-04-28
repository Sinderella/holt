# Phase 1: Schema + supervisor substrate - Research

**Researched:** 2026-04-28
**Domain:** Rust workspace bootstrap · process supervision · atomic-write filesystem semantics · serde defensive parsing · cold-start budget engineering
**Confidence:** HIGH (architecture, stack, and pitfalls are pre-locked in `docs/` and `.planning/research/`; this file distills them into planner-ready guidance and pins crate APIs verified against current registries 2026-04-28)

## Summary

Phase 1 lands the keystone `holt-schemas` crate (heartbeat type, atomic-write helper, non-panicking reader contract `read_heartbeat`) and the v0.1 wedge `holt-supervisor` (process-wrap integration, LKG cache, timings.jsonl, breaches.log) plus a thin `holt-cli` skeleton — alongside two passthrough placeholder crates (`holt-orchestrator`, `holt-render`) and one Phase 2 placeholder (`holt-hooks`) so the architecture-DAG CI rule (C2) can be enforced from day one.

All sixteen implementation decisions D-01 through D-16 are locked in CONTEXT.md. The planner does not pick crates, versions, or invariants — only module layout, internal naming, and test fixture organization (the Claude's-discretion list at CONTEXT.md §decisions ¶29). This research file pins the exact crate versions and APIs a developer needs at the keyboard to translate those decisions into code.

**Primary recommendation:** Plan Phase 1 in three logically-separable plan units that map to the natural seams in the locked decisions: (1) workspace + `holt-schemas` keystone (D-01 through D-08), (2) `holt-supervisor` wedge with one-chokepoint API (D-09 through D-13), (3) `holt-cli` skeleton + `--self-bench` + CI architecture-DAG enforcement (D-14 through D-16). Each unit has independent verifications mapped to the five ROADMAP success criteria.

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Workspace layout & toolchain**
- **D-01:** Single Cargo workspace at repo root with crates under `crates/holt-*/`. Six members declared (even though four ship behavior at v0.1 and two are passthrough placeholders) so the DAG enforced in CI matches the locked architecture from day one.
- **D-02:** `jiff` for ISO 8601 timestamps. No `chrono`. Documented at first commit per research/SUMMARY.md §6.
- **D-03:** `anyhow` for application errors at the binary boundary (`holt-cli`, `holt-supervisor` outer surface); `thiserror` only at internal lib boundaries (`holt-schemas` public types).
- **D-04:** Workspace `[profile.release]` set to `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`, `panic = "abort"`.

**`holt-schemas` keystone API surface**
- **D-05:** `Heartbeat` struct uses `#[serde(default)]` on all optional fields and is **not** `deny_unknown_fields`. `schema_version: u8` is the first declared field.
- **D-06:** `pub fn read_heartbeat(path: &Path) -> Result<Option<Heartbeat>, ReaderError>` — load-bearing C5 contract. Returns `Ok(None)` for: file missing, zero-byte, truncated JSON, unrecognized `schema_version`, missing required fields. Returns `Err` only for I/O errors that are NOT "file missing." Never panics, never `unwrap()`s.
- **D-07:** `pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()>` — same-directory tmp file with PID suffix (`<name>.holt-tmp.<pid>`), `fsync(2)` on the temp fd before `rename(2)`. Hand-rolled.
- **D-08:** Public `Heartbeat` and `LkgEntry` structs marked `#[non_exhaustive]`.

**`holt-supervisor` wedge composition**
- **D-09:** Single chokepoint API: `Supervisor::wrap_and_run(cmd, opts) -> SupervisorOutcome`. All supervised process spawning goes through this function. Internally calls `cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())` **before** `wrap(ProcessGroup::leader())`. C1 enforced at the only spawn site.
- **D-10:** LKG cache schema = single JSON file at `~/.cache/holt/lkg/<session_id>.json`, schema_version-tagged: `{schema_version: 1, stdout: String, exit_code: i32, captured_at: ISO8601, duration_ms: u64}`. Render path reads ONLY the `stdout` field on cache hit.
- **D-11:** Default timeout = **2 seconds**. Configurable via `holt run --timeout <N>` (`humantime::Duration`). On breach: `nix::sys::signal::killpg(pgid, SIGKILL)` → record breach → render LKG (or empty stdout if no LKG yet) → exit 0.
- **D-12:** `timings.jsonl` rotation: append-only JSONL with **5MB size cap**, single `.1` rotation on first overflow at write-time. Rotation happens only inside the writer.
- **D-13:** `breaches.log` = JSONL with **5MB / `.1`** rotation. One JSON object per breach: `{ts: ISO8601, kind: "timeout" | "parse_fail" | "spawn_fail", env_capture: {PATH, HOME, …allowlist}, stdin_excerpt: String (≤2KB), stderr_excerpt: String (≤4KB), exit_code: Option<i32>}`.

**Self-bench + CI architecture-DAG enforcement**
- **D-14:** `holt --self-bench` wraps the no-op `:` shell builtin (or `cmd /c exit 0` on Windows) ≥10 iterations, reports holt-only render-path overhead at p50 / p95 / p99, plus a single `PASS` / `FAIL` line vs the 20ms p95 budget on macOS arm64 / Linux x86_64 (40ms on Windows). Exits non-zero on FAIL. `--json` flag for machine consumption.
- **D-15:** `tests/architecture_dag.rs` walks `cargo metadata --format-version 1` resolved-graph JSON. Asserts no path from `holt-render` package node to `holt-supervisor` package node.
- **D-16:** CI matrix at v0.1: MSRV 1.87 on `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin` are required green; stable rust on the same two targets is informational. `x86_64-pc-windows-msvc` is allowed-failure. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are required gates.

### Claude's Discretion

The planner has flexibility on these:
- Module layout inside each crate (`mod.rs` vs `lib.rs` plus split files; pick based on test ergonomics).
- Exact `clap` derive shape for top-level vs subcommand structure beyond the locked entry points (`run`, `--self-bench`, `--version`).
- Internal symbol naming inside `holt-supervisor` (e.g., `SupervisorOptions` builder vs struct-with-defaults).
- Specific layout under `~/.cache/holt/` beyond `lkg/`, `timings.jsonl`, `breaches.log` (e.g., subdirectory for rotation backups).
- Whether `humantime` or hand-rolled parser handles `--timeout` arg (recommend `humantime` — D-11 already names it).
- Test fixture layout (`tests/fixtures/` vs `crates/<name>/tests/data/`).
- Whether `nix` or `libc` provides the `killpg` binding (planner picks based on transitive-dep audit; recommend `nix` for ergonomics — see Standard Stack).

### Deferred Ideas (OUT OF SCOPE)

- **`atomic-write-file` crate adoption** — STACK.md §3 audited fallback; adopt ONLY if ≥2 corrupted-heartbeat reports post-v0.1 launch.
- **Daemon optimization for >8 sessions** — gated on ≥3 user reports of ≥10-session lag.
- **Real `holt-render` and `holt-orchestrator` implementations** — v1.0 owns. Phase 1 declares the crates as workspace members but leaves them as `pub fn placeholder() {}` with the C2 invariant tested.
- **`holt-hooks` crate, `holt hook <event>` subcommand, `$XDG_RUNTIME_DIR/holt/sessions/` writer** — Phase 2 owns; HOOK-11's reader contract (this phase) is the only Phase 2 dependency landing here.
- **`holt install-hooks` subcommand, JSONC round-trip, `~/.claude/settings.json` mutation** — Phase 3 owns; C3, C4, H1, H12 land there.
- **`dist init`, Homebrew tap, `cargo binstall` metadata, README asciinema** — Phase 4 owns.
- **Plan-mode color flip, effort/thinking pill, stuck-loop detector** — v1.0 IN; Phase 1 only ensures heartbeat schema can carry the relevant fields without a schema bump.
- **`PreCompact` hook subscription** — v1.0; not part of the v0.1 hook list.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CORE-01 | Shim wraps `statusLine.command`; emits stdout unchanged on happy path | Pattern §3 (single-chokepoint supervisor); File Layout §holt-cli |
| CORE-02 | Per-fire timing log appended to `~/.cache/holt/timings.jsonl` | Pattern §6 (jsonl writer + rotation); Standard Stack jiff |
| CORE-03 | Last-known-good TTL cache — render previous output instantly | Pattern §4 (LKG schema + atomic write via D-07) |
| CORE-04 | Configurable timeout + Unix process-group kill via `process-wrap` v9.1.0 | Pattern §3 (process-wrap snippet); Pitfall §H3 (setpgid EPERM fallback) |
| CORE-05 | All spawned children pipe stdin/stdout/stderr (`Stdio::piped()` × 3) | Pattern §3 (chokepoint API); Don't Hand-Roll §process supervision |
| CORE-06 | Breach log appended to `~/.cache/holt/breaches.log` with full context | Pattern §7 (breach schema + writer); D-13 |
| CORE-07 | Sub-20ms cold-start overhead — measured via `holt --self-bench` | Pattern §8 (self-bench design); Standard Stack release profile |
| CORE-08 | Defensive stdin JSON parse — failures captured in breach log as parse-fail | Pattern §5 (defensive parse helper); Code Examples §parse_fail flow |
| CORE-09 | Render path never opens `breaches.log` or `timings.jsonl` for reading | Pattern §9 (C6 enforcement test); Pitfall §H9 |
| CORE-10 | `holt-render` crate has zero direct dep on `holt-supervisor` | Pattern §9 (architecture_dag test, D-15); Code Examples §dag walker |
| HOOK-11 | Reader treats stale-or-corrupt heartbeat as missing — `Ok(None)`, never `unwrap()` | Pattern §2 (read_heartbeat contract D-06); Code Examples §reader tests |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Heartbeat type definitions | `holt-schemas` | — | Keystone library — depends on nothing; serde derives + atomic-write helper [VERIFIED: ARCHITECTURE.md §2] |
| Atomic-write helper (`atomic_write`) | `holt-schemas` | — | Both `holt-supervisor` (LKG cache) and Phase 2's `holt-hooks` (heartbeat) need it; placing in keystone avoids duplication without violating C2 [CITED: CONTEXT.md D-07] |
| Non-panicking reader (`read_heartbeat`) | `holt-schemas` | — | C5 contract; consumed by Phase 2 round-trip tests and v1.0 orchestrator [CITED: CONTEXT.md D-06] |
| Process spawning + supervision | `holt-supervisor` | — | All supervised spawns through `Supervisor::wrap_and_run`; C1 chokepoint [CITED: CONTEXT.md D-09] |
| LKG cache read/write | `holt-supervisor` | `holt-schemas` (atomic-write helper) | Cache logic in supervisor; durable-write primitive in keystone [CITED: ARCHITECTURE.md §3 P0/P1] |
| `timings.jsonl` writer + rotation | `holt-supervisor` | — | Write-only output of supervisor; render path forbidden to read (C6) [CITED: CONTEXT.md D-12] |
| `breaches.log` writer + rotation | `holt-supervisor` | — | Same — write-only; rotation happens inside writer only [CITED: CONTEXT.md D-13] |
| CLI subcommand dispatch | `holt-cli` | — | Only `[[bin]]` target; depends on everything [CITED: ARCHITECTURE.md §2] |
| `--self-bench` runner | `holt-cli` | `holt-supervisor` | Wraps `:` builtin via supervisor's chokepoint and measures holt-only overhead [CITED: CONTEXT.md D-14] |
| Architecture-DAG test | `tests/architecture_dag.rs` (workspace integration test) | `cargo metadata` JSON | Walks resolved graph; lives at workspace root so it runs once per `cargo test` [CITED: CONTEXT.md D-15] |
| `holt-render` placeholder | `holt-render` | — | `pub fn placeholder() {}`; depends on `holt-schemas` and `holt-orchestrator` only — never `holt-supervisor` [CITED: deferred §placeholders] |
| `holt-orchestrator` placeholder | `holt-orchestrator` | — | `pub fn placeholder() {}`; depends on `holt-schemas` only [CITED: deferred §placeholders] |
| `holt-hooks` placeholder | `holt-hooks` | — | `pub fn placeholder() {}`; depends on `holt-schemas` only — Phase 2 fills in [CITED: deferred §holt-hooks Phase 2] |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust toolchain | **1.87** (Edition 2024) | Language | MSRV pin matching `process-wrap` 9.1.0 [VERIFIED: lib.rs/crates/process-wrap, 2026-04-28] |
| `process-wrap` | **9.1.0** (release date 2026-03-08) | Cross-platform process group / JobObject supervision | Successor to `command-group`; composable wrappers, std + tokio frontends. v9.1.0 declares MSRV 1.87 [VERIFIED: lib.rs/crates/process-wrap, 2026-04-28] |
| `serde` | 1.x | Derive `Serialize`/`Deserialize` on `Heartbeat`/`LkgEntry`/breach record | De facto standard [CITED: STACK.md §"Recommended Stack"] |
| `serde_json` | 1.x | JSON I/O — heartbeat round-trip, LKG cache, jsonl writers | Stock perf is sub-ms on small inputs [CITED: STACK.md §"Recommended Stack"] |
| `jiff` | **0.2.x** (latest 0.2.24 verified 2026-04-28) | ISO 8601 / RFC 3339 timestamps | D-02 lock; modern, no `chrono`-style timezone footguns [VERIFIED: docs.rs/jiff 2026-04-28] |
| `clap` (derive) | 4.5+ | CLI: `holt run`, `holt --self-bench`, `holt --version` | De facto standard [CITED: STACK.md §"Recommended Stack"] |
| `anyhow` | 1.x | Application-layer error type | D-03; right tool for binary boundary [CITED: STACK.md] |
| `thiserror` | 1.x | Internal error enums (e.g., `holt-schemas::ReaderError`) | D-03; matchable errors at lib boundary [CITED: STACK.md] |
| `humantime` | **2.3.0** | Parse `--timeout <N>` arg (e.g., `2s`, `1500ms`, `1h30m`) | Free-form duration parsing; D-11 names it directly [VERIFIED: docs.rs/humantime 2026-04-28] |
| `nix` | 0.27+ (or `libc` if planner audits and prefers) | `killpg(pgid, SIGKILL)` binding for D-11 fallback | Ergonomic Unix syscall bindings; `nix::sys::signal::killpg` is the named symbol in D-11 [CITED: CONTEXT.md D-11; planner discretion per §discretion] |
| `wait-timeout` | **0.2.1** | Wait on `std::process::Child` with deadline | `process-wrap` does NOT provide a `wait_timeout` helper; users compose with `wait-timeout` crate or hand-roll mpsc+thread. The crate is cross-platform Unix+Windows and returns `Result<Option<ExitStatus>>` where `None` means timeout fired [VERIFIED: docs.rs/wait-timeout 2026-04-28]. The planner may instead hand-roll with `std::sync::mpsc` + `std::thread::spawn` per STACK.md §1 — both are valid; `wait-timeout` is the smaller code surface |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde_with` | 3.x | `#[serde_as]` for compact ISO 8601 round-trip if `jiff` derives feel verbose | Optional; prefer plain `String` for `started`/`updated`/`captured_at` and round-trip via `jiff::Timestamp::from_str` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `nix` for `killpg` | `libc::killpg` directly | `libc` is no-deps but ergonomically rough; `nix` adds a small dep but the call site reads cleanly. Planner picks; both are acceptable. |
| `wait-timeout` crate | Hand-rolled `std::sync::mpsc::channel` + `std::thread::spawn(move \|\| child.wait())` + `recv_timeout` | Hand-rolled is ~15 lines and avoids one dep; `wait-timeout` is one line at the call site. Recommend hand-rolled to keep the cold-start budget tight (D-04 / CORE-07) — one fewer transitive dep on the render path |
| `chrono` | `jiff` | D-02 locks `jiff`. Do not propose `chrono`. |
| `simd-json` | `serde_json` | Pessimization for <1KB inputs [CITED: STACK.md §"Explicitly NOT Used"]. |
| `crossterm` (full) | None at v0.1 | Phase 1 has no terminal-size dependency — `--self-bench` text output is plain stdout. `terminal_size` 0.3+ comes in v1.0 with rendering [CITED: STACK.md] |
| `tokio` (any) | `std::thread`, `std::process::Command`, `std::sync::mpsc` | No async runtime at v0.1 [CITED: PROJECT.md key decisions]. Audit transitive `tokio` pull-through with `cargo tree -i tokio` after each dep add [CITED: STACK.md] |

**Installation (workspace root `Cargo.toml`):**

```toml
[workspace]
resolver = "2"
members = [
    "crates/holt-schemas",
    "crates/holt-supervisor",
    "crates/holt-hooks",         # Phase 2 placeholder at v0.1
    "crates/holt-orchestrator",  # v1.0 placeholder at v0.1
    "crates/holt-render",        # v1.0 placeholder at v0.1
    "crates/holt-cli",
]

[workspace.package]
edition = "2024"
rust-version = "1.87"
license = "MIT"
repository = "https://github.com/<user>/holt"

[workspace.dependencies]
# Pinned exactly per CONTEXT.md / STACK.md
process-wrap = "=9.1.0"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
jiff = "0.2"
clap = { version = "4.5", features = ["derive"] }
anyhow = "1"
thiserror = "1"
humantime = "2.3"
nix = { version = "0.27", features = ["signal"] }   # killpg binding (planner-discretion: libc is acceptable substitute)

[profile.release]
lto = "thin"
codegen-units = 1
strip = "symbols"
panic = "abort"
```

**Per-crate `Cargo.toml` snippets** (all crates inherit `edition` / `rust-version` from workspace):

```toml
# crates/holt-schemas/Cargo.toml
[package]
name = "holt-schemas"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
jiff.workspace = true
thiserror.workspace = true
```

```toml
# crates/holt-supervisor/Cargo.toml
[package]
name = "holt-supervisor"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
holt-schemas = { path = "../holt-schemas" }
process-wrap.workspace = true
serde.workspace = true
serde_json.workspace = true
jiff.workspace = true
anyhow.workspace = true

[target.'cfg(unix)'.dependencies]
nix.workspace = true
```

```toml
# crates/holt-cli/Cargo.toml
[package]
name = "holt-cli"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[[bin]]
name = "holt"
path = "src/main.rs"

[dependencies]
holt-schemas = { path = "../holt-schemas" }
holt-supervisor = { path = "../holt-supervisor" }
holt-render = { path = "../holt-render" }            # passthrough at v0.1; here so DAG is honest
holt-orchestrator = { path = "../holt-orchestrator" }# passthrough at v0.1
holt-hooks = { path = "../holt-hooks" }              # placeholder at v0.1
clap.workspace = true
anyhow.workspace = true
humantime.workspace = true
```

```toml
# crates/holt-render/Cargo.toml — CRITICAL: NO holt-supervisor dep. Ever. (C2)
[package]
name = "holt-render"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
holt-schemas = { path = "../holt-schemas" }
holt-orchestrator = { path = "../holt-orchestrator" }
# NOTE: adding holt-supervisor here MUST fail tests/architecture_dag.rs (D-15). Don't.
```

**Version verification commands the planner runs at first commit:**

```bash
cargo update -p process-wrap --precise 9.1.0
cargo tree -i tokio                               # MUST be empty (no transitive tokio)
cargo tree -p holt-render                         # MUST NOT show holt-supervisor in output
cargo metadata --format-version 1 | jq '.packages[] | select(.name == "process-wrap") | .version'  # → "9.1.0"
```

## Architecture Patterns

### System Architecture Diagram

```
                          (Phase 1 — v0.1 cold path)

   CC fires statusLine.command
            │
   stdin    │  argv[..] = ["holt", "run", "--", <wrapped>]  or "--self-bench"
            ▼
   ┌──────────────────────────────────────────────────────────────┐
   │  holt-cli (binary entry point)                               │
   │                                                              │
   │  ┌──────────────────────┐                                    │
   │  │ clap derive parse    │ — selects subcommand/mode          │
   │  └────────────┬─────────┘                                    │
   │               │                                              │
   │               ▼                                              │
   │  ┌──────────────────────────────────────────────────────┐    │
   │  │ Default render mode (CORE-01..09)                    │    │
   │  │                                                      │    │
   │  │   1. read CC stdin → defensive parse helper          │    │
   │  │      (parse failures → breach kind="parse_fail")     │    │
   │  │   2. Supervisor::wrap_and_run(cmd, opts)             │    │
   │  │      ─────────────────────────────────────           │    │
   │  │      a. .stdin/stdout/stderr(Stdio::piped()) ×3      │    │
   │  │      b. .wrap(ProcessGroup::leader()) [unix]         │    │
   │  │      c. .spawn() → child PID = pgid                  │    │
   │  │      d. wait with deadline (mpsc + thread)           │    │
   │  │         on TIMEOUT:                                  │    │
   │  │           - killpg(pgid, SIGKILL)                    │    │
   │  │             (verify setpgid return; PPID-walk        │    │
   │  │              fallback if EPERM — H3)                 │    │
   │  │           - return SupervisorOutcome::Breach         │    │
   │  │         on EXIT 0 with stdout:                       │    │
   │  │           - return SupervisorOutcome::Ok             │    │
   │  │           - update LKG cache via D-07 atomic_write   │    │
   │  │   3. Match outcome:                                  │    │
   │  │      Ok(stdout)     → emit stdout to OUR stdout      │    │
   │  │      Breach         → read LKG (or empty) → emit;    │    │
   │  │                       append breaches.log entry      │    │
   │  │      ParseFail      → read LKG (or empty) → emit;    │    │
   │  │                       append breaches.log entry      │    │
   │  │      SpawnFail      → read LKG (or empty) → emit;    │    │
   │  │                       append breaches.log entry      │    │
   │  │   4. Append timings.jsonl entry (always — every fire)│    │
   │  │   5. Exit 0 (NEVER bubble errors to CC)              │    │
   │  └──────────────────────────────────────────────────────┘    │
   └──────────────────────────────────────────────────────────────┘

   Side-effects on disk (all atomic writes via holt-schemas::atomic_write):

   ~/.cache/holt/lkg/<sid>.json       — read on breach, written on Ok
   ~/.cache/holt/timings.jsonl        — append-only; rotates at 5MB → .1
   ~/.cache/holt/breaches.log         — append-only; rotates at 5MB → .1

   ┌──────────────────────────────────────────────────────────────┐
   │  Architectural rule (C6): the render path                    │
   │  reads lkg/<sid>.json. It NEVER reads timings.jsonl or       │
   │  breaches.log. Those are write-only outputs.                 │
   └──────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
holt/                                    # cargo workspace root
├── Cargo.toml                          # [workspace] members + [profile.release] (D-04)
├── rust-toolchain.toml                 # channel = "1.87"
├── .github/workflows/ci.yml            # fmt + clippy + test matrix (D-16)
├── tests/
│   └── architecture_dag.rs             # workspace-level test (D-15)
└── crates/
    ├── holt-schemas/
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs                  # pub use for Heartbeat, LkgEntry, atomic_write, read_heartbeat
    │   │   ├── heartbeat.rs            # struct Heartbeat (D-05, D-08)
    │   │   ├── lkg.rs                  # struct LkgEntry (D-08)
    │   │   ├── reader.rs               # read_heartbeat (D-06)
    │   │   ├── writer.rs               # atomic_write (D-07)
    │   │   └── error.rs                # ReaderError (thiserror)
    │   └── tests/
    │       └── reader_contract.rs      # 5 cases for D-06 → ROADMAP success criterion #4
    ├── holt-supervisor/
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs                  # pub use for Supervisor, SupervisorOptions, SupervisorOutcome
    │   │   ├── supervisor.rs           # wrap_and_run chokepoint (D-09)
    │   │   ├── lkg.rs                  # LKG cache read/write
    │   │   ├── timings.rs              # timings.jsonl writer + rotation (D-12)
    │   │   ├── breaches.rs             # breaches.log writer + rotation (D-13)
    │   │   └── kill.rs                 # killpg + EPERM PPID-walk fallback (H3)
    │   └── tests/
    │       ├── killpg_no_orphans.rs    # ROADMAP success criterion #2
    │       └── chokepoint_audit.rs     # grep: only one .wrap(ProcessGroup::leader()) call
    ├── holt-hooks/                     # Phase 2 placeholder
    │   ├── Cargo.toml
    │   └── src/lib.rs                  # pub fn placeholder() {}
    ├── holt-orchestrator/              # v1.0 placeholder
    │   ├── Cargo.toml
    │   └── src/lib.rs                  # pub fn placeholder() {}
    ├── holt-render/                    # v1.0 placeholder — MUST NOT depend on holt-supervisor
    │   ├── Cargo.toml
    │   └── src/lib.rs                  # pub fn placeholder() {}
    └── holt-cli/
        ├── Cargo.toml
        ├── src/
        │   ├── main.rs                 # clap derive Cli; subcommand dispatch
        │   ├── run.rs                  # `holt run` default mode
        │   ├── self_bench.rs           # `holt --self-bench` (D-14)
        │   └── stdin.rs                # defensive CC stdin parse helper
        └── tests/
            ├── self_bench_smoke.rs     # ROADMAP success criterion #3
            └── render_path_no_read.rs  # asserts render path opens NEITHER breaches.log NOR timings.jsonl for reading (CORE-09)
```

**Module-layout note (Claude's discretion per CONTEXT.md):** Each crate's split into multiple files (vs single `lib.rs`) is the recommendation here, but a planner choosing single-file `lib.rs` for `holt-schemas` is fine — the type count is small (3 structs + 1 enum + 2 functions). The `holt-supervisor` split is more defensible because the chokepoint discipline (D-09) reads more clearly when `supervisor.rs` is a single file you can audit visually for "exactly one `.wrap(ProcessGroup::leader())` call site."

### Pattern 1: `process-wrap` chokepoint (D-09 — C1 enforcement)

**What:** All supervised process spawning goes through `Supervisor::wrap_and_run`. This is the only place in the codebase that ever calls `process_wrap::std::CommandWrap::wrap(ProcessGroup::leader())`. Any future feature that needs to spawn a child (Phase 4's `dist init`, v0.5's `holt doctor`) goes through this same function or breaks the audit.

**When to use:** Always. There is no alternative path.

**Example:**

```rust
// crates/holt-supervisor/src/supervisor.rs
//
// CHOKEPOINT FOR HARD CONSTRAINT C1 (always pipe stdio before wrap).
// Adding a second .wrap(ProcessGroup::leader()) call site anywhere in this
// workspace is an audit hazard — see crates/holt-supervisor/tests/chokepoint_audit.rs.
//
// Source: process-wrap v9.1.0 docs.rs (verified 2026-04-28)
//   https://docs.rs/process-wrap/9.1.0/process_wrap/

use std::process::Stdio;
use std::time::Duration;
use process_wrap::std::{CommandWrap, ProcessGroup};

pub struct SupervisorOptions {
    pub timeout: Duration,        // default Duration::from_secs(2) per D-11
    pub session_id: String,       // for LKG path
    pub stdin_bytes: Vec<u8>,     // CC's stdin to pass through to wrapped script
}

pub enum SupervisorOutcome {
    Ok { stdout: String, exit_code: i32, duration_ms: u64 },
    Breach { kind: BreachKind, exit_code: Option<i32>, duration_ms: u64, stderr_excerpt: String },
}

pub enum BreachKind {
    Timeout,
    ParseFail,
    SpawnFail,
}

pub fn wrap_and_run(
    program: &str,
    args: &[&str],
    opts: SupervisorOptions,
) -> SupervisorOutcome {
    let started = std::time::Instant::now();

    let mut command = CommandWrap::with_new(program, |c| {
        for a in args { c.arg(a); }
        // C1: pipe ALL THREE stdio streams BEFORE wrap. macOS SIGTTIN avoidance.
        // Inheriting parent TTY in a backgrounded process group → kernel sends SIGTTIN
        // → child stops, looks like a hang. Verified failure mode (openai/codex#8690,
        // elixir-lang/elixir#15036, cross-confirmed in PITFALLS.md H3).
        c.stdin(Stdio::piped())
         .stdout(Stdio::piped())
         .stderr(Stdio::piped());
    });

    #[cfg(unix)]    { command.wrap(ProcessGroup::leader()); }
    #[cfg(windows)] { command.wrap(process_wrap::std::JobObject); }

    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => return SupervisorOutcome::Breach {
            kind: BreachKind::SpawnFail,
            exit_code: None,
            duration_ms: started.elapsed().as_millis() as u64,
            stderr_excerpt: format!("spawn failed: {e}"),
        },
    };

    // Write stdin to child, then wait with deadline.
    // process-wrap v9.1.0 does NOT expose wait_timeout — compose with std::sync::mpsc
    // + std::thread::spawn (recommended; one fewer dep) OR wait-timeout = "0.2"
    // (smaller code surface). Planner discretion.

    // ... (see Code Examples §wait-with-deadline below)
}
```

**Test-time audit** (`crates/holt-supervisor/tests/chokepoint_audit.rs`):

```rust
// Asserts there is exactly one .wrap(ProcessGroup::leader()) call in the supervisor crate.
// If a contributor adds a second one (without going through wrap_and_run), this fails.

#[test]
fn only_one_wrap_call_site() {
    let src = std::fs::read_to_string("src/supervisor.rs").unwrap();
    let count = src.matches(".wrap(ProcessGroup::leader())").count();
    assert_eq!(count, 1, "C1 chokepoint violated: expected exactly 1 call to .wrap(ProcessGroup::leader()) in supervisor.rs, found {count}");
}
```

### Pattern 2: Defensive serde reader (D-06 — C5 enforcement)

**What:** `read_heartbeat` returns `Ok(None)` for every "file unreadable as a valid heartbeat" condition the C5 contract names. Returns `Err` only for genuine I/O failure that is NOT "file does not exist."

**When to use:** Phase 2's hook round-trip tests will call this. v1.0's orchestrator will call this on every render fire across ≤8 sessions.

**Example:**

```rust
// crates/holt-schemas/src/reader.rs
//
// Source: PITFALLS.md H5 + research/SUMMARY.md C5 + docs/05-schemas.md §1
// Returns Ok(None) for every "session unreadable" outcome. Never panics.

use std::fs;
use std::io;
use std::path::Path;
use crate::heartbeat::Heartbeat;
use crate::error::ReaderError;

pub fn read_heartbeat(path: &Path) -> Result<Option<Heartbeat>, ReaderError> {
    // Step 1: read file. ENOENT → Ok(None) (the "missing" case, not an error).
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ReaderError::Io(e)),
    };

    // Step 2: zero-byte file → Ok(None).
    if bytes.is_empty() { return Ok(None); }

    // Step 3: parse. Any serde error → Ok(None).
    let hb: Heartbeat = match serde_json::from_slice(&bytes) {
        Ok(h) => h,
        Err(_) => return Ok(None),
    };

    // Step 4: schema_version check. Unrecognized → Ok(None) (forward-compat per H8/H11).
    if hb.schema_version != 1 { return Ok(None); }

    Ok(Some(hb))
}
```

**Anti-pattern (must not appear in code):**

```rust
// WRONG — panics on corrupt file. Violates C5.
let hb: Heartbeat = serde_json::from_slice(&bytes).unwrap();

// WRONG — bubbles parse error to caller. Caller is the render path; render must not fail.
let hb: Heartbeat = serde_json::from_slice(&bytes)?;
```

### Pattern 3: Atomic same-directory write (D-07 — H2 mitigation)

**What:** Write to `<target>.holt-tmp.<pid>` in the **same directory** as the target, `fsync(2)` on the temp fd, then `rename(2)`. POSIX rename is atomic only within one filesystem; `EXDEV` is the failure mode if tmp and target are on different mounts. ext4 with `data=writeback` requires the fsync to close the delayed-allocation window (LWN /Articles/789600/).

**When to use:** Every disk write in Phase 1 — LKG cache, jsonl rotation. Phase 2 hook code reuses this.

**Example:**

```rust
// crates/holt-schemas/src/writer.rs
//
// Source: PITFALLS.md H2, research/STACK.md §3, LWN /Articles/789600/

use std::fs::{self, File};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "atomic_write: target has no parent directory")
    })?;

    // Tmp file lives in the SAME DIRECTORY as target — avoids EXDEV (cross-mount rename).
    let pid = std::process::id();
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "atomic_write: target is a directory")
    })?;
    let tmp = dir.join(format!("{}.holt-tmp.{pid}", file_name.to_string_lossy()));

    // 0600 perms — heartbeat / LKG / breach logs are user-private.
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)?;
    f.write_all(contents)?;
    f.sync_all()?;          // fsync(2) BEFORE rename — closes ext4 delayed-alloc window
    drop(f);

    fs::rename(&tmp, path)?;
    // (We deliberately do NOT fsync the directory at v0.1. Heartbeat / LKG are ephemeral;
    // a power-loss between rename and dirent flush is recoverable next fire.
    // Trigger to add directory fsync: ≥1 corrupted-on-power-loss report.)
    Ok(())
}
```

### Pattern 4: LKG cache schema (D-10)

```rust
// crates/holt-schemas/src/lkg.rs
//
// Source: CONTEXT.md D-10. Schema-version-tagged so future bumps are graceful.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]                                // D-08
pub struct LkgEntry {
    pub schema_version: u8,                     // always 1 at v0.1
    pub stdout: String,
    pub exit_code: i32,
    pub captured_at: String,                    // ISO 8601 via jiff::Timestamp::now().to_string()
    pub duration_ms: u64,
}

impl LkgEntry {
    pub const SCHEMA_VERSION: u8 = 1;
}
```

### Pattern 5: Defensive CC stdin parse (CORE-08)

The render path's stdin parse is also defensive; failure → breach kind=`parse_fail`, fall through to LKG, exit 0.

```rust
// crates/holt-cli/src/stdin.rs
//
// Source: CORE-08 + PITFALLS.md H5 (CC v2.1.119 stdin-shape regression).
// Note: at Phase 1 we do NOT yet need to interpret CC stdin fields — we only
// need to capture parse failures. Phase 2's hook crate consumes the fields.

use std::io::{self, Read};

pub enum StdinParseOutcome {
    Ok(serde_json::Value),
    ParseFail { excerpt: String },              // truncated to ≤2KB for breach log per D-13
    Empty,
}

pub fn slurp_and_parse() -> StdinParseOutcome {
    let mut buf = Vec::with_capacity(4096);
    if io::stdin().read_to_end(&mut buf).is_err() {
        return StdinParseOutcome::Empty;
    }
    if buf.is_empty() { return StdinParseOutcome::Empty; }

    match serde_json::from_slice::<serde_json::Value>(&buf) {
        Ok(v)  => StdinParseOutcome::Ok(v),
        Err(_) => {
            // Truncate excerpt to first 2048 bytes. Lossy UTF-8 is fine —
            // we're capturing for human debugging, not round-trip.
            let excerpt = String::from_utf8_lossy(
                &buf[..buf.len().min(2048)]
            ).into_owned();
            StdinParseOutcome::ParseFail { excerpt }
        }
    }
}
```

### Pattern 6: jsonl writer with 5MB / `.1` rotation (D-12, D-13)

**What:** Rotation happens **inside the writer**, never on the render path. Render path opens the file with `O_APPEND` for write — never for read. The rotation policy is "if append would push size past 5MB, rename current → `.1` (overwriting any existing `.1`), then start fresh." No compression, no multi-tier rollover.

**When to use:** `timings.jsonl` (every fire) and `breaches.log` (every breach).

**Example:**

```rust
// crates/holt-supervisor/src/timings.rs (and breaches.rs is structurally identical)
//
// Source: CONTEXT.md D-12. C6 enforced — this code only WRITES.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const MAX_BYTES: u64 = 5 * 1024 * 1024;        // 5MB cap per D-12

pub fn append_jsonl(path: &Path, line: &str) -> io::Result<()> {
    debug_assert!(line.ends_with('\n'), "caller must include trailing newline");

    if let Ok(meta) = fs::metadata(path) {
        if meta.len() + line.len() as u64 > MAX_BYTES {
            // Rotate: <name>.jsonl → <name>.jsonl.1 (overwrites existing .1).
            let mut backup: PathBuf = path.into();
            backup.set_extension(format!("{}.1",
                path.extension().and_then(|s| s.to_str()).unwrap_or("")));
            // Best-effort: ignore rename failure (e.g., target is read-only); writer must not block.
            let _ = fs::rename(path, &backup);
        }
    }

    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    f.write_all(line.as_bytes())?;
    // Deliberately NO fsync per line — append-and-lose-on-crash is the right tradeoff for
    // observability output. Different from D-07 which is for durability of LKG / heartbeat.
    Ok(())
}
```

### Pattern 7: breach log entry (D-13)

```rust
// crates/holt-supervisor/src/breaches.rs (record-construction excerpt)

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BreachRecord {
    pub ts: String,                                      // ISO 8601 via jiff
    pub kind: &'static str,                              // "timeout" | "parse_fail" | "spawn_fail"
    pub env_capture: serde_json::Map<String, serde_json::Value>,
    pub stdin_excerpt: String,                           // ≤2KB
    pub stderr_excerpt: String,                          // ≤4KB
    pub exit_code: Option<i32>,
}

// Allowlist for env_capture (D-13): never log secrets.
const ENV_ALLOWLIST: &[&str] = &[
    "PATH", "HOME", "USER", "SHELL", "TERM", "LANG", "LC_ALL",
    "XDG_RUNTIME_DIR", "TMPDIR",
    // Holt-specific:
    "HOLT_LABEL", "HOLT_NESTED", "HOLT_TRACE",
    // Common CC-relevant:
    "CLAUDE_PROJECT_DIR",
];
```

**Why an allowlist, not a blocklist:** The first time a user hits a breach, the next thing they do is paste it into a GitHub issue. Logging `AWS_SECRET_ACCESS_KEY` even once is a one-strike trust violation in line with `CONTRIBUTING.md` Architectural North Star #2 ("Be honest with users").

### Pattern 8: `holt --self-bench` (D-14)

**What:** Wrap `:` (POSIX shell builtin no-op; cmd `/c exit 0` on Windows) ≥10 times, measuring **only holt-only render-path overhead** — not the wrapped command's runtime. Report p50 / p95 / p99 + PASS/FAIL vs the 20ms p95 budget on macOS arm64 / Linux x86_64 (40ms on Windows).

**Measurement boundary:** "holt-only overhead" = (wall-clock time from `holt` process start to last byte written to our stdout) − (the wrapped `sh -c ':'` runtime as measured by `Instant::now()` immediately before and after `Supervisor::wrap_and_run`). The first quantity is impossible to measure from inside the process (the process has to exist before `Instant::now()` works), so the practical proxy is: subtract the supervised-child runtime from the function-entry-to-function-exit duration of `Supervisor::wrap_and_run`, plus measure CLI parse and stdout flush bookended by `Instant::now()` calls.

```rust
// crates/holt-cli/src/self_bench.rs
//
// Source: CONTEXT.md D-14, success criterion #3.

use std::time::{Duration, Instant};

pub struct BenchResult {
    pub iterations: u32,
    pub overhead_p50_us: u64,
    pub overhead_p95_us: u64,
    pub overhead_p99_us: u64,
    pub budget_p95_us: u64,                 // 20_000 on linux/mac; 40_000 on windows
    pub passed: bool,
}

pub fn run(iterations: u32) -> BenchResult {
    let mut samples_us: Vec<u64> = Vec::with_capacity(iterations as usize);

    let noop_program = if cfg!(windows) { "cmd" } else { "sh" };
    let noop_args: &[&str] = if cfg!(windows) { &["/c", "exit 0"] } else { &["-c", ":"] };

    for _ in 0..iterations {
        let t_total = Instant::now();
        let t_supervised_in = Instant::now();
        let _outcome = holt_supervisor::wrap_and_run(noop_program, noop_args, /* default opts */);
        let supervised_dur = t_supervised_in.elapsed();
        let total_dur = t_total.elapsed();

        // Holt-only overhead: total − wrapped-child-runtime.
        let overhead = total_dur.saturating_sub(supervised_dur);
        samples_us.push(overhead.as_micros() as u64);
    }

    samples_us.sort_unstable();
    let p = |frac: f64| samples_us[((samples_us.len() as f64 - 1.0) * frac).round() as usize];

    let budget = if cfg!(windows) { 40_000 } else { 20_000 };
    let p95 = p(0.95);

    BenchResult {
        iterations,
        overhead_p50_us: p(0.50),
        overhead_p95_us: p95,
        overhead_p99_us: p(0.99),
        budget_p95_us: budget,
        passed: p95 <= budget,
    }
}

// In main: if !result.passed { std::process::exit(1); }   per D-14.
```

**Important nuance:** the bench measures **warm cold-start** — every iteration starts inside an already-running process. To measure **true cold-start** (binary spawn → first stdout byte), the planner needs a separate harness that shell-executes `target/release/holt --self-bench --json --iterations 1` and parses elapsed wall time. STACK.md §"Cold-start budget" notes process-spawn + Rust runtime init is irreducible at ~2-4ms; the in-process bench cannot see that floor. **Recommendation:** ship in-process bench at v0.1; add the wall-clock harness as a CI script that wraps `time target/release/holt --self-bench` with shell — defer to v0.5's `holt doctor`.

### Pattern 9: Architecture-DAG enforcement test (D-15)

**What:** Walks `cargo metadata --format-version 1` resolved-graph JSON to assert no path from `holt-render` package to `holt-supervisor` package. Does NOT shell out to `cargo tree` (less reliable across `cargo` versions; harder to assert).

**When to use:** Once at workspace root (`tests/architecture_dag.rs`). Runs on every `cargo test`. CI fails if the edge is added.

**Example:**

```rust
// tests/architecture_dag.rs (workspace-level integration test)
//
// Source: CONTEXT.md D-15, research/SUMMARY.md C2.
// Asserts: no path from holt-render → holt-supervisor in the resolved dep graph.

use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Command;

#[test]
fn holt_render_does_not_depend_on_holt_supervisor() {
    // Run cargo metadata. The --format-version=1 schema is stable.
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .expect("cargo metadata --no-deps");
    assert!(out.status.success(), "cargo metadata --no-deps: {}", String::from_utf8_lossy(&out.stderr));

    // We need the *full* resolve graph (with deps), not just workspace members.
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1"])
        .output()
        .expect("cargo metadata");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    // Build pkg-id → [dep pkg-id] from .resolve.nodes.
    let nodes = v.pointer("/resolve/nodes").and_then(|n| n.as_array()).unwrap();
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();
    let mut name_to_id: HashMap<String, String> = HashMap::new();

    for n in nodes {
        let id   = n["id"].as_str().unwrap().to_string();
        let name = id.split_whitespace().next().unwrap().to_string();
        name_to_id.insert(name, id.clone());

        let dep_ids: Vec<String> = n["deps"].as_array().unwrap()
            .iter()
            .map(|d| d["pkg"].as_str().unwrap().to_string())
            .collect();
        deps.insert(id, dep_ids);
    }

    let render_id     = name_to_id.get("holt-render").expect("holt-render in graph");
    let supervisor_id = name_to_id.get("holt-supervisor").expect("holt-supervisor in graph");

    // BFS from holt-render. If we reach holt-supervisor, fail loudly.
    let mut seen: HashSet<&String> = HashSet::new();
    let mut q: VecDeque<&String> = VecDeque::new();
    q.push_back(render_id); seen.insert(render_id);

    while let Some(cur) = q.pop_front() {
        for d in deps.get(cur).unwrap_or(&vec![]) {
            assert_ne!(d, supervisor_id,
                "C2 VIOLATED: holt-render has a dependency path to holt-supervisor.\n\
                 The chain reached holt-supervisor via {cur}.\n\
                 Render path (20ms budget) MUST NOT depend on supervisor (unbounded user-script runtime).");
            if seen.insert(d) { q.push_back(d); }
        }
    }
}
```

### Anti-Patterns to Avoid

- **Multiple spawn sites bypassing `wrap_and_run`.** Any second `.wrap(ProcessGroup::leader())` call site silently disables the C1 audit. Caught by `chokepoint_audit.rs` — but the test only checks `supervisor.rs`, so don't put a spawn helper in `holt-cli` or anywhere else.
- **Reading `breaches.log` or `timings.jsonl` from anywhere other than (eventual) `holt doctor`.** Phase 1 has no doctor. So those files have ZERO readers in this phase. The test at `crates/holt-cli/tests/render_path_no_read.rs` enforces this for every `holt run` invocation.
- **`unwrap()` or `expect()` anywhere in `holt-schemas`.** The whole point of D-06 is "never panic on the render path." A single `.unwrap()` in this crate is a bug regardless of where in the call tree it appears.
- **Cross-mount tmp file in `atomic_write`.** Putting tmp in `/tmp` and target in `~/.cache/holt/lkg/` is `EXDEV`. The PR will work on the developer's laptop and fail under any container with `/tmp` mounted separately. D-07 says same-directory tmp; honor it.
- **Synchronous `child.wait()` without deadline.** Without `wait-timeout` or mpsc+thread, `wait()` blocks forever if the wrapped script hangs. CORE-04 / D-11 require the deadline.
- **`#[serde(deny_unknown_fields)]` on `Heartbeat`.** Explicitly forbidden by D-05. The whole point is forward-compat (PITFALLS.md H5: CC v2.1.119 added `effort.level`, `thinking.enabled`, `PostToolUse.duration_ms` — old hooks would crash if they `deny_unknown_fields`'d).
- **Logging full env in `breach.env_capture`.** Allowlist only, per D-13. Pasting a breach into GitHub leaks any non-allowlisted var.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cross-platform process group / job-object supervision | Custom `setpgid` + `killpg` + Windows JobObject wrapper | `process-wrap` v9.1.0 with `ProcessGroup::leader()` (Unix) / `JobObject` (Windows) | `rust-lang/rust#115241` ("Child::kill doesn't kill descendants") is open since 2023; `process-wrap` is the *known answer* to the question this whole project exists to address [VERIFIED: docs.rs/process-wrap/9.1.0] |
| ISO 8601 / RFC 3339 timestamp formatting | `format!("{}-{:02}-{:02}T{:02}:{:02}:{:02}Z", ...)` | `jiff::Timestamp::now().to_string()` for UTC; `jiff::Zoned::now().to_string()` for local | Manual ISO formatting gets timezones, leap seconds, and DST wrong; `jiff` 0.2 ships RFC 3339 + IANA TZ built-in [VERIFIED: docs.rs/jiff 2026-04-28] |
| Duration parsing for `--timeout` arg | Hand-rolled `2s` / `100ms` / `1h30m` parser | `humantime::Duration` 2.3.0 (clap-compatible — derive `Parser` + field type `humantime::Duration`) | Free-form duration parsing is a 200-line state machine you'll get wrong; `humantime` is 1 line in `Cargo.toml` [VERIFIED: docs.rs/humantime 2026-04-28] |
| JSON serialization | Manual byte-pushing | `serde_json` (with `serde` derive on the structs) | Hand-rolled JSON for the heartbeat is what PITFALLS.md H5 warns against — defensive parsing requires `#[serde(default)]` which is a derive feature |
| ANSI color output | `format!("\x1b[31m{x}\x1b[0m")` | `owo-colors` 4.2.3 + `supports-color` 3.x — but NOT in Phase 1 | Phase 1 has no rendering yet — `--self-bench` output is plain stdout. Defer `owo-colors` to v1.0 per CLAUDE.md tech stack table |
| Atomic file write | "I'll just rename" | Hand-rolled per D-07 (`atomic_write` in `holt-schemas`) — but with the LKG fsync-before-rename and same-dir-tmp discipline | This IS hand-rolled per D-07 — but the helper is shared (not per-call-site) so the discipline is enforced in one place. The audited fallback `atomic-write-file` is the trigger-gated swap [CITED: STACK.md §3] |
| Process kill timeout / waiting | `loop { thread::sleep + child.try_wait() }` | `wait-timeout` 0.2.1 OR `std::sync::mpsc` + `std::thread::spawn(\|\| child.wait())` + `recv_timeout` | Polling loops are a CPU cost on the render budget. Either dependency-free mpsc or one-line `wait-timeout` is correct [VERIFIED: docs.rs/wait-timeout 2026-04-28] |
| killpg syscall binding | Inline `unsafe { libc::killpg(...) }` everywhere | `nix::sys::signal::killpg(Pid::from_raw(pgid), Signal::SIGKILL)` (D-11 names `nix`) | D-11 already named the binding; planner discretion is `nix` vs `libc` per CONTEXT.md §discretion. Recommendation: `nix` for ergonomics |

**Key insight:** Phase 1 is short on novel logic — every load-bearing primitive (process supervision, atomic write, defensive JSON parse, timestamp formatting, duration parsing) has a known correct answer in the Rust ecosystem. Phase 1's value is in the **discipline of plumbing them together** with the chokepoint, the C5 contract, and the C2 / C6 architectural rules — not in re-deriving any of them.

## Runtime State Inventory

This is a **greenfield phase** — there is no pre-existing runtime state to migrate or rename. Per Step 2.5 guidance: when "nothing found in category" applies, state explicitly.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no databases, no datastores, no on-disk state predates Phase 1. The project's `git ls-files` shows only design docs (`.planning/`, `docs/`) and project-instruction files (`README.md`, `CLAUDE.md`, `CONTRIBUTING.md`). | None |
| Live service config | None — there are no external services in scope at v0.1 (no daemons, no servers, no cloud endpoints). | None |
| OS-registered state | None — no Task Scheduler / launchd / systemd / pm2 entries. The binary will be invoked by Claude Code reading `~/.claude/settings.json::statusLine.command` — but `holt install-hooks` writing to settings.json is **Phase 3**, not Phase 1. Phase 1 is invoked manually for testing. | None |
| Secrets/env vars | None — Phase 1 reads `PATH`, `HOME`, `USER`, etc. (the breach-log allowlist per D-13) but does not own or rename any secrets. | None |
| Build artifacts | None — there is no existing Rust build output. Phase 1 establishes `target/`. | None |

**The canonical question (per Step 2.5):** *After every file in the repo is updated, what runtime systems still have the old string cached, stored, or registered?* Answer for Phase 1: nothing — this is the first runtime code in the project. The next phase to ask this question is Phase 3 (`holt install-hooks` mutates settings.json).

## Common Pitfalls

### Pitfall H2: APFS vs ext4 atomic-rename divergence (BLOCKER, addressed by D-07)

**What goes wrong:** Heartbeat / LKG writer writes `<file>.tmp`, then renames. On macOS APFS this is transactionally atomic. On Linux ext4 with default `data=ordered`, POSIX says rename is atomic but ext4's *delayed allocation* means the new file's data may not have hit disk before the rename did — a power loss leaves a zero-byte file. The reader sees `{}` and (if it doesn't follow D-06) crashes serde.

**Why it happens:** Heartbeat writers fire on every PreToolUse / PostToolUse / Stop event. Probability of crash-during-write over a year is non-trivial. Reader must defend, writer must close the window.

**How to avoid:**
- **Writer side (D-07):** `fsync(2)` on the temp file descriptor BEFORE `rename(2)`. Costs ~1ms on SSD; closes the ext4 delayed-alloc window. Enforced in `holt-schemas::atomic_write`.
- **Reader side (D-06 / C5):** `read_heartbeat` returns `Ok(None)` for zero-byte file, truncated JSON, missing required fields. Never `unwrap()`s.
- **Test matrix:** include a Linux ext4 stress test that runs `atomic_write` 1000× with random `kill -9` interleaving and asserts every observed read parses cleanly with `serde_json::from_slice` (or returns `Ok(None)`).

**Warning signs:** Reader logs `serde error: EOF while parsing object`. Single user complaint becomes a cluster.

**Source:** PITFALLS.md H2; LWN /Articles/789600/; Apple APFS Features doc.

### Pitfall H3: `process-wrap` setpgid edge cases on macOS (BLOCKER, addressed by D-11 fallback)

**What goes wrong:** `process-wrap` uses `setpgid` so `killpg(SIGKILL)` reaches descendants. On macOS, three edge cases bite:
1. **Sandbox / SIP**: if holt is run from a sandboxed parent (Cursor, some VS Code variants), `setpgid` succeeds but `killpg` against descendants may be denied silently.
2. **Launch Services daemonization**: scripts that exec a `.app` re-parent under `launchd`. `killpg` on holt's process group misses them.
3. **Session leader**: if CC itself is run from `nohup` or `screen -d`, the existing process group may already be a session leader; `setpgid` then fails with `EPERM`.

**How to avoid:**
- Always check the return value of `setpgid` after spawn. `process-wrap` does this internally for `ProcessGroup::leader()` — treat `Err` from `.spawn()` as a `BreachKind::SpawnFail`, not a panic.
- After `killpg` + grace period (~100ms), fall back to walking `libproc::proc_listchildpids` (macOS) / `/proc/*/status` PPID chain (Linux) for descendants we know we spawned, and SIGKILL them individually. `kill.rs` in `holt-supervisor` owns this.
- Document the sandbox limitation in README under "Known limitations" (deferred to Phase 4).
- **The H3 fallback is required by CORE-04**: "verifies `setpgid` return value and falls back to libproc PPID-walk + per-PID SIGKILL if EPERM."

**Warning signs:** Breach log shows "killed parent, descendants survived." `pgrep -f sleep` after `holt run --timeout 1s -- bash -c 'sleep 5'` returns non-empty (this is the negation of ROADMAP success criterion #2).

**Source:** PITFALLS.md H3; openai/codex#8690; elixir-lang/elixir#15036; rust-lang/rust#115241.

### Pitfall H5: CC stdin JSON shape drift (QUALITY, addressed by D-05)

**What goes wrong:** CC ships breaking changes to its stdin envelope on every minor (v2.1.119 added `effort.level`, `thinking.enabled`, `PostToolUse.duration_ms`). A `Heartbeat` struct with `#[serde(deny_unknown_fields)]` would refuse to deserialize CC's new payloads after a CC upgrade.

**How to avoid:**
- **D-05 mandatory posture:** `#[serde(default)]` on every optional field (`session_id` is the only required one and that's a CC-given UUID). NO `deny_unknown_fields`. New fields silently accepted; missing fields fall back to the `default()` value.
- Snapshot 5 verbatim CC stdin JSONs in `tests/fixtures/cc-stdin/v2.1.119.json` etc. before Phase 2 code starts (the open question carried into Phase 2 from STATE.md).

**Phase 1 scope note:** The Phase 1 supervisor doesn't *interpret* CC stdin fields — it only captures parse failures (CORE-08). Phase 2's `holt-hooks` is where stdin field-by-field parsing matters. But the `Heartbeat` struct in `holt-schemas` is defined in Phase 1, and D-05 / D-08 must be honored from the first commit so Phase 2 doesn't have to refactor.

**Warning signs:** `holt peers` shows all sessions in `mode: default` after a CC upgrade — likely the hook's `Heartbeat` struct rejected the new payload shape.

**Source:** PITFALLS.md H5; FEATURES.md (research substrate, available via `.planning/research/SUMMARY.md` §2 #4).

### Pitfall H9: render path reading its own breach log (QUALITY, addressed by D-12 / D-13 / CORE-09)

**What goes wrong:** Naive doctor implementation reads the breach log on every fire to compute "trend" indicators. The breach log is the *output* of the render path. If reading it costs more than the render budget, the act of measuring slowdowns *causes* slowdowns. Storm.

**How to avoid:**
- **The render path NEVER reads `breaches.log` or `timings.jsonl`.** Those are written by the render path and read by `holt doctor` only (which doesn't exist at v0.1).
- Test at `crates/holt-cli/tests/render_path_no_read.rs` runs `holt run` 100× and asserts no read of `breaches.log` happened. On Linux, can use `strace -e openat,access` filtered by the file paths; cross-platform fallback is a stub-replaced `fs` interface in the unit test (mock the `fs::File::open` for those paths and assert it was never called).
- Codify in CONTRIBUTING.md "Architectural North Star" rule #1 elaboration — already present in CLAUDE.md as constraint C6.

**Warning signs:** `holt --self-bench` p99 grows over a week as the breach log grows. User reports "holt was fast at first, now it's slow."

**Source:** PITFALLS.md H9; research/SUMMARY.md C6.

### Pitfall: `wait_timeout` is not on `process-wrap`'s `WrappedChild`

**What goes wrong:** A planner reading `process-wrap` v9.1.0 docs sees `.spawn()` returns a wrapped child, expects a `wait_timeout` method by analogy to `tokio::process::Child`, and finds none. Worse: trying `child.wait()` blocks forever if the wrapped script hangs — the breach detector never fires.

**Why it happens:** `process-wrap` v9 deliberately keeps the std frontend's API surface narrow (`spawn`, `wait`, `try_wait`, `kill`). Timeout is the user's problem.

**How to avoid:**
- Use `wait-timeout` 0.2.1 (`use wait_timeout::ChildExt; child.wait_timeout(deadline)?`) — but `process-wrap`'s wrapped child is not a bare `std::process::Child`, so the planner needs to access the inner Child or call `wait()` on a separate thread.
- **Recommended pattern (no extra dep):**
  ```rust
  use std::sync::mpsc;
  let (tx, rx) = mpsc::channel();
  std::thread::spawn(move || { let _ = tx.send(child.wait()); });
  match rx.recv_timeout(opts.timeout) {
      Ok(Ok(status)) => /* exited cleanly */,
      Ok(Err(e))     => /* wait failed */,
      Err(_)         => /* TIMEOUT — call child.kill() to send killpg */,
  }
  ```
  But note: `child` is moved into the thread. If we need to `.kill()` after `recv_timeout` errors, we need `Arc<Mutex<>>` or the kill happens via signal directly to the pgid we already know. Since we own `pgid` (the spawned child's PID = pgid for a process-group leader), use `nix::sys::signal::killpg(Pid::from_raw(pgid), Signal::SIGKILL)` directly — bypassing the moved child handle entirely.

**Warning signs:** A `holt run --timeout 1s -- bash -c 'sleep 5'` hangs forever instead of breaching at 1s + 100ms grace.

**Source:** docs.rs/process-wrap/9.1.0 (verified 2026-04-28 — no `wait_timeout` on `WrappedChild`); STACK.md §1.

## Code Examples

### Example: full `Heartbeat` struct definition (D-05, D-08)

```rust
// crates/holt-schemas/src/heartbeat.rs
//
// Source: docs/05-schemas.md §1 (locked field set + ISO 8601 strings)
//         CONTEXT.md D-05 (#[serde(default)], NO deny_unknown_fields, schema_version first)
//         CONTEXT.md D-08 (#[non_exhaustive] for forward-compat)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Heartbeat {
    pub schema_version: u8,                                          // first field, always 1 at v0.1

    // Required: session_id is CC-provided and uniquely keys the file.
    pub session_id: String,

    // All other fields default-on-missing per D-05. Defaults defined here so older
    // hooks reading newer files (or vice versa) survive.
    #[serde(default)] pub pid: u32,
    #[serde(default)] pub started: String,                            // ISO 8601 via jiff
    #[serde(default)] pub updated: String,
    #[serde(default)] pub cwd: String,
    #[serde(default)] pub cwd_label: String,
    #[serde(default)] pub mode: Option<String>,                       // "default" | "plan" | ...
    #[serde(default)] pub current_tool: Option<String>,
    #[serde(default)] pub blocked_on: Option<String>,                 // null at v0.1 per HOOK-05
    #[serde(default)] pub context_pct_real: Option<f64>,
    #[serde(default)] pub burn_rate_usd_per_min: Option<f64>,
    #[serde(default)] pub last_assistant_at: Option<String>,
    #[serde(default)] pub model_display: Option<String>,
    #[serde(default)] pub writer_version: String,                     // populated in Phase 2 (HOOK-06); empty at v0.1
}

impl Heartbeat {
    pub const SCHEMA_VERSION: u8 = 1;
}
```

### Example: `read_heartbeat` test cases (D-06 → ROADMAP success criterion #4)

```rust
// crates/holt-schemas/tests/reader_contract.rs
//
// Five cases per CONTEXT.md D-06 + ROADMAP success criterion #4.
// Every case asserts Ok(None) — never Err, never panic.

use holt_schemas::{read_heartbeat, Heartbeat};
use std::fs;
use tempfile::tempdir;

#[test]
fn returns_ok_none_for_missing_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");
    let result = read_heartbeat(&path);
    assert!(matches!(result, Ok(None)), "expected Ok(None), got {result:?}");
}

#[test]
fn returns_ok_none_for_zero_byte_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.json");
    fs::write(&path, b"").unwrap();
    let result = read_heartbeat(&path);
    assert!(matches!(result, Ok(None)), "expected Ok(None), got {result:?}");
}

#[test]
fn returns_ok_none_for_truncated_json() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("trunc.json");
    fs::write(&path, br#"{"schema_version":1,"session_id":"abc","#).unwrap();   // missing closing brace
    let result = read_heartbeat(&path);
    assert!(matches!(result, Ok(None)), "expected Ok(None), got {result:?}");
}

#[test]
fn returns_ok_none_for_unrecognized_schema_version() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("future.json");
    fs::write(&path, br#"{"schema_version":99,"session_id":"abc"}"#).unwrap();
    let result = read_heartbeat(&path);
    assert!(matches!(result, Ok(None)), "expected Ok(None), got {result:?}");
}

#[test]
fn returns_ok_none_for_missing_required_fields() {
    // session_id is required; everything else has #[serde(default)] per D-05.
    let dir = tempdir().unwrap();
    let path = dir.path().join("partial.json");
    fs::write(&path, br#"{"schema_version":1}"#).unwrap();             // no session_id
    let result = read_heartbeat(&path);
    assert!(matches!(result, Ok(None)), "expected Ok(None), got {result:?}");
}

#[test]
fn returns_ok_some_for_valid_heartbeat() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("valid.json");
    fs::write(&path, br#"{"schema_version":1,"session_id":"abc"}"#).unwrap();
    let result = read_heartbeat(&path).unwrap();
    let hb = result.expect("expected Some");
    assert_eq!(hb.session_id, "abc");
    assert_eq!(hb.schema_version, 1);
}
```

### Example: kill-no-orphans test (CORE-04 → ROADMAP success criterion #2)

```rust
// crates/holt-supervisor/tests/killpg_no_orphans.rs
//
// Source: ROADMAP success criterion #2.
// Wraps `bash -c 'sleep 5'`, sets timeout to 1s, asserts:
//   1. kill happens within 100ms of breach (1.0s ± 100ms total)
//   2. no `bash` or `sleep` PIDs survive (pgrep -f sleep returns empty)

use std::process::Command;
use std::time::{Duration, Instant};

#[cfg(unix)]
#[test]
fn timeout_killpg_kills_descendants() {
    use holt_supervisor::{wrap_and_run, SupervisorOptions, SupervisorOutcome, BreachKind};

    let opts = SupervisorOptions {
        timeout: Duration::from_secs(1),
        session_id: "test-killpg".into(),
        stdin_bytes: vec![],
    };

    let started = Instant::now();
    let outcome = wrap_and_run("bash", &["-c", "sleep 5"], opts);
    let elapsed = started.elapsed();

    // (1) Breach happens within 100ms of the 1s deadline.
    assert!(matches!(outcome, SupervisorOutcome::Breach { kind: BreachKind::Timeout, .. }));
    assert!(elapsed >= Duration::from_secs(1));
    assert!(elapsed <= Duration::from_millis(1100), "killpg took too long: {:?}", elapsed);

    // (2) No orphaned descendants. Sleep grace for kernel to reap, then pgrep.
    std::thread::sleep(Duration::from_millis(100));
    let pgrep = Command::new("pgrep").args(["-f", "sleep 5"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&pgrep.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    // pgrep may return our own pid if test binary literally contains "sleep 5" in argv;
    // filter out anything that's ourself or our parent.
    let our_pid = std::process::id().to_string();
    let stragglers: Vec<&&str> = lines.iter().filter(|p| **p != our_pid).collect();
    assert!(stragglers.is_empty(), "orphaned descendants survived killpg: {stragglers:?}");
}
```

### Example: render-path-no-read test (CORE-09 → ROADMAP success criterion #3 second clause)

```rust
// crates/holt-cli/tests/render_path_no_read.rs
//
// Source: ROADMAP success criterion #3 second clause + CONTEXT.md C6.
// Verifies render path opens NEITHER breaches.log NOR timings.jsonl for reading.
//
// Strategy: Linux uses strace -e openat,access; macOS / Windows fall back to a
// stub-replaced fs interface in a unit test. Phase 1 ships the Linux strace path
// (the success-criterion language explicitly names strace); the macOS unit test is
// a follow-up if/when we add a `fs::Reader` indirection layer.

#[cfg(target_os = "linux")]
#[test]
fn render_path_does_not_open_observability_logs_for_reading() {
    use std::process::Command;
    let exe = env!("CARGO_BIN_EXE_holt");

    // Run holt under strace, capture openat/access calls.
    let out = Command::new("strace")
        .args([
            "-f", "-e", "trace=openat,access", "-o", "/tmp/holt-strace.txt",
            exe, "run", "--", "bash", "-c", "echo hello",
        ])
        .output()
        .expect("strace must be installed");
    assert!(out.status.success() || out.status.code() == Some(0));

    let trace = std::fs::read_to_string("/tmp/holt-strace.txt").unwrap();
    for line in trace.lines() {
        // Filter to lines that are reads (NOT O_WRONLY / O_APPEND).
        // openat(...,"<path>", O_RDONLY|...) is a read. O_WRONLY|O_APPEND is a write.
        if line.contains("breaches.log") {
            assert!(!line.contains("O_RDONLY") && !line.contains("O_RDWR"),
                "C6 VIOLATED: render path opened breaches.log for reading: {line}");
        }
        if line.contains("timings.jsonl") {
            assert!(!line.contains("O_RDONLY") && !line.contains("O_RDWR"),
                "C6 VIOLATED: render path opened timings.jsonl for reading: {line}");
        }
    }
}
```

## Project Constraints (from CLAUDE.md)

| Directive | Source | How Phase 1 Honors It |
|-----------|--------|----------------------|
| MSRV 1.87 / Edition 2024 | CLAUDE.md Technology Stack | `rust-toolchain.toml` channel = "1.87"; workspace `rust-version = "1.87"`, `edition = "2024"` |
| `process-wrap` v9.1.0 | CLAUDE.md Technology Stack | `[workspace.dependencies] process-wrap = "=9.1.0"` |
| `serde_json` with `preserve_order` feature | CLAUDE.md (for settings.json round-trip) | Phase 1 does NOT touch settings.json; `preserve_order` is a Phase 3 concern. **Do not add this feature in Phase 1's `holt-schemas` Cargo.toml** — adds 5KB binary cost without benefit |
| NO `simd-json` / `figment` | CLAUDE.md | Stock `serde_json` only |
| `jsonc-parser` lives ONLY in `holt-cli` | CLAUDE.md C4 | Phase 1 does not import `jsonc-parser` at all (Phase 3's concern). Don't add it to any Cargo.toml |
| `clap` 4.5+ derive | CLAUDE.md | `clap = { version = "4.5", features = ["derive"] }` in `holt-cli` |
| `anyhow` for app errors; `thiserror` only at lib boundaries | CLAUDE.md / D-03 | `anyhow` in `holt-cli` and `holt-supervisor` outer surface; `thiserror::Error` derive on `holt-schemas::ReaderError` |
| `terminal_size` 0.3+, NOT full `crossterm` | CLAUDE.md | Phase 1 has no terminal-size dependency yet; defer to v1.0 |
| `owo-colors` 4.2.3 + `supports-color` 3.x | CLAUDE.md | Defer to v1.0 |
| `fs2::FileExt::try_lock_exclusive()` | CLAUDE.md C3 | Phase 3 concern; not in Phase 1 |
| Atomic write: hand-rolled tmp + fsync + rename | CLAUDE.md | D-07 — `holt-schemas::atomic_write` |
| No async runtime at v0.1 | CLAUDE.md | No `tokio`; `cargo tree -i tokio` must be empty after every dep add |
| Cold-start budget sub-20ms macOS arm64 / Linux x86_64; 40ms acceptable on Windows | CLAUDE.md | D-14 self-bench gates this in CI |
| `cargo fmt && cargo clippy --all-targets -- -D warnings` clean before commit | CLAUDE.md | CI gate per D-16 |
| No `unwrap()` on the render path | CLAUDE.md | `read_heartbeat` (D-06); `Heartbeat` deserialize is via `serde_json::from_slice` returning `Result` (never `.unwrap()`); breach log writer treats failures as silent (no `unwrap`) |
| Always pipe stdio when spawning supervised processes (C1) | CLAUDE.md | `wrap_and_run` chokepoint (D-09) |
| `holt-render` MUST NOT depend on `holt-supervisor` (C2) | CLAUDE.md | `tests/architecture_dag.rs` (D-15) |
| JSONC-tolerant parsing only in `holt-cli` (C4) | CLAUDE.md | Phase 1 has no JSONC at all |
| Render path never reads `breaches.log` / `timings.jsonl` (C6) | CLAUDE.md | `tests/render_path_no_read.rs` (CORE-09) |
| One concept per PR | CLAUDE.md | Three plan units suggested in §Summary; each is one concept |

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `process-wrap` v6.0.0 | **v9.1.0** | 2026-03-08 | Pin `Cargo.toml` to `=9.1.0`; MSRV 1.87. The v6 → v9 jump is silent in API shape (still `ProcessGroup::leader()` + `JobObject`) but moved to MSRV 1.87 and stricter feature flags |
| `cargo-dist` | **`dist`** v0.31.0 | 2026-02-23 | Phase 4 concern; not Phase 1 |
| `chrono` for ISO 8601 | **`jiff`** 0.2 | Project D-02 | Newer crate, no TZ data shipping, RFC 3339 + IANA TZ built-in [VERIFIED: docs.rs/jiff 2026-04-28] |
| `command-group` | `process-wrap` (successor) | Pre-v0.1 | Already locked in CLAUDE.md |

**Deprecated/outdated:**
- `figment` for layered config: pessimization for our scope [CITED: STACK.md §"Explicitly NOT Used"]
- `simd-json` for small JSON inputs: SIMD setup cost dominates [CITED: STACK.md]
- `crossterm` (full crate) for cold-start-sensitive CLIs: pulls more than we need; defer to `terminal_size` until v1.0

## Assumptions Log

This research file is heavily grounded in pre-locked design substrate (`docs/`, `.planning/research/`, CONTEXT.md). The few claims tagged `[ASSUMED]` are listed below; the planner should confirm or note them as open questions for `/gsd-discuss-phase` revisits if they affect a plan.

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The `wait-timeout` crate's `ChildExt::wait_timeout` extension method is callable on `process-wrap` v9.1.0's `WrappedChild` (or its inner `std::process::Child`) | Don't Hand-Roll §timeout, Pitfall §wait_timeout | If `WrappedChild` doesn't expose its inner `Child`, the `wait-timeout` integration is not one-line and the planner must use the mpsc + thread pattern. **Recommendation:** plan around the mpsc + thread approach (no extra dep) and let the planner discover the `wait-timeout` integration is workable as a follow-up nicety, not a Phase 1 dependency [ASSUMED — based on docs.rs/process-wrap/9.1.0/std/index.html showing public `ChildWrapper` trait but partial method visibility in the rendered docs page] |
| A2 | `cargo metadata --format-version 1` `.resolve.nodes[].deps[].pkg` field name is stable across `cargo` 1.87+ | Code Examples §architecture_dag, D-15 | The exact JSON path for "this node's deps" might be `deps` or `dependencies` depending on cargo version. The test should be written defensively (try both) or validated against `cargo --version` output before assertion. **Mitigation:** the planner runs `cargo metadata --format-version 1 \| jq '.resolve.nodes[0]'` once to confirm field name before locking in the test [ASSUMED — Cargo metadata schema is documented stable but minor field renames have happened] |
| A3 | macOS Defender / Gatekeeper does NOT add overhead to `holt --self-bench` measurements when run from a sandboxed terminal | Pattern §8 self-bench | If macOS adds 5+ms per spawn under sandboxing, the 20ms p95 budget becomes hostile-environment dependent. **Mitigation:** the success criterion (#3 ROADMAP) names "macOS arm64 + Linux x86_64" as the green-required pair — Defender-induced overhead would surface there but is unlikely on dev hardware [ASSUMED — STACK.md says "40ms acceptable on Windows with Defender" but doesn't quantify macOS sandbox overhead] |
| A4 | `nix::sys::signal::killpg(Pid::from_raw(pgid), Signal::SIGKILL)` is the named symbol in D-11 | Standard Stack §nix; D-11 fallback | If the planner picks `libc::killpg(pgid, SIGKILL)` instead (per CONTEXT.md §discretion), the call site is 2 lines longer and uses `unsafe`. Both are correct; D-11 doesn't bind the choice [VERIFIED via CONTEXT.md §discretion: "planner picks based on transitive-dep audit"] |
| A5 | `pgrep -f sleep` is reliable enough on macOS for the killpg-no-orphans test | Code Examples §killpg_no_orphans test | macOS `pgrep` exists and supports `-f` but with subtle differences from Linux. The test may need to filter our own test-harness PID more carefully. **Mitigation:** the test code already filters `our_pid`; if false positives appear in CI, add `parent_pid` filter [ASSUMED — pgrep behavior is generic POSIX with macOS-specific quirks documented at `man pgrep`] |

**No `[ASSUMED]` claims affect locked decisions or hard constraints.** All five are operational details a planner can resolve at code-write time without revisiting the discuss-phase.

## Open Questions

1. **Is the `wait-timeout` crate or hand-rolled mpsc the recommended timeout primitive for `Supervisor::wrap_and_run`?**
   - What we know: `process-wrap` v9.1.0 does NOT expose a `wait_timeout` method [VERIFIED]. Both options are correct; both are cross-platform.
   - What's unclear: whether `WrappedChild` (the `process-wrap` child wrapper) exposes its inner `std::process::Child` so `wait-timeout::ChildExt` can be applied directly.
   - Recommendation: **plan the hand-rolled `mpsc + thread::spawn` approach**. It's ~15 lines, has no extra dep, and the timeout semantics are obvious from the call site. If the planner discovers `WrappedChild::into_inner()` exists during implementation, swapping to `wait-timeout` is a one-line code change. Either is acceptable for ROADMAP success criterion #2.

2. **Should the architecture-DAG test (D-15) shell out to `cargo metadata`, or use the `cargo_metadata` crate?**
   - What we know: D-15 says "walks `cargo metadata --format-version 1` resolved-graph JSON (NOT shells out to `cargo tree`)." It doesn't forbid using a parsed-JSON crate.
   - What's unclear: adding `cargo_metadata = "0.18"` as a dev-dependency adds ~6 transitive deps for a test-only convenience.
   - Recommendation: **shell out via `Command::new(env!("CARGO")).args(["metadata", ...])` and parse with `serde_json::from_slice::<Value>` directly**. The test's correctness is more important than its elegance, and this approach has zero new deps. The Code Examples §architecture_dag pattern above shows the approach working in ~40 lines.

3. **Should `breaches.log` rotation reset every Phase 1 release, or persist across upgrades?**
   - What we know: D-13 specifies 5MB cap with single `.1` rotation. Doesn't address upgrade behavior.
   - What's unclear: whether v0.1 → v0.1.1 upgrades preserve `breaches.log` content or start fresh.
   - Recommendation: **persist across upgrades.** Breach logs are user-debug surface; clearing them surprises users. The `writer_version` field on each line (deferred to Phase 2's HOOK-06, but already needed for the breach record per D-13's spirit) lets `holt doctor` filter by version. **Note for the planner:** the breach record schema in §Pattern 7 should include a `writer_version: String` field even at v0.1, populated from `env!("CARGO_PKG_VERSION")` — costs nothing and forward-compats Phase 2's H8 fix.

4. **Where exactly should `--self-bench` write its `--json` output?**
   - What we know: D-14 says `--self-bench --json` for machine consumption.
   - What's unclear: stdout (default) or a file flag.
   - Recommendation: **`--json` flag emits to stdout** (the default for CLI machine output) with a single JSON object containing `iterations`, `overhead_p50_us`, `overhead_p95_us`, `overhead_p99_us`, `budget_p95_us`, `passed`. Human-readable mode (no `--json`) prints a 4-line summary + the PASS/FAIL line. CI consumes via `holt --self-bench --json | jq .passed`.

5. **Does the LKG cache need TTL invalidation in Phase 1, or is "stale-but-rendered" acceptable for v0.1?**
   - What we know: CORE-03 says "TTL cache." D-10 schema includes `captured_at` and `duration_ms` but doesn't specify a max-age.
   - What's unclear: failure mode for "user wraps a fast script, kills CC for a week, restarts CC — should we render the week-old LKG?"
   - Recommendation: **render LKG unconditionally at Phase 1; suffix with `[stale]` after `5 × refreshInterval` per ARCHITECTURE.md §5.8** *if* `refreshInterval` is observable from CC stdin. If not, render the LKG with no staleness signal at v0.1. The ARCHITECTURE.md §5.8 staleness note is currently scoped to v0.5's `holt doctor`. **Planner choice:** drop the staleness suffix entirely from Phase 1 to keep the render path simple, and add it as a Phase 2 enhancement once heartbeat reading lands.

## Environment Availability

> Phase 1 establishes the dev environment; this audit confirms the toolchain pieces a planner needs at the keyboard are available.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (stable) | Every plan task | (planner verifies on dev box) | ≥1.87 required | None — install via `rustup install 1.87` |
| `cargo` | Every plan task | (with rustup) | matches toolchain | — |
| `bash` | killpg-no-orphans test (uses `bash -c 'sleep 5'`) | macOS / Linux yes; Windows requires git-bash or WSL | — | On Windows allowed-failure tier (D-16); skip test under `#[cfg(unix)]` per Code Examples |
| `pgrep` | killpg-no-orphans test post-condition | macOS / Linux yes | varies | If pgrep absent in the CI image, `ps -ef \| grep` substitution; CI image should include procps-ng |
| `strace` | render-path-no-read test (Linux only) | Linux only | varies | macOS / Windows: skip strace path; rely on stub-replaced fs interface (deferred — see Code Examples §render-path-no-read) |
| `jq` | (optional) parse `--self-bench --json` in CI | typically pre-installed in CI images | — | Use `python -c "import json,sys; print(json.load(sys.stdin)['passed'])"` as fallback |
| `tempfile` crate | Reader-contract test (creates temp dirs) | crates.io 3.x | — | dev-dependency in `holt-schemas/Cargo.toml` |

**Missing dependencies with no fallback:** None. The toolchain (rustc 1.87, cargo) is installed via `rustup`; everything else is either ubiquitous (bash on Unix) or has graceful fallbacks.

**Missing dependencies with fallback:**
- `strace`: Linux-specific; the macOS code path for the render-path-no-read test is deferred (per Code Examples §render-path-no-read) to a stub-replaced fs interface — defer until needed.

## Sources

### Primary (HIGH confidence)

- `/Users/thanats/projects/holt/.planning/phases/01-schema-supervisor-substrate/01-CONTEXT.md` — 16 locked decisions D-01 through D-16; planner does not change these
- `/Users/thanats/projects/holt/.planning/REQUIREMENTS.md` — CORE-01..10 + HOOK-11 phase-1 mapping
- `/Users/thanats/projects/holt/.planning/ROADMAP.md` §"Phase 1: Schema + supervisor substrate" — five success criteria
- `/Users/thanats/projects/holt/.planning/research/SUMMARY.md` §3 (hard constraints C1–C6), §6 (jiff vs chrono), §7 (HIGH confidence assessment)
- `/Users/thanats/projects/holt/.planning/research/STACK.md` — crate-version table, `process-wrap` invocation snippet, atomic-write pattern, MSRV rationale
- `/Users/thanats/projects/holt/.planning/research/ARCHITECTURE.md` §2 (workspace layout), §3 (build order P0–P12), §4 (data flow), §5 (failure-mode topology)
- `/Users/thanats/projects/holt/.planning/research/PITFALLS.md` H2 (atomic write), H3 (setpgid), H5 (defensive serde), H9 (read storm)
- `/Users/thanats/projects/holt/docs/02-scope.md` — locked v0.1 IN/OUT
- `/Users/thanats/projects/holt/docs/05-schemas.md` §1 (heartbeat schema, locked)
- `/Users/thanats/projects/holt/CLAUDE.md` — project conventions, technology stack table
- `/Users/thanats/projects/holt/CONTRIBUTING.md` — Architectural North Star priority order
- [process-wrap on docs.rs (v9.1.0)](https://docs.rs/process-wrap/9.1.0/process_wrap/) — verified 2026-04-28: API shape (CommandWrap::with_new + ProcessGroup::leader + JobObject)
- [process-wrap on lib.rs](https://lib.rs/crates/process-wrap) — verified 2026-04-28: version 9.1.0, MSRV 1.87, release date 2026-03-08
- [jiff on docs.rs](https://docs.rs/jiff/latest/jiff/) — verified 2026-04-28: version 0.2.24, `Zoned::now()` and `Timestamp::now()` API
- [humantime on docs.rs](https://docs.rs/humantime/latest/humantime/) — verified 2026-04-28: version 2.3.0
- [wait-timeout on docs.rs](https://docs.rs/wait-timeout/latest/wait_timeout/) — verified 2026-04-28: version 0.2.1, `ChildExt::wait_timeout`, cross-platform Unix+Windows

### Secondary (MEDIUM confidence)

- [Apple APFS Features — rename atomicity](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/Features/Features.html)
- [LWN: fsync-before-rename article](https://lwn.net/Articles/789600/) — ext4 `data=writeback` delayed-alloc rationale for D-07
- [openai/codex#8690](https://github.com/openai/codex/issues/8690) + [elixir-lang/elixir#15036](https://github.com/elixir-lang/elixir/issues/15036) — macOS setpgid + SIGTTIN cross-confirmation
- [rust-lang/rust#115241](https://github.com/rust-lang/rust/issues/115241) — `Child::kill` doesn't kill descendants; `process-wrap` is the answer

### Tertiary (LOW confidence)

- The exact field name `.resolve.nodes[].deps[].pkg` in `cargo metadata --format-version 1` JSON (Assumption A2) — Cargo schema is documented stable but minor renames have happened; the test should be validated against `cargo --version` once before locking in

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every crate version verified against docs.rs / lib.rs / crates.io on 2026-04-28
- Architecture: HIGH — the six-crate workspace, the C2 architecture-DAG rule, and the chokepoint discipline are pre-locked in `docs/03-orchestrator.md` and ARCHITECTURE.md; this research file translates them into Cargo.toml + module layout
- Pitfalls: HIGH for H2 / H3 / H5 / H9 (the four Phase 1 pitfalls) — all backed by GitHub-issue evidence and platform docs; MEDIUM for the assumptions in §Assumptions Log
- Code examples: HIGH for the API shapes (verified against docs.rs); MEDIUM for the exact `WrappedChild` method visibility (Assumption A1)

**Research date:** 2026-04-28
**Valid until:** 2026-05-28 (30 days — stack is stable; only `jiff` and `humantime` are sub-1.0 and could ship breaking minor releases; `process-wrap` 9.x has been on the v9 series since at least 2026-03-08)

---

*Phase: 01-schema-supervisor-substrate*
*Research consumed by: gsd-planner*
*Next action after RESEARCH.md commit: `/gsd-plan-phase 1`*
