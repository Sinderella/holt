# Architecture Research — holt

**Domain:** Rust CLI / process supervisor / multi-session statusLine renderer for Claude Code
**Researched:** 2026-04-28
**Confidence:** HIGH (architecture is locked in `docs/03-orchestrator.md` §TL;DR; this doc translates locked decisions into Rust workspace shape)

## Relationship to docs/

This file translates the architectural locks already made in `docs/` into a concrete Rust workspace shape and a build-order dependency graph the roadmapper can sequence on. It does **not** re-derive any of those locks.

- The wedge thesis ("missing runtime, not missing prompt-tool") lives in `docs/01-findings.md` §5 and is taken as given.
- The three locked decisions — files-on-disk + hooks-write + per-session JSON + statusline-reads (no daemon at v1.0); attention queue as headline; worktree as the unit — live in `docs/03-orchestrator.md` §TL;DR. They are inputs to this document, not outputs.
- The schemas (heartbeat v1, pet state v1, both at `schema_version: 1`) live in `docs/05-schemas.md`. Field semantics are not re-derived here; only the *flow* of those fields between components.
- The pet's role as orchestrator UI lives in `docs/04-pet.md` §7. Posture/companion-dot encoding is referenced, not re-specified.

What this doc adds: Rust crate boundaries, a build-order DAG, failure-mode topology per component, two data-flow diagrams (v0.1 cold path and v1.0 fanout path), and render-path budget annotations against the sub-20ms cold-start constraint.

---

## 1. System Overview — Component Graph

```
┌────────────────────────────────────────────────────────────────────────┐
│                         Claude Code (CC) Process                        │
│                                                                         │
│   stdin JSON ──┐                          ┌── fires hooks on events     │
│   (per fire)   │                          │   (PreToolUse / PostToolUse │
│                ▼                          ▼   / Stop / Notification /   │
│         ┌──────────────┐          ┌──────────────┐  SessionStart)       │
│         │ statusLine   │          │ hook command │                      │
│         │ .command =   │          │   = holt     │                      │
│         │   `holt`     │          │   hook ...   │                      │
│         └──────┬───────┘          └──────┬───────┘                      │
└────────────────┼─────────────────────────┼──────────────────────────────┘
                 │ (render path —          │ (write path —
                 │  on the 20ms budget)    │  off render path)
                 ▼                         ▼
┌────────────────────────────┐    ┌─────────────────────────────────┐
│   holt-cli (binary)        │    │   holt hook subcommand          │
│   ┌──────────────────────┐ │    │   (same binary, different arg)  │
│   │ holt-supervisor      │ │    │   ┌─────────────────────────┐   │
│   │  (process-wrap +     │ │    │   │ holt-hooks              │   │
│   │   timeout + breach   │ │    │   │  parse stdin JSON       │   │
│   │   log + LKG cache)   │ │    │   │  derive heartbeat fields│   │
│   └─────────┬────────────┘ │    │   │  atomic-rename write    │   │
│             ▼              │    │   └────────────┬────────────┘   │
│   ┌──────────────────────┐ │    │                │                │
│   │ user's existing      │ │    └────────────────┼────────────────┘
│   │ statusLine.command   │ │                     │
│   │ (wrapped child proc) │ │                     ▼
│   └──────────┬───────────┘ │     ┌─────────────────────────────────┐
│              │ stdout       │     │ $XDG_RUNTIME_DIR/holt/sessions/ │
│              │ (LKG-cached) │     │   <session_id>.json             │
│              │              │     │ (heartbeat, schema_version: 1)  │
│              ▼              │     │ — single writer, atomic rename  │
│   ┌──────────────────────┐ │     └─────────────────┬───────────────┘
│   │ holt-orchestrator    │◀┼─────────────────────────┘
│   │  (read all heartbeats│ │  (fanout read on every fire,
│   │   compute attention  │ │   N≤8 stat() + JSON parse)
│   │   queue, peer count) │ │
│   └─────────┬────────────┘ │
│             ▼              │
│   ┌──────────────────────┐ │     ┌─────────────────────────────────┐
│   │ holt-render          │ │     │ ~/.local/state/holt/pet/        │
│   │  (compose output:    │◀┼─────│   <name>.json (pet state v1)    │
│   │   wrapped output +   │ │     │ — read by `holt pet *` only     │
│   │   Nak sprite +       │ │     │ — NOT on the render path        │
│   │   companion dots +   │ │     └─────────────────────────────────┘
│   │   attention slot)    │ │
│   └─────────┬────────────┘ │     ┌─────────────────────────────────┐
└─────────────┼──────────────┘     │ ~/.cache/holt/timings.jsonl     │
              │ stdout                │ ~/.cache/holt/breaches.log      │
              ▼ (one line)            │ — append-only telemetry-for-self│
        ┌──────────┐                  └─────────────────────────────────┘
        │   CC     │
        │ renders  │
        │   bar    │
        └──────────┘
```

