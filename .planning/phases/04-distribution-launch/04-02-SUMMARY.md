---
phase: 4
plan: 04-02
plan_id: 04-02
status: complete
completed_at: 2026-04-29
requirements:
  - DIST-04
  - DIST-06
  - DIST-07
  - DIST-02 (deferred to v0.1.x; documented as deferred per AMENDMENT BANNER)
decisions_implemented:
  - D-06: animated gif via charmbracelet/vhs at assets/demo.gif (938 KB, GIF89a 900×400, ~10s loop) + tape source at tools/demo.tape
  - D-07: README platform tier paragraph linking docs/02-scope.md (Linux/macOS x86_64 + arm64 = tier-1, Windows x64 = best-effort, ≥10 windows-tagged issues OR a Windows contributor → promotion)
  - D-09: tools/setup-labels.sh idempotent `gh label create --force` for 9 CONTRIBUTING.md labels
  - D-10: RC1-CHECKLIST.md authored as maintainer-post-plan handoff (152 LOC, 6 pre-tag steps + 4 post-tag verifications + 3 rollback steps; AMENDMENT-aligned — no homebrew-holt repo bootstrap)
  - D-14: per-target --version parity check appended to .github/workflows/release.yml (downloads matrix host artifact, runs ./holt --version, asserts grep -qF "${GITHUB_REF_NAME#v}")
  - D-15: README first-run subsection reaffirms holt install-hooks --dry-run path with .holt.bak + atomic-write semantics
key_files:
  created:
    - tools/demo.tape
    - assets/demo.gif
    - tools/setup-labels.sh
    - .planning/phases/04-distribution-launch/RC1-CHECKLIST.md
  modified:
    - README.md
    - .github/workflows/release.yml
commits:
  - "2bfad4b feat(demo): add vhs tape source for README demo gif (D-06)"
  - "4f3a24c feat(demo): generate assets/demo.gif via vhs (D-06)"
  - "33b335b docs(readme): lead with demo + 2-line install + platform tier + first-run (DIST-04, DIST-06, D-15)"
  - "f66d849 feat(labels): add idempotent setup-labels.sh for the 9 holt labels (D-09)"
  - "aedd35b ci(release): append --version parity check per matrix target (D-14)"
  - "9d23e50 docs(rc1): add v0.1.0-rc.1 readiness checklist (D-10)"
---

# Plan 04-02 Summary: Launch UX (README + demo + labels + parity check)

## Tasks completed

