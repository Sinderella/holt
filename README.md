# holt

> *Where your Nak lives.*

A small Rust statusLine for Claude Code, with a small otter in it.

```
auth/feat  (•ω•)..>>  [Read]  $0.34   [3/7 you're up]   billing/fix (>.<)*
```
*Demo coming. Above is what the bar will look like at v1.0. The leftmost segment (`auth/feat`) is the worktree label — `repo/branch`. Your Nak (calm, leaning forward) is the otter in your current session, with two peer sessions trailing behind as dots. `[3/7 you're up]` reads as: three of seven peer sessions are waiting on you, and you're at the head of queue. The exhausted Nak on the right is your `billing/fix` session — eyes drooping because it's at 91% context.*

---

## What's a Nak?

Nak is an otter. Otters live in family groups and hold paws while they sleep so they don't drift apart in the current. Your Nak does the same with the otters in your other Claude Code sessions.

Underneath, **holt is a serious tool**: it wraps your existing statusLine, profiles it under load, kills runaway scripts cleanly, and tells you when your bar is silently failing. The otter is the part you'll show your friends. The runtime is the part that earns its place in your dotfiles.

## What it does

**Wraps your existing config.** Your `statusLine.command` keeps working. holt slips a runtime supervisor underneath it — per-fire timing, breach log, last-known-good cached render so the bar is never blocked, clean kill of orphaned children on timeout. Set it once and forget it.

**Profiles when something's wrong.** `holt doctor` actively load-tests your statusLine and prints a ranked culprit table — fork count, network calls, FS bytes, p95 latency. The fix-list nobody else gives you. Solves the "why does Claude Code feel sluggish?" mystery in under a minute.

**Surfaces what your other sessions are doing.** Run multiple Claude Code sessions in parallel? holt's bar shows the attention queue across all of them in your peripheral vision. Your Nak is calm; the auth-fix Nak is exhausted; the billing Nak is wedged. You know which session needs you next without leaving this one.

## Why

The Claude Code statusLine has three quiet failures and nobody is fixing them.

- **Misconfigured scripts make CC itself lag**, with no per-fire timing log, no breach warning, no `command_timeout` like Starship's. Anthropic's `claude --debug` only logs the first invocation per session. The failure mode is opaque.
- **The data CC passes in is partially wrong** — context % can read 169% because session-cumulative tokens get mistaken for current-context tokens.
- **Multi-session work has no glanceable visibility.** Every existing CC orchestrator (claude-squad, vibe-kanban, crystal) is a separate app you context-switch into. None surface peer-session state in the bar you already look at.

holt fixes the runtime, corrects the data, and surfaces multi-session state where you're already looking.

## Privacy

No telemetry. holt runs entirely on your machine. Roadmap features gate on GitHub issue counts, not analytics.

## Install

```bash
cargo install holt          # any Rust toolchain
cargo binstall holt         # prebuilt binaries (no compile)
brew install <user>/tap/holt   # tap link coming
```

**Platform support at v0.1:** Linux and macOS only. Windows is deferred — the cross-platform process-supervision work (JobObject, ConPTY) is real engineering and would compromise the v0.1 timeline. If you're on Windows and want this to work, file an issue or send a PR — we'll commit to Windows when there's a real signal of demand.

**Don't want the otter?** `holt install --no-pet` runs the full runtime supervisor and orchestrator without Nak. The diagnostic, breach log, and peer-session attention queue all work without her. Nak is the project's identity, but pure utility is one flag away.

## Status

Design phase. Code coming. Read [`docs/`](docs/INDEX.md) for the design rationale — pain-point research, MVP/1.0 scope, multi-session orchestrator architecture, and the Nak design doc.

## License

MIT. See [LICENSE](LICENSE) when it lands.

---

## The names, explained

**Nak** is short for **นาก** — Thai for *otter*. It's also a quiet nod to **NAK** (0x15), the byte a protocol sends when something's not okay. Nak's whole job is to tell you, gently, when something is.

**holt** is the den of an otter — where Nak lives. The home you barely notice until something feels off in it.

---

*holt is built for people who run more than one Claude Code session at a time and notice when their bar feels off. If that's you, the otter is for you too.*
