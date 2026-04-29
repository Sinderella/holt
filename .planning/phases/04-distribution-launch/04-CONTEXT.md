---
phase: 4
phase_name: Distribution + launch
status: planning
created: 2026-04-28
mode: smart-discuss (decisions captured inline; no user gray-area pause needed)
inputs:
  roadmap: .planning/ROADMAP.md (Phase 4 detail, success criteria #1..#5, requirements DIST-01..07)
  requirements: .planning/REQUIREMENTS.md (DIST-01..07)
  stack: .planning/research/STACK.md (Distribution table; `dist` v0.31.0 — 2026-02-23)
  project: .planning/PROJECT.md (north star + key decisions)
  state: .planning/STATE.md (Phase 4 ← NEXT; open question = `dist init` scaffold)
  contributing: CONTRIBUTING.md (label list already documented; just needs to be applied to repo)
  cargo: Cargo.toml (rust-version=1.87, edition=2024 already pinned; repository=https://github.com/Sinderella/holt)
  ci: .github/workflows/ci.yml (existing matrix Linux+macOS=required, Windows=allowed-failure)
requirements:
  - DIST-01
  - DIST-02
  - DIST-03
  - DIST-04
  - DIST-05
  - DIST-06
  - DIST-07
hard_constraints_in_scope:
  - C1, C2, C5, C6: must remain green from Phases 1–3 (architecture + reader contract + render-path no-read)
  - C3, C4: must remain green from Phase 3 (settings.json mutation + JSONC boundary)
  - No Phase 4 work changes any source crate's dependency graph; the 6 hard constraints are read-only invariants.
---

# Phase 4 — Distribution + launch

> ⚠️ **AMENDMENT 2026-04-29: Homebrew tap dropped from v0.1, deferred to v0.1.x.**
>
> The body of this document still discusses D-04 (Homebrew tap = `Sinderella/homebrew-holt`) and references `brew install Sinderella/holt` in D-04, D-15, and the ROADMAP success-criteria mapping for criterion #2. **Those are superseded.** v0.1 distribution is now `cargo binstall holt` + prebuilt GitHub-release tarballs only. macOS users get a documented `xattr -d com.apple.quarantine /usr/local/bin/holt` post-install workaround. DIST-02 is marked deferred in `.planning/REQUIREMENTS.md`.
>
> Re-add path: drop the `homebrew` installer back into `dist-workspace.toml` + restore `tap`/`formula`/`publish-jobs` keys + `dist generate --mode ci` + re-apply Windows continue-on-error hand-edit. Trigger criteria for revisiting: ≥3 issues filed reporting macOS Gatekeeper friction OR Apple Developer Program enrollment ($99/yr) becomes worthwhile for native notarization.
>
> The rest of D-01..D-16 stand unchanged — only the Homebrew-specific sub-decisions and install-command examples are affected.

## Phase goal (from ROADMAP.md)

A user can `brew install <user>/tap/holt` (macOS), `cargo binstall holt` (any platform with the toolchain), or download a prebuilt binary from a GitHub release for Linux x64 / macOS x64+arm64 / Windows x64, follow a three-line README, and watch a sub-ten-second asciinema/gif of the shim wrapping a slow statusLine — with the repo's `CONTRIBUTING.md` already routing the issue traffic the launch will produce.

## What's already in place (no rework)

- `Cargo.toml` workspace declares `edition = "2024"` and `rust-version = "1.87"` — DIST-05 requirement is **already met at the manifest level**; Phase 4 only needs a CI matrix entry that builds against pinned 1.87.
- `Cargo.toml` workspace declares `repository = "https://github.com/Sinderella/holt"` — the precondition for `dist`'s GitHub-release integration and `cargo binstall`'s URL resolution is already set.
- `CONTRIBUTING.md` is already committed at the repo root with the canonical label list (`bug`, `feature`, `question`, `windows`, `pet`, `runtime`, `orchestrator`, `good first issue`, `help wanted`); ROADMAP success criterion #5 only needs the labels **applied to the GitHub repo**, not authored.
- `README.md` exists with the v1.0 vision (Nak + peer dots) but does **not** lead with an asciinema/gif of the v0.1 wedge (wrapping + LKG cache); ROADMAP success criterion #3 requires that change.
- `holt --version` plumbing landed in Phase 1 (`crates/holt-cli/src/cli.rs`) and prints the `CARGO_PKG_VERSION` macro at compile time; verifying that the dist-published binary's `--version` output matches the tag is a smoke test, not new code.

## Decisions (D-01..D-16)

### D-01 — Run `dist init` for the scaffold; do not hand-write `dist.toml`

**Decision:** Phase 4 begins by running `dist init` from a clean working tree to generate the canonical `dist-workspace.toml` (and per-package `dist.toml` if the layout demands it). Confidence MEDIUM per `.planning/research/SUMMARY.md §5`. The generated scaffold is committed verbatim, then customized only for the items in D-02..D-04. **Do not** copy the snippet from `STACK.md` wholesale — `dist`'s schema has shifted since that snippet was written, and the generator is the authoritative source.

**Why:** `dist init` knows about the current schema, current GitHub Actions matrix recipes, and current binstall integration; recreating that by hand is bug-prone and duplicates the maintainer's job.

**Plan-time action:** Plan 04-01 starts with `dist init --yes` (or interactive if needed) executed against the existing workspace; the generated `dist-workspace.toml` and `.github/workflows/release.yml` get committed in their first task before any customization.

### D-02 — `dist` ships only the `holt-cli` binary; library crates and the phantom test package are excluded

**Decision:** The workspace has six crates plus a phantom `holt-workspace-tests` package that hosts `tests/architecture_dag.rs` + `tests/cli_dep_boundary.rs`. Of these, **only `holt-cli` produces a published binary** (`holt`). The other library crates (`holt-schemas`, `holt-supervisor`, `holt-hooks`, `holt-orchestrator`, `holt-render`) are internal — they are **not** published to crates.io at v0.1. The phantom `holt-workspace-tests` package already declares `publish = false`.

**Why:** Publishing internal libraries commits us to their public APIs forever; v0.1's keystone (`holt-schemas`) is already labeled `#[non_exhaustive]` precisely because we expect to revise field shapes through v1.0. The single binary is the entire user-facing surface.

**Plan-time action:** Plan 04-01 ensures every non-`holt-cli` crate's `Cargo.toml` carries `publish = false` (audit; add where missing). Plan 04-01 also configures `dist-workspace.toml` to scope artifact builds to `holt-cli` only.

### D-03 — Platform matrix: macOS + Linux required, Windows allow-failure

**Decision:** The dist platform matrix includes exactly four targets:

| Target triple | Tier | CI gate |
|---|---|---|
| `x86_64-unknown-linux-gnu` | tier-1 | required |
| `x86_64-apple-darwin` | tier-1 | required |
| `aarch64-apple-darwin` | tier-1 | required |
| `x86_64-pc-windows-msvc` | best-effort | allow-failure |

The Windows job's failure must **not** block the release of the three Unix artifacts. This honors `02-scope.md`'s locked Windows tier statement and ROADMAP success criterion #1 ("Windows allowed to fail the matrix without blocking the others").

**Why:** Windows-on-`process-wrap` works in principle (JobObject path), but our test surface is Unix-heavy, the Defender cold-start gotcha (D-15-budget) is unverified at v0.1, and the trigger-promotion rule is documented in CONTRIBUTING.md.

**Plan-time action:** Plan 04-01 customizes the dist-generated workflow to mark the Windows matrix entry `continue-on-error: true` (or dist's equivalent); the release-success gate is `Linux ∧ macOS-x64 ∧ macOS-arm64`.

### D-04 — Homebrew tap: `Sinderella/homebrew-holt`, auto-published by dist

**Decision:** The Homebrew tap repo is `github.com/Sinderella/homebrew-holt` (Homebrew's `homebrew-` prefix convention; user-facing install command stays `brew install Sinderella/holt` because Homebrew strips the prefix). `dist` populates the formula on every release via its built-in tap publisher. The user must create the empty repo manually before the first release; thereafter dist owns the formula contents.

**Why:** Tap auto-publishing is `dist`'s flagship feature; bypassing it (e.g., publishing manually) is more work and creates a divergence vector. The empty-repo-creation step is a one-time prerequisite documented in Plan 04-02.

**Plan-time action:** Plan 04-02 documents the prerequisite (`gh repo create Sinderella/homebrew-holt --public --description "Homebrew tap for holt"`) in the README install instructions so any contributor can reproduce the bootstrap.

### D-05 — `cargo binstall` metadata: rely on dist's auto-generation

**Decision:** `dist` v0.31.0 auto-generates `cargo binstall` metadata in the published GitHub release; `cargo binstall holt --dry-run` resolves the URL via the GitHub-release pattern dist enforces (`https://github.com/Sinderella/holt/releases/download/v{VERSION}/holt-{TARGET}.{FORMAT}`). **No manual `[package.metadata.binstall]` block is required** in `crates/holt-cli/Cargo.toml`.

**Why:** Adding manual binstall metadata duplicates dist's behavior and creates a drift surface (rename a target → both blocks must be edited). The dist-generated release artifacts already match binstall's default URL probe, per the dist 0.31 docs.

**Plan-time action:** Plan 04-01 verifies the `cargo binstall holt --dry-run` URL probe succeeds against the test pre-release tag (D-15) before declaring DIST-03 met. If it does **not** auto-resolve, fall back to manual `[package.metadata.binstall]` keyed `pkg-url` and `pkg-fmt` per binstall docs.

### D-06 — Demo medium: animated gif via `vhs`, committed to `assets/demo.gif`

**Decision:** The above-the-fold README demo is an animated GIF generated from a `vhs` tape (charmbracelet/vhs) committed at `tools/demo.tape`; the rendered binary lives at `assets/demo.gif`. **Not asciinema-cast** (requires external player or JS embed; doesn't render inline on GitHub repos).

**Why:** Inline-rendering is the priority — the README must show the demo in the user's first browse without external service dependencies. `vhs` is reproducible (text-based tape script), reasonable to keep in-repo (the gif is ~500KB-1MB for a 10s demo), and renders natively in GitHub markdown.

**Tape content:** A 10-second sequence:
1. `cat slow-statusline.sh` — show a 5-second-sleeping placeholder script.
2. `claude` opens, statusLine fires → bar blocks for 5s (simulated default behavior).
3. Edit `~/.claude/settings.json` to wrap the script with `holt run -- bash slow-statusline.sh`.
4. `claude` reopens → statusLine returns instantly (LKG fall-through), `holt --self-bench` shows sub-20ms p95.

**Plan-time action:** Plan 04-02 commits `tools/demo.tape` (text), generates `assets/demo.gif` via `vhs tools/demo.tape` (manual run; binary committed). README's first 30 lines embed `![demo](assets/demo.gif)`.

### D-07 — Platform tier statement: README "Install" section, links to `docs/02-scope.md`

**Decision:** The README's "Install" section (above the fold) carries an explicit one-paragraph platform tier statement: "Linux/macOS x86_64 + Apple Silicon are tier-1; Windows x64 is best-effort and may lag releases. Windows promotion to tier-1 triggers on (a) ≥10 issues tagged `windows` filed against the repo, OR (b) a Windows contributor adopting maintenance ownership — see [`docs/02-scope.md`](docs/02-scope.md)." The criteria match `docs/02-scope.md`'s locked text verbatim.

**Why:** ROADMAP success criterion #3 requires the platform tier statement above the fold with a link to the trigger criteria; this single paragraph satisfies both clauses.

**Plan-time action:** Plan 04-02 adds the paragraph during the README rewrite; verifies the doc link resolves locally; commits.

### D-08 — MSRV CI matrix entry: pin `1.87.0` toolchain on Linux+macOS, keep stable for fmt+clippy

**Decision:** Extend the existing `.github/workflows/ci.yml` with a dedicated MSRV job per Unix target that uses `dtolnay/rust-toolchain@1.87` (or the documented `1.87.0` actions input) and runs `cargo build --workspace --release` only — not the test suite (tests already run on stable). The existing stable jobs (clippy, fmt, test, self-bench) remain unchanged. ROADMAP success criterion #4 requires CI to fail on any MSRV regression.

**Why:** Locking MSRV to 1.87 protects users on stable-but-pinned toolchains (e.g., distros lagging behind Rust stable by one release). Restricting MSRV to a build-only job avoids running tests twice and keeps CI minutes flat.

**Plan-time action:** Plan 04-01 adds a `msrv` job stanza to `.github/workflows/ci.yml`; the job runs after the existing fmt+clippy gate; it is REQUIRED on Linux + macOS, ALLOWED-FAILURE on Windows.

### D-09 — Repo label application: idempotent script `tools/setup-labels.sh`

**Decision:** `tools/setup-labels.sh` is a shell script that runs `gh label create --force` for each of the nine labels documented in CONTRIBUTING.md. It is idempotent (the `--force` flag updates existing labels rather than failing). It is a **one-shot** that the maintainer runs before the first release; it is not wired into CI.

**Why:** ROADMAP success criterion #5 requires the labels exist on the GitHub repo, verifiable via `gh label list --json name`. The labels are CONTRIBUTING.md-authored and label-color/description metadata changes infrequently; codifying the apply step as a script preserves the ability to re-run it on a new fork without mining the doc by hand.

**Plan-time action:** Plan 04-02 commits `tools/setup-labels.sh` and runs it once against `Sinderella/holt`; verification reads `gh label list --json name` and asserts the nine labels are present.

### D-10 — Pre-release tag for end-to-end verification: `v0.1.0-rc.1`

**Decision:** Phase 4's verifier uses tag `v0.1.0-rc.1` (SemVer-compliant pre-release identifier) as the smoke-test trigger. After Phase 4 plans are complete, `git tag v0.1.0-rc.1 && git push origin v0.1.0-rc.1` triggers `release.yml`; the verifier asserts (a) artifacts upload to the GitHub release, (b) `holt --version` of the downloaded artifact prints `0.1.0-rc.1`, (c) `cargo binstall holt --version 0.1.0-rc.1 --dry-run` resolves the URL.

**Why:** Pre-release tags don't update Homebrew's "latest" pointer (Homebrew defaults skip pre-releases) and don't trigger downstream consumers, so the rc.1 tag is a safe end-to-end exercise.

**Plan-time action:** Plan 04-02's Wave-2 verification step covers this; the tag is **not** pushed during execution — the verifier flags the readiness, the maintainer pushes the tag manually after audit.

### D-11 — Release workflow trigger: tag push pattern `v*`

**Decision:** Keep `dist init`'s default trigger: push of any tag matching `v*` runs `release.yml`. Do not add manual workflow-dispatch triggers at v0.1.

**Why:** Tag-push is the canonical Rust-ecosystem release flow; manual dispatch invites accidental publishes from forks.

**Plan-time action:** Plan 04-01 verifies the dist-generated workflow's trigger is `on.push.tags: ['v*']`; if it is not, modify it to match.

### D-12 — Release artifact integrity: SHA-256 checksums via dist default

**Decision:** Enable dist's SHA-256 checksum generation (default in dist 0.31). Do not enable GPG signing at v0.1; the maintainer's commit signing (commit signing) is sufficient release-time identity, and GPG complicates contributor onboarding.

**Why:** Checksums catch CDN-corruption / mid-download tampering at zero cost; GPG-signing the artifacts adds key-management overhead that doesn't pay off until v1.0+.

**Plan-time action:** Plan 04-01 verifies the dist config emits `*.sha256` files for every release artifact.

### D-13 — `--version` smoke test: assert `cargo_pkg_version!()` matches the published tag

**Decision:** Phase 4 adds **one** integration test, `crates/holt-cli/tests/version_smoke.rs`, that asserts `holt --version`'s stdout starts with `concat!("holt ", env!("CARGO_PKG_VERSION"))`. This single test guards against a forgotten Cargo.toml version bump before tagging.

**Why:** ROADMAP success criterion #1 requires the dist-published artifact's `holt --version` print the tagged version. The test runs locally; the **release workflow** itself runs the binary with `--version` and asserts (via grep) that the output matches the dispatched tag — covered in D-14.

**Plan-time action:** Plan 04-01 adds the local test (≤20 LOC). Plan 04-02 adds the release-workflow grep assertion as a final job step.

### D-14 — Release-workflow `--version` parity check

**Decision:** The release workflow downloads the freshly-built artifact for the matrix's host triple, runs `./holt --version`, and `grep`s for the tag (with leading `v` stripped via `${GITHUB_REF_NAME#v}`). Failure of any matrix entry's parity check fails the release for that target.

**Why:** Catches the rare-but-fatal case where a bad `Cargo.toml` version slips through pre-commit (e.g., a maintainer hand-tagged a commit without bumping the version). One bash line, big payoff.

**Plan-time action:** Plan 04-02 amends `release.yml` post-`dist init`; the snippet is documented in the plan.

### D-15 — `holt install-hooks` budget reaffirmation in README

**Decision:** The README's install section reaffirms that `holt install-hooks` mutates `~/.claude/settings.json` atomically with `--dry-run` / `--print` escape hatches. **Do not** rerun the Phase 3 success criteria here — Phase 4 only documents the user-facing UX, not the implementation.

**Why:** Users will immediately try `holt install-hooks` after `brew install`; the README must steer them at the dry-run path before they trust the auto-mutation.

**Plan-time action:** Plan 04-02's README rewrite includes a "First-run" subsection: `brew install Sinderella/holt && holt install-hooks --dry-run` followed by `holt install-hooks` once the diff looks correct.

### D-16 — Phase 4 plan split: 2 plans, infra → launch UX

**Decision:** Phase 4 splits into two plans matching the established Phase 2 / Phase 3 cadence:

| Plan | Scope | Requirements | Wave |
|---|---|---|---|
| 04-01 | dist scaffold + Cargo.toml audit + binstall verification + MSRV CI matrix + version smoke test | DIST-01, DIST-03, DIST-05 | Wave 1 (infra) |
| 04-02 | README rewrite + asciinema/gif demo + platform tier statement + Homebrew tap docs + label setup script + release-workflow `--version` parity check + tag-push readiness verification | DIST-02, DIST-04, DIST-06, DIST-07 | Wave 2 (launch UX) |

**Why:** Plan 04-01 lands the toolchain/scaffold so the release workflow exists and can be exercised; Plan 04-02 then layers the user-facing surface (README, tap, labels) on top, with the verification phase exercising end-to-end via the rc.1 tag.

**Plan-time action:** Planner spawns two `gsd-planner` invocations in sequence (not parallel — Plan 04-02 references files Plan 04-01 creates, e.g., the dist-generated workflow).

## Open questions resolved (no carry-forward)

- **Dist scaffold mode** → run `dist init` not hand-write — D-01.
- **Platform matrix membership** → 4 targets, Windows allow-failure — D-03.
- **binstall metadata strategy** → rely on dist auto-generation, manual fallback documented — D-05.
- **Demo medium** → animated GIF via `vhs`, committed in-repo — D-06.
- **MSRV CI strategy** → dedicated `msrv` job pinned to 1.87.0, build-only — D-08.
- **Label apply mechanism** → idempotent shell script, run once — D-09.
- **End-to-end verification tag** → `v0.1.0-rc.1` pre-release tag — D-10.

## Open questions deferred to v0.1.x or v0.5

- **GPG-signed artifacts** — D-12 declines this for v0.1; revisit if/when ≥3 issues request it.
- **Cargo install via crates.io** (`cargo install holt`) — depends on publishing `holt-cli` to crates.io, which depends on us being willing to expose the workspace's internal crates' versions. v0.5 follow-up — out of v0.1 scope per D-02.
- **Notarized macOS binaries** — Apple Developer Program enrollment is $99/yr; we accept Gatekeeper friction for non-brew installs at v0.1. README documents the `xattr -d com.apple.quarantine` workaround. v0.5 follow-up.
- **Code signing for Windows binaries** — same logic as macOS notarization; v0.5+.

## Hard constraints (no Phase 4 work touches them)

C1, C2, C3, C4, C5, C6 are all read-only this phase. No source crate's dependency graph changes. The Phase 3 `tests/architecture_dag.rs` and `tests/cli_dep_boundary.rs` continue to enforce them on every PR.

## ROADMAP success criteria mapping

| ROADMAP must-have | Owning plan | Decisions involved |
|---|---|---|
| #1 — `v0.1.0-rc.1` release publishes 4 artifacts; `--version` matches tag | 04-01 (workflow scaffold) + 04-02 (parity check, tag readiness) | D-01, D-03, D-10, D-11, D-12, D-13, D-14 |
| #2 — `brew install` + `cargo binstall` resolve to dist artifacts | 04-02 (Homebrew tap docs) + 04-01 (binstall verification) | D-04, D-05 |
| #3 — README leads with demo + first-30-line install + platform tier | 04-02 | D-06, D-07, D-15 |
| #4 — `Cargo.toml` MSRV 1.87 / Edition 2024; CI matrix entry on 1.87 | 04-01 | D-08 (Cargo.toml already pinned) |
| #5 — Repo labels configured; CONTRIBUTING.md present | 04-02 | D-09 |

## Inputs to planner (prefetch list)

The `gsd-planner` instance(s) for Phase 4 should treat the following as authoritative and NOT re-research:

- `.planning/ROADMAP.md` (Phase 4 detail block — success criteria are the verification contract)
- `.planning/REQUIREMENTS.md` (DIST-01..07 wording)
- `.planning/research/STACK.md` §Distribution (dist v0.31.0 + Homebrew + binstall pattern)
- `Cargo.toml` (current workspace shape — confirms D-02's `publish = false` audit list)
- `.github/workflows/ci.yml` (existing CI — confirms what D-08 must extend)
- `CONTRIBUTING.md` lines 32–46 (label list — confirms D-09 enumeration)
- `README.md` (existing scaffold — confirms what D-06 + D-07 + D-15 must rewrite)
- This document (CONTEXT.md) — D-01..D-16 are locked decisions; the planner does not re-pose gray areas already resolved here.

The planner is invoked with `--skip-research` (RESEARCH.md not authored; CONTEXT.md is authoritative) per the established Phase 2 / Phase 3 pattern.
