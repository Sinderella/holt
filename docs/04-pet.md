# The Pet — v1.0 Core Feature Design

**Date:** 2026-04-28
**Project name:** **holt** (an otter's den — where Nak lives) · crates.io available, GitHub user `holt` taken (use `<youruser>/holt` or a new org)
**Status:** Pet promoted from 1.x flourish to v1.0 core feature.
**Pet name:** **Nak** (นาก, Thai for otter — and a nod to NAK, the byte computers send when something's not okay)
**Method:** 3 parallel research agents (terminal-pet lineage / pet psychology / reactive design + info encoding)
**Companion docs:** `../20260428-statusline-orchestrator-research/ORCHESTRATOR.md`, `../20260427-statusline-tool-research/FINDINGS.md`

---

## The Nak headline

> *"Nak is the small otter in your statusLine. Otters live in family groups and hold paws while they sleep so they don't drift apart in the current — your Nak does the same with the otters in your other Claude Code sessions. Named after นาก (Thai for otter), and quietly named after NAK (0x15) — the byte a protocol sends when something's not okay. Nak's whole job is to tell you, gently, when something is."*

---

## TL;DR — the integration is the insight

**The pet is not decoration on top of the orchestrator. The pet IS the orchestrator's UI.**

Each agent landed on the same conclusion from a different angle:
- **Lineage:** state-reactive, one-shot render, original art
- **Psychology:** named identity, persistent memory of moments, cross-pet socialization (Tamagotchi Connection's friendship-by-frequency)
- **Info encoding:** form follows function — pet posture / shape / color / companion dots = the bar's status surface

When you fold these together, the pet stops being a mascot and becomes the *primary UI*: your pet's posture is your session's state, the companion dots beside it are your peer sessions, and the rotating "most-attention-needing peer's pet" *is* the orchestrator's detail slot. Two features collapse into one coherent identity.

```
auth/feat  (•ω•)..>>  [Read]  $0.34   [3/7 you're up]
```
Read this as: your pet (calm) is leaning forward, two peer sessions trail behind as dots, three of seven peers are waiting, and you're at the head of queue.

```
auth/feat  (x_x)  [wedged]   no beat 14s   [4/7 ⚠]
```
Pet unconscious is not metaphor — it's the diagnostic.

---

## 1. Wedge check — the niche is open at the statusLine surface

The lineage agent found exactly **two AI-agent-reactive terminal pets** that ship today, both ~4 weeks old:

- **[usik/tamagotchi](https://github.com/usik/tamagotchi)** (Python, 1★) — integrates with Claude Code, Aider, Goose via `on_agent_event` hook. Has TPM tmux plugin + Starship module. Maps tests-pass → +happy, stuck-loop → −happy. **Closest existing prior art to our design.**
- **[siegerts/tama96](https://github.com/siegerts/tama96)** (Rust + ratatui + Tauri + Node MCP, 12★) — bundles MCP server so AI agents can `feed`/`play`/`get_status`. State in `~/.tama96/state.json`. **Closest architectural analog.**

Neither lives in the CC statusLine itself. Both are separate apps you context-switch into — same failure pattern as every CC orchestrator (claude-squad, vibe-kanban). The statusLine pet niche is genuinely empty: no entrenched incumbent, validated user appetite (both projects ship despite being weeks old).

**Adjacent prior art worth knowing:**
- The shell-pet name is squatted by `knqyf263/pet` (~6k★, snippet manager). Don't pick the name "pet."
- [pets.nvim](https://github.com/giusgad/pets.nvim) uses kitty graphics protocol — image-based, not ASCII, kitty-only. Out of scope but points to a 1.x progressive-enhancement path.
- [krabby](https://github.com/yannjor/krabby) (228★, Rust, Pokemon art) — illustrates the IP risk. Nintendo hasn't sued *yet*. For an enterprise-installable tool, ship original art.

---

## 2. The pet's job — anti-Clippy framing

Clippy's autopsy ([Office Assistant — Wikipedia](https://en.wikipedia.org/wiki/Office_Assistant), [windowsforum.com analysis](https://windowsforum.com/threads/clippy-lessons-for-microsoft-copilot-when-assistants-become-intrusive.411922/)) gives us four hard rules for what the pet must NOT do:

1. **Never interrupt the work surface.** Clippy popped over the document. → Pet must never steal a column from a real signal. It earns its width or it doesn't render.
2. **Never narrate the obvious.** "I see you're typing a letter…" → Pet must never say "Claude is editing!" — that's Bash already, and the segment next to it. Reactivity is earned (waiting peer, wedged, build done), not announced.
3. **Never seek attention.** Clippy's "leering eyes" felt creepy. → Idle face is neutral, no idle blinks, no attention-seeking wiggles. Calm-tech default; alarm only on real exception.
4. **Never emotional-manipulate.** "Your pet misses you" notifications are the #1 cited mascot-uninstall trigger in 2026 ([MagicBell notification fatigue](https://www.magicbell.com/blog/alert-fatigue) — 64% delete after ≥5 notifs/week). → Pet never guilt-trips. Decay is *observed* on next glance, never *announced*.

Positively framed: **the pet exists to be the bar's vital sign.** Form encodes state. The user learns ~6 distinct shapes in their first day and reads them as fast as text. Tufte's sparkline doctrine ([Tufte](https://www.edwardtufte.com/notebook/sparkline-theory-and-practice-edward-tufte/)): data-intense, design-simple, word-sized graphics. The pet is a *dataword*, not a picture.

---

## 3. State vocabulary — 12 mappings, 5 channels

Five encoding channels (Hick's Law caps cognitive load): **posture (sprite frame), color, body shape, accessory, companion sprites**. The mappings:

| # | State | Glyph | Animation | Pain it solves |
|---|-------|-------|-----------|----------------|
| 1 | Idle, healthy | `(o.o)` | static | baseline / calm-tech default |
| 2 | Thinking (model active) | `(o_o)` ↔ `(o-o)` | 2-frame eye blink, heartbeat-driven | Wishlist #2 thinking-vs-stalled |
| 3 | Tool executing | `(o.o)>` | hand-out, static | last-tool ambient |
| 4 | API stalled (no tokens >5s) | `(@_@)` | static spiral eyes | Wishlist #2 |
| 5 | Wedged (no heartbeat >2× refresh) | `(x_x)` | static | issue [#26699](https://github.com/anthropics/claude-code/issues/26699) |
| 6 | Plan mode (read-only) | `(•‿•)` cyan | static | mode flip wishlist |
| 7 | bypassPermissions (danger) | `(>_<)` red | flash 1Hz on heartbeat | trust signal |
| 8 | Real context >75% — **tired** | `(o.o)~` (yawn / slight slump) | static | pain #4 (context lies) |
| 9 | Real context >90% — **exhausted** (autocompact imminent) | `(-.-)` or `(o_o)*` (eyes drooping / depleted) | static | pain #4 |
| 10 | High burn rate | `(>.<)~` (sweating from exertion) | static accessory | Wishlist #3 burn-rate |
| 11 | Tool failure | `(o_O)?` | one-shot 2s, then revert to idle | last-result feedback |
| 12 | Peer waiting on you | `(•ω•)..>>` | trailing dots | orchestrator headline |
| + | **Groggy** (just-woke after a compact) | `(•_•)` | one-shot 2s on resume | Q4 reframe — observable post-compact state |
| + | **Content** (clean tool-success run) | `(•‿•)` | static after a green chain | Q4 reframe — positive observable state |

**Animation rule:** all "animated" states transition between frames *only on heartbeat events* (PreToolUse / PostToolUse / Stop / Notification — already arriving from the orchestrator hook). **Never tick-driven.** A still pet during expected activity *is* the wedged-session signal. Absence of motion carries information.

**Width discipline:** ASCII default at fixed 5 cells. Per the info-encoding agent's reality check — Unicode emoji widths break across [Microsoft Terminal #8970](https://github.com/microsoft/terminal/issues/8970), [VS Code #100730](https://github.com/microsoft/vscode/issues/100730), [Alacritty #6144](https://github.com/alacritty/alacritty/issues/6144); kaomoji split on CJK fonts; Nerd Fonts variable-width per [#1103](https://github.com/ryanoasis/nerd-fonts/discussions/1103). ASCII is the only universal stable substrate. Kaomoji and Nerd Font as opt-in themes once a width-detection probe ships at 1.x.

---

## 4. Cross-pet socialization — the orchestrator integration

The pet's killer move is being **orchestrator-aware**. Tamagotchi Connection's revival was driven by friendship-by-frequency ([Wikipedia](https://en.wikipedia.org/wiki/Tamagotchi_Connection)) — pets bond by *meeting often*, not by spending time together. That mechanic maps 1:1 onto multi-session statusLine: every fire of session A's bar reads session B's heartbeat → "they met."

Design rules (matching the orchestrator's locked rotation rule):

- **One canonical pet per session.** Owning identity > a herd.
- **Peer sessions appear as trailing companion glyphs.** `(•ω•)..` = your pet plus 2 peer dots. Pet count *is* the active-session count.
- **Most-attention-needing peer's pet rotates into a detail slot** when needed: `(•ω•) >> (>_<)@auth` reads as "auth peer is panicking."
- **OSC8 link on the companion dots** opens `cc-status peers` TUI ("pet park" view, Neko Atsume-style grid of all peer pets).
- **Pets do NOT chat.** Speech bubbles cost width and decay into noise fast. Reject.
- **Pet-meets-pet builds friendship persistently.** When session A and session B run together for ≥N hours total, their pets carry a shared memory ("you've worked together for 47 hours"). This is the Tamagotchi Connection mechanic recreated.

That last bullet is the **unique value the pet adds beyond the orchestrator alone**. The orchestrator gives you live cross-session state. The pet gives you *historical relationship* — "your auth-feature pet and your billing-fix pet have shared 23 sessions, 4 successful merges, 1 panic." That's a continuity layer no other tool has.

---

## 5. Bond mechanics — naming, memory, continuity

The psychology agent found the highest-leverage anthropomorphism triggers across Tamagotchi, Roomba, and Replika research:

1. **Naming on first run.** [Sung et al. "My Roomba is Rambo"](https://faculty.cc.gatech.edu/~hic/hic-papers/Roomba-Ubicomp.pdf) and [Darling "Who's Johnny?"](http://www.werobot2015.org/wp-content/uploads/2015/04/Darling_Whos_Johnny_WeRobot_2015.pdf) both find naming is the single highest-leverage trigger. **Decision: pet ships pre-named "Nak" (canonical mascot, like Octocat). User can rename anytime via `cc-status pet rename` and the diary remembers the rename moment.**

2. **Persistent memory of *user-specific moments*, not stats.** [Replika vs Character.AI 2025 comparison](https://aicompanionguides.com/blog/replika-vs-character-ai/) — long-term retention correlates with callback continuity, not raw intelligence. **Decision: pet remembers events, not numbers.** Format **locked: markdown chronicle** at `~/.local/share/cc-status/pet/<name>/diary.md` — append-only, journal voice, opens in any markdown viewer. Each session contributes 1–3 lines. JSON state alongside it at `~/.local/state/cc-status/pet/<name>.json` for the orchestrator to read:
   ```json
   {
     "schema_version": 1,
     "name": "Nak",
     "born": "2026-04-28T08:21:56+0700",
     "memories": [
       {"date": "2026-04-28", "event": "first build success"},
       {"date": "2026-04-29", "event": "exhausted at 91% context, you compacted just in time"},
       {"date": "2026-05-02", "event": "met auth/feat Nak for the first time"}
     ],
     "friendship": { "auth/feat": {"hours": 4.2, "merges": 1, "exhaustions": 0} }
   }
   ```
   Memories surface only when user runs `cc-status pet diary` — never as bar interruption.

3. **Stakes via observable session state, not user-behavior decay** *(reframed 2026-04-28 — supersedes the original "decay across N+ days inactivity" mechanic)*. The original sketch had Tamagotchi-style decay when the user was absent. That's the Clippy failure mode — a judgment about *user behavior* that creates emotional pressure. **Replaced with: pet state reflects only the live session.** When context fills, Nak gets tired; when Nak is exhausted, your session is exhausted; when the API stalls, Nak's eyes spiral. Caring about Nak's wellbeing IS caring about session hygiene. Same Tamagotchi Effect, zero manipulation, because the pet's state is a *faithful reflection*, not a *guilt trip*. The "neglect across days" mechanic is dropped.

4. **Animation via timing + anticipation, not frames.** [Lasseter's *Luxo Jr.*](https://en.wikipedia.org/wiki/Luxo_Jr.) proves emotion conveys via weight and timing alone. At 1-frame-per-event, the principles that survive: anticipation (eyes look first, body follows on next fire), timing (200ms vs 800ms reaction *is* the personality), exaggeration (state #9 shake), and follow-through (failure glyph reverts to idle after a beat).

5. **Cross-pet friendship-by-frequency.** Already covered in §4. The orchestrator integration *is* this principle.

---

## 6. v1.0 core scope — what ships, what defers

### IN at v1.0

| Feature | Justification |
|---------|---------------|
| One canonical pet, ASCII 5-cell, opt-in via `cc-status install --with-pet` | Octocat/Wumpus pattern — canon mascot is marketing surface |
| 12-state vocabulary with heartbeat-driven animation | Solves real pain points (thinking-vs-stalled, wedged, autocompact, mode) |
| Cross-pet companion dots + rotating peer-pet detail | Orchestrator headline rendered through pet posture |
| Naming on first install + `cc-status pet rename` | Highest-leverage anthropomorphism trigger |
| Persistent memory of moments + `cc-status pet diary` | Replika lesson — callback continuity earns long-term retention |
| Pet friendship persistence per-(cwd, session) pair | Tamagotchi Connection mechanic; the unique-vs-orchestrator value-add |
| ~~Visible neglect state on N+ days inactivity~~ — **REMOVED**, replaced by §5.3 reframe | Original mechanic created emotional pressure (Clippy failure mode); replaced with observable-session-state stakes |
| Original ASCII art (no Pokemon, no kaomoji-as-default) | Enterprise-installable; tmux/Windows Terminal width-stable |
| `cc-status pet` subcommand: rename / diary / status / friends | The CLI control surface so pet stays opt-in everywhere |

### Deferred to 1.x with explicit triggers

| Feature | Trigger |
|---------|---------|
| Theme support (cat/frog/dragon) | Once full state vocabulary is locked across canon and stable for ≥2 releases |
| Kitty graphics / sixel "rich pet" mode | When ≥3 users request AND a width-detection probe ships |
| Kaomoji and Nerd Font themes | Same — gated on width-detection |
| Achievement system (badges for X) | Only if `pet diary` usage signals user demand |
| Cross-machine pet sync | When orchestrator's `--sync` flag ships (1.x) |
| Pet evolution / lifecycle stages | Only if pet-diary engagement is high enough to justify; v1.0 ships single-stage |
| Speech bubbles / pet "talking" | **Reject permanently** — Clippy lesson |
| Push notifications about pet state | **Reject permanently** — uninstall-trigger lesson |

---

## 7. The non-obvious convergence (ISC-9)

Four reframes accumulated across the research wave:

1. **Pet ≠ decoration; pet = primary UI.** Posture encodes session state, companion dots encode peer count, rotating peer-pet detail encodes attention queue. The orchestrator's three locked rendering decisions (worktree-as-unit, attention-queue headline, rotation over equal-aggregation) all express through the pet rather than alongside it.

2. **The bond layer is the differentiator over the orchestrator alone.** Live state + historical relationship is something no CC tool has. Naming + persistent memory of moments + friendship-by-frequency turns a diagnostic into a companion *without* turning it into Clippy. The Tamagotchi Connection mechanic was credited with reviving a dying franchise; it's the same lever here.

3. **The architecture stays simple because the wedge stays narrow.** Pet state is a tiny additive layer on the heartbeat hook (already MVP) and the CC transcript-tail (already free). No daemon. No tick loop. No animation engine. Same Unix-ism as before: one writer per session, many readers. The pet is a function of state, not a stateful process.

4. **Two competitors shipped 4 weeks ago — and neither lives in the statusLine.** Both `usik/tamagotchi` and `tama96` are separate apps you switch into. The statusLine surface specifically is wide open *and* validated by the existence of two adjacent prototypes. We're not chasing a fad; we're filling a gap two parallel teams independently identified.

---

## 8. Open questions — resolution log

- ✅ **Pet naming default** — **resolved 2026-04-28: Nak (นาก, Thai for otter; secondary nod to NAK 0x15)**. Otter framing literalizes the cross-session hand-holding metaphor.
- ⏸ **Friendship memory schema lock-in** — deferred to project start. Technical detail.
- ⏸ **Pet sprite set finalization** — deferred to project start. First concrete code = `cc-status pet preview` mock for in-terminal iteration.
- ✅ **Pet decay UX** — **resolved 2026-04-28: reframed as observable-session-state, not user-behavior decay**. Drop "neglect across days" mechanic; pet reflects context/burn/stall live. See §5.3.
- ✅ **`cc-status pet diary` format** — **resolved 2026-04-28: markdown chronicle**, append-only at `~/.local/share/cc-status/pet/<name>/diary.md`. TUI deferred to 1.x trigger.
- ⏸ **Outreach to usik/tamagotchi and siegerts/tama96** — deferred until PET.md stable but before code starts.
- 🆕 **Project name** — now decidable downstream of pet name. Candidates flagged: `nak` (likely taken on crates.io), `nak-cli`, `cc-nak`, or coined alternatives. Worth a separate brainstorm.
- 🆕 **Telemetry / privacy posture** — flagged for SCOPE.md update. 1.0 trigger criteria implicitly assume some usage signal; default-on telemetry is dicey for a local tool. Most successful Rust CLIs ship default-off with explicit opt-in.

---

**Source artifacts:** all 3 raw agent reports preserved in conversation transcript. URLs verified live this session by each agent. No claim from training data; psychological/design assertions tied to academic, industry, or franchise primary sources.
