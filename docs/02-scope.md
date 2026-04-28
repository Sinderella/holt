# MVP and 1.0 Scope — Rust CC statusLine Tool

**Date:** 2026-04-28
**Project name:** **holt** (crates.io available)
**Pet name:** **Nak** (the otter living in your statusLine — see PET.md)
**Stack:** Rust (decided)
**Platform at v0.1:** Linux + macOS only. Windows deferred behind explicit trigger (≥10 Windows-tagged issues OR a Windows contributor steps up). README will say so loudly.
**License:** MIT.
**Privacy posture:** No telemetry. Trigger gating runs through GitHub issues only.
**Pet posture:** Default-on. Opt-out via `--no-pet` install flag or `[pet] enabled = false` config. Pet is the project's identity; asking users to opt in to the personality would weaken the brand.
**Companion docs:** `../20260427-statusline-tool-research/FINDINGS.md` for the wedge thesis · `../20260428-statusline-orchestrator-research/ORCHESTRATOR.md` for v1.0 wedge · `../20260428-statusline-pet-research/PET.md` for pet design

---

## TL;DR — the sequencing reframe

The non-obvious insight: **MVP should not render its own statusLine.** Every existing tool (ccstatusline, CCometixLine, ccburn, ccusage) started by competing on widgets — and they're already crowded there. The wedge thesis from the prior research (runtime hygiene + observability) lets you ship a tool that *wraps* whatever statusLine the user already has, adds timing + breach logging + last-known-good caching + a `doctor` profiler, and does **zero rendering of its own**. You're not v1 of "yet another statusLine"; you're the *substrate* underneath the existing ones. That's a smaller MVP, a sharper pitch, a friendlier launch (you compose with ccstatusline rather than replace it), and it's exactly the Rust-CLI-MVP launch shape: **one killer feature, two table-stakes, defer everything else**.

Three things make this the right call:

