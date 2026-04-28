# holt — design docs

A reading order. Skim top-down for the full story; jump straight to a doc if you know what you're looking for.

## Project facts

- **Project name:** holt (an otter's den — where Nak lives)
- **Pet name:** Nak (นาก, Thai for otter; secondary nod to NAK 0x15 — the byte computers send when something's not okay)
- **Stack:** Rust
- **Distribution:** cargo-dist (Linux x64, macOS x64+arm64, Windows x64) + Homebrew tap + cargo-binstall
- **Privacy posture:** No telemetry. Trigger gating runs through GitHub issues only.

## Phased scope

| Version | What ships | Pitch |
|---------|------------|-------|
| **v0.1** | Shim that wraps your existing `statusLine.command` + per-fire timing log + last-known-good cache + Unix process supervision + heartbeat hook | *"Wrap your existing config; never feel statusLine lag again."* |
| **v0.5** | + `holt doctor` — active script profiler with ranked culprit table | *"...and find out why your bar is slow."* |
| **v1.0** | + Multi-session orchestrator (cross-session attention queue) + Nak the otter as core feature (heartbeat-reactive state, hand-holding peer pets, markdown diary, persistent friendship memory) | *"...and see what your other CC sessions are doing without leaving this one."* |

## Reading order

1. **[01-findings.md](01-findings.md)** — initial pain points, wedge thesis, and the reframe that the CC statusLine problem is a runtime problem, not a prompt-tool problem
2. **[02-scope.md](02-scope.md)** — MVP/1.0 scope with In/Out tables, trigger-gated deferrals, and the Rust ecosystem reality check
3. **[03-orchestrator.md](03-orchestrator.md)** — multi-session orchestrator design, three locked architectural decisions, and why "files-on-disk + hooks" beats a daemon at this scale
4. **[04-pet.md](04-pet.md)** — Nak as v1.0 core feature, the integration insight ("the pet IS the orchestrator's UI"), state vocabulary, bond mechanics, and the resolution log of locked-vs-open questions
5. **[05-schemas.md](05-schemas.md)** — heartbeat JSON + pet state JSON v1 schemas, friendship aggregation rules, rename consequences, forward-compatibility plan

## Status

All four documents are alive and locked at the level of "what we'd start building" — but every claim is research-backed, every URL was retrieved fresh during the research wave, and every locked decision has a documented "why." Schema details, sprite finalization, and outreach drafts are deferred to project start (see resolution logs at the end of each doc).