The architectural seam is the dashed line between `holt-hooks` (writer) and `holt-orchestrator` (reader). They share **no in-memory state, no IPC, no lock**. They share a versioned schema and a filesystem location. That seam is what makes the project survive Anthropic API evolution (hook can be rewritten without touching the bar) and makes a daemon a future optimization rather than a v1.0 requirement.

---

## 2. Workspace Layout — Crate Boundaries

**Decision: multi-crate workspace, single published binary.**

```
holt/                           # cargo workspace root
├── Cargo.toml                  # [workspace] members
├── crates/
│   ├── holt-schemas/           # heartbeat + pet state types
│   │   └── src/lib.rs          #   serde Serialize/Deserialize
│   │                           #   schema_version constants
│   │                           #   atomic-rename helper
│   ├── holt-supervisor/        # the v0.1 wedge
│   │   └── src/lib.rs          #   process-wrap + ProcessGroup/JobObject
│   │                           #   timeout + killpg
│   │                           #   LKG TTL cache (file-backed)
│   │                           #   timings.jsonl + breaches.log
│   ├── holt-hooks/             # hook-side write logic
│   │   └── src/lib.rs          #   parse CC stdin envelope
│   │                           #   derive heartbeat fields
│   │                           #   atomic write
│   ├── holt-orchestrator/      # cross-session read logic (v1.0)
│   │   └── src/lib.rs          #   fanout stat()+read
│   │                           #   staleness detection (mtime)
│   │                           #   attention-queue ranking
│   │                           #   friendship aggregation
│   ├── holt-render/            # statusLine output assembly
│   │   └── src/lib.rs          #   Nak sprite vocabulary (12 states)
│   │                           #   companion-dot composer
│   │                           #   width-bounded layout
│   │                           #   ANSI color emission
│   └── holt-cli/               # the binary entrypoint
│       └── src/main.rs         #   subcommand dispatch
└── docs/
```

### Why this shape (and not a single crate, and not micro-crates)

| Option | Verdict | Reason |
|--------|---------|--------|
| Single crate, modules only | Rejected | Hook path and render path have *fundamentally different* perf budgets (50–100ms vs sub-20ms). Module boundaries enforce nothing; crate boundaries do — `holt-render` deliberately doesn't depend on `holt-hooks` so a slow hook helper can never accidentally land on the render path. |
| One crate per feature | Rejected | Cargo workspace overhead per crate is real. Six crates is the right number — one per *concern*, not per *feature*. |
| Workspace, one published binary | **Chosen** | Users `brew install holt` or `cargo binstall holt` and get one binary. The crate split is internal architecture, invisible to users. |

### Crate-level dependency rules (the DAG)

```
holt-schemas    ◀────────┐  ◀────────┐  ◀────────┐
   ▲                     │           │           │
   │                     │           │           │
holt-supervisor      holt-hooks  holt-orchestrator
   ▲                     ▲           ▲
   │                     │           │
   └─────────────┐       │           │
                 │       │       holt-render
                 │       │           ▲
                 │       │           │
                 └───────┴───────────┴───── holt-cli
```

