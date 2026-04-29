# Roadmap: holt

**Milestone:** v0.1 — Runtime hygiene wedge (the lovable MVP, target 3–4 weekends)
**Created:** 2026-04-28
**Granularity:** coarse (4 phases, 1–3 plans each)
**Parallelization:** enabled
**v1 Requirements covered:** 28 / 28 ✓

## Milestone goal

Ship a Rust shim that wraps the user's existing `statusLine.command` so Claude Code's statusLine never silently fails, never blocks input, and is observable enough to diagnose later. Land the heartbeat-write substrate that v0.5 (`holt doctor`) and v1.0 (orchestrator + Nak) will compose on top of — without making any of those promises at v0.1.

## Phases

- [x] **Phase 1: Schema + supervisor substrate** ✓ 2026-04-28 — `holt-schemas` keystone crate (heartbeat type, atomic-rename helper, non-panicking reader contract) + `holt-supervisor` (process-wrap with stdio piping, timeout/killpg, LKG cache, timings.jsonl, breaches.log) + `holt-cli` default render path (wrap + supervise + passthrough). Sub-20ms cold-start verified by `holt --self-bench` (p95=0us on macOS arm64). Verified passed: 5/5 ROADMAP must-haves green, 11/11 reqs, 16/16 decisions, 4/4 hard constraints (C1/C2/C5/C6) test-enforced, 28/28 workspace tests post-review-fix.
- [x] **Phase 2: Heartbeat hook (write side)** ✓ 2026-04-28 — `holt-hooks` crate + `holt hook <event>` subcommand. CC stdin envelope parse with defensive serde, atomic heartbeat write to `$XDG_RUNTIME_DIR/holt/sessions/<sid>.json` (with macOS / cache-dir fallback chain), `schema_version: 1`, `writer_version` field, `workspace.git_worktree` adoption. Verified passed: 5/5 ROADMAP must-haves green, 6/6 reqs (HOOK-01..06), 15/15 decisions, 4/4 hard constraints (C1/C2/C5/C6) preserved + new D-15 hook self-bench gate (sub-20ms p95) test-enforced. 51/51 workspace tests post-review-fix.
- [ ] **Phase 3: install-hooks UX** — `holt install-hooks` subcommand. Read-merge-write of `~/.claude/settings.json` with `fs2` exclusive lock, JSONC round-trip via `jsonc-parser` CST (preserves comments + key order), `--dry-run` and `--print` escape hatches, `.holt.bak` backup, fsync-before-rename.
- [ ] **Phase 4: Distribution + launch** — `dist` v0.31.0 scaffold (Linux x64, macOS x64+arm64, Windows x64 best-effort), Homebrew tap (`<user>/holt`), `cargo-binstall` metadata, MSRV 1.87 / Edition 2024 pin, README leading with asciinema/gif demo, CONTRIBUTING.md tags configured on the repo.

## Phase Details

### Phase 1: Schema + supervisor substrate

**Goal**: A `holt` binary that wraps the user's existing `statusLine.command`, supervises it with a configurable timeout + clean Unix process-group kill, falls through to a last-known-good cache on slow invocations, and writes per-fire timing + breach telemetry — all on the sub-20ms cold-start budget — while landing the keystone `holt-schemas` crate (heartbeat type + atomic-rename helper + non-panicking reader contract) that every subsequent phase depends on.

**Depends on**: Nothing (first phase; this is the keystone)

**Requirements**: CORE-01, CORE-02, CORE-03, CORE-04, CORE-05, CORE-06, CORE-07, CORE-08, CORE-09, CORE-10, HOOK-11

