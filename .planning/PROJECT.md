# holt

## What This Is

A small Rust statusLine for Claude Code — `holt` — that wraps the user's existing `statusLine.command`, supervises it under load, exposes a `holt doctor` profiler, and surfaces a multi-session attention queue across all of the user's CC sessions on the same machine. The bar's primary UI is **Nak**, a small ASCII otter (นาก) whose posture, body shape, and trailing companion dots encode session state, peer count, and the current attention queue. Built for power users who run 4–8 concurrent CC sessions and notice when their statusLine feels off.

## Core Value

**Make Claude Code's statusLine never silently fail, never block input, and tell the user — through Nak — exactly what each of their sessions is doing.** If everything else fails, the shim must wrap a user's existing statusLine without making it worse, and the doctor must be able to find the slow segment in someone's config in under 30 seconds.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

(None yet — ship to validate)

### Active

<!-- Current scope. Building toward these. v0.1 is the first shippable milestone; v0.5 and v1.0 follow as additive. -->

**v0.1 — Runtime hygiene wedge (the lovable MVP, target 3–4 weekends):**

- [ ] Shim binary that wraps the user's existing `statusLine.command` from `~/.claude/settings.json`
- [ ] Per-fire timing log to `~/.cache/holt/timings.jsonl` (duration, fork count, exit code, stderr capture)
- [ ] Last-known-good TTL cache — render previous output instantly, recompute in background, replace on completion
- [ ] Configurable timeout + clean Unix process-group kill (setpgid + killpg via `process-wrap`)
- [ ] Breach log to `~/.cache/holt/breaches.log` with full context (env, stdin, stderr) when timing exceeds threshold
- [ ] Heartbeat hook (`PreToolUse` / `PostToolUse` / `Stop` / `Notification` / `SessionStart`) writes session JSON to `$XDG_RUNTIME_DIR/holt/sessions/<sid>.json` (Linux) or `$TMPDIR/holt-$UID/sessions/<sid>.json` (macOS), atomic-rename, single-writer-per-file
- [ ] `holt install-hooks` — auto JSON-merge into `~/.claude/settings.json::hooks` with `--print` flag escape hatch and `--dry-run` preview
- [ ] Distribution: cargo-dist (Linux x64, macOS x64+arm64, Windows x64 best-effort) + Homebrew tap + cargo-binstall
- [ ] README leads with an asciinema/gif of the shim wrapping a slow statusLine — sub-20ms cold start on Unix-tier-1

**v0.5 — Diagnostic (additive on the v0.1 substrate):**

- [ ] `holt doctor` — fires the configured script 20× under load, ranks fork count / network calls / FS bytes / p95 latency, prints culprit table
- [ ] `holt doctor --share` — redacted bundle for bug reports (gated on breach-log format stability)

**v1.0 — Multi-session orchestrator + Nak as core feature (additive):**