**Rules** (enforced via `Cargo.toml` `[dependencies]`, checked in CI):

1. `holt-schemas` depends on **nothing** in this workspace. Pure types + serde derives + atomic-rename helper.
2. `holt-supervisor` depends only on `holt-schemas`. Knows nothing about peers, pet, or render.
3. `holt-hooks` depends only on `holt-schemas`. Cannot accidentally pull in process-wrap or terminal code.
4. `holt-orchestrator` depends only on `holt-schemas`. Read-only consumer of heartbeat files.
5. `holt-render` depends on `holt-schemas` and `holt-orchestrator`. Wrapped-output stdout is plumbed to render via a function arg, not a dependency on supervisor.
6. `holt-cli` depends on everything. The only crate that knows about all subcommands.

If a future PR adds a dep edge that violates these rules (e.g., `holt-render` importing `holt-supervisor`), CI fails. That's the architecture-as-code seam.

---

## 3. Build Order — v0.1 → v0.5 → v1.0

The dependency DAG dictates the build order. v0.1 ships a strict subset of crates; v0.5 and v1.0 are additive.

### v0.1 (the runtime hygiene wedge)

| Step | Crate(s) | What lands | Depends on |
|------|----------|------------|------------|
| 1 | `holt-schemas` | Heartbeat type + `schema_version` constant + atomic-rename helper. Tests for round-trip serde + tmp-write-rename idempotence. | — |
| 2 | `holt-supervisor` | `process-wrap` integration; timeout + clean killpg; LKG file cache at `~/.cache/holt/lkg/`; `timings.jsonl` writer; breach detector + `breaches.log` writer. | step 1 |
| 3 | `holt-hooks` | Parse CC hook stdin envelope (`session_id`, `cwd`, `hook_event_name`); derive `cwd_label` from `cwd`; write atomic heartbeat. **Hook does NOT need to populate every field at v0.1** — `current_tool`, `mode`, `context_pct_real`, `burn_rate_usd_per_min` can be `null` until v1.0 needs them. | step 1 |
| 4 | `holt-cli` | Entrypoint dispatch: default mode (render = wrap+supervisor+passthrough), `hook` subcommand, `install-hooks` subcommand. | steps 2, 3 |

At v0.1 there is **no `holt-orchestrator`**, **no `holt-render`** beyond literal stdout passthrough. The heartbeat is being written but nobody reads it yet. Deliberate — pre-stages the v1.0 substrate so v1.0 is "additive" not "rebuild."

### v0.5 (the diagnostic)

| Step | Crate(s) | What lands | Depends on |
|------|----------|------------|------------|
| 5 | `holt-supervisor` (extension) | `doctor` profiler — fires script 20× under controlled load, ranks by fork count / network call count / FS-read bytes / p95 latency. | v0.1 |
| 6 | `holt-cli` | `holt doctor` subcommand wiring + `holt doctor --share` redacted bundle (gated on breach-log format stability). | step 5 |

No new crates. `doctor` is a `holt-supervisor` capability extension.

### v1.0 (orchestrator + Nak)