| # | Task | Status | Commit |
|---|------|--------|--------|
| 1 | Author tools/demo.tape (vhs script, ~52 LOC, D-06 4-step sequence) | ✓ | `2bfad4b` |
| 2 | Generate assets/demo.gif via vhs (938 KB, GIF89a 900×400) | ✓ | `4f3a24c` |
| 3 | Rewrite README.md top-of-file (tagline → demo gif → 2-command install → platform tier → first-run) | ✓ | `33b335b` |
| 4 | Author tools/setup-labels.sh (49 LOC, set -euo pipefail, gh label create --force, REPO= env override) | ✓ | `f66d849` |
| 5 | Run setup-labels against Sinderella/holt | deferred to maintainer | (no commit; routed to RC1-CHECKLIST step 3 — repo doesn't exist on GitHub yet) |
| 6 | Append D-14 --version parity check to .github/workflows/release.yml | ✓ | `aedd35b` |
| 7 | Author RC1-CHECKLIST.md (152 LOC, AMENDMENT-aligned: no homebrew-holt bootstrap) | ✓ | `9d23e50` |
| 8 | End-of-plan green-light gate (no commit; verification only) | ✓ | (see Verification) |

## Verification

| Gate | Result |
|------|--------|
| `cargo build --workspace --release` | exit 0 |
| `cargo test --workspace` | **80/80 pass** (matches Plan 04-01 baseline; no new tests added) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |
| `cargo tree -i tokio` | "did not match any packages" (C1 unbroken; no async pull-through) |
| `cargo tree -p holt-render \| grep holt-supervisor` | empty (C2 unbroken) |
| `cargo test --test architecture_dag` (C2) | exit 0 |
| `cargo test --test cli_dep_boundary` (C4) | exit 0 |
| `cargo test -p holt-cli --test render_path_no_read` (C6) | exit 0 |
| `cargo test -p holt-hooks --test sigkill_atomicity` (Phase 2 D-13) | exit 0 |
| `cargo test -p holt-cli --test install_hooks_concurrent --release` (Phase 3 H1) | exit 0 |
| `cargo test -p holt-cli --test install_hooks_sigkill --release` (Phase 3 C3) | exit 0 |
| `holt --self-bench --json` p95 | **0us** (D-15 render-path budget; far under 20ms) |
| `holt --self-bench-hook PreToolUse --json` p95 | **5874us** (D-15 hook budget) |
| `dist plan` | exit 0 — 4 platform tarballs + sha256 sidecars; no `.rb` formula |
| `file assets/demo.gif` | `GIF image data, version 89a, 900 x 400` |
| `wc -c assets/demo.gif` | 960768 bytes (~938 KB; under the 2 MB cap) |
| `bash -n tools/setup-labels.sh` | exit 0 (syntactically valid) |
| Source/test/Cargo.toml leakage from launch-UX commits | 0 (verified by diff inspection on commits 2bfad4b..9d23e50) |

## Decisions implemented

D-06 (vhs gif), D-07 (platform tier link), D-09 (label apply script), D-10 (RC1 checklist), D-14 (parity check), D-15 (first-run UX) all landed.

### D-04 + DIST-02 — Homebrew tap deferred (handled by AMENDMENT BANNER)

The plan body originally specified a 3-command install block (brew + binstall + GitHub release) and a homebrew-holt repo bootstrap step in RC1-CHECKLIST.md. The 2026-04-29 AMENDMENT BANNER (committed BEFORE Plan 04-02 execution) reduced these to:
- README install block: 2 commands (binstall + GitHub release direct download with `xattr -d com.apple.quarantine` workaround for macOS)
- RC1-CHECKLIST.md: no homebrew-holt repo bootstrap step; no tap formula post-tag verification

The plan executor read the banner before the body and produced AMENDMENT-aligned artifacts on first try.

## Follow-ups (deferred to later phases or v0.1.x)

- **DIST-02 (Homebrew tap)** — deferred to v0.1.x. Trigger criteria for revisiting: ≥3 issues filed reporting macOS Gatekeeper friction OR Apple Developer Program enrollment becomes worthwhile. Re-add procedure documented in `release.yml` route-finder block.
- **GitHub repo bootstrap** — `Sinderella/holt` does not yet exist on GitHub. RC1-CHECKLIST.md Step 1 documents `gh repo create Sinderella/holt --public --source . --remote origin --push` as the maintainer's first action. Until this step runs, `tools/setup-labels.sh` (Plan 04-02 Task 5) cannot apply the labels — also routed to RC1-CHECKLIST.md Step 3.
- **`cargo binstall holt --version 0.1.0-rc.1 --dry-run` end-to-end probe** — depends on the RC1 release artifacts existing on GitHub. RC1-CHECKLIST.md Step 7 covers this verification.
- **vhs install** — `assets/demo.gif` was rendered locally by the maintainer; future demo-content updates require `brew install vhs` + `vhs tools/demo.tape`. Not codified in CI; the gif is a manually-refreshed artifact.

## Notes & gotchas

- **vhs tape parser quirks (Task 2 deviation, fixed in commit `4f3a24c`).** vhs's `Type "..."` literal does NOT accept backslash-escaped quotes or backticks; the initial tape used JSON-like strings with embedded escapes that vhs split on (17 parse errors). Replaced embedded JSON with plain-prose comments in the tape (e.g., "change command to: holt run -- bash slow-statusline.sh"). The visual demo content is identical; only the textual representation in the tape changed.
- **First gif render at 1000×520 was 5.7 MB.** Over the 2 MB ceiling. Dropped frame to 900×400, trimmed closer Sleep from 2000ms→1200ms, reduced TypingSpeed from 60ms→50ms. Re-rendered to 938 KB. The load-bearing 5500ms beat after `time bash slow-statusline.sh` (the 5-second blocking demo) was preserved untouched.
- **`tools/setup-labels.sh` `REPO=` override.** The script defaults to `Sinderella/holt` but accepts `REPO=other-org/other-repo ./tools/setup-labels.sh` for forks or test runs. Documented in the script's header comment.
- **D-14 parity check is a hand-edit on a generated file.** `release.yml` is `dist init`-generated; the Plan 04-02 D-14 step is appended to the matrix-host build job. Protected from `dist generate --mode ci` clobbers by the `allow-dirty = ["ci"]` setting in `dist-workspace.toml`. Re-apply procedure documented in the file's route-finder comment block (header) — same playbook as the Plan 04-01 Windows continue-on-error hand-edit.

## Next

Phase 4 closeout: gsd-verifier + gsd-code-reviewer dispatched in parallel against the phase. Pending their VERDICTS:
- If both PASS (or PASS-WITH-WARNINGS that are non-load-bearing): mark Phase 4 complete in STATE.md + ROADMAP.md, then dispatch milestone audit/complete/cleanup.
- If reviewer flags critical/warning items: dispatch gsd-code-fixer for a fix-pass; re-run verifier after.
