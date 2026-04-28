# Multi-Session Orchestrator — Synthesis

**Date:** 2026-04-28
**Project name:** **holt** · pet name: **Nak**
**Wedge:** v1.0 = "one bar across all your CC sessions on the same machine"
**Privacy posture:** No telemetry. Trigger gating runs through GitHub issues only.
**Method:** 3 parallel research agents (workflow audit / cache schema reverse-engineering / adjacent prior art)
**Companion docs:** `../20260427-statusline-tool-research/FINDINGS.md`, `../20260427-statusline-mvp-scope/SCOPE.md`

---

## TL;DR — three locked decisions and one reframe

The reframe: **the orchestrator is mostly a READER, not a writer.** Claude Code already writes the per-session transcript that contains 90% of what we need (current tool, mode, last activity, files touched, model, tokens). The shim only needs to write a tiny heartbeat. That makes v1.0 *smaller* than v0.5, not bigger — the doctor + shim infrastructure already gives us the cache layer; v1.0 is mostly a renderer + cross-session reader on top.

Three decisions to lock now (each backed by triangulating evidence across the 3 reports):

1. **Architecture: hooks-write + per-session file + statusline-reads (no daemon at MVP).** Rejecting the gitstatusd-style daemon as over-engineering for our shape (≤8 sessions, fires on turn boundaries not per-keystroke). Each CC session's hooks write a tiny heartbeat JSON to `$XDG_RUNTIME_DIR/cc-status/sessions/<sid>.json` (single writer per file = no race), and every session's bar reads all files on each fire. Daemon becomes a 1.x optimization if read-fanout becomes the bottleneck.

2. **Headline signal: the attention queue.** "N sessions waiting for you" (where waiting = needs permission, in plan mode pending acceptance, blocked on user input, or completed and idle). This directly answers the most-cited pain quote across the audit ("I tried running 2 and I can't keep up… the other window is done, tested, validated and waiting for me"). Rendering: a count badge plus a rotating "next-to-attend" peer detail. Not equal aggregation — k9s, gh run watch, and claude-squad all bail on equal rendering because terminal width forces a choice.

3. **Worktree as the unit, not abstract "session."** Per-worktree CC is the dominant pattern (incident.io, Morph, every blog post). The orchestrator should group/label peers by `cwd` → worktree label, not by session UUID. Users think in worktrees; bar should match.

---

## 1. Workflow audit — what people actually do