**Success Criteria** (what must be TRUE):
  1. `holt run -- bash -c "echo hello"` emits exactly `hello\n` on stdout (no holt chrome on the happy path); when the wrapped script exits 0 in <100ms, `~/.cache/holt/timings.jsonl` gains exactly one new line containing valid JSON with `duration_ms`, `fork_count`, `exit_code: 0`, and `stderr_capture: ""` for that fire.
  2. Wrapping `bash -c 'sleep 5'` with `holt run --timeout 1s --` kills the process group within 100ms of the timeout breach AND no orphaned `bash` or `sleep` PIDs remain (verified by `pgrep -f sleep` returning empty), AND the LKG cache (when present) is rendered in <2ms; if no LKG exists, `~/.cache/holt/breaches.log` gains one entry with the wrapped env, stdin JSON, and stderr.
  3. `holt --self-bench` reports holt-only render-path overhead under 20ms on macOS arm64 AND Linux x86_64 (≥10 iterations, p95) when the wrapped script returns instantly; the same probe asserts the render path opens neither `breaches.log` nor `timings.jsonl` for reading (verified via `strace -e openat,access` filter on Linux and a stub-replaced fs interface in the unit test on macOS).
  4. Feeding malformed CC stdin (`{"session_id":` truncated) to `holt run` does NOT error, does NOT panic — the binary captures a `parse_fail` event in `breaches.log`, falls through to the LKG cache (or empty stdout if no LKG yet), and exits 0; `holt-schemas::read_heartbeat()` exposed for use by Phase 2 returns `Result<Option<Heartbeat>, _>` and unit tests confirm it returns `Ok(None)` (never `Err` and never panics) for: zero-byte file, truncated JSON, unrecognized `schema_version`, missing required fields.
  5. `cargo tree --workspace --duplicates -p holt-render` reports zero direct or transitive dependency edge from `holt-render` to `holt-supervisor`; CI fails the PR if this edge is introduced (asserted via a `tests/architecture_dag.rs` test that walks `cargo metadata` output).

**Plans**: 3 plans
- [ ] 01-01-PLAN.md — Workspace skeleton + holt-schemas keystone (Heartbeat, atomic_write, read_heartbeat)
- [ ] 01-02-PLAN.md — holt-supervisor wedge (chokepoint, LKG, timings.jsonl, breaches.log, killpg + EPERM fallback)
- [ ] 01-03-PLAN.md — holt-cli (run / --self-bench / --version) + architecture_dag test + CI workflow

### Phase 2: Heartbeat hook (write side)

**Goal**: A `holt hook <event>` subcommand that, when invoked by Claude Code on `PreToolUse` / `PostToolUse` / `Stop` / `Notification` / `SessionStart`, parses CC's stdin envelope defensively and writes a `schema_version: 1` heartbeat JSON to the per-session file at the canonical XDG path (with documented fallback chain), atomically and durably — without ever bubbling an error back to Claude Code.

**Depends on**: Phase 1 (`holt-schemas` heartbeat type + atomic-rename helper; `holt-cli` subcommand-dispatch skeleton)

**Requirements**: HOOK-01, HOOK-02, HOOK-03, HOOK-04, HOOK-05, HOOK-06

**Success Criteria** (what must be TRUE):
  1. After firing `holt hook PreToolUse` with a captured CC v2.1.119 stdin fixture, the file `$XDG_RUNTIME_DIR/holt/sessions/<sid>.json` exists, is exactly one syntactically valid JSON object (verified via `jq .`), contains `schema_version: 1`, `writer_version` (a semver string matching `holt --version`), `pid`, `started`, `updated`, `cwd`, `cwd_label`, `mode`, `current_tool`, `blocked_on: null`, `last_assistant_at`, `model_display`, AND the file's permissions are `0600`.
  2. The hook subcommand produces the heartbeat for all five event types (`PreToolUse`, `PostToolUse`, `Stop`, `Notification`, `SessionStart`) AND each event correctly updates `updated` to the current timestamp AND `current_tool` reflects the in-flight tool on `PreToolUse` and is null on `Stop`/`Notification` — verified by replaying five fixture stdin JSONs in sequence and asserting the heartbeat fields after each.
  3. Killing the hook with `SIGKILL` mid-write leaves the target file either (a) in its pre-write state OR (b) at the new fully-written state — never a half-written file: verified by a stress test that runs `holt hook PreToolUse` 1000× with random `SIGKILL` interleaving and asserts every observed read of `<sid>.json` parses cleanly with `serde_json::from_slice` (zero-byte file is an acceptable "missing" outcome that the Phase 1 reader contract handles).
  4. With `$XDG_RUNTIME_DIR` unset and `$TMPDIR` unset, the hook still writes a heartbeat to `~/.cache/holt/sessions/<sid>.json` AND emits a single one-line stderr warning naming the fallback path; with all three locations un-writable, the hook exits 0 silently (never blocks CC) and writes a parse-fail-style entry to `breaches.log`.
  5. Feeding a CC stdin where `workspace.git_worktree` is absent (older CC) AND a stdin where it is present (CC v2.1.98+) both produce a non-empty `cwd_label` field: derived from `workspace.git_worktree` when present, falling back to `<repo>/<branch>` parsed from `cwd` otherwise — verified by two paired fixtures.