1. **Existing tools have stalled.** CCometixLine (the only Rust competitor, 2.8k stars) hasn't shipped in ~6 weeks; its [#118 Opus 4.6 crash](https://github.com/Haleclipse/CCometixLine/issues/118) is unfixed. Wrapping rather than replacing means their stalls don't matter to you.
2. **Process supervision cross-platform is harder than it looks** ([rust-lang/rust #115241](https://github.com/rust-lang/rust/issues/115241) — `std::process::Child::kill` doesn't kill descendants on Linux). If you scope MVP to *just the wedge*, you spend your week getting that one thing right. If you also build widgets, the wedge ships half-done.
3. **The Rust CLI MVP pattern says: anchor the pitch, lead with the screenshot, defer the config language.** Starship v0.2 had no TOML config; that came in v0.13, four months later ([Starship v0.2.0 release](https://github.com/starship/starship/releases/tag/v0.2.0)). Render-your-own-segments is the equivalent of "config language" in this comparison — defer it.

---

## 1. Gap analysis — what's missing across the field

Sourced from per-tool README/issue audits this session. Universal gaps (0-of-5 tools have any of these):

| Capability | ccstatusline | CCometixLine | ccburn | usage-bar | ccusage |
|---|---|---|---|---|---|
| Per-fire timing log | ✗ | ✗ | ✗ | ✗ | ✗ |
| Active doctor (script-under-load profile) | ✗ | ✗ | ✗ | ◐ plumbing only | ✗ |
| Breach warning (Starship `[WARN]` equivalent) | ✗ | ✗ | ✗ | ✗ | ✗ |
| Last-known-good TTL cache | ✗ | ✗ | ✗ | ✗ | ✗ |
| Autocompact buffer-math correction | ✗ | ✗ | ✗ | ✗ | ✗ |
| Plan-mode vs execute color flip | ✗ | ✗ | ✗ | ✗ | ✗ |
| Per-MCP-server cost segment | ✗ | ✗ | ✗ | ✗ | ✗ |
| Stuck-loop / "API stalled" detection | ✗ | ✗ | ✗ | ✗ | ✗ |
| Last-tool breadcrumb | ✗ | ✗ | n/a | ✗ | ✗ |

**Where CCometixLine specifically is weak** (the openings — the closest Rust competitor):
- 6-week stale; Opus 4.6 crash ([#118](https://github.com/Haleclipse/CCometixLine/issues/118)) unfixed
- No 5h/7d rate-limit support ([#109](https://github.com/Haleclipse/CCometixLine/issues/109)) despite CC v2.1.80 shipping `rate_limits` JSON
- Brittle terminal teardown — Ctrl+C leaves `─` artifacts ([#107](https://github.com/Haleclipse/CCometixLine/issues/107))
- No tokens/s ([#51](https://github.com/Haleclipse/CCometixLine/issues/51)), no MCP, no plan-mode, no doctor

**Where CCometixLine is strong (don't fight here):** binary cold-start, TOML+TUI config, model display polish, `--patch` cli.js trick.

---

## 2. The MVP — what ships in v0.1

**Pitch (one sentence):** *"Wrap your existing Claude Code statusLine and find out why it's slow — `cc-status doctor` profiles your script under load and tells you which segment is killing you."*

### IN

| # | Feature | One-line justification |
|---|---------|------------------------|
| 1 | **Shim binary that wraps your existing `statusLine.command`** | The Rust CLI MVP shape — single killer integration. User changes one line in `settings.json` and gets timing for free. |
| 2 | **Per-fire timing log** to `~/.cache/cc-status/timings.jsonl` (script duration, fork count, exit code, stderr capture) | Closes the observability gap that Anthropic leaves open ([statusLine docs](https://code.claude.com/docs/en/statusline) — only logs first invocation per session). Universal gap, all 5 tools missing. |
| 3 | **Last-known-good TTL cache** — render cached output instantly if previous fire is stale, recompute in background | Solves the cancel-on-new-event orphan-process bug class ([CC #18943](https://github.com/anthropics/claude-code/issues/18943)). Bar is never blocked. |
| 4 | **Configurable timeout + clean kill (Unix process group)** via `process-wrap` setpgid+killpg | The actual wedge — kills children too, not just the immediate child ([rust-lang #115241](https://github.com/rust-lang/rust/issues/115241)). |
| 5 | **`cc-status doctor`** — fires the configured script 20× under load, ranks fork count / network calls / FS bytes / p95 latency, prints culprit table | The headline demo. The README screenshot. The "wow" moment. |
| 6 | **Breach log** — when timing exceeds threshold, append to `~/.cache/cc-status/breaches.log` with full context (env, stdin, stderr) | Bug-report bundle that doesn't exist anywhere in the field today. |

### OUT (explicitly deferred)

| Feature | Why deferred |
|---------|--------------|
| Render your own segments (model, git, context %, tokens, …) | Crowded category; CCometixLine + ccstatusline + usage-bar + ccusage compete here. Ship as substrate, not competitor. |
| TOML config language | Starship pattern: shipped at v0.13, four months in. MVP ships env vars + CLI flags. |
| Plugin/preset system | Ship sane defaults; design plugin API once real users have asked for a specific module that doesn't fit. |
| OSC8 hyperlinks | Old-terminal corruption risk ([OSC8-Adoption list](https://github.com/Alhadis/OSC8-Adoption/)). Opt-in only at 1.0. |
| Nerd Fonts / Powerline | Width-math tar pit ([starship #7303](https://github.com/starship/starship/issues/7303)). ASCII-default safer at MVP. |
| Windows JobObject correctness | Unix-tier-1 at MVP; Windows runs but documented as best-effort. JobObject + ConPTY is a separate product surface. |
| Code signing / notarization | macOS Gatekeeper warning is acceptable friction at MVP; document the workaround. |
| Async runtime (tokio, etc.) | Unjustified for MVP feature set. Sync stdlib + threads. Audit deps for transitive tokio pull-through ([reqwest #1233](https://github.com/seanmonstar/reqwest/issues/1233)). |
| `simd-json` / `figment` | Pessimization for small JSON inputs ([serde-rs/json-benchmark](https://github.com/serde-rs/json-benchmark)); stock `serde` + `toml` is sub-millisecond. |
| Linux arm64 binary | Skip in cargo-dist matrix at MVP; add at 1.0. |

### MVP success criteria (objective)

- Sub-20 ms cold start on macOS arm64 / Linux x86_64 (separate Windows budget — 40 ms with Defender plausible)
- `cc-status doctor` produces a ranked culprit table from a fresh user's existing statusLine config in <30 s
- Wraps any of the 5 audited tools without modification
- Prebuilt binaries shipped via `cargo-dist` (Linux x64, macOS x64+arm64, Windows x64) on day one
- Homebrew tap auto-generated, `cargo-binstall` works
- README leads with an asciinema/gif of `cc-status doctor` finding a slow `curl` in someone's config

### MVP success criteria (the gut-check)

The "got me to install it" thing. Lifted from the launch-playbook agent: *"like ccstatusline, but with a `doctor` command that tells you why it's slow."* If the demo isn't that pithy, the MVP is too big.

---

## 3. The 1.0 — what "feature complete" looks like, with triggers

**1.0 means SemVer freeze, not feature freeze.** Starship hit 1.0 after ~100 releases ([release notes](https://github.com/starship/starship/releases/tag/v1.0.0)) and explicitly said *"no real cause for celebration"* — every signature feature shipped during 0.x. Translate: **`cc-status` 1.0 is the promise that user configs from 1.0 will still work in 1.x.** Adopt the same convention.

### Features deferred from MVP, with explicit triggers

| Feature | Tier | Trigger to ship |
|---------|------|-----------------|
| TOML config language | 0.x | When ≥3 users file issues against a hard-coded default |
| Render your own segments (model, git, context %) | 0.x | When telemetry shows >50% of users wrap an existing tool *and* request native segments to drop the wrap |
| `cc-status doctor --graph` ASCII timing chart | 0.x | Trivial extension of #5 above |
| `cc-status doctor --share` redacted bundle | 0.x | Once breach log format stable for ≥2 releases |
| SQLite trend tracking (last 1k fires) | 0.x | When users start asking "is my bar getting slower?" |
| Autocompact buffer-math correction | 0.x | Ship anytime — 1-day feature, gated only by knowing the right buffer constant |
| Plan-mode vs execute color flip | 0.x | Once stdin JSON exposes mode reliably (already does in 2.1.119) |
| Effort/thinking pill (from `effort.level` JSON) | 0.x | Already shippable — CC v2.1.119 added the field |
| Stuck-loop detector | 0.x | Built on `PostToolUse.duration_ms` (shipped v2.1.119) |
| MCP server health row | 0.x | When `claude mcp status` output is stable enough to parse |
| Subagent depth + count badge | 0.x | When SubagentStart hook payload audit-clean |
| Cost-per-task projection | 0.x | After 5h/7d rate-limit gauge ships and stabilizes |
| Curated preset gallery (~12) | 0.x | Once native rendering ships and ≥10 users have written custom configs worth promoting |
| Importing Starship TOML | 0.x | Network-effect move; ship after preset gallery |
| OSC8 deep-links (branch→PR, session→console) | 0.x | Opt-in via config flag; off by default until terminal allow-list ships |
| Nerd Fonts theme | 0.x | Behind `--unicode-width-mode` flag with terminal allow-list |
| Windows JobObject correctness | 0.x | When ≥10 Windows users have filed issues OR a contributor steps up |
| Linux arm64 binary | 0.x | Add to cargo-dist matrix at first .1 release |
| Code signing (macOS notarization, Windows EV cert) | 1.0 promise | When Gatekeeper/SmartScreen friction blocks ≥5 issues |
| Plugin runtime (oh-my-zsh-style) | **post-1.0, conditional** | Curated preset count ≥30 AND ≥3 require shell-out logic that `[custom.*]` blocks can't express |
| OpenTelemetry exporter | post-1.0 | When ≥3 users request OR CC's own OTel ([v2.1.117](https://code.claude.com/docs/en/changelog)) gains span correlation |
| `npx cc-status` wrapper | post-1.0 | Only if telemetry shows JS-ecosystem users blocked from native paths |
| Anthropic ships native timing → runtime supervisor obsolete | **trigger to pivot** | If CC ships `command_timeout` + per-fire timing log: shim layer downgrades to generator/diagnostic only; doctor stays valuable, breach log composes with native log |

### 1.0 north star (verbatim from agent synthesis, lightly edited)

A Rust binary you `brew install` or `cargo binstall`, a single TOML config that ports between machines, ten curated presets, AI-native segments (autocompact-corrected, subagent depth, MCP health, stuck-loop, plan-mode, effort/thinking) that work today on transcript-parse and auto-upgrade as Anthropic widens the JSON surface, last-known-good cached rendering, and — the wedge nobody else has — `cc-status doctor` that load-tests your config, tracks per-segment trends in SQLite, alerts on regressions, produces a redacted shareable bundle. Plugin runtime, OTel export, and oh-my-zsh sprawl explicitly deferred behind named triggers. **The 1.0 promise: your config from today still works in 1.x; your statusLine never silently fails again; when CC slows down you'll know why in seconds.**

---

## 4. The sequencing insight (ISC-9)

Three sequencing decisions that look counter-intuitive but are load-bearing:

1. **MVP doesn't render its own statusLine.** It wraps the user's existing one. The Rust CLI MVP launch pattern says one killer feature; the doctor + shim *is* that feature. Adding native rendering doubles the surface area, drags in width-math + Nerd Font + OSC8 issues, and forces you to compete on widgets where the field is already crowded. **Native rendering is a 0.5 milestone, not 0.1.**

2. **The runtime supervisor is the hardest single feature, and it ships in MVP.** Counter-intuitive because most MVPs cut hard things. But the cross-platform process-group kill ([process-wrap](https://crates.io/crates/process-wrap/6.0.0)) IS the wedge — without it you don't have a defensible thesis. The right cut is *Unix-correct first, Windows best-effort* — ship the wedge for 80% of users immediately and document the platform tier explicitly.

3. **TOML config defers past v0.5, not just past v0.1.** Starship explicitly: *"🚧 Configuration features and documentation are in the process of being developed"* at v0.2 ([release notes](https://github.com/starship/starship/releases/tag/v0.2.0)). TOML landed at v0.13. Translation: ship sane defaults until users complain about a specific default, *then* design config around the actual complaint. Premature config languages constrain product evolution and invite bikeshedding before there are users. Until you have real users, env vars + CLI flags + the wrapped command (read from `settings.json`) is the entire surface.

---

## 5. Open questions for the scoping conversation

- **Does the user want to commit to "Unix-tier-1, Windows-tier-2" explicitly in the README, or aim for parity?** Affects 4-6 weeks of scope.
- **MVP timeline target** — how many weekends? The minimum lovable MVP (shim + doctor + breach log + last-known-good + Unix process supervision + cargo-dist binaries) is ~2-4 weekends of focused work. Each deferred 0.x feature adds ~1 weekend.
- **Does `cc-status` get to telemetry?** The 1.0 trigger criteria assume some minimal opt-in usage signal ("≥3 users request X"). Without telemetry, gating is by GitHub issue count only — slower feedback loop.
- **First-user acquisition path** — HN Show post anchored to the doctor demo, or quieter rollout via the `awesome-claude-code` repo first? This affects MVP polish budget.
- **CCometixLine is stalled but not dead.** Worth reaching out to its maintainer before launch? Or just ship?

---

**Source artifacts:** all 4 raw agent reports preserved in conversation transcript; key URLs cited inline above. No claim synthesized from training data; every competitor feature claim verified against upstream README/issues this session.
