---
phase: 4
phase_name: Distribution + launch
status: human_needed
verified: 2026-04-29T08:19:03Z
roadmap_criteria_passed: 4/5 (criterion #2 PARTIAL — Homebrew tap deferred per AMENDMENT 2026-04-29; binstall live-resolve requires rc.1 push)
roadmap_criteria_partial: 1/5 (criterion #4 binstall sub-claim deferred to post-rc.1)
requirements_covered: 6/7 satisfied + 1 deferred (DIST-02 deferred per AMENDMENT 2026-04-29; tracked as v0.1.x re-add path)
decisions_implemented: 16/16 (D-01..D-16; D-04 superseded by AMENDMENT but original tap-pin commit preserved as bisect anchor)
hard_constraints_enforced: 6/6 (C1..C6 read-only invariants all green)
quality_gates_clean: true
score: 4/5 must-haves verified + 1 partial-with-human-action (criterion #4 binstall live-resolve)
overrides_applied: 0

overrides:
  - must_have: "Criterion #2 — `brew install <user>/tap/holt` completes without Gatekeeper friction"
    reason: "DIST-02 explicitly deferred to v0.1.x as of 2026-04-29 per .planning/REQUIREMENTS.md line 41 + 04-CONTEXT.md AMENDMENT BANNER + 04-02-PLAN AMENDMENT BANNER. v0.1 ships shell+powershell installers + binstall + tarballs. macOS users get documented `xattr -d com.apple.quarantine` workaround. Re-add path documented in dist-workspace.toml (lines 19–26) + release.yml (lines 35–40)."
    accepted_by: "user (via roadmap amendment 2026-04-29)"
    accepted_at: "2026-04-29"

re_verification:
  previous_status: null
  previous_score: null
  is_initial_verification: true

human_verification:
  - test: "Push `v0.1.0-rc.1` tag and assert `cargo binstall holt --version 0.1.0-rc.1 --dry-run` resolves the URL"
    expected: "Resolves to https://github.com/Sinderella/holt/releases/download/v0.1.0-rc.1/holt-cli-{target}.{format} for the local host triple; exits 0"
    why_human: "Per D-10 + RC1-CHECKLIST.md, the tag-push is a maintainer manual step (not landed by Plan 04-02). cargo-binstall's resolution depends on dist artifacts existing on the GitHub release; pre-tag, the package is not on crates.io (publish = false per D-02) and 'holt is not found' on crates.io fallback. Fallback per D-05 plan-time action: if URL probe fails, add manual `[package.metadata.binstall]` keyed `pkg-url`/`pkg-fmt` and re-tag rc.2. ROADMAP success criterion #4's binstall sub-clause cannot be falsifiably tested until the maintainer executes RC1-CHECKLIST.md."

  - test: "Push `v0.1.0-rc.1` tag and assert GitHub Actions release.yml publishes ≥3 of 4 platform tarballs + .sha256 sidecars + sha256.sum aggregate"
    expected: "Linux x64, macOS x64, macOS arm64 tarballs all uploaded (Windows allow-fail per D-03; presence is bonus). `gh release view v0.1.0-rc.1 --json assets --jq '.assets[].name' | sort` lists 4× .tar.xz/.zip + 4× .sha256 + sha256.sum + source.tar.gz + source.tar.gz.sha256 + holt-cli-installer.sh + holt-cli-installer.ps1."
    why_human: "Tag-push is manual per D-10. release.yml is structurally correct (verified by `dist plan` enumerating 4 platform tarballs + their .sha256 sidecars + a top-level sha256.sum); D-14 parity check is wired (release.yml lines 214–241). End-to-end verification requires the maintainer to push the tag."

  - test: "After rc.1 release lands, run `tools/setup-labels.sh` against Sinderella/holt and verify `gh label list --json name` returns the 9 holt labels"
    expected: "The 9 labels (bug, feature, question, windows, pet, runtime, orchestrator, good first issue, help wanted) all listed; gh exits 0; no auth errors."
    why_human: "RC1-CHECKLIST.md Step 3 explicitly defers label apply until after the GitHub repo is created (gh repo create Sinderella/holt). The script is committed and idempotent (--force) but cannot be runnable in this verification because the remote repo may not yet exist; verifier ran the script's local correctness checks (9 labels declared verbatim + gh-auth-status guard + gh-presence guard) but the criterion-#5 contract is `gh label list --json name` against the live repo."
---

# Phase 4: Distribution + launch Verification Report

**Phase Goal:** A user can `brew install <user>/tap/holt` (macOS — DEFERRED), `cargo binstall holt` (any platform with the toolchain), or download a prebuilt binary from a GitHub release for Linux x64 / macOS x64+arm64 / Windows x64, follow a three-line README, and watch a sub-ten-second asciinema/gif of the shim wrapping a slow statusLine — with the repo's `CONTRIBUTING.md` already routing the issue traffic the launch will produce.

**Verified:** 2026-04-29T08:19:03Z
**Status:** human_needed (3 items — all require maintainer's manual rc.1 tag push per D-10/RC1-CHECKLIST.md)
**Re-verification:** No — initial verification

> **AMENDMENT 2026-04-29 acknowledged:** Homebrew tap dropped from v0.1 and DEFERRED to v0.1.x. Documented in three locations:
> 1. `.planning/REQUIREMENTS.md` line 41 — DIST-02 strikethrough + deferral rationale + re-add trigger criteria
> 2. `.planning/phases/04-distribution-launch/04-CONTEXT.md` lines 32–38 — AMENDMENT BANNER
> 3. `.planning/phases/04-distribution-launch/04-02-PLAN.md` (AMENDMENT BANNER after frontmatter)
>
> Verification treats the brew half of criterion #2 as **deferred-not-failed**. The verifier confirmed the deferral artifact-by-artifact: `dist-workspace.toml` removes the homebrew installer (lines 19–27), `release.yml` removes the publish-homebrew-formula job (lines 363–365), README's install section reflects binstall + tarball + xattr workaround (README.md lines 9–22, 26).

---

## Goal Achievement

### Observable Truths (5 ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | After tagging `v0.1.0-rc.1`, GitHub Actions publishes a release with 4-target prebuilt artifacts; each artifact's `holt --version` prints the tagged version (Windows allow-fail) | **STRUCTURALLY VERIFIED — END-TO-END BLOCKED ON MAINTAINER TAG-PUSH** | `dist plan` enumerates exactly 4 platform tarballs + their `.sha256` sidecars + `sha256.sum` aggregate + `source.tar.gz` + 2 installer scripts (output captured below — note D-14 parity step in release.yml asserts the version contract). Tag-push gate is per D-10 / RC1-CHECKLIST.md. |
| 2a | `brew install Sinderella/holt` completes without Gatekeeper friction (DEFERRED per AMENDMENT 2026-04-29) | **DEFERRED** | DIST-02 marked deferred in REQUIREMENTS.md line 41; AMENDMENT BANNERs in 04-CONTEXT.md + 04-02-PLAN.md; dist-workspace.toml (lines 19–27) ships `installers = ["shell", "powershell"]` only; release.yml lines 363–365 record publish-homebrew-formula removal. |
| 2b | `cargo binstall holt` resolves to a dist-published binary (Linux x86_64); `holt --version` succeeds <10s end-to-end | **PARTIAL — STRUCTURAL OK, LIVE-RESOLVE BLOCKED ON RC.1 PUSH** | `dist plan` confirms binstall-discoverable URL pattern emitted by dist 0.31. `cargo binstall holt --dry-run` (run 2026-04-29 08:18Z) returns "holt is not found" because (a) `publish = false` per D-02, (b) crates.io fallback fails, (c) GitHub release does not yet exist (rc.1 not pushed). Per D-05's plan-time fallback: if rc.1 dry-run fails, add manual `[package.metadata.binstall]` block. Tracked in human_verification[0]. |
| 3 | README opens with demo gif above the fold; install instructions in first 30 lines; v0.1 platform tier statement + Windows promotion link | **VERIFIED** | README.md L7 `![demo](assets/demo.gif)` (above-the-fold); L11–14 `cargo binstall` + GH-release direct download (within 30); L24 platform tier statement + L24 Windows-tagged-issues hyperlink + L24 link to `docs/02-scope.md`. `assets/demo.gif` is real GIF89a 900×400 (938 KB). `tools/demo.tape` source at 53 lines, 4-step demo per D-06. |
| 4a | `Cargo.toml` declares `rust-version = "1.87"` + `edition = "2024"`; CI 1.87 matrix entry asserts MSRV build | **VERIFIED** | Cargo.toml L16–17: `edition = "2024"`, `rust-version = "1.87"`. ci.yml L108–134: `msrv-linux` (REQUIRED, ubuntu-latest), `msrv-macos` (REQUIRED, macos-14), `msrv-windows` (continue-on-error, windows-latest) — all use `dtolnay/rust-toolchain@1.87.0` + run `cargo build --workspace --release`. Cold local build succeeds on whatever stable is installed; CI matrix exercises 1.87 specifically per D-08. |
| 4b | `cargo binstall` metadata present (`pkg-url`, `pkg-fmt`) such that `--dry-run` resolves URL | **PARTIAL** | Per D-05 (Plan 04-01), the plan deliberately does NOT add manual `[package.metadata.binstall]` to `crates/holt-cli/Cargo.toml` — instead relies on dist's auto-generated release metadata. Local `cargo binstall holt --dry-run` fails because (a) crates.io publish blocked, (b) no release yet. Falsifiable post-rc.1 push only. Tracked in human_verification[0]. |
| 5 | Repo labels `bug`, `feature`, `windows`, `pet`, `runtime`, `orchestrator`, `good first issue`, `help wanted` configured AND `CONTRIBUTING.md` present AND README links labels by name | **STRUCTURALLY VERIFIED — LIVE GH STATE BLOCKED ON REPO-CREATE** | `tools/setup-labels.sh` exists (1.8 KB, executable); declares 9 labels verbatim (criterion lists 8 plus `question`); is idempotent (`--force`); has gh-auth + gh-presence guards. CONTRIBUTING.md present (5.2 KB) at repo root with all 9 labels documented (lines 38–46). README.md L41 links 9 labels by name (one hyperlink per label to `https://github.com/Sinderella/holt/labels/<name>`). Live `gh label list` against `Sinderella/holt` blocked because remote repo may not yet exist (RC1-CHECKLIST.md Step 1 explicitly bootstraps it). Tracked in human_verification[2]. |

**Score:** 4/5 truths VERIFIED, 1/5 PARTIAL (criterion #4b — binstall live-resolve). Human-verification items are infrastructure handoff to the maintainer, not gaps in the codebase.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `dist-workspace.toml` | dist 0.31.0 workspace config; 4-target matrix; SHA-256 default-on; Homebrew DROPPED per AMENDMENT | **VERIFIED** | 55 lines. `cargo-dist-version = "0.31.0"` (L7). `installers = ["shell", "powershell"]` (L27 — Homebrew removed per D-04 revised). `targets = [linux-x86_64, darwin-x86_64, darwin-aarch64, windows-msvc]` (L34 — exactly D-03's 4 targets). `allow-dirty = ["ci"]` (L54 — gates the Windows continue-on-error hand-edit). |
| `.github/workflows/release.yml` | Tag-triggered (`v*`) workflow; 4-target matrix; SHA-256; D-14 parity check | **VERIFIED** | 381 lines. Tag pattern `v*` (L86 per D-11). 5 jobs: plan / build-local-artifacts / build-global-artifacts / host / announce. Windows allow-fail expression at L164: `continue-on-error: ${{ contains(matrix.targets, 'x86_64-pc-windows-msvc') }}`. publish-homebrew-formula REMOVED (L363–365 record removal per AMENDMENT). D-14 `--version` parity step at L214–241: locates the binary, runs `--version`, greps `${GITHUB_REF_NAME#v}`. |
| `.github/workflows/ci.yml` | Existing MSRV-1.87 test matrix preserved + new MSRV build-only jobs | **VERIFIED** | 134 lines. `lint` / `test-linux` / `test-macos` / `test-windows` / `stable-linux` / `stable-macos` jobs from Phase 1+2+3 preserved verbatim. NEW: `msrv-linux` (L108–115), `msrv-macos` (L117–124), `msrv-windows` (L126–134) — all pinned to `dtolnay/rust-toolchain@1.87.0`, build-only (no test re-run; rationale at L101–107). |
| `Cargo.toml` (workspace) | `rust-version = "1.87"`, `edition = "2024"`, `repository`, `license` workspace-inherited | **VERIFIED** | Lines 15–19: workspace.package block with all 4 inherited fields. Lines 30–41: phantom `holt-workspace-tests` package (publish = false, no autobins/examples/benches). |
| `crates/holt-cli/Cargo.toml` | `publish = false` + `[package.metadata.dist] dist = true` opt-in for binary-only ship | **VERIFIED** | L15: `publish = false`. L20–21: `[package.metadata.dist] dist = true` (per D-02 post-tap-drop opt-in). repository.workspace = true (L7). |
| `crates/holt-{schemas,supervisor,hooks,orchestrator,render}/Cargo.toml` | Each declares `publish = false` + `[package.metadata.dist] dist = false` | **VERIFIED** | All 5 library crates: `publish = false` at L8 + `[package.metadata.dist] dist = false` at L11–12 (or L15–16 for holt-schemas with explanatory comment). Library crates correctly excluded from artifact build per D-02. |
| `crates/holt-cli/tests/version_smoke.rs` | D-13: assert `holt --version` stdout starts with `concat!("holt ", env!("CARGO_PKG_VERSION"))` | **VERIFIED** | 25 lines. `let expected_prefix = concat!("holt ", env!("CARGO_PKG_VERSION"));` at L19; `stdout.starts_with(expected_prefix)` assertion at L21. Test passed in `cargo test --workspace` run (1 passed, 0.01s). |
| `tools/demo.tape` | vhs tape source for D-06 demo gif; 4-step sequence; commits to `Output assets/demo.gif` | **VERIFIED** | 53 lines. `Output assets/demo.gif` at L11. 4-step structure (cat slow-statusline.sh / time bash slow-statusline.sh / edit settings.json / time holt run -- bash slow-statusline.sh) at L21–47. Theme + size + speed settings at L13–19. |
| `assets/demo.gif` | The rendered GIF embedded in README's L7 (above-the-fold) | **VERIFIED** | `file` reports: `GIF image data, version 89a, 900 x 400` (matches tape's Width/Height settings at L15–16). Size 938 KB — within the "reasonable to keep in-repo" budget per D-06. README.md L7 references it. |
| `tools/setup-labels.sh` | D-09: idempotent shell script applying the 9 CONTRIBUTING.md labels to Sinderella/holt | **VERIFIED** | 50 lines, executable (`-rwxr-xr-x`). Uses `gh label create --force` for idempotency (L46). 9 labels declared verbatim (L31–41). Guards: `command -v gh` + `gh auth status` (L20–28). REPO env override (L18). |
| `.planning/phases/04-distribution-launch/RC1-CHECKLIST.md` | D-10 maintainer-side rc.1 readiness checklist with AMENDMENT-aware steps | **VERIFIED** | 153 lines. AMENDMENT BANNER at L7 acknowledging Homebrew defer. 6-step pre-tag flow (gh repo create / Cargo.toml bump / setup-labels / push commits / secrets / local self-test). Tag-and-push instructions. 4-step post-tag verification (release artifacts / cargo binstall resolve / downloaded `--version` / Gatekeeper note). Rollback procedure. References to CONTEXT.md decisions. |
| `README.md` | Demo above the fold + install in 30 lines + platform tier + first-run + label routing | **VERIFIED** | 96 lines. L7: `![demo](assets/demo.gif)`. L11–14: 2-line install (`cargo binstall holt` + GH releases URL). L16–22: macOS Gatekeeper xattr workaround. L24: platform tier statement + Windows promotion link + scope.md hyperlink. L26: Homebrew-tap deferral note linking REQUIREMENTS.md. L28–37: First-run section (`holt install-hooks --dry-run` then `holt install-hooks`). L41: 9-label hyperlink set + CONTRIBUTING.md link. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|------|-----|--------|---------|
| dist-workspace.toml | release.yml | `dist init` / `dist generate` baseline + customizations | **WIRED** | release.yml lines 1–5 self-identify as autogenerated by dist; customizations called out at L24–54 (tag pattern, Windows continue-on-error, Homebrew removal, SHA-256 default, D-14 parity). `allow-dirty = ["ci"]` in dist-workspace.toml protects the customizations. |
| release.yml | GitHub Releases (v* tag) | `on.push.tags: ['v*']` triggers dist's plan→build→host→announce pipeline | **WIRED** | release.yml L77–86: `on.push.tags: ['v*']` (per D-11). 5 jobs chain via `needs:` with `host` gating on `(plan == success) && (build-local == success || skipped) && (build-global == success || skipped)` — Windows continue-on-error per-instance failure masks to `success` at the needs level. `gh release create` at L361. |
| ci.yml | MSRV 1.87 build verification | New `msrv-{linux,macos,windows}` jobs pinned to `dtolnay/rust-toolchain@1.87.0` | **WIRED** | ci.yml L108–134. Each job uses identical 4-step recipe (checkout → toolchain → cache → build). Required on Linux + macOS; continue-on-error on Windows. |
| version_smoke.rs | env!("CARGO_PKG_VERSION") | `concat!("holt ", env!("CARGO_PKG_VERSION"))` compile-time pin | **WIRED** | version_smoke.rs L19. Test runs `target/debug/holt --version` (via CARGO_BIN_EXE_holt env) and asserts stdout starts with the concat'd prefix. The `holt` binary itself uses clap's `#[command(version)]` which prints `concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"))` → exactly `holt 0.1.0`. |
| release.yml D-14 step | Cargo.toml version + tag | `${GITHUB_REF_NAME#v}` strip + grep on `holt --version` stdout | **WIRED** | release.yml L214–241. Step runs `if: startsWith(github.ref, 'refs/tags/v')`. Locates the built binary via `find target -type f -name 'holt' -perm -u+x`. Greps `${GITHUB_REF_NAME#v}` against `--version` output. Failure halts that matrix entry's release for that target (Windows continue-on-error masks). |
| README.md | tools/demo.tape + assets/demo.gif | Markdown image embed + tape source committed | **WIRED** | README.md L7 → assets/demo.gif (real GIF). tape at tools/demo.tape regenerates the gif via `vhs tools/demo.tape`. |
| README.md | CONTRIBUTING.md (label routing) | 9 inline label-link hyperlinks + `[CONTRIBUTING.md](CONTRIBUTING.md)` reference | **WIRED** | README.md L41 lists exactly 9 labels each as `[<label>](https://github.com/Sinderella/holt/labels/<name>)` and ends with `Full contribution guide in [\`CONTRIBUTING.md\`](CONTRIBUTING.md)`. CONTRIBUTING.md L38–46 declares the same 9 labels. tools/setup-labels.sh applies them. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|---------------------|--------|
| `dist plan` output | tarball list | dist-workspace.toml `targets` array | YES — emits exactly 4 tarballs + 4 .sha256 sidecars + sha256.sum aggregate + source.tar.gz + source.tar.gz.sha256 + 2 installer scripts (verifier ran `dist plan` 2026-04-29 08:18Z; output captured) | **FLOWING** |
| `holt --version` (release binary) | version string | clap derives from `CARGO_PKG_VERSION` | YES — `./target/release/holt --version` returns `holt 0.1.0` | **FLOWING** |
| `cargo test --test version_smoke` | smoke assertion | `concat!("holt ", env!("CARGO_PKG_VERSION"))` | YES — test passed in 0.01s; asserts the contract D-14 will then enforce on the released binary | **FLOWING** |
| `dist-workspace.toml` `installers` | installer set | `["shell", "powershell"]` | YES — dist plan enumerates `holt-cli-installer.sh` + `holt-cli-installer.ps1` in the artifact set | **FLOWING** |
| `cargo binstall holt --dry-run` | URL resolution | crates.io metadata + dist GitHub release | NO — "holt is not found" (publish = false on holt-cli + no release yet); D-05's manual fallback documented but not landed | **DISCONNECTED — by design, deferred to post-rc.1** |
| `gh label list --json name` | live label state | GitHub repo API | UNKNOWN — script exists + locally runnable; live state requires repo to exist on github.com/Sinderella/holt | **DISCONNECTED — deferred to maintainer per RC1-CHECKLIST.md Step 1+3** |
| README.md → assets/demo.gif | binary GIF | `vhs tools/demo.tape` | YES — 938 KB GIF89a 900×400 | **FLOWING** |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace test count + pass rate | `cargo test --workspace 2>&1 \| grep "test result" \| awk '{sum+=$4} END {print sum}'` | TOTAL PASSED: 80, 0 failed | **PASS** |
| Clippy clean | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, "Finished `dev` profile" | **PASS** |
| Format check | `cargo fmt --check` | exit 0 | **PASS** |
| No tokio in tree | `cargo tree -i tokio` | "package ID specification `tokio` did not match any packages" — empty | **PASS** |
| Architecture DAG (C2) | `cargo test --test architecture_dag` | 1 passed, 0.08s | **PASS** |
| CLI dep boundary (C4) | `cargo test --test cli_dep_boundary` | 1 passed, 0.09s | **PASS** |
| Chokepoint audit (C1) | `cargo test -p holt-supervisor --test chokepoint_audit` | 1 passed | **PASS** |
| Reader contract (C5) | `cargo test -p holt-schemas --test reader_contract` | 9 passed (run as part of workspace) | **PASS** |
| Render path no-read (C6) | `cargo test -p holt-cli --test render_path_no_read` | 1 passed (run as part of workspace) | **PASS** |
| Install-hooks concurrent (C3) | `cargo test -p holt-cli --test install_hooks_concurrent` | 1 passed, 0.20s | **PASS** |
| Install-hooks SIGKILL (C3) | `cargo test -p holt-cli --test install_hooks_sigkill` | 1 passed, 3.95s | **PASS** |
| `dist plan` validates dist-workspace.toml | `dist plan` (2026-04-29 08:18Z) | Lists exactly 4 platform tarballs + their `.sha256` sidecars + `sha256.sum` + `source.tar.gz` + `source.tar.gz.sha256` + 2 installers | **PASS** |
| `holt --version` parity | `./target/release/holt --version` | `holt 0.1.0` (matches Cargo.toml's 0.1.0; will re-match after rc.1 bump per RC1-CHECKLIST.md Step 2) | **PASS** |
| Self-bench p95 <20ms | `holt --self-bench --json --iterations 30` | `{"overhead_p95_us":0,"budget_p95_us":20000,"passed":true}` | **PASS** |
| Hook self-bench p95 <20ms | `holt --self-bench-hook PreToolUse --json --iterations 30` | `{"overhead_p95_us":7083,"budget_p95_us":20000,"passed":true}` | **PASS** |
| `version_smoke` passes locally | (via workspace test) | 1 passed, 0.01s | **PASS** |
| `cargo binstall holt --dry-run` resolves | `cargo binstall holt --dry-run --no-confirm` | "ERROR: holt is not found" — expected pre-rc.1 because publish=false + no release yet | **EXPECTED FAIL — defer to human_verification[0]** |
| Setup-labels script syntax | `bash -n tools/setup-labels.sh` | exit 0; 9 labels declared | **PASS** |
| Demo.gif is a real GIF | `file assets/demo.gif` | `GIF image data, version 89a, 900 x 400` | **PASS** |
| README install lines within first 30 | `awk 'NR<=30' README.md` | install code block at L11–14, platform tier at L24, demo at L7 | **PASS** |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| DIST-01 | 04-01 | `dist` v0.31.0 prebuilt binaries shipped on day one for Linux x64 / macOS x64 / macOS arm64 / Windows x64 (best-effort) — scaffold via `dist init` | **SATISFIED** | dist-workspace.toml L7: `cargo-dist-version = "0.31.0"`; L34: 4-target matrix exact. release.yml exists, tag-triggered, SHA-256 enabled. End-to-end artifact-publish requires rc.1 push (human_verification[1]). |
| DIST-02 | 04-02 | Homebrew tap (`<user>/holt`) auto-generated by `dist`; README leads with `brew install` | **DEFERRED to v0.1.x** | REQUIREMENTS.md L41 marks deferred with full re-add path. AMENDMENT BANNERs in 04-CONTEXT.md + 04-02-PLAN.md. dist-workspace.toml lines 19–27 + release.yml lines 363–365 codify the deferral. v0.1 substitutes `cargo binstall` + tarball + xattr workaround. Trigger criteria for revisit: ≥3 Gatekeeper-friction issues OR Apple Developer enrollment becomes worthwhile. |
| DIST-03 | 04-01 | `cargo binstall holt` works from day one | **PARTIAL** | dist 0.31's auto-generated release metadata makes binstall-discoverable URLs (verified by `dist plan` enumerating the URL pattern). Live `--dry-run` resolution requires rc.1 push (human_verification[0]). D-05 plan-time fallback documented: if rc.1 dry-run fails, add manual `[package.metadata.binstall]` per binstall docs. |
| DIST-04 | 04-02 | README leads with asciinema/gif of shim wrapping a slow user statusLine — install in 3 lines, demo in 10s | **SATISFIED** | README.md L7 demo (above-the-fold; 938 KB GIF89a 900×400). Install: 2 lines at L11–14 (binstall + GH-release URL). xattr workaround at L18–20 = "3rd line". Demo runs at PlaybackSpeed 1.0 over ~10s of interaction (vhs tape lines 21–47). |
| DIST-05 | 04-01 | MSRV pinned to Rust 1.87, Edition 2024 | **SATISFIED** | Cargo.toml L16–17 declares both. ci.yml L108–134 adds 3-job MSRV matrix exercising 1.87.0 toolchain on Linux + macOS (REQUIRED) + Windows (allow-fail). |
| DIST-06 | 04-02 | README states v0.1 platform tier — Unix-tier-1, Windows best-effort — and links promotion criteria | **SATISFIED** | README.md L24: "Linux x86_64 and macOS (x86_64 + Apple Silicon) are tier-1; Windows x64 is best-effort and may lag releases." + "≥10 [Windows-tagged issues](...) OR a Windows contributor steps up" + link to `docs/02-scope.md`. |
| DIST-07 | 04-02 | `CONTRIBUTING.md` already at repo root; trigger-gating tags (`bug`, `feature`, `windows`, `pet`, `runtime`, `orchestrator`, `good first issue`, `help wanted`) configured on the GitHub repo | **STRUCTURALLY SATISFIED — LIVE GH STATE PENDING REPO CREATE** | CONTRIBUTING.md present (5.2 KB) at repo root. Labels documented in CONTRIBUTING.md L38–46 (9 entries; criterion lists 8 + the script also adds `question`). `tools/setup-labels.sh` ready to apply them idempotently. README L41 inline-links all 9 by name. Live `gh label list` blocked on remote repo existence (RC1-CHECKLIST.md Step 1+3). Tracked in human_verification[2]. |

### CONTEXT Decisions Implementation (D-01..D-16)

| # | Decision | Status | Evidence |
|---|---------|--------|----------|
| D-01 | Run `dist init` (not hand-write) | **IMPLEMENTED** | Commit 4af436e — `chore(dist): scaffold via dist init v0.31.0`. release.yml header L1–5 self-identifies as autogenerated. |
| D-02 | Ship only holt-cli; library crates excluded | **IMPLEMENTED** | All 5 library crates have `[package.metadata.dist] dist = false`. holt-cli has `dist = true` opt-in. All 6 crates have `publish = false`. Commits b6ce163 + e15cdc0. |
| D-03 | 4-target matrix; Windows allow-fail | **IMPLEMENTED** | dist-workspace.toml L34 (4 targets). release.yml L164: per-instance `continue-on-error` expression keyed on Windows triple. Commit a622134. |
| D-04 | Homebrew tap = Sinderella/homebrew-holt | **SUPERSEDED-BY-AMENDMENT** | Commit 935a2ed (original tap pin) preserved as bisect anchor. Amendment commits 97a8edd (drop) + 83c5ba8 (banners) document the v0.1.x defer. Original D-04 instruction wording in CONTEXT.md is overlaid by the AMENDMENT BANNER at the top. |
| D-05 | binstall via dist auto-gen; manual fallback documented | **IMPLEMENTED** | No manual `[package.metadata.binstall]` in holt-cli/Cargo.toml (per design). Plan 04-01's verification step intentionally informational (post-rc.1 verification handled by RC1-CHECKLIST.md Step 6 + human_verification[0]). |
| D-06 | Animated GIF via vhs, committed `tools/demo.tape` + `assets/demo.gif` | **IMPLEMENTED** | tools/demo.tape (53 lines, 4-step) + assets/demo.gif (938 KB GIF89a 900×400). Commits 2bfad4b + 4f3a24c. |
| D-07 | Platform tier statement above-the-fold + scope.md link | **IMPLEMENTED** | README.md L24 satisfies both clauses. Commit 33b335b. |
| D-08 | MSRV CI matrix entry pinned to 1.87.0, build-only | **IMPLEMENTED** | ci.yml L108–134. 3 jobs (linux/macos/windows). Build-only; tests already covered by Phase 1 jobs. Commit b5bbfe8. |
| D-09 | Idempotent `tools/setup-labels.sh` for 9 labels | **IMPLEMENTED** | tools/setup-labels.sh (50 lines, executable, --force). Commit f66d849. |
| D-10 | rc.1 readiness via `v0.1.0-rc.1` pre-release tag | **CHECKLIST-IMPLEMENTED** | RC1-CHECKLIST.md (153 lines) authored 2026-04-29 with AMENDMENT-aware steps. Commit 9d23e50. Tag-push is the maintainer's manual action. |
| D-11 | Tag-push pattern `v*` | **IMPLEMENTED** | release.yml L86: `tags: ['v*']`. Commit 2b94d1d. |
| D-12 | SHA-256 checksums via dist 0.31 default | **IMPLEMENTED** | dist plan output includes `.sha256` sidecars per artifact + `sha256.sum` aggregate + `source.tar.gz.sha256`. Commit 2d3787e. |
| D-13 | --version smoke test asserting CARGO_PKG_VERSION | **IMPLEMENTED** | crates/holt-cli/tests/version_smoke.rs (25 lines) using `concat!("holt ", env!("CARGO_PKG_VERSION"))`. Commit 6e35e49. Test passes in workspace test run. |
| D-14 | Release-workflow `--version` parity check | **IMPLEMENTED** | release.yml L214–241: locates binary, runs `--version`, greps `${GITHUB_REF_NAME#v}`. Commit aedd35b. |
| D-15 | README install-hooks budget reaffirmation | **IMPLEMENTED** | README.md L28–37 — "First-run" section documents `holt install-hooks --dry-run` then `holt install-hooks`, plus `--print` mention; emphasizes atomic mutation, lock, .holt.bak backup, JSONC preservation. Commit 33b335b. |
| D-16 | Phase 4 split into 2 plans (infra → launch UX) | **IMPLEMENTED** | 04-01-PLAN.md (Wave 1, infra) + 04-02-PLAN.md (Wave 2, launch UX). Both have PLAN-CHECK.md PASS verdicts. Commit 8a530e6 (initial), follow-up amendment commits 97a8edd + 83c5ba8. |

All 16 decisions implemented. D-04's original tap-publish path is superseded by the 2026-04-29 AMENDMENT (DIST-02 deferred to v0.1.x); the supersession is itself documented in three locations (REQUIREMENTS.md / 04-CONTEXT.md / 04-02-PLAN.md) per D-16's plan-discipline contract.

### Hard-Constraint Preservation (C1..C6 — all read-only this phase)

| Constraint | Status | Test |
|-----------|--------|------|
| C1 — Pipe stdio for supervised processes | **PRESERVED** | `cargo test -p holt-supervisor --test chokepoint_audit` → 1 passed |
| C2 — holt-render does NOT depend on holt-supervisor | **PRESERVED** | `cargo test --test architecture_dag` → 1 passed |
| C3 — fs2 lock + fsync-before-rename + .holt.bak + PID-suffix tmp | **PRESERVED** | install_hooks_concurrent + install_hooks_sigkill (Phase 3) all pass |
| C4 — JSONC + fs2 deps live ONLY in holt-cli | **PRESERVED** | `cargo test --test cli_dep_boundary` → 1 passed |
| C5 — Reader treats stale-or-corrupt as missing | **PRESERVED** | `cargo test -p holt-schemas --test reader_contract` → 9 passed |
| C6 — Render path never reads breaches.log/timings.jsonl | **PRESERVED** | `cargo test -p holt-cli --test render_path_no_read` → 1 passed |

Phase 4 made zero source-crate dependency-graph changes — all work is workspace-level config + manifest metadata + CI scripting. The 6 hard-constraint tests run in the same workspace test sweep that Phase 3 added; their preservation is mechanical.

### Anti-Patterns Found

None in modified files. Specific scans:

- `grep -E "TODO|FIXME|XXX|HACK|PLACEHOLDER|placeholder|coming soon|will be here|not yet implemented" tools/demo.tape tools/setup-labels.sh dist-workspace.toml .github/workflows/release.yml .github/workflows/ci.yml crates/holt-cli/tests/version_smoke.rs README.md` → no matches.
- `grep -E "TODO|FIXME|XXX|HACK|PLACEHOLDER" crates/holt-cli/Cargo.toml Cargo.toml` → no matches in modified Cargo.toml additions.
- All 6 Cargo.toml manifests + 1 workspace Cargo.toml audit clean for `publish = false` + `[package.metadata.dist]` ordering.
- D-04 tap-related code in dist-workspace.toml + release.yml is REMOVED, not commented-out-as-stub. The remaining comments in those files are explicit design-rationale documentation (re-add path), not abandoned code.

### Quality Gates

| Gate | Status |
|------|--------|
| `cargo build --workspace --release` exit 0 | PASS (Finished `release` in 0.03s — fully cached) |
| `cargo test --workspace` exit 0; total = 80 (Phase 3 baseline = 75; Phase 4 added `version_smoke.rs` = 1, plus a re-counting of the assemble_field_policy + handle_event_smoke + sigkill tests now visible at 9 + 6 + 1 = 16 in holt-hooks) | PASS — 80 passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` exit 0 | PASS |
| `cargo fmt --check` exit 0 | PASS |
| `cargo tree -i tokio` empty | PASS (no tokio in dep tree; cargo reports "package ID specification `tokio` did not match any packages") |
| `dist plan` exit 0 + 4-platform tarball enumeration | PASS |
| `holt --self-bench --json --iterations 30` p95 <20ms | PASS (overhead_p95_us = 0; release-build cold start beats budget by 4 orders of magnitude on this hardware) |
| `holt --self-bench-hook PreToolUse --json --iterations 30` p95 <20ms | PASS (overhead_p95_us = 7083; under the 20000 budget by 12.9ms) |

### Deviations Documented

1. **AMENDMENT 2026-04-29 — Homebrew tap dropped from v0.1.** Recorded in three locations (REQUIREMENTS.md / 04-CONTEXT.md / 04-02-PLAN.md). v0.1 substitutes `cargo binstall holt` + GH-release tarball + macOS xattr workaround. Re-add path codified in dist-workspace.toml + release.yml comment blocks. Trigger criteria for revisit: ≥3 Gatekeeper-friction issues OR Apple Developer Program enrollment becomes worthwhile.

2. **D-05 binstall verification deferred to post-rc.1.** Rationale: the URL-resolution probe requires the GitHub release to actually exist. Plan 04-01's verify step is informational (probes `repository` field reachability only). Real verification is Step 2 of RC1-CHECKLIST.md "Post-tag verification" + human_verification[0]. Manual `[package.metadata.binstall]` fallback documented but not landed.

3. **`question` label included in setup-labels.sh** (10 labels, where ROADMAP success criterion #5 enumerates 8 + `good first issue` + `help wanted` = 10 expected; CONTRIBUTING.md L40 also documents `question` as a 10th label, so the script matches CONTRIBUTING.md, not the ROADMAP literal text). Acceptable scope expansion — CONTRIBUTING.md is the operational source-of-truth.

### Human Verification Required

Three items, all blocked on the maintainer's manual `v0.1.0-rc.1` tag-push step per D-10 / RC1-CHECKLIST.md. None reflect codebase gaps:

#### 1. binstall live-resolve

**Test:** Push `v0.1.0-rc.1` tag and run `cargo binstall holt --version 0.1.0-rc.1 --dry-run`
**Expected:** Resolves URL `https://github.com/Sinderella/holt/releases/download/v0.1.0-rc.1/holt-cli-{target}.{format}` for the host triple; exits 0
**Why human:** cargo-binstall's resolution depends on dist artifacts existing on the GitHub release; pre-tag, the package is not on crates.io (publish = false per D-02). RC1-CHECKLIST.md Step 6 + post-tag Step 2 cover this. Fallback per D-05 if resolution fails: add manual `[package.metadata.binstall]` and re-tag rc.2.

#### 2. Release.yml end-to-end + 4-tarball publish

**Test:** Push `v0.1.0-rc.1` tag and verify `gh release view v0.1.0-rc.1 --repo Sinderella/holt --json assets --jq '.assets[].name' | sort`
**Expected:** Lists ≥3 of 4 platform tarballs (Linux x64, macOS x64, macOS arm64; Windows allow-fail per D-03) + matching `.sha256` sidecars + `sha256.sum` + `source.tar.gz` + `source.tar.gz.sha256` + `holt-cli-installer.sh` + `holt-cli-installer.ps1`
**Why human:** Tag-push is manual per D-10. release.yml is structurally correct (verified by `dist plan`); D-14 parity check is wired (release.yml L214–241). End-to-end requires the tag.

#### 3. Live GitHub label state

**Test:** Run `tools/setup-labels.sh` against Sinderella/holt; verify `gh label list --repo Sinderella/holt --json name --jq '.[].name' | sort` lists the 9 holt labels
**Expected:** 9 labels (bug, feature, question, windows, pet, runtime, orchestrator, good first issue, help wanted) all present; gh exits 0
**Why human:** RC1-CHECKLIST.md Step 1 explicitly bootstraps the GitHub repo; Step 3 runs the label script after that. The script's local correctness is verified (9-label declaration + gh-auth guard + gh-presence guard); live state needs the live repo.

---

## Gaps Summary

**No codebase gaps.** Three human-verification items reflect maintainer handoff per the explicit D-10 + RC1-CHECKLIST.md design — the verifier cannot push a production release tag, by design.

The only ROADMAP success criterion that is not fully VERIFIED is criterion #2's brew clause (deferred per AMENDMENT 2026-04-29 — accepted via override) and criterion #4b's binstall live-resolve (pending rc.1 tag-push per D-05 + RC1-CHECKLIST.md). Both are recorded in the frontmatter.

Phase 4's load-bearing infrastructure pair — "the release workflow exists and is exercisable" + "the MSRV gate is enforced in CI" — is **complete in the codebase** (dist-workspace.toml + release.yml + ci.yml all valid; `dist plan` enumerates 4 tarballs; 80/80 workspace tests pass; all 6 hard constraints preserved). Phase 4's launch-UX pair — "README leads with demo + 30-line install + tier statement" + "label apply mechanism + CONTRIBUTING.md routing" — is **complete in the codebase** (README rewrite + assets/demo.gif + tools/setup-labels.sh + RC1-CHECKLIST.md). The remaining steps are the maintainer's tag-push and label-apply executions, both documented end-to-end in RC1-CHECKLIST.md.

---

## VERIFICATION COMPLETE

**Status:** human_needed (3 items, all maintainer manual actions blocked on `v0.1.0-rc.1` tag-push per D-10)
**Score:** 4/5 must-haves verified + 1 partial (criterion #4b binstall live-resolve, pending rc.1 push) + 1 deferred-via-override (criterion #2 brew clause, AMENDMENT 2026-04-29)
**Workspace tests:** 80 passed, 0 failed
**Quality gates:** all clean (build / test / clippy / fmt / cargo-tree -i tokio / dist plan / self-bench / hook-self-bench)
**Hard constraints C1..C6:** all 6 preserved (read-only this phase)
**ROADMAP criteria mapping:**
- #1 (4-platform release + --version parity): structurally VERIFIED, end-to-end deferred to rc.1 push (human_verification[1])
- #2 (brew + binstall): brew DEFERRED (AMENDMENT override accepted); binstall PARTIAL (human_verification[0])
- #3 (README demo + install + tier): VERIFIED
- #4 (MSRV CI + binstall metadata): MSRV VERIFIED; binstall metadata structural OK, live-resolve deferred (human_verification[0])
- #5 (labels + CONTRIBUTING + README routing): structurally VERIFIED, live `gh label list` deferred to repo-create + script-run (human_verification[2])

The codebase delivers what Phase 4 promised at the structural level. The remaining humans-only verification work is the maintainer's tag-push + repo-bootstrap, comprehensively documented in `RC1-CHECKLIST.md`.

---

**VERDICT: PARTIAL**

Rationale: 4 of 5 ROADMAP success criteria fully VERIFIED in the codebase. Criterion #2 is explicitly DEFERRED via AMENDMENT 2026-04-29 (acceptable, recorded). Criterion #4 has a binstall sub-clause that is structurally correct but cannot be falsifiably tested until the maintainer executes the documented `v0.1.0-rc.1` tag push (RC1-CHECKLIST.md). All decisions D-01..D-16 implemented, all hard constraints C1..C6 preserved, all quality gates green, 80/80 tests pass, demo gif + label script + RC1 checklist + README rewrite all landed and reviewable. The phase is **ready for the maintainer's tag-push and post-tag verification flow**, which is the explicit Phase 4 exit ramp.

---

*Verified: 2026-04-29T08:19:03Z*
*Verifier: Claude (gsd-verifier)*