| Step | Crate(s) | What lands | Depends on |
|------|----------|------------|------------|
| 7 | `holt-hooks` (extension) | Populate remaining heartbeat fields: `current_tool`, `mode`, `context_pct_real` (autocompact-corrected), `burn_rate_usd_per_min` (rolling window). Bounded reverse-tail of transcript ~200 lines. | v0.1 |
| 8 | `holt-orchestrator` | Fanout reader (≤8 heartbeat files, stat()+JSON parse, mtime-based staleness). Attention-queue ranking. Friendship aggregation against pet state. | step 7 |
| 9 | `holt-render` | Nak 12-state sprite table; companion-dot rendering; rotating attention slot; width-bounded layout (5-cell ASCII pet, 80-char default budget); ANSI color. | step 8 |
| 10 | `holt-schemas` (extension) | Pet state schema v1 + memory cap (200 events) + archive-at-cap to `<name>.archive.jsonl`. | v0.1 |
| 11 | `holt-cli` | Subcommands: `peers` (TUI grid), `pet rename` / `pet diary` / `pet status` / `pet friends`, `migrate-state`. Plan-mode color flip flag on default render. Autocompact buffer-math (1-day fix; touches `holt-hooks`'s context derivation). | steps 8, 9, 10 |

**The DAG sequencing recommendation for the roadmap:**

```
v0.1 phases (in order):
  P0  schema substrate          → holt-schemas
  P1  process-supervisor wedge  → holt-supervisor
  P2  hook-write substrate      → holt-hooks (minimal heartbeat)
  P3  install-hooks UX          → holt-cli (install-hooks + render passthrough)
  P4  distribution              → cargo-dist + brew tap + binstall
  ─── v0.1 ships here ───

v0.5 phases:
  P5  doctor profiler           → holt-supervisor extension + holt-cli

v1.0 phases:
  P6  rich heartbeat            → holt-hooks extension (transcript tail)
  P7  cross-session reader      → holt-orchestrator (no pet yet)
  P8  Nak sprite vocabulary     → holt-render (single-session pet first)
  P9  companion-dots + queue    → holt-render × holt-orchestrator integration
  P10 pet bond layer            → holt-schemas pet types + holt-cli pet *
  P11 friendship aggregation    → holt-orchestrator extension
  P12 polish (color flip, autocompact, peers TUI)
```

P7 ships pet-less attention queue first; P8 ships pet without orchestrator awareness; P9 fuses them. **Each phase is a standalone shippable** — if Nak slips, P7 ships as 0.9; if orchestrator regresses, P8's single-session pet ships independently.

---

## 4. Data Flow Diagrams

### 4.1 v0.1 cold path (render = wrap + supervise + passthrough)

```
CC fires statusLine.command       ┌── stdin JSON (per-fire)
        │                          │
        ▼                          ▼
   ┌─────────────────────────────────────────────────────┐
   │  holt-cli (default subcommand)                      │
   │                                                     │
   │  1. Read stdin into buffer (small, ~few KB)         │
   │  2. Read user's wrapped command from settings cache │  ── 200µs
   │  3. Spawn wrapped command with process-wrap         │  ── fork+exec
   │     (ProcessGroup on Unix / JobObject on Windows)   │     by user's
   │  4. Pipe stdin → child; collect stdout, stderr      │     command
   │  5. Wait with deadline (configurable, default 1s)   │  ── DOMINANT
   │     → on timeout: killpg(pid) + use LKG cache       │
   │     → on success: write LKG cache; passthrough      │
   │     → on breach (>threshold): append breaches.log   │
   │  6. Append timings.jsonl                            │  ── 100µs
   │  7. Print stdout to OUR stdout                      │
   └─────────────────────────────────────────────────────┘
        │
        ▼
   CC reads stdout, renders bar
```

**Render-path budget annotations** (sub-20ms cold start = the constraint):

| Step | Budget | Rationale |
|------|--------|-----------|
| 1. stdin slurp | 200µs | Small JSON, single read |
| 2. settings.json (cold) | 1ms | One-shot at startup; cache afterwards |
| 2. settings.json (warm) | 10µs | In-memory cache, invalidate on mtime change |
| 3. process-wrap fork+exec | dominated by user's command | Out of our control; this IS the variable holt is wrapping |
| 5. wait | depends on user's command | LKG cache short-circuits when child exceeds budget |
| 5. LKG cache hit path | 1ms | Read previous output from disk, return immediately |
| 6. timings.jsonl append | 100µs | Open-append-close; small line |
| 7. stdout flush | 50µs | Direct write, line-buffered |
| **holt-only overhead** | **<2ms** | Everything except the wrapped command's own runtime |

The 20ms budget is overhead-budget, not wall-clock-budget — if the user's wrapped script takes 800ms, holt's job is to either return cached output in <2ms or kill + LKG fallback in <(timeout+2)ms. **Holt's own additive cost on a hot render is ~1.5ms.**

### 4.2 v1.0 fanout path (render = wrap + supervise + orchestrator-fanout + Nak)

```
CC fires statusLine.command          ┌── stdin JSON (per-fire)
        │                             │
        ▼                             ▼
   ┌────────────────────────────────────────────────────────────────┐
   │  holt-cli (default subcommand)                                 │
   │                                                                │
   │  ON RENDER PATH (the 20ms budget):                             │
   │  ┌──────────────────────────────────────────────────────────┐  │
   │  │ A. Spawn wrapped cmd (as v0.1)             ── async-ish  │  │
   │  │ B. While child runs in background:                       │  │
   │  │    fanout-read $XDG_RUNTIME_DIR/holt/sessions/*.json     │  │
   │  │    (≤8 files, stat+open+parse = ~5ms total)              │  │
   │  │ C. Compute attention-queue rank from B's results         │  │
   │  │    + own session's heartbeat (from disk OR derived now)  │  │
   │  │ D. Wait for child → use stdout OR LKG fallback           │  │
   │  │ E. Compose output:                                       │  │
   │  │      [wrapped child output]                              │  │
   │  │    + " "                                                 │  │
   │  │    + Nak sprite (state from C)                           │  │
   │  │    + companion-dots (peer-count from B)                  │  │
   │  │    + attention slot (rotating most-attention peer)       │  │
   │  │ F. Print composed line                                   │  │
   │  └──────────────────────────────────────────────────────────┘  │
   │                                                                │
   │  OFF RENDER PATH (no budget — fire-and-forget):                │
   │  G. Append timings.jsonl                                       │
   │  H. (If significant event) update pet diary in background      │
   └────────────────────────────────────────────────────────────────┘
```

**Why fanout (step B) ~5ms is realistic:**

- 8 sessions × `stat()` ≈ 8 × 50µs = 400µs
- 8 sessions × open + 1KB read + `serde_json::from_slice` = 8 × ~500µs = 4ms
- staleness check (compare `mtime` to `now − 2*refreshInterval`) and ranking are O(N) over 8 elements

This document does not re-derive the ~5ms claim; it allocates against it (see `docs/03-orchestrator.md` §3).

**Hook-side write path** (off render, parallel concern): CC fires PreToolUse / PostToolUse / Stop / Notification / SessionStart → spawns `holt hook <event>` → reads CC envelope from stdin → updates fields per event type → atomic write. Hook has 50–100ms budget. **The architectural seam: the hook can do expensive things; the render path can't.**

---

## 5. Failure-Mode Topology

For each major component, what happens when reality bites. Symptom → mitigation → which crate owns the fix.

### 5.1 Heartbeat file is corrupt mid-write (despite atomic rename)

**Symptom:** `holt-orchestrator` reads a heartbeat and `serde_json::from_slice` returns Err.
**Why:** atomic rename is per-filesystem; cross-mount renames degenerate. Also: SIGKILL between `tmp` write and rename leaves a half-written `.tmp`.
**Mitigation:**
- `holt-schemas::atomic_write` checks tmp and target are on the same device (`MetadataExt::dev`); errors loudly if not.
- `holt-orchestrator::read_heartbeat` treats parse errors as "session unreadable" and excludes that session from the attention-queue. Logs once per session per process. **Never fails the render.**
- `.tmp` files older than 60s are GC'd by the next hook invocation in the same session.

**Owner:** `holt-schemas` (write-side guarantee), `holt-orchestrator` (read-side tolerance).

### 5.2 User has 12 sessions (above the design ceiling of ~8)

**Symptom:** Fanout cost grows linearly. At 12 sessions, ~7.5ms; at 20, ~12ms.
**Mitigation:**
- `holt-orchestrator` enforces a **soft cap** (default 16) on heartbeat files read per fire. If `glob` returns more, sort by `mtime` desc, take top-N, log a one-time warning to `breaches.log`.
- The bar's display is unaffected: user still sees `[N/12 waiting]` honestly.
- This is the **trigger to ship a daemon at 1.x**: when ≥3 issues report ≥10-session usage with measurable lag.

**Owner:** `holt-orchestrator`.

### 5.3 `~/.claude/settings.json` is JSON-with-comments

**Symptom:** `holt install-hooks` runs `serde_json::from_str` on settings.json and fails. Verified: `serde_json` is strict JSON only ([docs.rs/serde_json](https://docs.rs/serde_json)), no JSONC.
**Why:** Some users hand-edit `~/.claude/settings.json` and add `// comments` or trailing commas.
**Mitigation:**
- `holt-cli install-hooks` uses [`json_comments`](https://crates.io/crates/json_comments) (lightweight strip-then-parse) to read, then [`jsonc-parser`](https://crates.io/crates/jsonc-parser) for in-place edit that **preserves comments** when adding the `hooks` block.
- `--print` flag prints the snippet to add and exits, never touching settings.json.
- `--dry-run` prints proposed merged content + unified diff; user confirms.
- A `.bak` of original settings.json is always written first.

**Owner:** `holt-cli` only. Add `json_comments` + `jsonc-parser` to **`holt-cli` exclusively** — keep them off the render path.

### 5.4 macOS sandbox prevents access to `$TMPDIR/holt-$UID/`

**Symptom:** Hook gets `EPERM` writing heartbeat (some hardened-runtime third-party tools, sandboxed terminals).
**Mitigation:**
- `holt-schemas::heartbeat_path()` resolves the directory by trying in order: `$XDG_RUNTIME_DIR/holt`, `${TMPDIR}/holt-${UID}`, `${HOME}/.local/state/holt/runtime`. Each candidate is write-probed; first success wins (cached in env var for child hooks).
- If all three fail, hook writes nothing and exits 0 — never blocks CC, never errors loudly. The wrapper functionality still works.

**Owner:** `holt-schemas`.

### 5.5 User's `statusLine.command` itself spawns holt (recursion guard)

**Symptom:** Fork bomb if `holt install` runs twice or user wraps a wrapper.
**Mitigation:**
- `holt-supervisor::spawn_wrapped` sets `HOLT_NESTED=1` on spawned child env.
- `holt-cli` checks `HOLT_NESTED=1` at startup. If present **and** the configured wrapped command's argv[0] resolves to the holt binary itself: refuse to wrap; print one-line stderr warning; return LKG cache or empty.
- `holt doctor --check` includes a self-recursion check (runs configured wrap with `HOLT_TRACE=1` and reports if any descendant matches the holt binary path).

**Owner:** `holt-supervisor` + `holt-cli`.

### 5.6 Other failure modes (compact)

| # | Scenario | Mitigation summary | Owner |
|---|----------|--------------------|-------|
| 5.6 | Two CC sessions race on same heartbeat file (theoretical, session_id collision) | Atomic rename means readers see one coherent state. Most-recent-event semantics are convergent; lost write at most one event. Out of v1.0 scope; flag if telemetry shows ≥3 reports. | `holt-schemas` doc |
| 5.7 | Heartbeat schema v1 → v2 mid-fleet | `holt-orchestrator` switches on `schema_version` and handles both for ≥1 minor release. v1 binaries reading v2 files: refuse render of that peer; log "please upgrade." Heartbeats are ephemeral so no migration; pet state uses `holt migrate-state` with `<file>.v1-backup`. | `holt-schemas` + `holt-orchestrator` |
| 5.8 | LKG cache stale enough to mislead | `holt-supervisor::lkg_cache` records mtime; if `now - mtime > 5 × refreshInterval`, suffix output with `[stale]` (default-on). Five consecutive failures → single coalesced breach line. `holt doctor` reports consecutive-failure streaks. | `holt-supervisor` |
| 5.9 | Pet-state file write conflicts (multi-session memory writes) | Pet state is read-modify-write (memories cap), so it uses advisory `fcntl(F_SETLK)` / `LockFile`. Lock hold <10ms; **off render path**. If lock acquisition >200ms, skip with warning. Memory loss acceptable; render correctness non-negotiable. | `holt-schemas` (pet writer; heartbeat writer remains lock-free) |
| 5.10 | cargo-dist matrix fails on a target | Windows is "best-effort" at v0.1. CI publishes Linux + macOS even if Windows fails, with release-notes note. README says "Windows: tier 2" loudly. | Distribution / CI |

---

## 6. Patterns and Anti-Patterns

### Patterns to follow

**Hook-writes / binary-reads (the seam).** Many small writers (one hook per CC event), one fast reader (render-path binary). No shared memory, no IPC, no daemon — the filesystem is the queue. Eventual consistency on hook-fire-to-render-fire latency (≤300ms typical). Owns: `holt-hooks` ↔ `holt-orchestrator`, AND `holt-cli pet *` ↔ pet state file.

**Last-known-good cached rendering.** Render previous output instantly; recompute in background; replace on completion. Powerlevel10k's instant-prompt pattern, ported to a synchronous CLI. Up to 1× refreshInterval staleness; mitigated by `[stale]` marker after 5×. Owns: `holt-supervisor`'s LKG cache.

**Posture-as-state (form-encoded UI).** Sprite shape *is* the data. No text label says "thinking" — the eye-frame says it. No badge says "peer waiting" — the trailing dots and rotating peer-pet say it. Discoverability cost mitigated by `holt pet preview` and a single README section. Owns: `holt-render` — sprite vocabulary lives in a const table, not behavioral code.

### Anti-patterns to avoid

**Tick-driven animation.** A 200ms re-fire timer destroys the wedged-session signal AND adds wakeups to a tool on every developer's hot path. Animate strictly on heartbeat events; absence of motion is information.

**Sharing in-memory state between hook and render.** A SQLite or shared-memory mmap re-introduces locking, schema migration, transactions, daemon-style failure modes — all eliminated by the file-per-session design. Daemon is a 1.x optimization, not v1.0 architecture.

**Expensive logic in `holt-render`.** Render is on the 20ms budget; computation belongs on hook's 50–100ms budget. Autocompact buffer-math, burn-rate, transcript tail — all in `holt-hooks`. `holt-render` consumes pre-derived fields off the heartbeat.

**Letting `holt-supervisor` know about pet or peers.** Supervisor must work as a wrapper-only tool when pet/peers are absent (v0.1 ships without them). Bleeding pet semantics into supervisor breaks the layered-substrate model. Supervisor writes `breaches.log`; the hook learns about breaches the same way it learns everything else.

---

## 7. Integration Points

### External

| Surface | Pattern | Notes |
|---------|---------|-------|
| CC stdin JSON (per-fire to statusLine) | Read once, parse to internal type | v2.1.119 added `effort.level`, `thinking.enabled`, `PostToolUse.duration_ms`. Hook consumes; render does not. |
| CC hooks (PreToolUse/PostToolUse/Stop/Notification/SessionStart) | Hook command spawns `holt hook <event>`; reads CC envelope from stdin | Off render path. 50–100ms budget. |
| CC transcript JSONL (`~/.claude/projects/{hash}/{sid}.jsonl`) | Tail-only bounded read at hook-time | v1.0 only. ~last 200 lines for `current_tool`, `mode`, `burn_rate_usd_per_min`. |
| `~/.claude/settings.json` | Read with JSONC tolerance; mutate via `install-hooks` only | See §5.3. |
| Process group / Job object (OS-native) | `process-wrap` ([docs.rs/process-wrap](https://docs.rs/process-wrap)) — ProcessGroup on Unix (setpgid+killpg), JobObject on Windows | Sync stdlib API; no tokio. |
| Filesystem (`$XDG_RUNTIME_DIR` / `$TMPDIR/holt-$UID`) | Atomic-rename writes; mtime-based liveness | See §5.4 fallback chain. |
| Distribution (cargo-dist + Homebrew tap + cargo-binstall) | Build matrix: Linux x64, macOS x64+arm64, Windows x64 best-effort | |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `holt-hooks` → `holt-orchestrator` | Filesystem (heartbeat JSON) | THE seam. Atomic rename. Schema-versioned. |
| `holt-supervisor` → `holt-render` | Function arg (wrapped child stdout as `String`) | Same process; no IPC. |
| `holt-orchestrator` → `holt-render` | Function arg (computed `OrchestratorView`) | Same process. |
| `holt-orchestrator` → pet state (write) | Filesystem with advisory lock | Off render path; lock acceptable. |
| `holt-cli pet *` → pet state (read) | Filesystem (`<name>.json`) | No lock for reads. |

---

## 8. Scaling Considerations

| Scale | Architecture Adjustments |
|---|---|
| 1–4 sessions (typical) | v0.1 architecture sufficient. Fanout cost ~2ms. |
| 4–8 sessions (design ceiling) | v0.1 architecture still fits. Fanout cost ~5ms. **This is the design point.** |
| 9–16 sessions | Soft cap at 16 (§5.2). Top-16-by-mtime; user sees `[N/16+]`. ~10ms fanout. Acceptable. |
| 17+ sessions | Trigger to design daemon (1.x). gitstatusd-style local-socket daemon pre-aggregates fanout. Architecture survives — hooks still write per-session files; daemon only changes the *reader* side. |

### Scaling priorities (when they break, in order)

1. **Fanout read at 16+ sessions.** Fix: optional daemon (1.x).
2. **`serde_json` per heartbeat.** Fix: `simd-json` *if* benchmarks show >1ms per file. Currently sub-ms for our payload — flagged for re-verification at 1.x.
3. **Transcript tail at 100MB+ JSONL.** Fix: hook caches last-read offset per session in a sidecar file; reads only new bytes since last fire.
4. **Pet-state lock contention with many sessions hitting memory-worthy events at once.** Fix: queue-based memory writes via fire-and-forget child process. Defer until measured.

---

## 9. Sources

- `/Users/thanats/projects/holt/docs/01-findings.md` (wedge thesis, locked architecture inputs)
- `/Users/thanats/projects/holt/docs/02-scope.md` (sequencing reframe — informs build order)
- `/Users/thanats/projects/holt/docs/03-orchestrator.md` (three locked architectural decisions, ~5ms fanout claim)
- `/Users/thanats/projects/holt/docs/04-pet.md` (pet-as-orchestrator-UI, animation rule)
- `/Users/thanats/projects/holt/docs/05-schemas.md` (heartbeat v1 + pet state v1, atomic-rename rule)
- `/Users/thanats/projects/holt/CONTRIBUTING.md` (architectural North Star priority order)
- [process-wrap crate docs](https://docs.rs/process-wrap/) — verified: ProcessGroup (Unix setpgid+killpg) + JobObject (Windows); sync std API path exists, no tokio dependency required (HIGH)
- [serde_json crate docs](https://docs.rs/serde_json/) — verified: strict JSON only, no JSONC support (HIGH)
- [json_comments crate](https://crates.io/crates/json_comments) — strip-then-parse adapter for JSONC (MEDIUM)
- [jsonc-parser crate](https://crates.io/crates/jsonc-parser) — comment-preserving parser for in-place settings.json edits (MEDIUM)

---

*Architecture research for: holt — Rust statusLine for Claude Code with multi-session orchestration*
*Researched: 2026-04-28*
