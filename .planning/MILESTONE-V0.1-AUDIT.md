---
milestone: v0.1
status: complete (codebase); maintainer rc.1 tag-push pending
created: 2026-04-28
completed: 2026-04-29
phases: 4
plans: 9
commits: ~110
---

# v0.1 Milestone Audit — "Runtime hygiene wedge (the lovable MVP)"

## Phase outcomes

| Phase | Plans | Tests | Hard constraints | Verifier | Reviewer | Fix-pass |
|---|---|---|---|---|---|---|
| 1 — Schema + supervisor substrate | 3 | 28/28 | C1, C2, C5, C6 enforced | PASS | reviewed; all critical+warning fixed | applied |
| 2 — Heartbeat hook (write side) | 2 | 51/51 (28→43→51) | C1, C2, C5, C6 preserved + D-15 hook self-bench | PASS | 11/11 critical+warning fixed | applied |
| 3 — install-hooks UX | 2 | 80/80 | **all 6 enforced** (C3, C4 newly via `tests/cli_dep_boundary.rs`) | PASS | 6/6 critical+warning fixed; 5 info deferred | applied |
| 4 — Distribution + launch | 2 | 80/80 | all 6 preserved | **PARTIAL** (gaps are maintainer-only rc.1 tag-push items) | PASS-WITH-WARNINGS; 3/4 warnings fixed; WR-02 deferred + rationale | applied |

**Workspace test count at milestone exit: 80/80 green.**

## Requirements coverage (28 v1 REQ-IDs)

| Group | Count | Status |
|---|---|---|
| CORE-01..10 | 10 | all implemented (Phase 1) |
| HOOK-01..06 | 6 | all implemented (Phase 2) |
| HOOK-07..10 | 4 | all implemented (Phase 3) |
| HOOK-11 | 1 | implemented (Phase 1 — render path test gate) |
| DIST-01, 03..07 | 6 | implemented (Phase 4) |
| DIST-02 (Homebrew tap) | 1 | **deferred to v0.1.x** per AMENDMENT 2026-04-29 |
| **Total implemented** | **27 / 28** | |
| **Total deferred** | **1 / 28** | trigger criteria documented |

## Hard constraints (C1..C6) — all 6 test-enforced

| ID | Constraint | Test |
|---|---|---|
| C1 | Always pipe stdio when spawning supervised processes | `crates/holt-supervisor/tests/chokepoint_audit.rs` |
| C2 | `holt-render` MUST NOT depend on `holt-supervisor` | `tests/architecture_dag.rs` |
| C3 | `~/.claude/settings.json` mutation: fs2 lock + fsync + rename + .holt.bak | `crates/holt-cli/tests/install_hooks_concurrent.rs` + `install_hooks_sigkill.rs` |
| C4 | JSONC handling lives ONLY in `holt-cli` | `tests/cli_dep_boundary.rs` |
| C5 | Reader treats stale-or-corrupt heartbeat as missing — never panics | `crates/holt-schemas/tests/reader_contract.rs` |
| C6 | Render path never reads `breaches.log` or `timings.jsonl` | `crates/holt-cli/tests/render_path_no_read.rs` |

## Performance gates — all green

| Metric | Target | Actual |
|---|---|---|
| `holt --self-bench` p95 (render path overhead) | < 20ms | **0us** (well under) |
| `holt --self-bench-hook PreToolUse` p95 | < 20ms | **5874us** (3× headroom) |
| `holt install-hooks --dry-run` p95 | < 500ms | within budget |
| Workspace cold compile (`cargo build --workspace --release`) | < 60s | ~7s on macOS arm64 (user dev box) |

## Distribution stack at HEAD