**Plans**: 2 plans
- [ ] 02-01-PLAN.md — Fixture capture + holt-hooks crate (HookStdin defensive parse, HookEvent enum, three-tier fallback path, assemble_heartbeat, atomic_write + 0o600, parse_fail / unwritable breach routing, 1000× SIGKILL atomicity test)
- [ ] 02-02-PLAN.md — holt-cli `holt hook <event>` subcommand wiring + integration tests + D-15 hook self-bench gate added to CI

### Phase 3: install-hooks UX

**Goal**: A `holt install-hooks` subcommand that idempotently merges holt's hook entries into the user's `~/.claude/settings.json` without corrupting any pre-existing hooks, without losing JSONC comments or key order, without racing concurrent editors, and with a `--dry-run` / `--print` escape hatch for users who'd rather paste manually.

**Depends on**: Phase 2 (knows which hook event names + commands to merge in; reuses `holt-cli` subcommand-dispatch skeleton from Phase 1)

**Requirements**: HOOK-07, HOOK-08, HOOK-09, HOOK-10

**Success Criteria** (what must be TRUE):
  1. After running `holt install-hooks` against a fresh `~/.claude/settings.json` containing `{ "statusLine": { "command": "ccstatusline" } }`, `cat ~/.claude/settings.json | jq '.hooks'` shows holt's five hook entries (`PreToolUse`, `PostToolUse`, `Stop`, `Notification`, `SessionStart`) merged in, the original `statusLine` block is preserved unchanged, AND `~/.claude/settings.json.holt.bak` exists and equals the pre-merge file byte-for-byte.
  2. Running `holt install-hooks` against a settings.json containing `// my hook` comments and a user-defined `PreToolUse` entry leaves the comments verbatim (round-trip via `jsonc-parser` CST), preserves the user's `PreToolUse` entry alongside holt's, AND key order in the resulting file matches the source file's key order (asserted via byte-position diff of recognized keys).
  3. `holt install-hooks --dry-run` and `holt install-hooks --print` both exit 0 without modifying `~/.claude/settings.json` (asserted via `mtime` comparison before/after) — `--dry-run` prints a unified diff to stdout; `--print` emits the JSON snippet to stdout for manual paste.
  4. Two concurrent `holt install-hooks` invocations from separate terminals against the same settings.json produce a result equivalent to running them serially: the second invocation either acquires the `fs2::FileExt::try_lock_exclusive()` lock and produces an idempotent no-op result, OR exits non-zero within 200ms with a "another holt install-hooks is running" message — never produces a torn write (verified by a 50× concurrent-invocation stress test that asserts the final file always parses with both `serde_json` and `jsonc-parser`).
  5. Killing `holt install-hooks` with `SIGKILL` mid-write leaves either the original `settings.json` intact OR the new merged version intact — never a half-written file (verified by `fsync(2)` on the temp fd before `rename(2)` and by a `dm-flakey`-style power-loss simulation that asserts `serde_json::from_str` succeeds on every observed state of the file).

**Plans**: TBD

### Phase 4: Distribution + launch

**Goal**: A user can `brew install <user>/tap/holt` (macOS), `cargo binstall holt` (any platform with the toolchain), or download a prebuilt binary from a GitHub release for Linux x64 / macOS x64+arm64 / Windows x64, follow a three-line README, and watch a sub-ten-second asciinema/gif of the shim wrapping a slow statusLine — with the repo's `CONTRIBUTING.md` already routing the issue traffic the launch will produce.

**Depends on**: Phase 1 (binary must compile and run end-to-end). Independent of Phase 3 — README and asciinema work, dist scaffold, and Homebrew tap can proceed in parallel with install-hooks development.

**Requirements**: DIST-01, DIST-02, DIST-03, DIST-04, DIST-05, DIST-06, DIST-07

