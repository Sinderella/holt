# `v0.1.0-rc.1` release readiness checklist

**Owner:** maintainer (manual). This plan (`04-02-PLAN.md`) does NOT push the tag; this checklist documents the maintainer's pre-tag / tag-push / post-tag verification flow per CONTEXT decision D-10.

**Preconditions:** Plan 04-01 + Plan 04-02 both complete and committed; `dist plan` passes locally; all hard constraints (C1..C6) green; `cargo test --workspace` passes.

> ⚠️ **AMENDMENT 2026-04-29:** Homebrew tap deferred to v0.1.x. Pre-tag step "create `Sinderella/homebrew-holt`" is REMOVED. Post-tag step "tap formula auto-published" is REMOVED. v0.1 distribution is `cargo binstall holt` + prebuilt GitHub-release tarballs only. See `.planning/phases/04-distribution-launch/04-CONTEXT.md` AMENDMENT BANNER for re-add procedure when revisiting in v0.1.x.

---

## Pre-tag steps

Run these BEFORE pushing `v0.1.0-rc.1`:

1. **Create the GitHub repo** (one-time bootstrap; skip if `gh repo view Sinderella/holt` succeeds):
   ```bash
   gh repo create Sinderella/holt \
     --public \
     --description "A small Rust statusLine for Claude Code, with a small otter in it." \
     --homepage "https://github.com/Sinderella/holt" \
     --source . --remote origin --push
   ```

   Rationale: At plan-execution time the repo did not yet exist on GitHub (`gh api repos/Sinderella/holt` returned 404). The setup-labels script (Task 4) and the `dist`-emitted release workflow both target `Sinderella/holt`; the repo must exist before either is exercised.

2. **Bump Cargo.toml versions.** Update `version` in EACH crate's `Cargo.toml` to `0.1.0-rc.1`:
   ```bash
   for c in holt-schemas holt-supervisor holt-hooks holt-orchestrator holt-render holt-cli; do
     sed -i.bak 's/^version = "0\.1\.0"$/version = "0.1.0-rc.1"/' "crates/${c}/Cargo.toml"
     rm -f "crates/${c}/Cargo.toml.bak"
   done
   cargo update
   git add -A && git commit -m "chore(release): bump versions to 0.1.0-rc.1 (D-10)"
   ```

   Rationale: `env!("CARGO_PKG_VERSION")` resolves to whatever's in Cargo.toml at compile time. The D-13 version smoke test (`crates/holt-cli/tests/version_smoke.rs`) and the D-14 release-workflow parity check both depend on Cargo.toml's version matching the tag. SemVer-compliant pre-release identifiers like `0.1.0-rc.1` are valid in both Cargo and dist's URL templating.

3. **Apply the 9 issue labels** (deferred at plan-execution time because `Sinderella/holt` did not yet exist on GitHub; run AFTER Step 1):
   ```bash
   gh auth status                             # confirms login is active for an account with admin/maintain on Sinderella/holt
   gh auth switch --user Sinderella           # if your active gh account isn't Sinderella, switch first
   ./tools/setup-labels.sh                    # idempotent; safe to re-run
   gh label list --repo Sinderella/holt --json name --jq '.[].name' | sort
   ```

   Expected last-line output is the 9 holt labels (plus any GitHub defaults the repo was created with). Rationale: ROADMAP success criterion #5 requires the 9 labels exist on the repo before launch. The script is idempotent — running it twice is harmless.

4. **Push the existing local commits to the new remote.** After Step 1 created the remote with `--source . --remote origin --push` (or if you skipped that, configure the remote manually):
   ```bash
   git remote -v   # confirm origin -> Sinderella/holt
   git push -u origin main
   git log --oneline origin/main | head -5   # confirm the 99-commit history landed
   ```

   Rationale: The tag-push step below pushes a single tag, but the workflow needs the underlying commits already present on the remote.

5. **Confirm GitHub Actions secrets.** dist's release workflow needs `GITHUB_TOKEN` (auto-provided by Actions) at v0.1; no other secrets are required because v0.1 doesn't sign macOS/Windows binaries (deferred to v0.1.x+ per CONTEXT.md "Open questions deferred"). Verify:
   ```bash
   gh secret list --repo Sinderella/holt
   ```

   The default `GITHUB_TOKEN` is implicit. No additional secrets needed.

6. **Local self-test.** Build + test + bench locally to catch any pre-tag regression:
   ```bash
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ./target/release/holt --self-bench --json --iterations 30
   ./target/release/holt --self-bench-hook PreToolUse --json --iterations 30
   dist plan
   ```

   All must pass. The version_smoke test in particular asserts `holt --version` contains the bumped `0.1.0-rc.1`.

## Tag and push

Once all pre-tag steps pass:

```bash
git tag v0.1.0-rc.1
git push origin v0.1.0-rc.1
```

