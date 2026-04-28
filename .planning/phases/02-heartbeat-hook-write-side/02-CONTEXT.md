# Phase 2: Heartbeat hook (write side) - Context

**Gathered:** 2026-04-28
**Status:** Ready for planning
**Mode:** `--auto` (Claude picked recommended defaults; no interactive Q&A)

<domain>
## Phase Boundary

A `holt hook <event>` subcommand that, when invoked by Claude Code on `PreToolUse` / `PostToolUse` / `Stop` / `Notification` / `SessionStart`, parses CC's stdin JSON envelope defensively (`#[serde(default)]` everywhere, no `unwrap()` on the parse), then writes a `schema_version: 1` heartbeat JSON to the per-session file at the canonical XDG path with a documented fallback chain — atomically and durably — and **never bubbles an error back to Claude Code** (always exits 0 from CC's perspective, even on failure to write).

**In scope:** `holt-hooks` crate (parsing logic, fallback path resolution, heartbeat assembly), `holt hook <event>` CLI subcommand wiring in `holt-cli`, the five-event subscription set, defensive stdin parse with `parse_fail` breach routing, atomic per-session file write at canonical XDG path with macOS / cache-dir fallback, schema_version=1 + writer_version + workspace.git_worktree adoption, 0600 file permissions, 1000× SIGKILL stress test, captured CC v2.1.119 stdin fixtures (the Phase 2 prerequisite carried over from Phase 1's open questions).

**Out of scope:** `holt install-hooks` subcommand and any `~/.claude/settings.json` mutation (Phase 3 owns C3, C4, JSONC handling, locking). The cross-session reader / orchestrator (v1.0). The `PreCompact` hook (v1.0 trigger fired but not in v0.1 subscription set per docs/02-scope.md). Any "rich" heartbeat fields beyond what HOOK-01..HOOK-06 require (current_tool, mode, context_pct_real, burn_rate_usd_per_min — those land in Phase 5/v1.0 per research/SUMMARY.md §"Recommended phase split"). Native rendering (still wrap-don't-compete at v0.1).

</domain>

<decisions>
## Implementation Decisions

### CC stdin fixtures (Phase 2 prerequisite — carried from Phase 1 open questions)

- **D-01:** Capture verbatim CC v2.1.119+ stdin JSONs for `PreToolUse`, `PostToolUse`, `Stop`, `Notification`, `SessionStart` to `crates/holt-hooks/tests/fixtures/cc-stdin/v2.1.119/<event>.json` **before any heartbeat write code is written**. Source: invoke `claude` locally with a known statusLine that dumps stdin to a file, capture once per event. If running in CI without claude installed, the planner provides synthetic fixtures matching the documented field shape from `code.claude.com/docs/en/changelog` v2.1.119 entry. **Workspace.git_worktree** field present (CC v2.1.98+) and **effort.level** with `xhigh` value MUST appear in at least one fixture so PITFALLS.md H5 + research/SUMMARY.md §"6 new features the docs missed" #2/#4 are exercised by tests.

- **D-02:** Fixtures are **golden** — committed to the repo, reviewed at PR time. A fixture-update path lives in `crates/holt-hooks/tests/fixtures/README.md` documenting how to refresh them when CC ships a stdin shape change. Fixtures versioned by CC version (`v2.1.119/`, `v2.1.120/`, …) — keep all versions, never delete; defensive parse must succeed on every prior fixture (forward-compat).

### `holt-hooks` crate API surface

- **D-03:** Single public entry point: `pub fn handle_event(event: HookEvent, stdin: &[u8]) -> HookOutcome`. `HookEvent` is `enum { PreToolUse, PostToolUse, Stop, Notification, SessionStart }`. `HookOutcome` is `enum { Wrote { path: PathBuf, bytes: usize }, FellBack { path: PathBuf, reason: FallbackReason }, ParseFailed { breach: BreachRecord }, Unwritable { breach: BreachRecord } }`. The CLI dispatcher in `holt-cli` always returns exit 0 to CC regardless of `HookOutcome` variant — failure goes to `breaches.log` only, never bubbled to CC's stderr.

- **D-04:** Defensive stdin parse via `serde_json::from_slice::<HookStdin>(bytes)` with `#[serde(default)]` on every field of `HookStdin`. On parse error: write a `parse_fail` entry to `breaches.log` (uses Phase 1 helper) capturing first 2KB of stdin, then exit 0. **Never panic.** Per research/SUMMARY.md §"6 new features" #4 — defensive parse is a load-bearing v0.1 escape hatch.

- **D-05:** **Heartbeat assembly is pure** — `pub fn assemble_heartbeat(event: HookEvent, stdin: &HookStdin, env: &Env) -> Heartbeat` returns the populated `holt_schemas::Heartbeat` struct WITHOUT writing. Separating assembly from write enables unit-testing field derivation (especially `cwd_label` derivation per D-08) without touching disk.

### Fallback path chain

- **D-06:** Writer path resolution chain (in this order; first writable wins):
  1. `$XDG_RUNTIME_DIR/holt/sessions/<sid>.json` (Linux convention)
  2. `$TMPDIR/holt-$UID/sessions/<sid>.json` (macOS convention; `$TMPDIR` defaults to `/var/folders/...` on macOS)
  3. `~/.cache/holt/sessions/<sid>.json` (universal fallback — also used when both above are unset or unwritable)
  4. **All three unwritable** → exit 0 silently AND write a `parse_fail`-style entry (`kind: "unwritable"`) to `~/.cache/holt/breaches.log` if THAT is writable; if breaches.log is also unwritable, exit 0 with no further action (CC must never see a hook error).

- **D-07:** Path resolution is computed ONCE per invocation and the chosen path is NOT cached across hook fires (each fire re-evaluates so a freshly-mounted `$XDG_RUNTIME_DIR` is picked up immediately). The `<sid>` segment comes from `stdin.session_id`; if absent or empty, the writer falls back to a deterministic hash of `(stdin.cwd, stdin.transcript_path)` to avoid clobbering. Document the hash policy in `crates/holt-hooks/src/path.rs` rustdoc.

### Heartbeat field derivation

- **D-08:** `cwd_label` derivation order (per HOOK-06 + research/SUMMARY.md §"6 new features" #2):
  1. If `stdin.workspace.git_worktree` is present and non-empty → use it verbatim (CC v2.1.98+ first-class field).
  2. Else if `stdin.cwd` parses to a `<repo>/<branch>` shape (heuristic: contains `/.git` ancestor) → derive `<repo>/<branch>` from the cwd basename + `git rev-parse --abbrev-ref HEAD` if the binary can find git in PATH (best-effort; fallback to cwd basename only).
  3. Else → use `stdin.cwd` basename verbatim.
  Two paired fixtures (`v2.1.119/PreToolUse.json` with workspace.git_worktree present, and a second `pre-2.1.98/PreToolUse.json` synthetic without it) MUST exercise both branches in `assemble_heartbeat` unit tests.

- **D-09:** `current_tool` field policy:
  - On `PreToolUse` → `Some(stdin.tool_name)` (the tool about to fire)
  - On `PostToolUse` → `None` (the tool has finished; null reflects "no in-flight tool")
  - On `Stop` / `Notification` / `SessionStart` → `None`
  This is the simplest variant that satisfies success criterion #2 of ROADMAP without prematurely landing the v1.0 "rich heartbeat" fields.

- **D-10:** `blocked_on` is always `None` at v0.1 (the field is reserved for v1.0 attention queue; serialize it explicitly so the JSON shape is forward-compatible without a schema bump). `last_assistant_at` and `model_display` are populated from `stdin.last_assistant_at` and `stdin.model.display_name` if present, else `None` / `""`.

- **D-11:** `writer_version` is populated from `env!("CARGO_PKG_VERSION")` at compile time of the `holt-cli` binary (NOT `holt-hooks` crate, because Plan 01-02's WR-08 fix landed `writer_version` plumbing through `SupervisorOptions`; reuse that plumbing pattern here — accept the version as a string parameter to `handle_event`'s `Env` struct, plumbed from the binary). This keeps `holt-hooks` independent of binary versioning and avoids divergence between supervisor and hook subcommand reporting different versions.

### Atomic write + permissions

- **D-12:** Use `holt_schemas::atomic_write` (Phase 1's hand-rolled same-dir tmp + PID suffix + fsync(2) + rename(2)) for the heartbeat write. After the rename, set 0600 permissions via `std::os::unix::fs::PermissionsExt::set_mode(0o600)` (Unix-only); on Windows, the file is created with the user's default ACL and the 0600 success-criterion clause is skipped (`#[cfg(unix)]` gated test). Permissions are set on the FINAL file path, not the tmp file (the rename inherits the tmp's mode anyway, but be explicit).

- **D-13:** SIGKILL atomicity test (`tests/sigkill_atomicity.rs`, `#[cfg(unix)]`): fork 1000× a child that runs `holt hook PreToolUse` with a known fixture; mid-write SIGKILL via random delay 0..15ms; after each iteration, parent reads the target file with `serde_json::from_slice` and asserts the result is one of: (a) zero-byte/missing (acceptable per Phase 1 reader contract C5), (b) the prior valid heartbeat (rename never happened), or (c) the new valid heartbeat. **No half-written file is ever observable.** Test budget: <30s wall clock (1000 forks × ~25ms ≈ 25s on macOS arm64).

### Subcommand wiring in `holt-cli`

- **D-14:** Extend the existing `holt run` clap derive structure with a new `holt hook <event>` subcommand. `event` is a positional string parsed via `clap::ValueEnum` mapped to `HookEvent`. The CLI dispatcher reads stdin, calls `handle_event`, ignores the `HookOutcome` (logs it via the existing `breaches.log` machinery if it's an error variant), exits 0. **No new top-level binary** — `holt hook` is a subcommand of the same `holt` binary, per CLAUDE.md "single-binary, sub-20ms cold start" North Star.

- **D-15:** **Hook subcommand cold-start budget = same sub-20ms render-path budget** because CC fires hooks synchronously and any holt-induced lag IS user-visible lag. Plan 01-03's `--self-bench` flow MUST be extended in this phase to cover `holt hook <event>` invocations (or a dedicated `holt --self-bench-hook <event>` flag), asserting hook fires under 20ms p95 on macOS arm64 / Linux x86_64. Add the gate to `.github/workflows/ci.yml` as part of `test-linux` and `test-macos` jobs.

### Claude's Discretion

The planner has flexibility on these — they're below the architecture-decision waterline:

- Module split inside `crates/holt-hooks/src/`: `mod.rs` vs `lib.rs` plus split files (`stdin.rs`, `assemble.rs`, `path.rs`, `event.rs`); planner picks based on test ergonomics.
- Whether the `Env` struct passed to `handle_event` is a `pub struct` or constructed via builder.
- Exact JSON shape of `HookOutcome::FellBack` `reason` field (string discriminant vs nested enum).
- Specific `<sid>` hash algorithm if `session_id` is missing (planner picks; document in path.rs).
- Whether `assemble_heartbeat` accepts `&HookStdin` or owns it.
- Test fixture file naming convention (`v2.1.119/PreToolUse.json` vs `2026-04-28-v2.1.119-PreToolUse.json` — pick whichever scans easier).
- Whether the SIGKILL stress test runs the 1000× loop in-process via `nix::unistd::fork` or out-of-process via `Command::new(env!("CARGO_BIN_EXE_holt")).args([...])` — the latter is simpler and cleaner; planner decides.
- Internal naming of the path-resolution helper (`resolve_writer_path` vs `pick_session_dir` etc.).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents (gsd-phase-researcher, gsd-planner) MUST read these before research and planning.**

### Project anchors (locked, authoritative)

- `.planning/PROJECT.md` — north star, hard constraints C1–C6, key locked decisions.
- `.planning/REQUIREMENTS.md` — REQ-IDs HOOK-01 through HOOK-06 mapped to Phase 2 (HOOK-11 is Phase 1).
- `.planning/ROADMAP.md` §"Phase 2: Heartbeat hook (write side)" — five success criteria.
- `.planning/STATE.md` — current position, accumulated context.
- `CONTRIBUTING.md` — Architectural North Star priority order.
- `CLAUDE.md` — project conventions, technology stack, hard constraints C1–C6.

### Phase 1 artifacts (just shipped — Phase 2 builds on these directly)

- `.planning/phases/01-schema-supervisor-substrate/01-CONTEXT.md` — locked decisions D-01..D-16 from Phase 1 (`atomic_write` helper API, `Heartbeat` struct shape, defensive parse posture).
- `.planning/phases/01-schema-supervisor-substrate/01-RESEARCH.md` — concrete code snippets (atomic write hand-rolled implementation, `#[serde(default)]` posture, `#[non_exhaustive]` rationale).
- `.planning/phases/01-schema-supervisor-substrate/01-VERIFICATION.md` — what's already proven green (Phase 2 should not regress these gates).
- `crates/holt-schemas/src/lib.rs` and submodules — the keystone API to consume: `read_heartbeat`, `atomic_write`, `Heartbeat`, `LkgEntry`, `ReaderError`. Phase 2 ONLY adds — does not modify these public types beyond appending fields if absolutely needed (and a field-append needs an explicit decision in this CONTEXT — none today).
- `crates/holt-supervisor/src/breaches.rs` — `append_breach` API; Phase 2's parse_fail / unwritable paths route through this, NOT a new logger.

### Research substrate

- `.planning/research/SUMMARY.md` §2 (drift: CC v2.1.119 stdin shape regression, defensive parse mandate), §3 (C5 reader contract — Phase 2's heartbeat writes must round-trip cleanly through Phase 1's `read_heartbeat`), §4 ("Phase 2 — Hook write + install-hooks UX" sub-section), §5 ("Phase 5 / P6 — CC v2.1.119 stdin shape fixtures" — actually a Phase 2 prerequisite per research/SUMMARY.md note).
- `.planning/research/STACK.md` — `serde_json` `preserve_order` requirement (Phase 1 fix-pass landed CR-01 already), no `simd-json`.
- `.planning/research/PITFALLS.md` — H2 (ext4 atomic-rename — already enforced by atomic_write helper), H4 (XDG fallback chain), H5 (defensive serde — D-04 here), H6 (mode fallback chain — D-12 0600 enforcement).

### Locked design docs

- `docs/02-scope.md` — v0.1 IN/OUT tables, the five-event subscription list (PreCompact deferred to v1.0).
- `docs/05-schemas.md` — heartbeat v1 schema lock (`schema_version: 1`, required fields list — D-08, D-09, D-10, D-11 all flow from this).

### External (read on demand only — do not pre-fetch)

- [Claude Code statusLine docs](https://code.claude.com/docs/en/statusline) — hook event names + stdin envelope shape.
- [Claude Code changelog](https://code.claude.com/docs/en/changelog) — v2.1.83–2.1.120 stdin field history (validate fixtures against this).
- [APFS rename atomicity](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/Features/Features.html) — guarantees holt-hooks relies on (already validated for Phase 1).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets (Phase 1 just shipped these)

- **`holt_schemas::atomic_write`** (`crates/holt-schemas/src/writer.rs`) — same-dir tmp + PID suffix + fsync(2) + rename(2). **Use this for the heartbeat write — DO NOT reimplement.** Phase 1 fix-pass CR-05 hardened the error path (tmp file cleaned up on every failure mode).
- **`holt_schemas::Heartbeat`** (`crates/holt-schemas/src/heartbeat.rs`) — defensive parse posture (`#[serde(default)]`, `#[non_exhaustive]`, `schema_version: u8` first). Phase 2 populates this; serializes it; never modifies the struct shape.
- **`holt_schemas::read_heartbeat`** (`crates/holt-schemas/src/reader.rs`) — Phase 2 round-trip tests use this to confirm Phase 2's writes parse via the C5 contract. Phase 1 fix-pass WR-02 broadened the soft-fail set to include PermissionDenied / IsADirectory.
- **`holt_supervisor::breaches::append_breach`** (`crates/holt-supervisor/src/breaches.rs`) — `parse_fail` + `unwritable` routing. **Reuse this — DO NOT add a second breach logger.** 5MB rotation policy is shared.
- **`holt_supervisor::paths::default_cache_root`** (`crates/holt-supervisor/src/paths.rs`) — `~/.cache/holt/` resolver with WR-01 fix-pass landed (falls back to temp dir, not CWD, when HOME unset). Phase 2 reuses this as the third tier of the fallback chain (D-06).

### Established Patterns (set by Phase 1)

- **Defensive parse posture** — `#[serde(default)]` on all optional fields; `serde_json::from_slice` returning `Err` is a `parse_fail` breach event, never a panic. Carry this verbatim into `holt-hooks::HookStdin`.
- **Atomic write** — same-dir tmp + PID suffix + fsync(2) + rename(2). The Phase 1 helper handles this; Phase 2 calls it.
- **5MB JSONL rotation** — for `breaches.log` (already in supervisor crate). Phase 2 doesn't need to add new rotation infra; it just emits records via `append_breach`.
- **`#[forbid(unsafe_code)]` at every crate root** — Phase 1 WR-09 fix landed this on all 6 crates. `crates/holt-hooks/src/lib.rs` already inherits the forbid lint.
- **Phantom `[package]` for workspace-root tests** — Phase 1 Plan 01-03 used `tests/architecture_dag.rs` via a phantom `holt-workspace-tests` package. Phase 2 likely doesn't need new workspace-root tests, but if it does (e.g., end-to-end CC fixture round-trip), reuse the phantom-package mechanism.

### Integration Points

- **`holt-cli/src/cli.rs`** — extend the clap derive enum with `Hook { event: HookEvent }`. The dispatcher in `holt-cli/src/main.rs` calls `holt_hooks::handle_event(event, stdin_bytes)` and exits 0.
- **`crates/holt-hooks/`** — currently a placeholder (`pub fn placeholder() {}` from Phase 1 Plan 01-01). Replace placeholder with real implementation; do NOT remove the crate from `[workspace] members` (C2 architecture-DAG test depends on the crate's continued existence).
- **`crates/holt-hooks/Cargo.toml`** — add deps: `holt-schemas = { path = "../holt-schemas" }`, `holt-supervisor = { path = "../holt-supervisor" }` (for `append_breach`), `serde`, `serde_json` (with `preserve_order` feature, inherited from workspace), `jiff`, `anyhow`, `nix` (Unix-only — for `umask` / `chmod` if needed beyond `PermissionsExt`). **Do NOT add tokio, simd-json, fs2, jsonc-parser, or any other forbidden crate.**
- **`crates/holt-cli/src/stdin.rs`** — Phase 1 fix-pass CR-04 added a 200ms deadline to stdin slurp. The hook subcommand can reuse this same slurp helper but with a longer deadline (5s? or wait until EOF — hooks don't have to write within the render-path budget but D-15 sets a sub-20ms target anyway). Planner picks.

</code_context>

<specifics>
## Specific Ideas

- **The Phase 2 prerequisite is non-negotiable: capture CC v2.1.119+ stdin fixtures BEFORE writing heartbeat assembly code.** Without real fixtures, the defensive-parse posture is unfalsifiable and PITFALLS.md H5 (Windows v2.1.119 regression + xhigh effort.level breakage) would silently drift. Plan tasks should put fixture capture as a Wave 1 task; assembly + write tasks come after.
- **Hook fires on the render path, but with relaxed budget vs `holt run`.** CC fires hooks synchronously; any lag is user-visible. The 20ms p95 budget D-15 sets is conservative — measured locally, even a stat + json parse + atomic_write should clock <5ms. The budget exists to catch regressions early.
- **The five-event subscription list is fixed at v0.1: `PreToolUse`, `PostToolUse`, `Stop`, `Notification`, `SessionStart`.** PreCompact (CC v2.1.105) is a v1.0 trigger. Do NOT subscribe to it in Phase 2.
- **Writer never bubbles errors to CC.** This is in the goal statement. Every error path returns exit 0; failures route to `breaches.log` only. The CLI dispatcher's only `Result` consumption is "log the error variant if any, then exit 0."
- **Round-trip test mandatory:** Phase 2 must include a test that calls `holt hook PreToolUse` with a fixture, then calls `holt_schemas::read_heartbeat` on the result file and confirms `Some(Heartbeat { schema_version: 1, .. })` round-trips cleanly. This locks the C5 reader contract from the writer side.

</specifics>

<deferred>
## Deferred Ideas

- **`PreCompact` hook subscription** — v1.0 trigger fired (CC v2.1.105 shipped) but not in v0.1 subscription list. Add to `HookEvent` enum at v1.0; Phase 2 leaves the enum at 5 variants.
- **Rich heartbeat fields** — `mode`, `context_pct_real`, `burn_rate_usd_per_min`, `last_user_at` are v1.0 territory per research/SUMMARY.md "Recommended Phase Split" §"Phase 5". Phase 2 only populates the fields HOOK-01..HOOK-06 require.
- **Cross-session reader / orchestrator** — v1.0. Phase 2 only writes; the v1.0 orchestrator reads.
- **Token usage / `effort.level` aggregation** — depends on CC #52089 trigger-watch and is v1.0. Phase 2 does NOT compute or persist token math.
- **`holt install-hooks` subcommand** — Phase 3 owns. Phase 2 ships the `holt hook <event>` command that install-hooks merges into `~/.claude/settings.json`; Phase 2 produces NO settings.json mutation code.
- **Daemon optimization** — gated on ≥3 user reports of ≥10-session lag. Files-on-disk + hooks remains the v1.0 model.
- **eCryptfs-aware first-run warning** — `holt doctor --first-run` (v0.5). Phase 2 does NOT detect or warn about eCryptfs latency.
- **schema_version: 2 migration path** — `#[non_exhaustive]` from Phase 1 lets v2 readers degrade. Phase 2 does NOT design v2; only ensures `schema_version: 1` is tagged on every write.

</deferred>

---

*Phase: 02-heartbeat-hook-write-side*
*Context gathered: 2026-04-28 (--auto mode; recommended defaults grounded in PROJECT.md, REQUIREMENTS.md, research/SUMMARY.md, research/PITFALLS.md, docs/02-scope.md, docs/05-schemas.md, and Phase 1 SUMMARY/VERIFICATION/REVIEW-FIX artifacts)*