**Success Criteria** (what must be TRUE):
  1. After tagging a `v0.1.0-rc.1` release on the repo, GitHub Actions (driven by `dist init`-generated workflow) successfully publishes a release with prebuilt artifacts for `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, AND `x86_64-pc-windows-msvc` (Windows allowed to fail the matrix without blocking the others, per the best-effort tier); each artifact is downloadable and `holt --version` prints the tagged version.
  2. On a fresh macOS arm64 machine, `brew install <user>/tap/holt` completes without Gatekeeper friction (no quarantine attribute on the installed binary; `holt --version` runs from a terminal-launched shell without prompting); on a Linux x86_64 machine, `cargo binstall holt` resolves to the dist-published binary (NOT cargo-build) and `holt --version` succeeds in <10s end-to-end.
  3. The repo's `README.md` opens with an asciinema or animated GIF embedded above the fold that shows holt wrapping a slow user statusLine with the LKG cache kicking in, AND the install instructions appear within the first 30 lines, AND the v0.1 platform tier statement (Unix-tier-1 / Windows best-effort) appears with a link to the trigger criteria for promoting Windows (≥10 Windows-tagged issues OR a Windows contributor steps up).
  4. `Cargo.toml` declares `rust-version = "1.87"` and `edition = "2024"` AND `cargo build --workspace` succeeds on a stock Rust 1.87 toolchain (asserted in CI via a 1.87 matrix entry); `cargo binstall` metadata is present (`pkg-url`, `pkg-fmt`) such that `cargo binstall holt --dry-run` resolves a download URL without errors.
  5. The GitHub repo has issue labels `bug`, `feature`, `windows`, `pet`, `runtime`, `orchestrator`, `good first issue`, and `help wanted` configured (verified via `gh label list --json name`) AND `CONTRIBUTING.md` is committed at the repo root (no rewrite for v0.1 — already present per project setup) AND the README's bug-report section links the labels by name.

**Plans**: TBD
**UI hint**: yes

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Schema + supervisor substrate | 3/3 | Complete ✓ | 2026-04-28 |
| 2. Heartbeat hook (write side) | 2/2 | Complete ✓ | 2026-04-28 |
| 3. install-hooks UX | 0/? | Not started | - |
| 4. Distribution + launch | 0/? | Not started | - |

## Parallelization

- **Phase 1** must complete first (keystone crate `holt-schemas` blocks everything; `holt-supervisor` blocks Phase 4's binary).
- **Phase 2** depends on Phase 1's `holt-schemas` API.
- **Phase 3** depends on Phase 2 (needs the canonical hook command-line that gets merged into settings.json) but is otherwise independent of Phase 4.
- **Phase 4** depends on Phase 1 only (it ships the binary; install-hooks is a subcommand of that binary, but the dist scaffold + Homebrew tap + README + asciinema work do not require install-hooks to be complete).

**Recommended execution waves:**
1. Phase 1 (sequential — keystone)
2. Phase 2 (sequential after Phase 1)
3. Phase 3 + Phase 4 (parallel — independent of each other once Phase 2 lands)

## Deviations from suggested split

The research summary suggested 3 phases (Schema+Supervisor / Hooks+install-hooks / Distribution). This roadmap splits hooks-write from install-hooks because:

1. STACK.md and SUMMARY.md flagged install-hooks as "its own ~weekend of work" requiring a JSONC strategy spike (Open Question #1) — different blocking pitfalls (H1 settings.json race, H12 concurrent install, C3 lock + fsync, C4 JSONC-only-in-CLI) from the heartbeat write side (H2 ext4 atomic-rename, H4 XDG fallback, H5 stdin drift).
2. install-hooks targets a different filesystem surface (`~/.claude/settings.json`, user-edited, JSONC-tolerant) than the heartbeat write (`$XDG_RUNTIME_DIR`, holt-only, strict JSON).
3. Splitting them lets Phase 3 and Phase 4 run in parallel waves — install-hooks UX work and distribution polish/README work proceed simultaneously once the binary substrate ships.
4. HOOK-11 (non-panicking reader contract) is placed in Phase 1 (not Phase 2 with the other HOOK reqs) because it's a property of the `holt-schemas` API surface and must be in place before any heartbeat reader code (including Phase 2's round-trip tests) is written. C5 enforced from the keystone outward.

The 4-phase split fits the coarse granularity (3–5 phases) and respects the C2 crate DAG (`holt-render` is not built at v0.1; it's a passthrough no-op in `holt-cli`).

---

*Roadmap created: 2026-04-28*
