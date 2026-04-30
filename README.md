# holt

> *Where your Nak lives.*

A small Rust statusLine for Claude Code that wraps your existing config and tells you when your bar is silently failing.

![demo](assets/demo.gif)

## Install

```bash
# Build from source (cargo + Rust 1.87+; ~30s):
cargo install --git https://github.com/Sinderella/holt --tag v0.1.0-rc.1 holt-cli
```

Or download a prebuilt macOS binary (Intel + Apple Silicon) from the [Releases page](https://github.com/Sinderella/holt/releases/tag/v0.1.0-rc.1). Each tarball ships with a `.sha256` sidecar — verify before extracting:

```bash
shasum -a 256 -c holt-cli-aarch64-apple-darwin.tar.xz.sha256
tar -xJf holt-cli-aarch64-apple-darwin.tar.xz
mv holt-cli-aarch64-apple-darwin/holt ~/.local/bin/holt   # or wherever you keep binaries
```

On macOS, the prebuilt path may trip Gatekeeper because v0.1 binaries are unsigned. Strip the quarantine flag once after extracting:

```bash
xattr -d com.apple.quarantine "$(command -v holt)"
```

(Apple Developer Program enrollment for native notarization is on the v0.1.x roadmap.)

**Platform tier at v0.1:** Linux x86_64 and macOS (x86_64 + Apple Silicon) are tier-1; Windows x64 is best-effort and may lag releases. The v0.1.0-rc.1 release ships macOS-only prebuilt artifacts; Linux and Windows users build from source via `cargo install --git`. Windows is promoted to tier-1 when ≥10 [Windows-tagged issues](https://github.com/Sinderella/holt/issues?q=is%3Aissue+label%3Awindows) are filed against the repo OR a Windows contributor steps up. See [`docs/02-scope.md`](docs/02-scope.md) for the full v0.1 scope statement and the trigger criteria.

A Homebrew tap (`brew install Sinderella/holt`) is **deferred to v0.1.x** — re-added once macOS Gatekeeper friction is reported by ≥3 users or Apple notarization becomes worthwhile. See [`.planning/milestones/v0.1-REQUIREMENTS.md`](.planning/milestones/v0.1-REQUIREMENTS.md) (DIST-02) for the deferral rationale.

### First-run

After install, wire holt into Claude Code's hook system:

```bash
holt install-hooks --dry-run        # shows the diff vs your current ~/.claude/settings.json
holt install-hooks                  # applies the merge once the diff looks correct
```

`holt install-hooks` mutates `~/.claude/settings.json` atomically — it acquires an exclusive lock, writes a `.holt.bak` backup, fsync-then-renames the merged file in place, and never half-writes. JSONC comments and key order in your existing settings are preserved. Use `--print` instead of the default to emit just the JSON snippet for manual paste.

### Reporting issues

When you file an issue, label it so it routes correctly: [`bug`](https://github.com/Sinderella/holt/labels/bug) for breakage, [`feature`](https://github.com/Sinderella/holt/labels/feature) for requests, [`question`](https://github.com/Sinderella/holt/labels/question) for design discussion, [`windows`](https://github.com/Sinderella/holt/labels/windows) (counts toward the Windows-tier-1 trigger), [`pet`](https://github.com/Sinderella/holt/labels/pet) for Nak / sprites / diary, [`runtime`](https://github.com/Sinderella/holt/labels/runtime) for the supervisor / breach log / `holt doctor`, [`orchestrator`](https://github.com/Sinderella/holt/labels/orchestrator) for cross-session / heartbeat / peer awareness, [`good first issue`](https://github.com/Sinderella/holt/labels/good%20first%20issue), or [`help wanted`](https://github.com/Sinderella/holt/labels/help%20wanted). Full contribution guide in [`CONTRIBUTING.md`](CONTRIBUTING.md).

```
auth/feat  (•ω•)..>>  [Read]  $0.34   [3/7 you're up]   billing/fix (>.<)*
```
*Above is what the bar will look like at v1.0. The leftmost segment (`auth/feat`) is the worktree label — `repo/branch`. Your Nak (calm, leaning forward) is the otter in your current session, with two peer sessions trailing behind as dots. `[3/7 you're up]` reads as: three of seven peer sessions are waiting on you, and you're at the head of queue. The exhausted Nak on the right is your `billing/fix` session — eyes drooping because it's at 91% context.*

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