- `dist` v0.31.0 + binstall metadata (auto-gen via dist)
- 4-target matrix: Linux x64 (required), macOS x64 (required), macOS arm64 (required), Windows x64 (continue-on-error)
- SHA-256 sidecars on every artifact + sha256.sum aggregate
- MSRV 1.87 build-only CI matrix (Linux + macOS REQUIRED, Windows ALLOWED-FAILURE)
- Tag-push trigger pattern `v*` (matches v0.1.0-rc.1 SemVer pre-release)
- Per-target `--version` parity check (D-14) wired in `release.yml`
- README leads with vhs-rendered demo gif (`assets/demo.gif`, 377 KB) + 2-command install + platform tier statement linking `docs/02-scope.md` + first-run UX + 9-label routing

## Decision history (highlights)

- **D-08 narrowing (Phase 2):** `git rev-parse` heuristic for `cwd_label` deferred to v1.0; shelling out to git on the render path would violate D-15 budget.
- **D-04 revision (Phase 4, AMENDMENT 2026-04-29):** Homebrew tap dropped from v0.1; deferred to v0.1.x. Trigger criteria for revisiting documented in 3 places (`dist-workspace.toml`, `release.yml` route-finder, `04-CONTEXT.md` AMENDMENT BANNER).

## Maintainer handoff (RC1-CHECKLIST.md, 153 LOC)

Three steps remain to ship v0.1.0-rc.1:

1. **Create the GitHub repo** — `gh repo create Sinderella/holt --public --source . --remote origin --push`. The local repo is currently remote-less.
2. **Apply issue labels** — `./tools/setup-labels.sh` (idempotent, applies the 9 CONTRIBUTING.md labels via `gh label create --force`).
3. **Tag and push the rc.1 release** — bump `Cargo.toml` versions to `0.1.0-rc.1` (workspace + holt-cli), then `git tag v0.1.0-rc.1 && git push origin v0.1.0-rc.1`. The release workflow auto-publishes 4 platform tarballs + sha256s + cargo-binstall-resolvable artifact URLs.

Post-tag verification (also documented in RC1-CHECKLIST.md §7):
- 4 release artifacts uploaded
- `cargo binstall holt --version 0.1.0-rc.1 --dry-run` resolves
- Downloaded binary's `holt --version` prints `holt 0.1.0-rc.1` (D-14 server-side; smoke-checked client-side)
- `gh label list --repo Sinderella/holt --json name --jq '.[].name'` returns the 9 labels

## Outstanding deferrals (not blocking v0.1)

- DIST-02 (Homebrew tap) — v0.1.x; trigger ≥3 Gatekeeper-friction issues OR Apple Developer Program enrollment.
- D-08 branch 2 (cwd_label git rev-parse heuristic) — v1.0; needs `holt doctor` seam.
- Code signing (macOS notarization, Windows EV cert) — v0.1.x+; trigger-gated.
- Cargo install via crates.io — v0.5+; depends on internal-library API stability.

## Files for next-milestone (v0.5) onboarding

The v0.5 milestone is `holt doctor` — the load-tester. The substrate it builds on is locked:

- `crates/holt-schemas` (keystone — `Heartbeat`, `LkgEntry`, `atomic_write`, `read_heartbeat`)
- `crates/holt-supervisor` (process supervision, breach routing, LKG cache)
- `crates/holt-hooks` (heartbeat write side; `HookOutcome` enum)
- `crates/holt-cli` (binary; subcommand dispatch + install-hooks + self-bench gates)
- 6 hard constraints C1..C6 test-enforced — these are the v0.5 invariants

## Verdict

**v0.1 milestone codebase work: COMPLETE.**

The substrate is shipped: a `holt` binary that wraps the user's existing statusLine, supervises it, writes heartbeats, merges into `~/.claude/settings.json` JSONC-tolerant, and ships via `cargo binstall` + GitHub releases on Linux x64 / macOS x64+arm64 / Windows x64 (best-effort).

Three maintainer-side handoff steps remain — they are infrastructure (gh repo create, labels apply, tag push), not code. RC1-CHECKLIST.md documents the maintainer flow end-to-end with rollback procedure.
