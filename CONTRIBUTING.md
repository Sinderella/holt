# Contributing to holt

Thanks for considering. holt is a small project with a deliberately narrow scope; this doc explains what's in scope, what's out, and how to be useful here.

## What holt is

A small Rust statusLine for Claude Code that wraps your existing config, profiles it under load, and shows you what your other CC sessions are doing — with a small reactive otter named Nak. See [`README.md`](README.md) and [`docs/INDEX.md`](docs/INDEX.md) for the full pitch.

## What's in scope

- The runtime supervisor (timing, breach log, last-known-good cache, clean-kill on timeout)
- `holt doctor` and its sub-commands
- The multi-session orchestrator (heartbeat + per-session state + cross-session render)
- Nak — the pet, her sprites, her diary, her friendships
- Distribution (cargo-dist + brew + binstall)
- Documentation, examples, presets

## What's out of scope (and why)

These have been considered and explicitly rejected:

- **Push notifications about pet state.** Notification fatigue is a top mascot-app uninstall trigger. holt never pushes.
- **Speech bubbles / pet "talking."** Clippy lesson. The pet exists, it doesn't narrate.
- **Tick-driven animation.** The bar fires on CC events; the pet animates only when something happens. This is a hard architectural constraint, not a feature toggle.
- **Plugin / extension runtime.** Curated presets ship; sandbox plugin runtime is post-1.0 and only behind explicit triggers (≥30 community presets requesting capabilities the `[custom.*]` blocks can't express).
- **Telemetry / analytics.** holt runs entirely on your machine. Roadmap features gate on GitHub issue counts, not analytics.
- **Cross-machine pet sync** (at v0.1–v1.0). Same-machine first; sync is a 1.x stretch behind explicit `--sync` opt-in.
- **A separate dashboard app / TUI.** holt is a statusLine. `holt peers` may eventually exist as a TUI sub-command for drilling into peer state, but the bar IS the product.
- **Pokemon/copyrighted art.** Original ASCII sprites only.

## How to contribute

### Filing issues

We use GitHub issues for bug reports, feature requests, and trigger gating. Roadmap items in [`docs/02-scope.md`](docs/02-scope.md) explicitly gate on issue counts (e.g., "ship Windows when ≥10 Windows-tagged issues"). If you want a feature, file the issue — that's how we measure demand.

**Labels we use:**
- `bug` — something is broken
- `feature` — something we don't have but should
- `question` — clarification, design discussion
- `windows` — Windows-specific report (counts toward Windows-tier-1 trigger)
- `pet` — sprites, diary, bond mechanics
- `runtime` — shim, doctor, breach log, supervision
- `orchestrator` — heartbeat, peer awareness, cross-session render
- `good first issue` — small, well-scoped, no domain context required
- `help wanted` — domain expertise or testing on a platform we lack

### Pull requests

Before writing code, file or comment on an issue describing what you want to do. Solo maintainer; surprise-100-line PRs get harder to review and harder to merge. A 30-second alignment check up front saves both of us time.

PR expectations:
- One concept per PR. Mixed-concern PRs get split.
- Tests for runtime-affecting changes. Sprites/text don't need tests; behavior does.
- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` clean.
- Documentation updated where the change touches user-visible surfaces.

### Code of conduct

Be kind, be specific, assume good faith, leave the room better than you found it. Standard Contributor Covenant applies; full text TBD.

## Maintainer expectations

This is currently a solo-maintainer project. Realistic response times:

- **Issues:** First response within 1 week. Resolution depends on scope.
- **PRs:** First review within 2 weeks. Trivial PRs may merge same-day.
- **Discord/social:** No expectation. Issues are the canonical channel.

If maintenance lapses for >30 days without explanation, that's a signal something happened in real life — feel free to ping (or fork). The project is not allowed to silently rot like CCometixLine did; the issue tracker will state status if maintenance pauses.

## Architectural North Star

When in doubt, optimize for these in order:

1. **Don't make Claude Code lag.** holt's whole reason to exist is making the statusLine *better* — never worse. Anything that risks a blocking call on the render path is a no.
2. **Be honest with users.** No telemetry. No notifications. No emotional manipulation. The pet reflects observable state; it never guilt-trips.
3. **Stay small.** Single-binary, single-purpose, sub-20ms cold start on Linux+macOS. Reach for Rust ecosystem sophistication only when measurement justifies it.
4. **Survive Anthropic's evolution.** The runtime hygiene wedge will erode as CC matures. Design every feature to either degrade gracefully or strengthen as Anthropic's APIs grow. Don't bet on Anthropic staying broken.
5. **Honor the bond.** Nak's diary is the long-term value. Every change that touches the pet should ask: does this make a long-time user feel more attached, or less?

These five rules win arguments. If a proposed change conflicts with one, the change loses.

## License

MIT. By contributing, you agree your contributions are licensed under the same.
