# Starship-for-Claude-Code: Research Findings

**Date:** 2026-04-27
**Scope:** Pre-design research only. No architecture, no stack proposal.
**Method:** 4 parallel research agents (Perplexity / Claude / Gemini / Codex) with live web search.

---

## TL;DR — three things to internalize before any design discussion

1. **The lag isn't where you think it is.** statusLine doesn't fire per keystroke. Per [official docs](https://code.claude.com/docs/en/statusline): it fires on turn boundaries + permission/vim mode changes, debounced 300ms, **cancellable mid-flight, no built-in timeout, no per-call timing log**. Typing-echo lag attributed to statusLine is usually orphaned subprocesses (cancelled curls, half-finished jq pipelines) leaking into the next turn — not your script blocking input. That reframes the bug class: the tool's job is **process-lifecycle hygiene + observability**, not just "fast modules."

2. **The biggest pain is invisibility, not slowness.** Every observation channel converged on the same gap: `claude --debug` only logs the *first* statusLine invocation per session, only reads first line of stdout, has no breach-warning equivalent to Starship's `[WARN] Executing command "X" timed out`. Users build elaborate scripts, watch CC get sluggish, and have **no signal** which segment caused it. This is the wedge.

3. **The moat over Starship is AI-coding-specific signals, not theming.** Cosmetic ports of Starship (powerline, presets, gradients) are already saturated by ccstatusline / CCometixLine / ccburn. The unmet need is signals that only make sense for an AI agent: autocompact countdown with corrected buffer math, "thinking vs API stalled" liveness, per-MCP-server token cost, plan-mode badge, last-tool breadcrumb, stuck-loop detection. None of these exist in Starship and most don't exist in the CC ecosystem yet.

---

## 1. Pain points (ranked, sourced)

| # | Pain | Evidence |
|---|------|----------|
| 1 | statusLine silently doesn't execute / disappears | Issues [#17020](https://github.com/anthropics/claude-code/issues/17020), [#43826](https://github.com/anthropics/claude-code/issues/43826), [#29383](https://github.com/anthropics/claude-code/issues/29383), [#5863](https://github.com/anthropics/claude-code/issues/5863), [#52997](https://github.com/anthropics/claude-code/issues/52997), [#14125](https://github.com/anthropics/claude-code/issues/14125), [#6526](https://github.com/anthropics/claude-code/issues/6526) |
| 2 | Slow scripts cause input echo lag | [#18943](https://github.com/anthropics/claude-code/issues/18943) — heavy jq pipeline → 70–90% input echo delay |
| 3 | Doesn't refresh during turns (when you most want it) | [#50679](https://github.com/anthropics/claude-code/issues/50679) — 4-min gap of zero invocations during one tool run |
| 4 | Token/context fields are wrong (cumulative, not current) | [#13783](https://github.com/anthropics/claude-code/issues/13783) — 340k of 200k = 169% impossible |
| 5 | Transcript parsing is brittle and slow | [#11535](https://github.com/anthropics/claude-code/issues/11535), [#21022](https://github.com/anthropics/claude-code/issues/21022) (102 MB JSONL froze CC entirely) |
| 6 | Helper tools can fork-bomb the user's own machine | [ccusage #459](https://github.com/ryoppippi/ccusage/issues/459) — 34 parallel processes, 3+ GB RAM, load avg 21.7 |
| 7 | Cross-platform fragility (Windows especially) | [#14125](https://github.com/anthropics/claude-code/issues/14125), [#6526](https://github.com/anthropics/claude-code/issues/6526), [#52997](https://github.com/anthropics/claude-code/issues/52997) |
| 8 | tmux/terminal width race — 1-char-per-line render | [#27158](https://github.com/anthropics/claude-code/issues/27158) |
| 9 | Discoverability — every user reinvents the same wheel | [#30341](https://github.com/anthropics/claude-code/issues/30341) |
| 10 | Default bash approach doesn't scale | [Felipe Elias 2026-03-17](https://felipeelias.github.io/2026/03/17/claude-statusline.html) |

**Surprises:**
- Anthropic's own JSON contract was wrong for months — anyone displaying context % from official inputs was lying to users.
- A popular helper (ccusage) DoS'd users' machines from inside statusLine before adding file-based locking.
- Same script can pass in one terminal and silently fail in another on the same OS.

---

## 2. Starship feature inventory (port-relevance flagged)

| Feature | Port? | Why |
|---------|-------|-----|
| ~80 built-in modules with `detect_files` / `detect_extensions` triggers | **Adapt** | The pattern is right; the modules are wrong (no node/aws/k8s — replace with CC-specific signals) |
| TOML format with `format = "$mod1$mod2"` composition + per-module override tables | **Port** | Sane, declarative, beats shell-script |
| Two-stage gate: `when` (cheap predicate) → `command` (expensive) | **Port directly** | The single most copyable pattern. Keeps custom modules from regressing perf |
| `command_timeout = 500ms`, `scan_timeout = 30ms`, both tunable | **Port + tighten** | Circuit breaker, not a knob. CC currently has neither |
| Concurrent module evaluation; slow modules killed at timeout with `[WARN]` log | **Port** | Solves observability gap simultaneously |
| `STARSHIP_LOG=trace starship timings` profiler | **Port + extend** | Headline feature for a CC tool — `cc-status doctor` |
| 12 named presets, one-line install (`starship preset gruvbox-rainbow -o ...`) | **Port** | Starship's growth engine. CC has no equivalent |
| `[palettes.NAME]` + `palette = "NAME"` for swap-without-rewriting | **Port** | First-class color palettes |
| `right_format` for left/right zones | **Port** | Free win |
| **No result cache** between renders (Starship's biggest scar) | **Don't port** | CC fires on turn boundaries, not per-keystroke — a tiny TTL cache buys you everything Starship's missing |
| **No async pre-render / instant-prompt** | **Don't port** | But adopt the p10k pattern: render last-known-good cached, recompute async, replace on completion |

Why people love Starship (verbatim from sources):
- *"sane, short and declarative config won me over"* — [Bulimov post-p10k migration](https://bulimov.me/post/2025/05/11/powerlevel10k-to-starship/)
- *"shell-agnostic — if I move away from zsh I won't change a thing"* — same post
- *"suddenly everything is faster (I was blaming iterm)"* — [HN commenter](https://news.ycombinator.com/item?id=20990076)

Pattern: **TOML over shell-script, portability, subjectively faster even when benchmarks are close.**

---

## 3. Wishlist — Top 10 ranked

1. **Autocompact countdown with corrected buffer math** — fixes the universal "context lies" pain. [zenn.dev writeup](https://zenn.dev/trust_delta/articles/claude-code-context-warning-001?locale=en)
2. **"Thinking vs API stalled" liveness indicator** — [#40453](https://github.com/anthropics/claude-code/issues/40453)
3. **Burn-rate sparkline + depletion ETA** — most-cloned community feature ([leeguooooo/claude-code-usage-bar](https://github.com/leeguooooo/claude-code-usage-bar))
4. **Per-MCP-server token-cost segment** — `claude mcp status` already exposes the data ([Scott Spence post](https://scottspence.com/posts/optimising-mcp-server-context-usage-in-claude-code))
5. **5h-block + 7d-limit dual gauge with thresholds** — leverages v2.1.80 `rate_limits` JSON ([ccburn](https://github.com/JuanjoFuchs/ccburn))
6. **Plan-mode vs execute-mode color flip** — full-bar mode signal, zero clutter
7. **Last-tool-used transient breadcrumb** — ambient awareness without `/btw`
8. **Conditional segments + flex truncation** — Starship-grade polish, table stakes
9. **Community preset gallery + one-line install** — Starship's growth playbook
10. **OSC8 deep-links** (branch → PR, session → console, file → IDE) — turns the bar into a launcher

Existing community tools to know: [ccstatusline](https://github.com/sirmalloc/ccstatusline), [CCometixLine](https://github.com/Haleclipse/CCometixLine), [ccburn](https://github.com/JuanjoFuchs/ccburn), [claude-code-usage-bar](https://github.com/leeguooooo/claude-code-usage-bar), [ccusage statusline](https://ccusage.com/guide/statusline), [Felipe Elias post](https://felipeelias.github.io/2026/03/17/claude-statusline.html).

---

## 4. Why CC statusLine lags — the technical truth

**Execution model** (from [official docs](https://code.claude.com/docs/en/statusline)):
- Fires on assistant message boundaries + permission/vim mode changes
- Debounced 300ms
- **Cancellable mid-flight** if a new event arrives while running
- `refreshInterval` (≥1s) is *additive*, opt-in
- Reads only first line of stdout
- stdin = JSON session data
- **No documented per-script timeout, no timing log, no breach warning**

**Top 6 root causes (ranked by frequency in real bug reports):**
1. Synchronous network calls (cost APIs, GitHub, telemetry) — see ccusage's file-locking workaround
2. Reading multi-MB transcript JSONL on every fire ([#21022](https://github.com/anthropics/claude-code/issues/21022): 102 MB froze CC)
3. Subprocess pipeline storms — [#18943](https://github.com/anthropics/claude-code/issues/18943) spawns 3 jq processes per fire
4. Slow git in big repos — docs explicitly call this out as a footgun
5. Cold-starting node/bun/python per refresh — `npx ccsomething` = 200–600 ms per fire
6. Network filesystems (iCloud, OneDrive, NFS) under `~/.claude/`

**Reproducible misconfig** (constructed from #18943 + #21022 patterns):
```json
{
  "statusLine": {
    "type": "command",
    "command": "input=$(cat); curl -s --max-time 10 https://httpbin.org/delay/3; jq -r '.workspace.current_dir' <<< \"$input\"; jq -r '.model.display_name' <<< \"$input\"; ls -la ~/.claude/projects/*/*.jsonl | wc -l",
    "refreshInterval": 1
  }
}
```
Predicted symptom: 3s background work per turn; cancel-on-new-event leaves orphaned curl processes; CC feels sluggish without any error surfaced.

**How others solved it:**
- **Starship:** `command_timeout` + `scan_timeout`, kill-on-timeout, WARN log, `starship timings`
- **Powerlevel10k:** instant-prompt cache rendered before plugins load; expensive segments async via `gitstatus`
- **tmux:** `status-interval`, `#(shell-command)` runs externally — UI never blocks
- **zsh-async:** worker subshell + self-pipe + `zle -F` callback

Shared pattern: **render last-known-good immediately, recompute in background, replace on completion, enforce a budget, log breaches loudly.** CC currently does ~half of step 1 and none of 3–4.

---

## 5. Non-obvious insight (the ISC-9 reframe)

Going in, the framing was: *"Starship is great → port it to CC."*

The four-agent convergence reframes that:

> **The CC statusLine problem isn't a missing-prompt-tool problem — it's a missing-runtime problem.** Users don't suffer because they can't write modules in TOML. They suffer because (a) Anthropic's runtime fires their script in a way that makes orphan processes and silent failures the *default*, (b) the data it passes in is partially wrong by default (token math), and (c) there's no observability to catch any of it. Starship's gift is a *config language*; the wedge for a CC-native tool is being a *better runtime* — process supervision, timing, breach logging, last-known-good caching, doctor diagnostics — with the config language as a downstream nicety.

Three concrete wedges that fall out of that reframe:

1. **`cc-status doctor`** — fires the configured script 20× under load, profiles fork/exec count, network calls, FS reads, prints ranked culprit list. Starship's `timings` is single-fire and passive; CC needs *active* diagnosis because the failure mode is opaque.
2. **Last-known-good cached rendering** — a tiny TTL cache (CC fires on turn boundaries, not per-keystroke, so cache is cheap) means the bar is never blocked on a slow segment. Solves p10k's instant-prompt problem and Starship's no-cache scar in one move.
3. **Buffer-math correction layer** — built-in `context_remaining` segment that does the autocompact buffer math correctly even when Anthropic's JSON is wrong. Default trust for the official inputs is the wrong default.

Everything else (presets, gradients, OSC8 links, plan-mode color flip) is downstream of those three.

---

## 6. Open questions for the scoping conversation

- Does the tool need to ship its own runtime/binary, or is it a config generator that emits a shell command? (Trade-off: binary = better process lifecycle; generator = lower install friction.)
- Will Anthropic add timeout/timing observability to `claude` itself, making the runtime wedge temporary? (If yes: ship as a generator. If no: ship as a runtime.)
- How important is config portability to existing Starship users? (Importing Starship TOML as a starter is a free network-effect move.)
- Plugin / module distribution — do you want oh-my-zsh-style sprawl or Starship-style curated PR-only presets?

---

**Source files:** all 4 raw agent reports preserved in conversation transcript. No findings synthesized from training data; every claim traces to a URL retrieved this session or is explicitly flagged `(synthesized)` / `(my hypothesis — unverified)`.