| Finding | Source |
|---------|--------|
| **4-8 concurrent sessions is the realistic ceiling** for power users (incident.io: "four or five", one engineer at "seven"; Morph: "4-6 effectively before attention bottleneck") | [incident.io](https://incident.io/blog/shipping-faster-with-claude-code-and-git-worktrees), [Morph](https://www.morphllm.com/run-claude-code-parallel) |
| **Per-worktree** is the dominant pattern (`claude -w`, custom worktree managers, every guide leads with this) | [incident.io](https://incident.io/blog/shipping-faster-with-claude-code-and-git-worktrees), [Dan Does Code](https://www.dandoescode.com/blog/parallel-vibe-coding-with-git-worktrees) |
| **Per-project tmux/iTerm panes** is second (the andynu "100+ projects" gist) | [andynu gist](https://gist.github.com/andynu/13e362f7a5e69a9f083e7bca9f83f60a) |
| **Pair-with-self / rhythm pattern** — "give one session a long task, switch to the other" — explicitly endorsed by Anthropic-aligned guides | [MindStudio](https://www.mindstudio.ai/blog/claude-code-parallel-sessions) |
| **Compute-bound parallelism** (run N sessions to dodge rate limits) is rare and called out as side-effect, not goal | [HN thread](https://news.ycombinator.com/item?id=46902368) |

### The pain points (verbatim quotes)

- **Reviewer bottleneck (#1):** *"I tried running 2 and I can't keep up, I'm defining specs and the other window is done, tested, validated and waiting for me."* — [HN](https://news.ycombinator.com/item?id=46902368)
- **Cost surprise:** *"each agent run against a real codebase probably spends 20-50k tokens just on context… hits millions of tokens a day before any actual work happens."* — [HN](https://news.ycombinator.com/item?id=47629485)
- **Lock contention:** *"gave up on worktrees and hacked together a solution with fine-grained lockfiles for editing, running builds, etc."* — same HN thread
- **Cross-session interference (real bug):** [#35741](https://github.com/anthropics/claude-code/issues/35741) — interrupts firing across all parallel terminals
- **Wedged sessions:** [#26699](https://github.com/anthropics/claude-code/issues/26699) — stuck on rate-limit indefinitely

### Top 3 signals to surface (evidence-backed)

1. **Attention queue** — "N sessions waiting for you" (the killer signal)
2. **Wedged/stalled** — sessions hung on rate-limit or interrupt loops (silent failures users currently discover by accident)
3. **Aggregate burn this hour** — glanceable spend meter, not post-hoc invoice

---

## 2. Cache schema — what CC already gives us for free

### What's already on disk (no shim needed)

CC writes append-only JSONL at `~/.claude/projects/{project_hash}/{session_uuid}.jsonl`. Eight observed line types; the four useful ones for the orchestrator:

| Line type | Useful field | Tail-extraction |
|-----------|--------------|-----------------|
| `assistant` | `message.model`, `message.usage.input_tokens`, `message.content[].tool_use` | last assistant line gives current model + last tool in flight |
| `user` | `message.content[].tool_result.is_error` | last tool-result line gives last success/fail |
| `permission-mode` | `permissionMode` ∈ `default`/`plan`/`acceptEdits`/`bypassPermissions` | **this IS the plan↔execute mode signal** — don't reinvent |
| `system` | `subtype: "stop_hook_summary"`, `turn_duration` | turn-end signal + duration |

**Tail-only is safe** because the file is append-only and line-delimited. Bounded reverse-tail of ~200 lines per session covers everything we need.

### Hooks ARE the cross-session bus

Per-event hooks: `PreToolUse`, `PostToolUse`, `SessionStart`, `SessionEnd`, `Stop`, `SubagentStop`, `UserPromptSubmit`, `Notification`, `PermissionRequest`.

Common envelope: `{ session_id, transcript_path, cwd, hook_event_name, ...event_specific }`.

A 20-line hook that appends `{ts, session_id, event, ...}` to a shared JSONL gives us a real-time cross-session firehose without patching CC.

### What CC doesn't write (the shim must add — minimal list)

1. **Heartbeat JSON** — `$XDG_RUNTIME_DIR/cc-status/sessions/<sid>.json` updated on every PreToolUse / PostToolUse / Stop / Notification / SessionStart hook. Schema: `{pid, cwd_label (worktree/repo), last_tool, mode, blocked_on, mtime, started}`. Single writer per file → no lock needed.
2. **PID ↔ session map** — written once at SessionStart (CC has no pidfile of its own; `~/.claude/ide/{pid}.lock` only exists when an IDE attaches).
3. **Per-fire timing** — already in v0.1 shim from prior scope.
4. **Idle/working derivation** — derived from `last permission-mode` + age of last `assistant` line; not a new field, just derived state.

That's it. Four small additions. Everything else reconstructs from CC's own state.

---

## 3. Prior art — where the design tensions resolve

### Files-on-disk vs daemon (the disagreement between agents)

The cache-schema agent says hooks-write-to-files; the prior-art agent says local daemon over Unix socket (gitstatusd template). Both are evidence-backed; the resolution is **shape-of-load**:

- **gitstatusd needs a daemon** because shells fire prompts on every keystroke and git status is expensive. In-memory cache amortizes filesystem cost across thousands of fires/sec.
- **Our orchestrator does NOT** because (a) statusLine fires on turn boundaries (300ms debounce), not per-keystroke; (b) sessions cap at ~8; (c) heartbeat reads are stat() + small JSON read, not a shell-out fork chain.

**Lock decision:** Files-on-disk for MVP and v1.0. Each session's hook is the writer (single-writer-per-file). Statusline binary is the reader (per-fire fanout across N≤8 files = ~5ms total). Daemon becomes a 1.x optimization gated on real measured fanout cost.

The prior-art agent's `$XDG_RUNTIME_DIR` location is correct: it's per-user, NOT cloud-synced (avoids the iCloud/OneDrive SQLite-corruption class), and tmpfs on Linux. **Crucially: don't use `~/.claude/orchestrator/` because users sync that.** Use `$XDG_RUNTIME_DIR` (Linux) / `$TMPDIR/cc-status-<uid>/` (macOS — there's no XDG_RUNTIME_DIR by default).

### Render: rotation, not equal aggregation

k9s, `gh run watch`, claude-squad all converge on the same conclusion: **don't render N backends equally** in a constrained surface — switch context, rotate, or compact. For a single statusLine row (~80 chars), the answer is rotation:

- **Aggregate counter** is constant: `[3/7 waiting]`
- **Detail slot** rotates per fire: shows the *most-attention-needing* peer (waiting longest first, then wedged, then completed-and-idle, then most-recently-active)
- **Tail indicator** for compactness: `+N more`
- **Opt-in `cc-status peers` TUI** for the full grid view (a 1.x command, not statusline)

### Dead-session detection

Heartbeat + TTL beats lockfiles (Baeldung / PostgresApp #573 literature). Each session's heartbeat file's `mtime` is the liveness signal; reader treats `now - mtime > 2 × refreshInterval` as stale. PID + start-time guards against PID reuse. `kill -0 {pid}` as the floor check.

### What's confirmed open

Every existing CC orchestrator (claude-squad, crystal/Nimbalyst, vibe-kanban, agent-of-empires, Codeman, tmux-claude-mcp-server) is a *separate app you context-switch into*. **None surface peer-session state inside the CC statusLine itself.** vibe-kanban is sunsetting — users didn't want another tab. The wedge is genuinely uncontested at the statusLine surface.

The closest prior art is **ccmanager** ([repo](https://github.com/kbwo/ccmanager)) which already tracks Waiting/Busy/Idle states and exposes `[active/busy/waiting]` counts per project — but it's a TUI you `cd` into, not a bar in every session. It's a reference floor for the state model.

---

## 4. The reframed v0.1 → v0.5 → v1.0 path

The earlier SCOPE.md sketched MVP (shim+doctor) → 1.0 (rendering + AI-native segments). The orchestrator wedge collapses some of that:

| Version | Scope | Pitch | Persistent value |
|---------|-------|-------|------------------|
| **v0.1** | Shim wraps user's existing `statusLine.command` + per-fire timing + last-known-good cache + hook-driven heartbeat write | "Wrap your existing config; never feel statusLine lag again." | Insurance + heartbeat-as-byproduct |
| **v0.5** | + `cc-status doctor` (active load-test profiler) | "...and find out why your bar is slow." | Diagnostic |
| **v1.0** | + cross-session reader + attention-queue render + worktree labels + rotating peer detail | "And see what your other 6 CC sessions are doing without leaving this one." | **Daily attention earned: every glance tells you which peer needs you next** |

The shim's hook-writes (already MVP) become the cross-session bus (1.0 reader). Same architecture, more value at each version. **No pivot — pure additive.**

### Open scope decisions for v1.0

- **Cost rollup at 1.0 or 1.x?** — strong evidence for the pain (HN cost-surprise quote), but cost telemetry shape is shifting under us (CC v2.1.80 added `rate_limits` JSON; more coming). I'd lean *yes at 1.0* — it's a 1-day extension once heartbeat schema includes per-session token deltas — but accept the schema may need a v1.1 break.
- **`cc-status peers` TUI command?** — a separate `peers` subcommand that shows the full grid (4-8 sessions × N attributes). Probably yes at 1.0 — it's the natural counterpart to the rotating bar, costs ~1 weekend, and lets users drill in when the bar isn't enough.
- **Cross-machine sync?** — explicitly deferred to a 1.x `--sync` flag with explicit user opt-in. Same-machine first.
- **Privacy / multi-user?** — explicitly never. Hard out.

---

## 5. Sequencing insight (the second non-obvious bit)

The earlier insight: MVP doesn't render its own statusLine. The orchestrator-wedge addition: **the writer is a hook, not the binary itself.** This matters because:

1. **The binary stays cold-start sensitive.** Hooks fire from CC's process; they can take their time (50-100ms is fine, hooks aren't on the render path). The bar's read pass stays sub-20ms.
2. **The binary stays installable without setup.** Users who want orchestration install the hook (one curl-pipe-bash, or a `cc-status install-hooks` command). Users who only want the shim/doctor never touch hooks. The orchestrator is *opt-in via hook installation*, not a binary feature flag.
3. **CC schema bumps are absorbed by the hook, not the bar.** When CC adds new event types or stdin fields, you update the hook (which is a tiny script), not the binary. The bar reads a fixed heartbeat schema you control.

That separation — hook writes, binary reads — is the architectural seam that makes the project survive CC evolution. It's also the natural Unix-ism: many small writers, one fast reader.

---

## 6. Open questions for the next conversation

- **Hook installation UX** — `cc-status install-hooks` writes to `~/.claude/settings.json::hooks`. That mutates user settings; do we ship a YAML/JSON merge tool, or print instructions and let user paste?
- **Heartbeat schema lock-in** — what fields go in v1 of the heartbeat? Locking too early forces v2 schema bump; locking too late slows MVP.
- **Worktree label derivation** — how do we render `cwd` cleanly? `~/p/auth/feature-x` vs `auth:feature-x` vs `[feature-x]`? The fewer chars the more peers fit.
- **CC v2.1.119 just added `effort.level` + `thinking.enabled` + `PostToolUse.duration_ms` to stdin JSON.** That covers some of what we'd otherwise read from transcript-tail. Worth re-checking the exact statusLine JSON shape before locking heartbeat schema.
- **ccmanager outreach** — closest existing analog. Worth a friendly message before we ship something that overlaps?

---

**Source artifacts:** all 3 raw agent reports preserved in conversation transcript. URLs cited inline. Field-level claims about CC's on-disk state verified against `/Users/thanats/.claude/` this session by the cache-schema agent.