This triggers `.github/workflows/release.yml` per D-11. The workflow:
- Builds the 4-target matrix (Linux x64, macOS x64, macOS arm64, Windows x64; Windows allow-fail per D-03)
- Runs the per-target D-14 parity check (`--version` matches `${GITHUB_REF_NAME#v}`) for every matrix instance
- Generates SHA-256 checksums per artifact (D-12 default-on)
- Publishes the GitHub release with all artifacts attached (Homebrew tap auto-publish removed per AMENDMENT 2026-04-29 — deferred to v0.1.x)

## Post-tag verification

After the workflow completes, verify the success contract:

1. **Release artifacts uploaded** (≥3 of 4 targets — Windows is allow-fail):
   ```bash
   gh release view v0.1.0-rc.1 --repo Sinderella/holt --json assets --jq '.assets[].name' | sort
   ```
   Expected: at minimum `holt-cli-x86_64-unknown-linux-gnu.tar.xz` (+ `.sha256`), `holt-cli-x86_64-apple-darwin.tar.xz` (+ `.sha256`), `holt-cli-aarch64-apple-darwin.tar.xz` (+ `.sha256`), `sha256.sum`, `source.tar.gz` (+ `.sha256`), `holt-cli-installer.sh`, `holt-cli-installer.ps1`. Windows artifacts (`holt-cli-x86_64-pc-windows-msvc.zip` + `.sha256`) may or may not be present; if present, also good. If ALL four target archives are present, even better — Windows wasn't expected to fail at v0.1, only allowed to.

2. **`cargo binstall` resolves the artifacts:**
   ```bash
   cargo binstall holt --version 0.1.0-rc.1 --dry-run
   ```
   Expected: resolves the tarball URL to `https://github.com/Sinderella/holt/releases/download/v0.1.0-rc.1/holt-cli-{target}.{format}` for the local host triple. Exit 0.

   If this fails with a URL mismatch, plan 04-01's D-05 fallback applies: add `[package.metadata.binstall]` to `crates/holt-cli/Cargo.toml` per the documented manual override and re-tag `v0.1.0-rc.2`.

3. **Downloaded binary's `--version` matches the tag:**
   ```bash
   # Download the host's tarball, extract, and run --version:
   curl -sL https://github.com/Sinderella/holt/releases/download/v0.1.0-rc.1/holt-cli-x86_64-unknown-linux-gnu.tar.xz \
     | tar -xJf - -C /tmp
   /tmp/holt-cli-x86_64-unknown-linux-gnu/holt --version
   ```
   Expected: prints `holt 0.1.0-rc.1` (the D-13 contract). The release-workflow's D-14 step ALREADY asserted this server-side; this is the human-side smoke check.

4. **macOS Gatekeeper note** (manual, only if a macOS user reports breakage):
   ```bash
   xattr -d com.apple.quarantine /usr/local/bin/holt   # or wherever the user installed it
   ```
   The README documents this workaround for the manual-download path. If ≥3 issues are filed reporting Gatekeeper friction, that's the trigger to revisit the Homebrew tap (per AMENDMENT 2026-04-29 in 04-CONTEXT.md) or pursue Apple Developer Program enrollment for native notarization.

## Rollback

If post-tag verification fails:

1. **Delete the release** (artifacts and metadata):
   ```bash
   gh release delete v0.1.0-rc.1 --repo Sinderella/holt --yes --cleanup-tag
   ```
   `--cleanup-tag` also removes the git tag from the remote.

2. **Local tag delete:**
   ```bash
   git tag -d v0.1.0-rc.1
   ```

3. **Diagnose, fix, re-tag** as `v0.1.0-rc.2` (do NOT reuse `rc.1` — `cargo binstall` caches and downstream consumers may have already cached the artifact URL even if the release was deleted within minutes).

## Why this checklist exists (D-10 rationale)

Per CONTEXT.md D-10: "Pre-release tags don't update Homebrew's 'latest' pointer (Homebrew defaults skip pre-releases) and don't trigger downstream consumers, so the rc.1 tag is a safe end-to-end exercise." With the AMENDMENT 2026-04-29 dropping Homebrew from v0.1, that rationale narrows to "rc.1 doesn't surface in `cargo binstall holt` (no version pin) and doesn't tag a stable release on GitHub" — same shape, narrower scope. The checklist makes the maintainer's pre-tag / tag-push / post-tag flow explicit so a future fork-maintainer or returning maintainer (after extended context loss) doesn't have to reverse-engineer the readiness contract from CONTEXT.md + ROADMAP.md + 7 plan files.

## References

- CONTEXT.md D-04 (Homebrew tap — DEFERRED), D-10 (rc.1 trigger), D-13 (version smoke), D-14 (parity check), D-15 (install-hooks UX)
- AMENDMENT BANNERS in `.planning/phases/04-distribution-launch/04-CONTEXT.md` and `.planning/phases/04-distribution-launch/04-02-PLAN.md` (2026-04-29 — Homebrew tap drop)
- ROADMAP success criterion #1 (rc.1 publishes 4 artifacts; --version matches tag)
- ROADMAP success criterion #2 (`cargo binstall` resolves; manual download + Gatekeeper workaround documented in README)
- 04-01-PLAN.md (dist scaffold, version smoke test)
- 04-02-PLAN.md (D-14 parity check, README first-run, label apply)
