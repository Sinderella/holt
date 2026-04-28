# Feature Research — holt

**Domain:** Rust statusLine for Claude Code with multi-session orchestration and an ASCII otter pet (Nak)
**Researched:** 2026-04-28
**Confidence:** HIGH (verifies against [CC changelog](https://code.claude.com/docs/en/changelog) and ecosystem repo state as of this date)

## Relationship to docs/

This file does NOT redo the wishlist ranking (`docs/01-findings.md` §3 owns that), the Nak state vocabulary (`docs/04-pet.md` §3 owns that), or the wedge thesis (`docs/01-findings.md` §5 owns that). It augments `docs/02-scope.md` §2 (MVP IN) and §3 (deferred-with-triggers) by:

1. **Re-categorizing the v0.1 IN list** as table stakes vs differentiators (the IN/OUT split doesn't carry that distinction)
2. **Verifying the ~17 deferred-with-triggers entries** against the CC changelog through 2026-04-28 (versions 2.1.83 through 2.1.120) and against ecosystem releases since the docs were locked
3. **Surfacing two NEW features** that should be on the radar but weren't in the design docs (issue #48040 sub-agent cost; `workspace.git_worktree` adoption)
4. **Calling out two trigger-fired items** where the underlying CC capability shipped, moving them off "deferred" status

The IN list itself (`docs/02-scope.md` §2.IN) is treated as locked. The categories below describe *how to think about it for roadmap purposes*, not what to add or remove.

---

## Feature Landscape

### Table Stakes (Users Will Uninstall If Missing)

These are the v0.1 floor. A statusLine-wrapper that lacks any of these is broken on arrival — every existing tool ships them or it doesn't get adopted.

| Feature | Why Expected | Complexity | Source |
|---------|--------------|------------|--------|
| Wraps existing `statusLine.command` without modification | Five competing tools already render widgets (ccstatusline, CCometixLine, ccburn, usage-bar, ccusage); new tool must compose, not replace | MEDIUM | `docs/02-scope.md` §2.IN #1 |
| Sub-20ms cold start on Linux/macOS | Anything slower defeats the wedge — wrapping a slow script with a slow shim is worse than the original | MEDIUM | `docs/02-scope.md` MVP success criteria |
| Last-known-good TTL cache (render previous output instantly) | p10k pattern; CC fires on turn boundaries so cache is cheap; without it the bar is no better than the wrapped script | MEDIUM | `docs/01-findings.md` §5; `docs/02-scope.md` §2.IN #3 |
| Configurable timeout + clean process-group kill | The actual wedge — `std::process::Child::kill` doesn't kill descendants ([rust-lang #115241](https://github.com/rust-lang/rust/issues/115241)); without it, orphans pile up | HIGH | `docs/02-scope.md` §2.IN #4 |
| Per-fire timing log to disk | Universal gap (0/5 tools have it); without it, "find slow segment" is impossible — kills the doctor demo | LOW | `docs/02-scope.md` §1 gap analysis; `docs/02-scope.md` §2.IN #2 |
| Heartbeat hook installation that doesn't break user's `~/.claude/settings.json` | `holt install-hooks` mutates user-config; if it corrupts settings.json, project is dead at first install | MEDIUM | `docs/02-scope.md` §2.IN; `PROJECT.md` v0.1 |
| cargo-dist binaries for Linux x64, macOS x64+arm64 on day one | "Install via Homebrew or `cargo binstall`" is the floor — `cargo install --git` is a non-starter for non-Rust users | LOW | `docs/02-scope.md` MVP success criteria |
| Survives CC v2.1.119-style statusLine regression on Windows | Issue [#52997](https://github.com/anthropics/claude-code/issues/52997) shows CC's own statusLine integration breaks on patch releases; holt must degrade gracefully when stdin JSON is malformed | MEDIUM | New since 2026-04-28 — see "New since 04-28" below |

### Differentiators (Why Someone Picks holt over ccstatusline + ccburn)

These are the wedge. None of the five audited tools ship any of them.

| Feature | Value Proposition | Complexity | Source |
|---------|-------------------|------------|--------|
| Breach log with full context (env, stdin, stderr) on threshold cross | Bug-report bundle that doesn't exist in the field; turns "my bar is slow" into a one-paste reproducer | LOW | `docs/02-scope.md` §2.IN #6 |
| `holt doctor` — script-under-load profiler with ranked culprit table | The headline demo. Starship's `timings` is single-fire and passive; holt is *active* diagnosis | HIGH | `docs/02-scope.md` §2.IN #5 (v0.5) |
| Cross-session attention queue rendered in every session's bar | Genuinely uncontested: every existing CC orchestrator (claude-squad, vibe-kanban, ccmanager) is a separate app you context-switch into | MEDIUM | `docs/03-orchestrator.md` §TL;DR |
| Worktree-as-unit labeling derived from `cwd` | Per-worktree CC is the dominant pattern (incident.io, Morph); orchestrator groups by `cwd` → label, not by session UUID | LOW | `docs/03-orchestrator.md` §TL;DR #3 |
| Aggregate burn-this-hour rollup across all live sessions | Real pain (HN cost-surprise quote); heartbeat already carries per-session token deltas, aggregation is a tiny extension | LOW | `docs/03-orchestrator.md` §4 open scope; `PROJECT.md` v1.0 |
| Nak — the otter as primary UI (posture = state, dots = peers, rotating peer-pet = attention queue) | The integration insight: pet collapses two features into one coherent identity. No CC tool ships a heartbeat-reactive ASCII pet at all | MEDIUM | `docs/04-pet.md` §TL;DR |
| Cross-pet friendship-by-frequency (Tamagotchi Connection mechanic) | Live state + historical relationship — no other tool has the continuity layer; this is the differentiator over the orchestrator alone | MEDIUM | `docs/04-pet.md` §4 |
| Markdown pet diary with rename history | Replika lesson — callback continuity earns long-term retention; opens in any markdown viewer, no proprietary format | LOW | `docs/04-pet.md` §5.2 |
| Buffer-math-corrected autocompact countdown | Pain #4 universal: Anthropic's own JSON was wrong for months, anyone displaying official context % is lying to users | LOW | `docs/01-findings.md` §3 #1 |
| Plan-mode vs execute-mode color flip | Full-bar mode signal, zero clutter; `permission-mode` already exposed | LOW | `docs/01-findings.md` §3 #6; `docs/03-orchestrator.md` §2 |

### Anti-Features (Hard Rejects)

These have been considered and explicitly rejected. Re-proposals require a documented trigger.

| Anti-Feature | Why Requested | Why Rejected | Alternative |
|--------------|---------------|--------------|-------------|
| Push notifications about pet/session state | Surface appeal: "tell me when the build finishes!" | Notification fatigue is the #1 cited mascot-app uninstall trigger ([MagicBell 2026](https://www.magicbell.com/blog/alert-fatigue): 64% delete after ≥5/week) | Pet posture changes on next glance; user pulls signal, never gets pushed to | `docs/02-scope.md` Out; `docs/04-pet.md` §2 #4 |
| Speech bubbles / pet "talking" | Personality through dialogue feels alive | Clippy's autopsy. Pet exists; it doesn't narrate. Bash already announces what's running | Posture + glyph carry the meaning at 5 cells | `docs/04-pet.md` §2 #2, §4 |
| Tick-driven animation | "It looks dead during slow turns" | Hard architectural constraint — a still pet during expected activity *is* the wedged-session signal. Absence of motion carries information | Animate only on CC events (PreToolUse / PostToolUse / Stop / Notification) | `docs/04-pet.md` §3 animation rule; `CONTRIBUTING.md` |
| Plugin / extension runtime at v1.0 | Starship/oh-my-zsh growth pattern | Premature config language constrains evolution before there are users; sandbox runtime is a separate product | Curated presets via PR; plugin runtime gated on ≥30 community presets requesting unmappable capabilities | `docs/02-scope.md` §3 post-1.0 |
| Telemetry / analytics | "How do you know what to build?" | User runs holt entirely on their own machine; trust is the product | Trigger-gating runs through GitHub issue counts only — slower but honest | `PROJECT.md` Constraints; `CONTRIBUTING.md` |
| Cross-machine pet sync at v1.0 | "I want my Nak on my laptop AND desktop" | Same-machine first; sync introduces conflict resolution and identity merge problems | Deferred to 1.x behind explicit `--sync` opt-in | `docs/02-scope.md` Out; `docs/04-pet.md` §6 deferred |
| Separate dashboard / TUI app | "I want a grid view of all sessions" | holt is a statusLine. Every CC orchestrator that is a separate app (claude-squad, vibe-kanban) failed for the same reason — context switch breaks the flow | `holt peers` is a sub-command, not a separate product surface | `CONTRIBUTING.md` |
| Pokemon / copyrighted art | "Just use a Charmander sprite" | krabby (228★, Pokemon art) is one C&D away from ruin; enterprise-installable means original | Original ASCII otter (Nak); themes via PR-only | `PROJECT.md` Out; `docs/04-pet.md` §1 |
| Render holt's own segments at v0.1 | "Why wrap when you could replace?" | Crowded category; ccstatusline / CCometixLine / ccburn / usage-bar all compete on widgets. Wedge is runtime hygiene, not widgets | Native rendering is a v0.5+ trigger when ≥50% of users want to drop the wrap | `docs/02-scope.md` §1 |
| TOML config language at v0.1 | "Starship has TOML" | Starship's TOML landed at v0.13, four months in; premature config language invites bikeshedding before there are users | Sane defaults + env vars + CLI flags until users complain about a specific default | `docs/02-scope.md` Out |
| OSC8 deep-links + Nerd Fonts at v0.1 | "Make it pretty" | Old-terminal corruption risk ([OSC8-Adoption](https://github.com/Alhadis/OSC8-Adoption/)) and width-math tar pit ([starship #7303](https://github.com/starship/starship/issues/7303)) | ASCII default; opt-in once a width-detection probe ships at 1.x | `docs/02-scope.md` Out; `docs/04-pet.md` §3 width discipline |
| User-behavior pet decay across days | "Tamagotchi mechanic for engagement" | Original Tamagotchi-style decay is the Clippy emotional-manipulation pattern. Caring about Nak ≠ caring about Nak's hunger; caring about Nak = caring about session hygiene | Pet state reflects observable session state only (context %, burn rate, stall) | `docs/04-pet.md` §5.3 reframe; `CONTRIBUTING.md` north star #2 |

---

## Triggers fired since 2026-04-28

Two `docs/02-scope.md` §3 deferred-with-triggers entries have effectively shipped on Anthropic's side and should move into v0.1/v0.5/v1.0 evaluation:

| Deferred entry | Trigger criterion | Status as of 2026-04-28 | Recommended action |
|----------------|-------------------|-------------------------|--------------------|
| **Plan-mode vs execute-mode color flip** | "Once stdin JSON exposes mode reliably (already does in 2.1.119)" — `docs/02-scope.md` §3 | `permissionMode` confirmed in transcript; `effort.level` and `thinking.enabled` shipped in stdin in v2.1.119 ([changelog](https://code.claude.com/docs/en/changelog)). Trigger fired before doc lock | Move to v1.0 IN (already in `PROJECT.md` v1.0 list — confirm) |
| **Effort/thinking pill (from `effort.level` JSON)** | "Already shippable — CC v2.1.119 added the field" — `docs/02-scope.md` §3 | Shipping. xhigh added v2.1.111 (April 16, 2026); both ccstatusline and Daniel3303's tool [issue #23](https://github.com/daniel3303/ClaudeCodeStatusLine/issues/23) flag rendering xhigh as a known gap | Move to v1.0 IN — but only if/when holt renders its own segments. Until then, defer with the rest of native rendering |
| **Stuck-loop detector (built on `PostToolUse.duration_ms`)** | `docs/02-scope.md` §3: "Built on `PostToolUse.duration_ms` (shipped v2.1.119)" | Confirmed shipped v2.1.119 (April 23, 2026). Heartbeat hook can record `duration_ms` directly from `PostToolUse` event payload | Move to v1.0 (Nak's wedged-state already depends on this signal — `docs/04-pet.md` §3 state #5) |

The remaining ~14 deferred-with-triggers items in `docs/02-scope.md` §3 stay deferred — none of their triggers (issue counts, telemetry signals, schema stabilization periods) have fired in the 0 days since doc lock. Re-evaluate at v0.1 launch.

## New since 2026-04-28

Features that the design docs don't cover but that the CC ecosystem started caring about between doc lock and now (or that doc lock missed):

### 1. Sub-agent token aggregation rollup (signal at v1.0+)

**Source:** [Issue #48040](https://github.com/anthropics/claude-code/issues/48040) (April 14, 2026, closed as duplicate of #43945, OPEN upstream)
**Why:** When users run a multi-agent SDL pipeline (orchestrator + ticket-fetcher + architect + implementer + reviewer + tester + auditor), the statusLine's `cost` and `context_window` only reflect the primary session. Sub-agents spend silently. After a full pipeline run, the bar shows "a small fraction of actual spend."
**holt's angle:** holt's heartbeat-per-session model already solves this — every session writes to `$XDG_RUNTIME_DIR/holt/sessions/<sid>.json`, and the cross-session reader can sum across them. The CC issue is asking for a feature holt's architecture inherently provides. Recommend: aggregate burn rollup at v1.0 should explicitly include sub-agent sessions (not just top-level CC sessions), and the README should call this out as a free win from the architecture.
**Roadmap implication:** Adds half a paragraph to the v1.0 pitch; no new code beyond what's already scoped.

### 2. `workspace.git_worktree` — adopt the CC-native field (v0.1 quality-of-life)

**Source:** [Changelog v2.1.98](https://code.claude.com/docs/en/changelog) (April 9, 2026): "Added `workspace.git_worktree` to the status line JSON input, set whenever the current directory is inside a linked git worktree"
**Why:** `docs/02-scope.md` and `PROJECT.md` worktree-label derivation comes from `cwd` parsing. CC now exposes a first-class field. Reading it (a) is more robust than parsing `cwd`, (b) automatically tracks `git worktree add`/`remove`, (c) survives non-standard worktree layouts.
**holt's angle:** Trivial heartbeat schema addition at v0.1: read `workspace.git_worktree` from stdin if present, fall back to `cwd`-parse otherwise. Forward-compatible; degrades to today's behavior on older CC.
**Roadmap implication:** One-line change to v0.1 heartbeat schema; mention in `docs/05-schemas.md` when it's written.

### 3. `PreCompact` hook — wire pet "exhausted → groggy" transition (v1.0)

**Source:** [Changelog v2.1.105](https://code.claude.com/docs/en/changelog) (April 13, 2026): "Added PreCompact hook support: hooks can now block compaction by exiting with code 2 or returning `{decision:block}`"
**Why:** `docs/04-pet.md` §3 has Nak go from "exhausted" (#9, 90%+ context) to "groggy" (just-woke after compact, +). Today the only way to detect compact is to observe context % drop between fires. With PreCompact + (existing) Stop hooks, holt knows *exactly* when the compact starts and ends — pet animation can be precise instead of inferred.
**holt's angle:** Heartbeat hook should subscribe to PreCompact in addition to PreToolUse / PostToolUse / Stop / Notification / SessionStart. Adds one event to the writer. No schema bump (the heartbeat already carries `last_event`).
**Roadmap implication:** Add to v1.0 hook subscription list; minor — call out in `docs/05-schemas.md`.

### 4. Defensive read of stdin JSON (v0.1 hardening)

**Source:** [Issue #52997](https://github.com/anthropics/claude-code/issues/52997) — statusLine regression in v2.1.119 on Windows; `effort.level` rendering broken in two community tools. Plus `/clear` dropping session-name (search results above).
**Why:** CC's stdin JSON shape is shifting underneath every statusLine tool. `effort.level` schema for xhigh broke ccstatusline; v2.1.119 broke statusLine execution on Windows. holt's "be a better runtime" wedge requires graceful degradation when stdin is malformed.
**holt's angle:** v0.1 already plans for breach logging; expand to capture *parse failures* on stdin separately. If `serde_json` parse fails, emit "stdin parse fail" + last-known-good-cached output — never bubble the error to the user. This is one of the table-stakes items above; flagging as new because the CC v2.1.119 regression is a concrete recent failure mode.
**Roadmap implication:** v0.1 — small extension to existing breach-log code path; not a new feature, but should be in REQUIREMENTS.md as an explicit acceptance criterion.

### 5. Competitive pressure: ccstatusline shipped Token Speed widgets, Skills widget (during the same week)

**Source:** [ccstatusline v2.2.1+](https://github.com/sirmalloc/ccstatusline) — added Input Speed / Output Speed / Total Speed widgets (configurable 0-120s rolling window), Skills widget with last/count/list modes, Vim Mode widget, Git widget link modes.
**Why:** The crowded-widgets category is getting more crowded. ccstatusline is actively shipping; the wedge-thesis position (we don't compete on widgets, we wrap them) holds *more* not less. But every new widget ccstatusline ships is one users won't get from holt's native rendering when/if it lands.
**holt's angle:** **Reinforces** the v0.1 IN list (wrap, don't compete). When/if native rendering lands at v0.5+, the trigger criterion should explicitly include "is the widget gap so wide that wrapping ccstatusline + holt is a worse experience than just running ccstatusline?" If yes, native rendering is justified. If no — and right now no — keep wrapping.
**Roadmap implication:** No code change. Anti-feature reinforcement: native rendering's trigger criterion should reference ccstatusline's current widget set as the floor.

### 6. `/statusline-setup` skill is now first-party (April 22, 2026)

**Source:** Search results above — Anthropic's own `/statusline-setup` skill installs a shell statusline showing dir + git branch + model + context% + 5h rate-limit.
**Why:** First-party onboarding now exists. Users who would have installed holt to *get a statusline* now have a click-button alternative. holt's pitch must be "I have a statusline already and it's slow / I want multi-session awareness / I want Nak" — never "I need a statusline."
**holt's angle:** Pitch sharpening, not a feature. README's first paragraph should assume the user already has a statusline (whether from `/statusline-setup`, ccstatusline, or hand-rolled). Strengthens the wedge thesis (`docs/01-findings.md` §5).
**Roadmap implication:** Marketing/positioning, not code. Note for the v0.1 README draft.

---

## Feature Dependencies

```
[Heartbeat hook + per-session JSON write]  (v0.1)
          ├──enables──> [Cross-session reader]            (v1.0)
          │                    └──enables──> [Attention queue render]   (v1.0)
          │                    └──enables──> [Aggregate burn rollup]    (v1.0)
          │                    └──enables──> [Sub-agent cost rollup]    (v1.0, NEW)
          │                    └──enables──> [Cross-pet friendship]    (v1.0)
          │
          ├──enables──> [Pet state derivation]            (v1.0)
          │                    └──requires──> [Nak state vocabulary]    (v1.0)
          │                    └──requires──> [PreCompact hook subscribe] (v1.0, NEW)
          │
          └──enables──> [worktree label]                 (v0.1 if cwd-derived; v0.1+ if workspace.git_worktree adopted)

[Per-fire timing log]  (v0.1)
          ├──enables──> [Breach log]                     (v0.1)
          │                    └──enables──> [holt doctor culprit table] (v0.5)
          │                    └──enables──> [holt doctor --share bundle] (v0.5+ trigger)
          └──enables──> [Last-known-good cache]          (v0.1, parallel)

[holt install-hooks]  (v0.1)
          └──enables──> [Heartbeat hook]                 (v0.1; same milestone, just a separate concept)
```

### Dependency Notes

- **Heartbeat hook is the single biggest unlock.** Everything in v1.0 — orchestrator, pet, friendship, sub-agent rollup — reads from the per-session JSON the hook writes. Get the schema right at v0.1 (`docs/05-schemas.md` v1) and the v1.0 work is mostly UI on top of pre-existing data.
- **Per-fire timing log → breach log → doctor is the v0.5 chain.** Each step is additive on the prior; no v0.5 work is wasted if doctor slips, because the timing log + breach log stand alone as v0.1 features.
- **Pet posture depends on derivation rules over heartbeat data.** Adding a pet state (e.g., "groggy") never requires a heartbeat schema bump — only a derivation-rule change. This is what `docs/05-schemas.md` v1 is locking.

---

## MVP Definition

### Launch With (v0.1) — the lovable MVP, 3-4 weekends

The IN list from `docs/02-scope.md` §2, re-categorized:

**Table-stakes core:**
- [ ] Shim binary that wraps existing `statusLine.command`
- [ ] Last-known-good TTL cache
- [ ] Configurable timeout + clean Unix process-group kill
- [ ] Per-fire timing log (`~/.cache/holt/timings.jsonl`)
- [ ] Heartbeat hook + atomic-rename per-session JSON
- [ ] `holt install-hooks` (auto JSON-merge with `--print` + `--dry-run` escape hatches)
- [ ] cargo-dist binaries (Linux x64, macOS x64+arm64, Windows x64 best-effort)

**Differentiator core (rest of v0.1):**
- [ ] Breach log with full context (env, stdin, stderr) on threshold cross
- [ ] README leads with asciinema/gif of the wrap working

**v0.1 hardening (from "New since 04-28"):**
- [ ] Defensive stdin JSON parse — never bubble parse errors to user
- [ ] Read `workspace.git_worktree` if present, fall back to `cwd`-parse

### Add After Validation (v0.5)

- [ ] `holt doctor` — script-under-load profiler with ranked culprit table
- [ ] `holt doctor --share` — redacted bundle (gated on breach-log format stability for ≥2 releases)

### Add at v1.0

The full `PROJECT.md` v1.0 list, plus the three "trigger fired" features that move in:

- [ ] Cross-session reader + attention queue + worktree labels + rotating peer detail
- [ ] `holt peers` TUI sub-command
- [ ] Aggregate burn-this-hour rollup (incl. sub-agent sessions, addressing CC #48040)
- [ ] Nak — 12-state vocabulary (per `docs/04-pet.md` §3) + 5-cell ASCII + heartbeat-driven
- [ ] Pet naming + diary + cross-pet friendship + companion dots
- [ ] Autocompact buffer-math correction
- [ ] Plan-mode vs execute-mode color flip *(trigger fired)*
- [ ] PreCompact hook subscription (powers Nak's groggy-after-compact transition)
- [ ] Stuck-loop detector via `PostToolUse.duration_ms` *(trigger fired)*

### Future Consideration (1.x+)

Per `docs/02-scope.md` §3 deferred-with-triggers — TOML config language, native segments, OSC8, Nerd Fonts, Linux arm64, Windows JobObject, code signing, plugin runtime, OpenTelemetry exporter, theme support, kitty/sixel pet, achievement system, cross-machine sync. All gated on their respective triggers; none have fired since 04-28.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Shim wrap + timing log | HIGH | LOW | P1 (v0.1) |
| Last-known-good cache | HIGH | MEDIUM | P1 (v0.1) |
| Process-group kill on timeout | HIGH | HIGH | P1 (v0.1) |
| Breach log | HIGH | LOW | P1 (v0.1) |
| Heartbeat hook | HIGH | MEDIUM | P1 (v0.1) |
| `holt install-hooks` | HIGH | MEDIUM | P1 (v0.1) |
| cargo-dist + Homebrew + binstall | HIGH | LOW | P1 (v0.1) |
| `workspace.git_worktree` adoption | LOW | LOW | P1 (v0.1) — new |
| Defensive stdin parse | MEDIUM | LOW | P1 (v0.1) — new |
| `holt doctor` ranked culprits | HIGH | HIGH | P2 (v0.5) |
| `holt doctor --share` | MEDIUM | LOW | P2 (v0.5+) |
| Cross-session attention queue | HIGH | MEDIUM | P1 (v1.0) |
| Worktree labels | MEDIUM | LOW | P1 (v1.0) |
| Aggregate burn (incl. sub-agents) | HIGH | LOW | P1 (v1.0) |
| `holt peers` TUI | MEDIUM | MEDIUM | P2 (v1.0) |
| Nak — full vocabulary | HIGH | MEDIUM | P1 (v1.0) |
| Pet diary + naming + friendship | MEDIUM | MEDIUM | P1 (v1.0) |
| Autocompact buffer-math fix | HIGH | LOW | P1 (v1.0) |
| Plan/execute mode flip | MEDIUM | LOW | P1 (v1.0) — trigger fired |
| Stuck-loop detector | HIGH | LOW | P1 (v1.0) — trigger fired |
| PreCompact hook subscribe | LOW | LOW | P1 (v1.0) — new |

**Priority key:** P1 = must-have for the named version; P2 = should-have; P3 = nice-to-have, future.

---

## Competitor Feature Analysis

| Feature | ccstatusline (v2.2.1+) | CCometixLine (~6wk stale) | ccburn | holt approach |
|---------|------------------------|---------------------------|--------|---------------|
| Per-fire timing log | ✗ | ✗ | ✗ | **v0.1** — wedge |
| Last-known-good cache | ✗ | ✗ | ✗ | **v0.1** — wedge |
| Active doctor profiler | ✗ | ✗ | ✗ | **v0.5** — wedge |
| Breach log | ✗ | ✗ | ✗ | **v0.1** — wedge |
| Cross-session attention queue | ✗ | ✗ | ✗ | **v1.0** — wedge |
| Heartbeat-reactive ASCII pet | ✗ | ✗ | ✗ | **v1.0** — wedge |
| Token Speed widgets | ✓ (April 2026) | ✗ | partial | Defer — not the wedge |
| Skills widget | ✓ (v2.2.1) | ✗ | ✗ | Defer — not the wedge |
| Powerline themes | ✓ | partial | ✗ | Reject at v0.1 (Nerd Fonts width tar pit) |
| 5h/7d rate-limit display | ✓ | ✗ ([#109](https://github.com/Haleclipse/CCometixLine/issues/109)) | ✓ (core) | Wraps user's existing — not in scope at v0.1 |
| `effort.level` rendering | partial ([gap on xhigh](https://github.com/daniel3303/ClaudeCodeStatusLine/issues/23)) | ✗ | ✗ | Defer to v1.0 if/when native rendering ships |
| Opus 4.6+ crash fix | ✓ | ✗ ([#118](https://github.com/Haleclipse/CCometixLine/issues/118)) | n/a | Avoid the model-string-coupling pattern that bit CCometixLine |

**Pattern:** holt and ccstatusline are non-overlapping. ccstatusline competes on widgets; holt competes on runtime hygiene. They compose — `statusLine.command = holt wrap "ccstatusline ..."` is the intended use. The README should say so.

---

## Sources

- [Claude Code changelog (current)](https://code.claude.com/docs/en/changelog) — verified entries 2.1.83 through 2.1.120 (March 25 through April 28, 2026)
- [Claude Code statusLine docs](https://code.claude.com/docs/en/statusline) — stdin JSON shape
- [Issue #48040 — sub-agent cost aggregation in statusLine](https://github.com/anthropics/claude-code/issues/48040) (NEW pain since 04-28)
- [Issue #52997 — statusLine v2.1.119 Windows regression](https://github.com/anthropics/claude-code/issues/52997) (NEW failure mode since 04-28)
- [Issue #15677 — sub-agent context sizes in statusline API](https://github.com/anthropics/claude-code/issues/15677)
- [Issue #27916 — display active subagent count in statusline](https://github.com/anthropics/claude-code/issues/27916)
- [ccstatusline](https://github.com/sirmalloc/ccstatusline) — competitor that shipped during the doc-lock period
- [CCometixLine](https://github.com/Haleclipse/CCometixLine) — closest Rust competitor, still stale
- [ccburn](https://github.com/JuanjoFuchs/ccburn) — rate-limit / burn focus
- [Daniel3303 ClaudeCodeStatusLine #23 — xhigh rendering gap](https://github.com/daniel3303/ClaudeCodeStatusLine/issues/23)
- `docs/01-findings.md` (wishlist + wedge thesis)
- `docs/02-scope.md` (MVP IN/OUT, deferred-with-triggers)
- `docs/03-orchestrator.md` (cross-session architecture)
- `docs/04-pet.md` (Nak design and OUT list)
- `PROJECT.md` (synthesized active scope)
- `CONTRIBUTING.md` (north star, in/out scope)

---
*Feature research for holt; researched 2026-04-28*
