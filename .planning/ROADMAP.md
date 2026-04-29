# Roadmap: holt

**Status:** v0.1 shipped 2026-04-29. Next milestone (v0.5: `holt doctor`) not yet planned.

## Shipped milestones

- **v0.1 — Runtime hygiene wedge (the lovable MVP)** ✓ 2026-04-29 → [milestones/v0.1-ROADMAP.md](milestones/v0.1-ROADMAP.md). 5 phases, 9 plans, 121 commits, 6,039 Rust LOC, 80/80 tests, all 6 hard constraints C1..C6 enforced. 27/28 v1 requirements satisfied + 1 deferred (Homebrew tap → v0.1.x). Maintainer ship: `./tools/bootstrap-github.sh && cargo bump 0.1.0-rc.1 && git tag v0.1.0-rc.1 && git push origin v0.1.0-rc.1` (per `phases/04-distribution-launch/RC1-CHECKLIST.md`).

## Next milestone

Run `/gsd-new-milestone` to define v0.5 (`holt doctor` — load-tester + culprit table). The v2 requirements section in the archived `milestones/v0.1-REQUIREMENTS.md` carries the preserved scope notes; new requirements get authored fresh.