- [ ] Cross-session reader — every fire reads all heartbeat files (≤8 sessions), treats `mtime > 2 × refreshInterval` as stale
- [ ] Attention-queue render — `[N/M waiting]` aggregate counter with rotating most-attention-needing peer detail
- [ ] Worktree labels — `<repo>/<branch>` (e.g., `auth/feat`) derived from `cwd`, override via `HOLT_LABEL` env var
- [ ] Aggregate burn-this-hour rollup (Top-3 signal #3 from research; heartbeat already carries `burn_rate_usd_per_min`)
- [ ] `holt peers` TUI subcommand — full grid drilldown for sessions × attributes (the bar stays primary; this is opt-in)
- [ ] Nak — 12-state ASCII vocabulary, 5-cell width, heartbeat-driven (never tick-driven), default-on, opt-out via `--no-pet` install flag or `[pet] enabled = false` config
- [ ] Naming on first install (default `Nak`) + `holt pet rename`; rename history append-only, past memories preserve original name
- [ ] Pet diary — `~/.local/share/holt/pet/<name>/diary.md` markdown chronicle, append-only, `holt pet diary` subcommand
- [ ] Pet state JSON — `~/.local/state/holt/pet/<name>.json` per `docs/05-schemas.md` v1 schema, memory cap 200 events, archive at `<name>.archive.jsonl`
- [ ] Cross-pet friendship aggregation — co-alive heartbeats accumulate hours/merges/exhaustions per (cwd_label) peer; thresholds tunable in `~/.config/holt/config.toml`
- [ ] Companion-dot rendering — peer count as trailing dots beside Nak; rotating peer-pet detail in attention slot
- [ ] Autocompact buffer-math correction (1-day shippable; corrects pain #4 from `docs/01-findings.md`)
- [ ] Plan-mode vs execute-mode color flip (CC stdin already exposes `permission-mode`)

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- **Push notifications about pet or session state** — Notification fatigue is the #1 cited mascot-app uninstall trigger ([MagicBell](https://www.magicbell.com/blog/alert-fatigue): 64% delete after ≥5/week). holt never pushes.
- **Speech bubbles / pet talking** — Clippy's autopsy. The pet exists; it doesn't narrate. (`docs/04-pet.md` §2)
- **Tick-driven animation** — Hard architectural constraint. Pet animates only on CC events. A still pet during expected activity *is* the wedged-session signal.
- **Plugin / extension runtime** — Curated presets ship; sandbox plugin runtime is post-1.0 and only behind explicit triggers (≥30 community presets requesting capabilities `[custom.*]` blocks can't express).
- **Telemetry / analytics** — Runs entirely on user's machine. Trigger-gating runs through GitHub issue counts only.
- **Cross-machine pet sync (v0.1–v1.0)** — Same-machine first; sync is a 1.x stretch behind explicit `--sync` opt-in.
- **A separate dashboard / TUI app** — holt is a statusLine. `holt peers` is a sub-command, not a separate product surface.
- **Pokemon / copyrighted art** — Original ASCII sprites only. (See `krabby` IP risk, `docs/04-pet.md` §1.)
- **Render holt's own segments at v0.1** — Crowded category (CCometixLine, ccstatusline, ccburn, usage-bar all compete here). Ship as substrate that wraps existing tools, not as competitor. Native rendering is a v0.5+ trigger when telemetry-via-issues shows >50% of users want to drop the wrap.
- **TOML config language at v0.1** — Starship pattern: TOML at v0.13, four months in. Ship sane defaults + env vars + CLI flags until users complain about a specific default.
- **Render-path async runtime (tokio etc.)** — Unjustified for current scope; sync stdlib + threads. Audit deps for transitive tokio pull-through.
- **`simd-json` / `figment`** — Pessimization for small JSON inputs. Stock `serde` + `toml` is sub-millisecond.
- **Linux arm64 binary at v0.1** — Add to cargo-dist matrix at first .1 release.
- **Code signing (macOS notarization, Windows EV cert)** — Gatekeeper warning is acceptable friction at MVP; document the workaround. Trigger-gated to 1.0 promise once friction blocks ≥5 issues.
- **Windows JobObject + ConPTY correctness** — Unix-tier-1 at MVP. Windows runs but documented as best-effort. JobObject + ConPTY is a separate product surface, gated on ≥10 Windows-tagged issues OR a Windows contributor stepping up.
- **OSC8 deep-links + Nerd Fonts at v0.1** — Old-terminal corruption risk and width-math tar pit. ASCII-default; opt-in via flag once a width-detection probe ships at 1.x.
- **Pre-launch outreach to CCometixLine / ccmanager / usik-tamagotchi / siegerts-tama96** — Ship and let issues land; engage post-launch only.

## Context

- **Wedge thesis** (`docs/01-findings.md`): The CC statusLine problem isn't a missing-prompt-tool problem — it's a missing-runtime problem. Anthropic's own runtime fires scripts in a way that makes orphan processes and silent failures the default; the data passed in is partially wrong (token math); there's no observability to catch any of it. holt's gift is being a *better runtime* — process supervision, timing, breach logging, last-known-good caching, doctor diagnostics — with the config language as a downstream nicety.
- **Sequencing reframe** (`docs/02-scope.md`): MVP does NOT render its own statusLine. It wraps the user's existing one. Native rendering doubles the surface area, drags in width-math + Nerd Font + OSC8 issues, and forces competition on widgets where the field is already crowded. Native rendering is a 0.5+ trigger.
- **Architecture reframe** (`docs/03-orchestrator.md`): Files-on-disk + hooks. Hook writes, binary reads. CC already writes 90% of the per-session state in transcripts; holt's hook only adds a tiny heartbeat. Single writer per file → no locking. Daemon is a 1.x optimization gated on measured fanout cost (≤8 sessions × stat() + small JSON read = ~5ms total).
- **Pet integration insight** (`docs/04-pet.md`): The pet is not decoration. The pet IS the orchestrator's UI. Posture encodes session state; companion dots encode peer count; rotating peer-pet detail encodes the attention queue. Cross-pet friendship-by-frequency (Tamagotchi Connection mechanic) is the unique value-add over the orchestrator alone — live state + historical relationship.
- **Schemas locked at v1** (`docs/05-schemas.md`): Heartbeat JSON and pet state JSON both at `schema_version: 1`. Atomic-rename writes; bounded-tail reads. Schema bumps require migration with a one-way `holt migrate-state` subcommand and `<file>.v1-backup` safeguard.
- **Adjacent prior art**: ccstatusline, CCometixLine (closest Rust competitor, 2.8k★, ~6 weeks stale; Opus 4.6 crash unfixed), ccburn, usage-bar, ccusage. Two pet-statusline projects shipped ~4 weeks ago (`usik/tamagotchi`, `siegerts/tama96`) but neither lives in the statusLine itself — both are separate apps you context-switch into. The statusLine surface is uncontested.
- **Anthropic-evolution risk**: If CC ships native `command_timeout` + per-fire timing log, the runtime supervisor downgrades to a generator/diagnostic. The doctor stays valuable; the breach log composes with Anthropic's. Architecture survives by separating hook-writes from binary-reads.

## Constraints

- **Tech stack**: Rust (locked) — single-binary, sync stdlib + threads, no async runtime at v0.1, audit transitive `tokio` pull-through (e.g., `reqwest #1233`).
- **Performance**: Sub-20ms cold start on macOS arm64 / Linux x86_64 (Windows budget separate, ~40ms acceptable with Defender).
- **Platform v0.1**: Linux + macOS only. Windows runs but best-effort. README says so loudly.
- **Distribution**: cargo-dist (Linux x64, macOS x64+arm64, Windows x64) on day one + Homebrew tap auto-generated + cargo-binstall.
- **Privacy**: Zero telemetry. Trigger-gating runs through GitHub issue counts only.
- **License**: MIT. Standard Contributor Covenant.
- **Repo home**: `<user>/holt` under primary GitHub user account (GitHub user `holt` is taken).
- **Architectural North Star** (`CONTRIBUTING.md`, in priority order): (1) Don't make Claude Code lag. (2) Be honest with users — no telemetry, no notifications, no emotional manipulation. (3) Stay small — single-binary, sub-20ms cold start. (4) Survive Anthropic's evolution — degrade gracefully or strengthen as APIs grow. (5) Honor the bond — every pet-touching change asks: does this make a long-time user feel more attached, or less?
- **Width discipline**: ASCII at fixed 5 cells for the pet. Unicode emoji widths break across MS Terminal #8970, VS Code #100730, Alacritty #6144; Nerd Fonts variable per nerd-fonts #1103. ASCII is the only stable substrate at v1.0; kaomoji and Nerd Font are opt-in themes once a width-detection probe ships at 1.x.
- **Pet ethics**: No emotional manipulation. Pet decay reflects observable session state (context %, burn rate, stall) — never user-behavior decay across days. (`docs/04-pet.md` §5.3.)
- **Hard rejects**: Push notifications, speech bubbles, tick animation, Pokemon art, cross-machine sync at v1.0, separate dashboard app, plugin runtime at v1.0.

## Key Decisions

<!-- Decisions that constrain future work. Add throughout project lifecycle. -->

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust as the implementation stack | Single-binary, sub-20ms cold start, mature ecosystem (`process-wrap` for cross-platform process-group kill) | — Pending |
| Wrap existing statusLine instead of rendering own at v0.1 | Crowded native-render category; runtime-hygiene wedge is genuinely empty; smaller MVP surface; composes with rather than competes against ccstatusline et al. | — Pending |
| Files-on-disk + hooks (no daemon) at v1.0 | Statusline fires on turn boundaries (300ms debounce), not per-keystroke; ≤8 sessions; stat() + small JSON read = ~5ms total. Daemon is a 1.x optimization. | — Pending |
| `$XDG_RUNTIME_DIR` for heartbeats (NOT `~/.claude/`) | `~/.claude/` syncs (iCloud/OneDrive SQLite-corruption class). `$XDG_RUNTIME_DIR` is per-user, tmpfs on Linux, never synced. macOS fallback: `$TMPDIR/holt-$UID/`. | — Pending |
| Pet default-on (Nak), opt-out via `--no-pet` | Pet IS the orchestrator's UI; opting in to personality weakens the brand. Asking opt-out is honest and one flag away. | — Pending |
| ASCII 5-cell sprites at v1.0 (no kaomoji/Nerd Fonts default) | Width breakage in MS Terminal, VS Code, Alacritty; Nerd Fonts variable. ASCII is universal. Themes deferred behind 1.x width-detection probe. | — Pending |
| Pet name `Nak` canonical (Octocat/Wumpus pattern) | Naming on first run is highest-leverage anthropomorphism trigger ([Sung Roomba](https://faculty.cc.gatech.edu/~hic/hic-papers/Roomba-Ubicomp.pdf), [Darling](http://www.werobot2015.org/wp-content/uploads/2015/04/Darling_Whos_Johnny_WeRobot_2015.pdf)). User can rename anytime; diary preserves history. | — Pending |
| Pet stakes = observable session state, NOT user-behavior decay | Replaces original Tamagotchi-style "neglect across days" mechanic. Removes Clippy-pattern emotional manipulation. Caring about Nak = caring about session hygiene. | — Pending |
| Schemas locked at `schema_version: 1` | Both heartbeat JSON and pet state JSON. v0.x changes are free; post-v1.0 changes require version bump + `holt migrate-state` migration with `<file>.v1-backup`. | — Pending |
| Worktree label format `<repo>/<branch>` | Default rendering. Override via `HOLT_LABEL` env var. ~10 chars per peer, readable, matches user mental model. | — Pending |
| Aggregate burn-this-hour ships at v1.0 | Heartbeat already carries `burn_rate_usd_per_min`; aggregation is a small extension. Real pain (HN cost-surprise quote). Accept potential v1.1 schema bump if CC widens `rate_limits` shape. | — Pending |
| `holt peers` TUI ships at v1.0 | Natural counterpart to the rotating bar; ~1 weekend; the bar stays primary. Drill-in slot for when 80 chars isn't enough. | — Pending |
| `holt install-hooks` auto-merges settings.json (default) with `--print` escape | JSON-aware merge with `--dry-run` preview and `.bak` backup beats print-and-paste; `--print` covers nervous-installer case. | — Pending |
| MVP target: 3–4 weekends ("lovable MVP" shape) | Matches `docs/02-scope.md` IN list exactly: shim + timing + last-known-good + supervision + breach log + heartbeat hook. Doctor moves to v0.5. | — Pending |
| Quiet rollout via `awesome-claude-code` first; HN Show at v0.5 | Lower polish bar at v0.1; iterate on real users; don't burn launch capital before doctor lands. | — Pending |
| No pre-launch outreach to competitors / adjacent projects | Ship and let issues land. Engage post-launch only. CONTRIBUTING.md tags route reports cleanly. | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-28 after initialization*
