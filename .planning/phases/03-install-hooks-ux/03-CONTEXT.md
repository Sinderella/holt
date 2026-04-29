# Phase 3: install-hooks UX - Context

**Gathered:** 2026-04-28
**Status:** Ready for planning
**Mode:** `--auto` (Claude picked recommended defaults; no interactive Q&A)

<domain>
## Phase Boundary

A `holt install-hooks` subcommand that idempotently merges holt's five-event hook entries (`PreToolUse` / `PostToolUse` / `Stop` / `Notification` / `SessionStart`, the same set Phase 2 ships) into the user's `~/.claude/settings.json` without corrupting any pre-existing hooks, without losing JSONC comments or key order, without racing concurrent editors, and with `--dry-run` / `--print` escape hatches for users who'd rather paste manually. C3 (file locking + fsync-before-rename + PID-suffix tmp + `.holt.bak` backup) and C4 (JSONC handling lives ONLY in `holt-cli`) are the load-bearing constraints.

**In scope:** `holt install-hooks` subcommand wiring in `holt-cli`, JSONC strategy spike fixture corpus, JSONC round-trip via `jsonc-parser = "0.26"` CST (preserves comments + key order), `fs2 = "0.4"` exclusive-lock acquisition + 200ms timeout, fsync-before-rename + PID-suffixed tmp file via Phase 1's `holt_schemas::atomic_write` helper (or extension), `.holt.bak` backup before mutation, `--dry-run` (prints unified diff to stdout, exits 0), `--print` (prints JSON snippet for manual paste, exits 0), 50× concurrent-invocation stress test, SIGKILL-mid-write atomicity test, paired fixtures exercising both raw JSON and JSONC-with-comments paths.

**Out of scope:** any `~/.claude/settings.json` mutation by other holt subcommands (only `holt install-hooks` writes; `holt run` and `holt hook` never touch it). Generic JSONC support outside `holt-cli` (C4 — JSONC stays in `holt-cli` only). v1.0 settings.json features like updating an existing holt hook entry to a newer hook command shape (Phase 3 is install-only; uninstall + update are deferred). Distribution scaffolding / Homebrew tap / `dist init` (Phase 4 owns). Schema_version: 2 migration paths (v1.0).

</domain>

<decisions>
## Implementation Decisions

### JSONC strategy spike (Phase 3 prerequisite — carried over from Phase 1 + 2 open questions)

- **D-01:** **Drive the JSONC composition question to ground BEFORE writing merge code.** Ship a `crates/holt-cli/tests/fixtures/settings/` corpus with at least 6 paired JSONC inputs that exercise: (a) clean JSON no comments, (b) line comments only (`// ...`), (c) block comments only (`/* ... */`), (d) trailing commas, (e) comments inside `hooks` array (not just at top level), (f) user-defined `PreToolUse` entry alongside holt's. Each fixture has a "expected output" sibling showing what the merger should produce.

- **D-02:** **Strategy = pure `jsonc-parser` CST round-trip. NO `json_comments` strip-then-parse.** Rationale: `json_comments` strips the comment AST; `jsonc-parser` CST preserves byte positions + comment nodes. Composing them creates byte-offset drift on round-trip. The CST API supports in-place edit (insert / replace / append at JSON pointer paths) without a second parse — this is the path. `json_comments` is **not** a Phase 3 dep. Pin `jsonc-parser = "0.26"` with `cst` feature enabled in workspace dependencies (NOT enabled inline — workspace pin so the C4 boundary is unambiguous).

- **D-03:** **Confine the JSONC dep to `holt-cli/Cargo.toml` only.** C4 from research/SUMMARY.md §3 — `jsonc-parser` MUST NOT appear in any other crate's manifest. Verify via a CI gate (similar to architecture_dag.rs): grep all `crates/*/Cargo.toml` and assert `jsonc-parser` appears only in `crates/holt-cli/Cargo.toml`. Lands in this phase as `tests/jsonc_boundary.rs` (workspace-root test).

### File locking + atomic write (C3)

