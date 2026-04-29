---
phase: 4
plan: 04-01
plan_id: 04-01
status: complete
completed_at: 2026-04-29
requirements:
  - DIST-01
  - DIST-03
  - DIST-05
decisions_implemented:
  - D-01: scaffold via `dist init` (committed verbatim before customization)
  - D-02: scope dist artifact builds to holt-cli only; publish = false on all 6 source crates; [package.metadata.dist] dist = true on holt-cli (added post-tap-drop so publish=false doesn't hide the binary from dist)
  - D-03: 4-target matrix (Linux x64, macOS x64+arm64, Windows x64) with Windows `continue-on-error: ${{ contains(matrix.targets, 'x86_64-pc-windows-msvc') }}`
  - D-04: ~~Homebrew tap = Sinderella/homebrew-holt~~ → DEFERRED to v0.1.x (commit 97a8edd) — see AMENDMENT BANNER on 04-CONTEXT.md
  - D-05: cargo binstall metadata via dist auto-generation (manual fallback documented; binstall dry-run probe deferred to RC1-CHECKLIST since no GitHub release exists yet)
  - D-08: MSRV CI job pinned to 1.87.0, build-only, REQUIRED on Linux+macOS, ALLOWED-FAILURE on Windows
  - D-11: tag-push trigger pattern `v*` (broader than dist's default; v0.1.0-rc.1 matches)
  - D-12: SHA-256 checksums emitted alongside every artifact (dist 0.31 default; verified via `dist plan`)
  - D-13: `crates/holt-cli/tests/version_smoke.rs` asserts `stdout.starts_with(concat!("holt ", env!("CARGO_PKG_VERSION")))`
key_files:
  created:
    - dist-workspace.toml
    - .github/workflows/release.yml
  modified:
    - Cargo.toml (workspace package fields preserved)
    - .github/workflows/ci.yml (MSRV 1.87.0 build-only matrix entry added)
    - crates/holt-cli/Cargo.toml (publish = false; [package.metadata.dist] dist = true)
    - crates/holt-cli/tests/version_smoke.rs (replaced literal "0.1.0" assertion with cargo_pkg_version)
    - crates/holt-hooks/Cargo.toml (publish = false; [package.metadata.dist] dist = false to hide sigkill_test_driver bin)
    - crates/holt-orchestrator/Cargo.toml (publish = false; [package.metadata.dist] dist = false)
    - crates/holt-render/Cargo.toml (publish = false; [package.metadata.dist] dist = false)
    - crates/holt-schemas/Cargo.toml (publish = false; [package.metadata.dist] dist = false)
    - crates/holt-supervisor/Cargo.toml (publish = false; [package.metadata.dist] dist = false)
commits:
  - "5754934 → 6e91044 chore(workspace): inherit repository field on all 6 crates (D-02 dist prereq) [history-rewrite rewrite hash: 6e91044]"
  - "9402092 → 4af436e chore(dist): scaffold via dist init v0.31.0"
  - "459813a → b6ce163 chore(dist): scope artifact builds to holt-cli only (D-02)"
  - "735970e → a622134 chore(dist): trim to 4 targets + mark Windows continue-on-error (D-03)"
  - "b9f09ab → 935a2ed chore(dist): pin Homebrew tap to Sinderella/homebrew-holt (D-04) — REVERTED in 97a8edd"
  - "56e0875 → 2d3787e chore(dist): confirm SHA-256 checksums enabled (D-12)"
  - "2e842e5 → e15cdc0 chore(workspace): publish = false on all 6 crates (D-02)"
  - "b9cf375 → 2b94d1d chore(release): normalize trigger to v* + document customizations (D-11, D-03)"
  - "cd29e4a → b5bbfe8 ci(msrv): add 1.87.0 build-only matrix (D-08)"
  - "e6c9f6e → 59cdb36 fix(holt-cli): unbreak doc_lazy_continuation lint in lock.rs prose (Rule 3)"
  - "5f92481 → 6e35e49 test(version): assert --version against CARGO_PKG_VERSION (D-13)"
  - "97a8edd chore(dist): drop Homebrew tap from v0.1, defer to v0.1.x (D-04 revised)"
  - "83c5ba8 docs(planning): mark Homebrew tap deferred to v0.1.x — amendment banners"
---

# Plan 04-01 Summary: dist scaffold + MSRV CI gate + version smoke test

## Tasks completed

| # | Task | Status | Commit ([redacted]) |
|---|------|--------|----|
| 1 | Pre-flight `dist` install audit (cargo install dist; binary is `dist`, crate is `cargo-dist`) | ✓ | (no commit; documented in next commit body) |
| 2 | `dist init` scaffold + commit verbatim baseline | ✓ | `4af436e` |
| 3a | Customize dist-workspace.toml: scope to holt-cli only | ✓ | `b6ce163` |
| 3b | Customize: trim to 4 targets + Windows continue-on-error | ✓ | `a622134` |
| 3c | Customize: ~~pin Homebrew tap~~ (later reverted) | ✓→reverted | `935a2ed` → reverted by `97a8edd` |
| 3d | Customize: confirm SHA-256 checksums | ✓ | `2d3787e` |
| 4a | publish = false audit on all 6 source crates | ✓ | `e15cdc0` |
| 4b | repository.workspace = true on all 6 crates (Rule-3 prereq for dist) | ✓ | `6e91044` |
| 5 | Sanity-check release.yml trigger + Windows allow-fail | ✓ | `2b94d1d` |
| 6 | Add MSRV 1.87.0 build-only CI job | ✓ | `b5bbfe8` |
| 7 | Replace version_smoke.rs with CARGO_PKG_VERSION assertion (D-13) | ✓ | `6e35e49` |
| 8 | Local `cargo binstall holt --dry-run` smoke (informational) | partial | (deferred to RC1-CHECKLIST — no GitHub release exists yet) |
| 9 | End-of-plan green-light gate: full hard-constraint suite + lint + fmt + self-benches | ✓ | (no commit; verification only — see "Verification" below) |
| follow-up | Drop Homebrew tap from v0.1 (D-04 revised) | ✓ | `97a8edd` |
| follow-up | Amendment banners on planning docs (CONTEXT, plan 04-02, REQUIREMENTS) | ✓ | `83c5ba8` |
| follow-up | Rule-3 fix: doc_lazy_continuation lint in lock.rs prose | ✓ | `59cdb36` |

## Verification (final, post-tap-drop)

| Gate | Result |
|------|--------|
| `cargo build --workspace --release` | exit 0 |
| `cargo test --workspace` | **80/80 pass** (matches Phase 3 baseline; version_smoke is a 1:1 replacement, not addition) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |
| `cargo tree -i tokio` | empty |
| `cargo tree -p holt-render \| grep holt-supervisor` | empty (C2 unbroken) |
| `dist plan` | exit 0 — 4 platform tarballs (Linux x64, macOS x64+arm64, Windows x64), no `.rb` formula, sha256 sidecars + sha256.sum aggregate |
| `holt --self-bench --json` p95 | **0us** (D-15 render-path budget, far under 20ms) |
| `holt --self-bench-hook PreToolUse --json` p95 | **5874us** (D-15 hook budget; flaky on first cold run, stable thereafter) |
| All Phase 1/2/3 constraint tests (architecture_dag, cli_dep_boundary, chokepoint_audit, reader_contract, render_path_no_read, sigkill_atomicity, install_hooks_concurrent, install_hooks_sigkill) | exit 0 |

## Decisions implemented

D-01..D-13 from `04-CONTEXT.md` all landed (D-04 revised mid-plan to defer Homebrew tap to v0.1.x — see follow-ups + 04-CONTEXT.md AMENDMENT BANNER).

### D-04 narrowing — Homebrew tap deferred to v0.1.x

CONTEXT.md D-04 originally specified Homebrew tap auto-publishing via `Sinderella/homebrew-holt`. Mid-plan (2026-04-29 ~13:30), the tap was dropped from v0.1 distribution after the user questioned the value. **Justification:** macOS Gatekeeper friction is the only thing the tap solves at v0.1, and the friction is acceptable for a dev-tooling audience that's already comfortable with terminals. Avoids the commitment to a second public repo. Re-add path documented in `release.yml` route-finder block + `dist-workspace.toml` D-04 deferral comment + AMENDMENT BANNERS on `04-CONTEXT.md` and `04-02-PLAN.md`.

DIST-02 marked deferred-to-v0.1.x in REQUIREMENTS.md. Trigger criteria for revisiting: ≥3 issues filed reporting macOS Gatekeeper friction OR Apple Developer Program enrollment becomes worthwhile.

## Follow-ups (deferred to later phases or v0.1.x)

- **DIST-02 (Homebrew tap)** — deferred to v0.1.x per the in-flight scope change. Documented in REQUIREMENTS.md + AMENDMENT BANNERS.
- **D-08 branch 2** (cwd_label git rev-parse) — deferred from Phase 2 to v1.0; unaffected by Phase 4.
- **`cargo binstall` real dry-run probe** — Plan 04-01 Task 8 attempted the dry-run; without an actual GitHub release the probe 404s. Deferred to RC1-CHECKLIST.md post-tag verification step.

## Notes & gotchas

- **`dist` binary vs `cargo-dist` crate.** The crates.io crate is still named `cargo-dist` v0.31.0 but the installed binary is named `dist`. Documented in commit body of `4af436e`. Install command: `cargo install cargo-dist --locked` → produces `~/.cargo/bin/dist`.
- **`dist init`'s `-t` whitelist semantics.** `dist init -t x86_64-unknown-linux-gnu -t x86_64-apple-darwin -t aarch64-apple-darwin -t x86_64-pc-windows-msvc` ADDS targets to the default set rather than replacing it; the resulting array contained `aarch64-unknown-linux-gnu` (Linux arm64) which is out-of-scope per `docs/02-scope.md`. Task 3b trimmed to the 4 D-03 targets explicitly.
- **`publish = false` blocks dist visibility.** Adding `publish = false` to `holt-cli/Cargo.toml` (D-02) caused `dist plan` to report "no distable binaries". Solved by adding `[package.metadata.dist] dist = true` (commit `97a8edd`) — explicit opt-in for dist while remaining unpublished on crates.io.
- **`repository.workspace = true` on all 6 crates is a `dist generate` prerequisite.** Without it, `dist generate` refused to emit `release.yml` because workspace inheritance of `repository` is not auto-resolved by dist. Hoisted as a Rule-3 fix in commit `6e91044` ahead of Task 4's `publish = false` audit.
- **`sigkill_test_driver` `[[bin]]` leak.** `crates/holt-hooks/Cargo.toml` declares a test driver binary for the Phase 2 D-13 SIGKILL atomicity test. dist 0.31 ships any `[[bin]]` it finds; without `[package.metadata.dist] dist = false` on `holt-hooks`, dist would have published a `holt-hooks-{platform}.tar.gz` artifact for every release. Closed in commit `b6ce163` (Plan 04-01 Task 3a).
- **`allow-dirty = ["ci"]` in dist-workspace.toml.** Protects the hand-edits in `release.yml` (Windows continue-on-error per D-03; `--version` parity check per D-14 added in Plan 04-02; Homebrew job removal comment per D-04 revision) from being clobbered on a deliberate `dist generate --mode ci` run. Future maintainers re-applying generation must temporarily comment out this line, regen, then re-apply the hand-edits — procedure documented in dist-workspace.toml's D-03 comment block.

## git-history-cleanup rewrite (2026-04-29)

99 commits in history were rewritten via `git-history-cleanup` mid-plan to fix a misidentified GitHub username (`Sinderella` → `Sinderella`) and author (Sinderella @ domain → Sinderella @ rawsec@gmail.com). All commit hashes referenced in this summary are POST-rewrite. The [redacted] hashes are preserved as history-rewrite `commit-oid` headers in the new fast-export but are not addressable from `git log`. Backup of [redacted] `.git/` retained at `/tmp/local-backup/git-backup-20260429-132810.tgz` (1.2 MB). Commit signatures (commit signing) were stripped by the rewrite — all 99 commits now show `N` per `git log --pretty='%G?'`. Re-signing deferred; new commits are signed normally.

## Next

Plan 04-02 wires the launch UX: vhs demo gif + README rewrite + setup-labels.sh + release-workflow `--version` parity check + RC1-CHECKLIST.md. 8 tasks, Wave 2.