- **D-04:** **`fs2 = "0.4"` exclusive lock** acquired via `FileExt::try_lock_exclusive()` on the settings.json file itself (NOT a separate lock file). 200ms blocking-with-timeout pattern: try once, sleep 50ms, try again, repeat 4× max → if still locked, exit non-zero with `another holt install-hooks is running (or settings.json is locked by another editor)` to stderr. Lock is held for the read-merge-write window; released on file drop.

- **D-05:** **Confine the `fs2` dep to `holt-cli/Cargo.toml` only.** Same C4-style boundary as JSONC — the lock semantics are user-edit-aware and shouldn't leak to render-path crates. Add to the same `tests/jsonc_boundary.rs` boundary check (or rename to `tests/cli_dep_boundary.rs` covering both `jsonc-parser` and `fs2`).

- **D-06:** **Tmp file shape:** `~/.claude/settings.json.holt-tmp.<pid>` per C3. NOT `.bak` (vim's namespace per research/SUMMARY.md §3 C3). Reuse Phase 1's `holt_schemas::atomic_write` helper for the rename machinery, BUT wrap it with the C3 lock acquisition + JSONC round-trip in `holt-cli`. The helper's same-dir tmp + fsync(2) + rename(2) discipline is unchanged.

- **D-07:** **Backup file = `~/.claude/settings.json.holt.bak`** (NOT `.bak`). Created BEFORE mutation begins, equals pre-merge bytes byte-for-byte (success criterion #1 second clause). Backup is overwritten on each subsequent `holt install-hooks` run — single backup retained, not a versioned chain. Rationale: settings.json is a long-lived user file; backup chains accumulate cruft. Document this single-backup policy in `holt install-hooks --help`.

### Idempotency + merge semantics

- **D-08:** **Idempotency = "holt's five canonical hook entries appear in settings.json with the documented shape, in addition to whatever else the user already has."** The merger does NOT replace existing user-defined hook entries with the same event name — it co-exists (CC's hook system supports multiple hooks per event, fired in array order). If holt already has an entry for a given event, the merger replaces it (in-place edit at the same JSON pointer); it does NOT append a duplicate.

- **D-09:** **Hook entry shape — exact bytes for each event:**
  ```json
  {
    "matcher": "*",
    "hooks": [
      { "type": "command", "command": "holt hook PreToolUse" }
    ]
  }
  ```
  Where the `command` string is the literal `holt hook <event>` for each of the 5 events. The `matcher: "*"` matches all tool names (CC's hook matcher syntax). Detection of "holt's entry" for replacement-vs-append decision uses an exact match on `command: "holt hook <event>"` substring (so a user-edited variant doesn't get accidentally clobbered).

- **D-10:** **Detect-by-command-substring policy:** the merger walks all `hooks.<event>[]` array entries and treats any entry whose `hooks[].command` contains `"holt hook "` as "ours" — replaceable. User-defined entries (any other `command` value) are preserved verbatim alongside ours. The substring check intentionally accepts variants like `"PATH=... holt hook PreToolUse"` (env-var prefix) so users can wrap our entry without losing it on next install.

### Escape hatches

- **D-11:** **`--dry-run`:** prints a unified diff (`-` prefix old / `+` prefix new) to stdout showing exactly what `holt install-hooks` would change. Exits 0. Does NOT acquire the fs2 lock (read-only). Does NOT touch settings.json or the .bak. The diff is generated by computing both pre-merge and post-merge JSONC bytes, then running them through the standard `similar = "2"` crate (or hand-rolled diff if pulling `similar` is too heavy — defer to planner) to produce unified-diff output.

- **D-12:** **`--print`:** prints the JSON snippet `holt install-hooks` would merge in (just the holt-specific hook entries, not the full file) to stdout. Exits 0. Does NOT acquire the fs2 lock. Users copy-paste this into their own settings.json manually. Output is pretty-printed (2-space indent) so it's paste-ready.

- **D-13:** **`--help` UX:** explains the three modes (default = mutate, `--dry-run` = show diff, `--print` = show snippet), the `.holt.bak` single-backup policy, the lock-timeout behavior, and the "holt hook <event>" detection heuristic. The help text is the primary documentation surface — keep it tight (≤40 lines).

### Concurrency + atomicity tests

- **D-14:** **50× concurrent-invocation stress test** (`crates/holt-cli/tests/install_hooks_concurrent.rs`, `#[cfg(unix)]`): spawn 50 child processes via `std::process::Command`, each running `holt install-hooks` against a shared temp `~/.claude/settings.json` fixture. Wait for all to exit, assert: (a) the final file parses as valid JSONC via `jsonc-parser`, (b) holt's 5 hook entries are present exactly once each, (c) any pre-existing user `PreToolUse` entry survived, (d) no `.holt-tmp.<pid>` file remains in `~/.claude/`. Test budget: <30s wall clock.

- **D-15:** **SIGKILL atomicity test** (`crates/holt-cli/tests/install_hooks_sigkill.rs`, `#[cfg(unix)]`): fork a child running `holt install-hooks`, SIGKILL at random delay 0..30ms (longer window than Phase 2's hook write — the merge is more expensive). After each iteration, parent reads `~/.claude/settings.json` and asserts: (a) parses cleanly via both `serde_json::from_str` and `jsonc-parser::parse_to_ast`, AND (b) is either the pre-merge state OR the post-merge state — never half-written. Loop count = 200× (lower than Phase 2's 1000× because each iteration is slower); test budget: <60s wall clock.

### Subcommand wiring in `holt-cli`

- **D-16:** Extend `crates/holt-cli/src/cli.rs` clap derive enum with `InstallHooks { #[arg(long)] dry_run: bool, #[arg(long)] print: bool }`. Mutually exclusive flags via clap's `conflicts_with`. The dispatcher in `holt-cli/src/main.rs` calls `install_hooks::run(dry_run, print)`. Place implementation at `crates/holt-cli/src/install_hooks.rs` (parallel to existing `hook.rs` from Phase 2).

- **D-17:** **Cold-start budget for `holt install-hooks` is RELAXED to <500ms** because (a) it's user-invoked, not on the render path, and (b) it does I/O (lock, read, parse JSONC, merge, write, fsync, rename). The 20ms self-bench gate does NOT apply. CI verifies the command completes within 500ms on a synthetic fixture.

### Claude's Discretion

The planner has flexibility on these — they're below the architecture-decision waterline:

- Module split inside `crates/holt-cli/src/install_hooks/`: single file vs `mod.rs` + `merge.rs` + `lock.rs` + `diff.rs` + `print.rs`.
- Whether `--dry-run` uses the `similar` crate or hand-rolled diff (small diff — hand-roll is plausible).
- Exact JSON Pointer or CST node-walk strategy for inserting holt entries (jsonc-parser supports both).
- Whether the 50× concurrent test uses out-of-process Command spawn or in-process fork.
- Specific error message wording for the lock-timeout case (so long as it includes "holt install-hooks" and a hint about settings.json).
- Whether `--help` is hand-rolled or auto-generated by clap (planner picks).
- File naming for the JSONC fixture corpus (`fixtures/settings/<scenario>.input.json` + `.expected.json` pairs is one option).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before research and planning.**

### Project anchors

- `.planning/PROJECT.md` — north star, hard constraints C1–C6.
- `.planning/REQUIREMENTS.md` — REQ-IDs HOOK-07..HOOK-10 mapped to Phase 3.
- `.planning/ROADMAP.md` §"Phase 3: install-hooks UX" — five success criteria.
- `.planning/STATE.md` — current position, JSONC spike open question.
- `CLAUDE.md` — project conventions, technology stack, hard constraints (C3 + C4 are the load-bearing pair).
- `CONTRIBUTING.md` — Architectural North Star priority order.

### Phase 1 + Phase 2 artifacts (just shipped)

- `.planning/phases/01-schema-supervisor-substrate/01-CONTEXT.md` (D-07 atomic_write helper API)
- `.planning/phases/01-schema-supervisor-substrate/01-VERIFICATION.md` (atomic_write hardened post-CR-05)
- `.planning/phases/02-heartbeat-hook-write-side/02-VERIFICATION.md` (current binary surface — `holt run` / `holt hook` / `holt --self-bench` / `holt --self-bench-hook` / `holt --version`; Phase 3 adds a 6th entry point `holt install-hooks`)
- `crates/holt-cli/src/cli.rs` (clap derive structure to extend)
- `crates/holt-cli/src/main.rs` (dispatcher to extend)
- `crates/holt-schemas/src/writer.rs` (atomic_write helper; Phase 3 calls it for the merged-file write)

### Research substrate

- `.planning/research/SUMMARY.md` §3 (C3 + C4 — load-bearing for this phase), §5 ("Phase 2 / P3 — JSONC strategy spike" — actually Phase 3, not Phase 2; resolved here as D-01 + D-02), §6 (`atomic-write-file` adopt only on corruption reports — not adopting now).
- `.planning/research/STACK.md` — `jsonc-parser` v0.26+ with `cst` feature, `fs2` v0.4 (use `try_lock_exclusive()`), forbidden-crate list (no `simd-json`, etc.).
- `.planning/research/PITFALLS.md` H1 (settings.json corruption — load-bearing for this phase), H7 (Gatekeeper — N/A for Phase 3 binaries; Phase 4 owns).

### Locked design docs

- `docs/02-scope.md` — five-event subscription list (PreCompact deferred to v1.0).

### External (read on demand)

- [jsonc-parser docs.rs](https://docs.rs/jsonc-parser/latest/jsonc_parser/) — CST API for in-place edit + comment preservation.
- [fs2 docs.rs](https://docs.rs/fs2/latest/fs2/) — `FileExt::try_lock_exclusive()` semantics on macOS / Linux / Windows.
- [LWN: fsync-before-rename](https://lwn.net/Articles/789600/) — already cited in Phase 1 D-07; same rationale applies.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets (Phase 1 + 2)

- **`holt_schemas::atomic_write`** (`crates/holt-schemas/src/writer.rs`) — same-dir tmp + PID suffix + fsync(2) + rename(2). Phase 1 fix-pass CR-05 hardened the error path. Phase 3 calls it for the post-merge file write — DO NOT reimplement.
- **`holt-cli` clap derive structure** (`crates/holt-cli/src/cli.rs`) — already has Run / Hook subcommands + --self-bench / --self-bench-hook / --version flags. Phase 3 extends with `InstallHooks { dry_run, print }`.
- **`holt-cli/src/main.rs` dispatcher pattern** — match on `cli::Command::*` variants; call subcommand `run` function; exit with the returned code. Phase 3 follows the same pattern.
- **`holt_supervisor::breaches::append_breach`** — N/A for Phase 3 (install-hooks failures bubble to stderr + non-zero exit, NOT a breach event because it's not on the render path).

### Established Patterns (set by Phase 1 + 2)

- **One concept per PR** — Phase 3's plans should mirror Phase 2's structure (1 plan for the library + spike, 1 plan for the CLI wiring + tests).
- **Defensive parse posture** — applies even outside the hook context: `serde_json::from_str` on a malformed settings.json should produce a useful error (the file path + parse error), NOT panic. Same for `jsonc-parser::parse_to_ast`.
- **Atomic write + fsync-before-rename** — Phase 1 helper handles this; Phase 3 reuses it.
- **`#[forbid(unsafe_code)]` at every crate root** — already in place for all 6 crates (Phase 1 WR-09 fix-pass). Phase 3 adds no unsafe.
- **Workspace test for boundary enforcement** — Phase 1 P0 set the architecture_dag.rs precedent (workspace-root test that walks `cargo metadata`). Phase 3 adds `tests/cli_dep_boundary.rs` covering jsonc-parser + fs2 confined to holt-cli.

### Integration Points

- **`crates/holt-cli/src/cli.rs`** — extend clap derive with `InstallHooks { dry_run, print }`. The `--self-bench-hook <event>` precedence pattern from Phase 2 doesn't apply here (no equivalent flag).
- **`crates/holt-cli/src/main.rs`** — add dispatch arm for `cli::Command::InstallHooks { dry_run, print } => install_hooks::run(dry_run, print)`.
- **`crates/holt-cli/Cargo.toml`** — add deps: `jsonc-parser = { version = "0.26", features = ["cst"] }`, `fs2 = "0.4"`. Optionally `similar = "2"` for the unified diff (planner picks). NO new deps in any other crate.
- **`tests/cli_dep_boundary.rs` (workspace root)** — new test that walks `cargo metadata` (reusing the BFS pattern from `tests/architecture_dag.rs`) and asserts `jsonc-parser` and `fs2` appear ONLY in `holt-cli`'s package. Companion to architecture_dag.rs.

</code_context>

<specifics>
## Specific Ideas

- **The JSONC spike is non-negotiable as Wave 1's first task.** D-01's fixture corpus must exist BEFORE any merge code is written so the merger's behavior is falsifiable from day one. Without fixtures, the comment-preservation invariant is unfalsifiable.
- **`fs2` and `jsonc-parser` are the two new deps that flip from "forbidden" to "allowed in holt-cli only."** All other forbidden crates remain forbidden everywhere. Plan 03-XX should NOT introduce tokio, simd-json, figment, chrono, fs2-elsewhere, jsonc-parser-elsewhere, owo-colors, supports-color, terminal_size, atomic-write-file, crossterm, wait-timeout, cargo_metadata.
- **The `.holt.bak` naming is load-bearing.** `.bak` is vim's namespace; using it would conflict with vim's swap-file recovery. Document the exact naming in `--help` output.
- **Lock-timeout messaging is user-facing.** "another holt install-hooks is running (or settings.json is locked by another editor)" — keep this wording so support tickets can grep for it. The 200ms timeout is a balance: long enough to handle a quick concurrent write, short enough that users don't think it's hung.
- **CI must run the install-hooks integration tests against a temp `~/.claude/`.** The tests MUST NOT touch the developer's real `~/.claude/settings.json`. Use `tempfile::tempdir()` (test-only — the dev-dependency flip from Phase 2 WR-08 doesn't apply here; tempfile is fine in dev-deps) + override the home directory via `HOME=$tempdir` in the test setup.
- **Concurrent-invocation success criterion is "equivalent to serial execution."** That means: post-stress, the file is in the same state as if all 50 invocations had run one after the other. holt's 5 entries present exactly once; user's pre-existing entries preserved.

</specifics>

<deferred>
## Deferred Ideas

- **`holt uninstall-hooks` subcommand** — v0.5+. Phase 3 is install-only.
- **`holt install-hooks --update` for replacing v0.1 entries with v0.5 / v1.0 entries** — once schema_version: 2 lands. Phase 3 is v1-only.
- **Updating individual hook entry commands without re-running the full merge** — v1.0 quality-of-life. Phase 3's idempotency means re-running install-hooks is the supported update path.
- **Atomic settings.json across multiple holt versions running concurrently on the same machine** — gated on user reports. Single-backup policy is sufficient for v0.1.
- **`atomic-write-file` crate adoption** — STACK.md §3 audited fallback; adopt only on ≥2 corruption reports. Phase 3 reuses Phase 1's hand-rolled `atomic_write`.
- **Distribution + Homebrew tap + `dist init`** — Phase 4 owns.
- **Native rendering (replacing the wrap-don't-compete posture)** — v0.5+ trigger.
- **`PreCompact` hook subscription** — v1.0 (CC v2.1.105 fired). Phase 3's install-hooks merges only the v0.1 five-event subscription.
- **`holt install-hooks --quiet` / verbose levels** — keep `--help` output simple; if user demand surfaces, add later.
- **Full JSON Pointer-based partial updates** — Phase 3's CST in-place edit is sufficient; no JSON Pointer URI scheme.

</deferred>

---

*Phase: 03-install-hooks-ux*
*Context gathered: 2026-04-28 (--auto mode; recommended defaults grounded in PROJECT.md, REQUIREMENTS.md, research/SUMMARY.md, research/PITFALLS.md, research/STACK.md, docs/02-scope.md, and Phase 1 + 2 artifacts)*
