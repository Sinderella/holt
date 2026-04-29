---
phase: 4
phase_name: distribution-launch
status: findings_present
depth: standard
files_reviewed: 14
findings:
  critical: 0
  warning: 4
  info: 4
  total: 8
reviewed: 2026-04-29
files_reviewed_list:
  - dist-workspace.toml
  - .github/workflows/release.yml
  - .github/workflows/ci.yml
  - Cargo.toml
  - crates/holt-schemas/Cargo.toml
  - crates/holt-supervisor/Cargo.toml
  - crates/holt-hooks/Cargo.toml
  - crates/holt-orchestrator/Cargo.toml
  - crates/holt-render/Cargo.toml
  - crates/holt-cli/Cargo.toml
  - crates/holt-cli/tests/version_smoke.rs
  - README.md
  - tools/demo.tape
  - tools/setup-labels.sh
  - assets/demo.gif (binary; properties verified)
  - .planning/phases/04-distribution-launch/RC1-CHECKLIST.md
---

# Phase 4: Code Review Report

**Reviewed:** 2026-04-29
**Depth:** standard
**Files Reviewed:** 14 (+ 1 binary asset)
**Status:** findings_present

## Summary

Phase 4 lands the dist v0.31.0 scaffold (`dist-workspace.toml` +
`release.yml`), MSRV CI gate (`ci.yml`), `--version` smoke + parity tests
(D-13 + D-14), README rewrite with demo gif + 2-command install + platform
tier, vhs tape source for the gif, idempotent label setup script, and the
maintainer's `v0.1.0-rc.1` readiness checklist. The Homebrew tap drop
(2026-04-29 amendment) is cleanly applied at HEAD: no live `tap = ...`
config, no `publish-homebrew-formula` job, no `brew install ...` line in
the README install block, no `homebrew` installer in `dist-workspace.toml`.
The amendment banners + RC1-CHECKLIST AMENDMENT are correctly recorded.

Hard constraints C1..C6 remain green by inspection: no Phase-4 file adds
`tokio` or `holt-supervisor` to any forbidden crate's deps; `cargo tree -i
tokio` reports "did not match any packages" (i.e. tokio is absent from the
resolved graph); `holt-render`'s tree contains zero `holt-supervisor`
edge; no `unwrap()` / `expect()` / `panic!()` was added on the render
path (`crates/holt-cli/src/run.rs` and `crates/holt-render/src/lib.rs`
are unchanged in Phase 4 and remain panic-free); no new `#[allow(...)]`
attributes were added; `jsonc-parser` and `fs2` remain confined to
`holt-cli` (cli_dep_boundary test still gates this).

No CRITICAL findings. No security vulnerabilities (the `${GITHUB_REF_NAME#v}`
expansion uses `grep -qF` fixed-string mode and never feeds user-controlled
content to a shell metacharacter; `setup-labels.sh` quotes every variable
expansion and uses `set -euo pipefail`).

WARNING-level findings cluster around three themes:

1. **`xattr` workaround target path mismatches `install-path`.** The
   README's macOS Gatekeeper instructions point at `/usr/local/bin/holt`,
   but `dist-workspace.toml` sets `install-path = "CARGO_HOME"` — the
   shell installer drops the binary in `~/.cargo/bin/holt`, NOT
   `/usr/local/bin/`. A user who runs the documented `xattr` command
   verbatim will get "No such file or directory."
2. **`cargo binstall holt` resolves only via dist's auto-generated
   release metadata, which has not yet been verified end-to-end.**
   `holt-cli` carries `publish = false`, so the crate is not on
   crates.io; binstall's primary lookup path (crates.io ↔
   `[package.metadata.binstall]` template) does not apply. Whether
   binstall picks up dist 0.31's release-asset URL pattern by name alone
   (no version pin) is the post-rc.1 verification step in
   `RC1-CHECKLIST.md` §2; until the maintainer has run that, the README
   line `cargo binstall holt   # any platform with cargo + cargo-binstall`
   is an unverified claim.
3. **D-14 parity-check binary discovery is `head -1` over a `find`.**
   If `dist build` ever stages more than one matching binary
   (e.g. host-triple binary alongside the matrix-target binary on
   cross-compile), the wrong binary's `--version` could be asserted.

INFO findings are minor — comment drift in `dist-workspace.toml`
hardcoding the same wrong `/usr/local/bin/holt` path; `setup-labels.sh`'s
default-repo coupling; the `$EDITOR` token in `tools/demo.tape` is typed
literally rather than expanded; and `assets/demo.gif` properties verify
clean (938KB, GIF89a, 900x400).

## Warnings

### WR-01 | macOS Gatekeeper `xattr` workaround targets the wrong install path

**File:** `README.md:19` (also echoed in `dist-workspace.toml:22` and `RC1-CHECKLIST.md:120`)
**Constraint:** ROADMAP success criterion #2 ("manual download + Gatekeeper workaround documented in README")
**Issue:** README documents:
```bash
xattr -d com.apple.quarantine /usr/local/bin/holt
```
But `dist-workspace.toml:36` sets:
```toml
install-path = "CARGO_HOME"
```
dist's shell installer (`installer.sh`) honors this and places the binary
at `${CARGO_HOME:-$HOME/.cargo}/bin/holt`, NOT `/usr/local/bin/holt`. A
macOS user who:

1. Downloads the tarball manually (the path the `xattr` workaround is
   for — binstall and the shell installer would normally have already
   stripped quarantine via the redirect path),
2. Extracts it,
3. Runs the documented `xattr` command verbatim,

… will hit `xattr: /usr/local/bin/holt: No such file: No such file or
directory`. The user must know to substitute their actual install path
(typically `~/.cargo/bin/holt` if they ran the shell installer, or
wherever they manually moved the binary). This contradicts README's
"three-line, working-demo-in-ten-seconds" promise from CONTEXT.md D-07.

The same wrong path is repeated in two follow-on artifacts:
- `dist-workspace.toml:22`: `# documents `xattr -d com.apple.quarantine
  /usr/local/bin/holt` as the post-` (this is a comment, but it
  perpetuates the mistake when a future maintainer reads it as
  authoritative).
- `RC1-CHECKLIST.md:120`: `xattr -d com.apple.quarantine /usr/local/bin/holt
  # or wherever the user installed it` — the inline comment shows
  awareness, but the example path is still wrong for the default
  installer.

**Fix:** Replace the README's example with the actual default install
path, plus the `# or wherever you put it` qualifier:

```bash
# If you grabbed the tarball manually and extracted it into ~/.cargo/bin
# (the shell installer's default — see `install-path = "CARGO_HOME"` in
# the workspace's dist config), strip the quarantine flag once:
xattr -d com.apple.quarantine ~/.cargo/bin/holt
# Or wherever you copied the extracted `holt` binary to.
```

Also update the dist-workspace.toml comment (line 22) and the
RC1-CHECKLIST.md example (line 120) to use the same path so all three
references agree. This is a low-risk doc fix; no source code touches.

---

### WR-02 | README's `cargo binstall holt` line is an unverified claim until rc.1 lands

**File:** `README.md:12`
**Constraint:** REQUIREMENTS.md DIST-03 ("`cargo binstall holt` works from
day one (binstall metadata in `Cargo.toml`)")
**Issue:** The README's install block names:
```bash
cargo binstall holt   # any platform with cargo + cargo-binstall
```

But:
1. `crates/holt-cli/Cargo.toml:15` declares `publish = false`, so
   `holt` (or `holt-cli`) is NOT on crates.io. cargo-binstall's
   primary lookup is crates.io for both the crate's manifest and the
   `[package.metadata.binstall]` URL template.
2. There is no `[package.metadata.binstall]` block in
   `crates/holt-cli/Cargo.toml` (verified — the file ends at the
   `[dev-dependencies]` block at line 67 with no binstall stanza).
3. `dist-workspace.toml` does not reference binstall metadata directly;
   the v0.31.0 dist release contract does emit a `dist-manifest.json`
   with installer URLs that cargo-binstall ≥1.10 can read via its
   `--git` resolver, but `cargo binstall holt` (no `--git`, no
   crates.io publication) is not a supported invocation surface today.

CONTEXT.md D-05 acknowledges this risk: "If this fails with a URL
mismatch, plan 04-01's D-05 fallback applies: add
`[package.metadata.binstall]` to `crates/holt-cli/Cargo.toml`". The
RC1-CHECKLIST.md §2 documents the post-rc.1 verification:
```bash
cargo binstall holt --version 0.1.0-rc.1 --dry-run
```
But the unconditional README line "any platform with cargo +
cargo-binstall" implies it just-works today, which is unverified at
HEAD and may require the D-05 manual fallback to actually succeed —
especially because `holt` (the bare crate name without a version pin)
without crates.io publication has no resolver anchor whatsoever.

**Fix:** Either (a) tone down the README claim until rc.1's binstall
dry-run has been verified by the maintainer:

```markdown
cargo binstall holt   # once the v0.1.0-rc.1 release is published — see RC1-CHECKLIST.md
```

or (b) preemptively add the manual `[package.metadata.binstall]` block
in `crates/holt-cli/Cargo.toml` to guarantee the URL resolves regardless
of dist's auto-generation:

```toml
[package.metadata.binstall]
pkg-url  = "{ repo }/releases/download/v{ version }/{ name }-{ target }{ archive-suffix }"
pkg-fmt  = "txz"          # macOS + Linux tarball default; Windows uses .zip via override
```
plus a `[package.metadata.binstall.overrides.x86_64-pc-windows-msvc]
pkg-fmt = "zip"` block. This is the documented fallback in
04-CONTEXT.md D-05 and would survive a future dist version change too.

Recommend (a) for v0.1 (the simpler doc fix) plus a note in RC1-CHECKLIST
§2 to flip to (b) immediately if the `--dry-run` fails. Either way,
do not ship the launch with a confidently-stated install command that
hasn't been smoke-tested against the rc.1 release.

---

### WR-03 | D-14 parity check resolves the binary via `head -1` over a non-deterministic `find`

**File:** `.github/workflows/release.yml:222`
**Constraint:** D-14 (per-target `--version` parity assertion)
**Issue:**
```bash
BIN=$(find target -type f \( -name 'holt' -o -name 'holt.exe' \) -perm -u+x 2>/dev/null | head -1)
```
The `find` walks the entire `target/` tree. dist 0.31's matrix instance
should produce exactly one matching binary at
`target/<triple>/release/holt[.exe]`, so today this returns the right
binary. But:

1. `head -1` over `find`'s default output order (filesystem traversal
   order, not lexicographic) is non-deterministic across kernels /
   filesystems. Linux ext4 + macOS APFS happen to produce stable
   inode-traversal order in this case, but the `find` man page does not
   guarantee it.
2. If `dist build` is ever invoked with `--artifacts=local --target=...`
   on a runner that already has a cached host-triple build (e.g.
   macOS-arm64 runner cross-building x86_64-apple-darwin while the
   `target/release/holt` from a prior step lingers), `find` would
   return TWO matches and `head -1` could pick the wrong one. The
   `--version` parity check would still fire (both binaries are built
   from the same Cargo.toml), but the success message would mislead
   debugging if the two ever differed (e.g. an in-progress
   release-bump-then-rebuild).
3. The check is inside a `set -euo pipefail` block. If the `find`
   command fails (impossible in practice, but a permissions glitch on
   `target/` could cause it), `pipefail` propagates the error through
   `head -1` — but the empty-string fallback at line 223 (`if [ -z
   "$BIN" ]; then ... exit 1`) handles that case.

Probability of a real wrong-binary pick today: very low. The concern is
forward-stability — a future contributor adding a `cargo build` step
ahead of `dist build` (for caching, e.g.) would silently change which
binary the parity check hits.

**Fix:** Constrain the `find` to dist's known staging location:
```bash
# dist 0.31 stages the matrix binary under target/<triple>/release/.
# Locate exactly that path, not whatever else may be in target/.
BIN=$(find target -type f -path '*/release/holt' -o -path '*/release/holt.exe' 2>/dev/null | sort | head -1)
```
Or, more strictly, parse `dist-manifest.json` (already produced one step
earlier) for the `binaries[].path` field and use that directly — that's
the single source of truth dist itself produced and would survive any
future staging-path change in dist. The latter is cleaner but adds a
`jq` dependency; the former is sufficient and stays inside POSIX find.

---

### WR-04 | `tools/demo.tape` types `$EDITOR ~/.claude/settings.json` literally — vhs does not shell-expand

**File:** `tools/demo.tape:38`
**Constraint:** D-06 (vhs reproducibility — `vhs tools/demo.tape` must
produce a comparable gif)
**Issue:**
```
Type "$EDITOR ~/.claude/settings.json   # change command to: holt run -- bash slow-statusline.sh"
Enter
Sleep 1500ms
```
vhs's `Type` directive emits the literal string into the simulated
terminal. The terminal shell (whatever vhs spawns inside the recording
window) DOES expand `$EDITOR` at execution time — that's what the
existing tape relies on. But:

1. If the rendering host (whoever runs `vhs tools/demo.tape` to
   reproduce the gif) has `$EDITOR` unset, the typed line collapses to
   just `~/.claude/settings.json`, which the shell tries to execute as
   a command (file is not executable, so the shell prints a "Permission
   denied" or "command not found" error inside the recording).
2. If `$EDITOR` is set to something interactive (vim, nano), vhs's
   `Sleep 1500ms` is too short — the editor opens, then the next
   directive (`Type "# Re-fire..."`) starts typing INTO the editor
   rather than into the shell. This produces a visible glitch in the
   gif.
3. The `Sleep 1500ms` between Step 3 (open editor) and Step 4 (next
   shell command) is shorter than the typical interactive-editor
   open-+-close cycle, so the rendering on a different host could look
   different from the committed gif depending on `$EDITOR`.

The committed `assets/demo.gif` evidently rendered cleanly (938KB,
GIF89a, 900x400 — file properties OK), so the original recording host
had a `$EDITOR` setting that produced acceptable output and the timing
worked out. But the `tools/demo.tape` source as committed is not
hermetic — re-rendering on CI, or by a contributor with a different
`$EDITOR`, produces an inconsistent gif.

**Fix:** Either (a) hardcode an editor that's available on every vhs
runner and behaves predictably:
```
Type "vi ~/.claude/settings.json   # change command to: holt run -- bash slow-statusline.sh"
Enter
Sleep 800ms
Type ":q"          # exit vi without modifying — the gif is showing the workflow, not the actual edit
Enter
Sleep 700ms
```
or (b) replace the editor step with a direct shell snippet that mutates
`~/.claude/settings.json` deterministically (e.g., a `sed -i` command
that the user could literally copy):
```
Type "# Edit ~/.claude/settings.json — change statusLine.command to:"
Enter
Type "#   \"holt run -- bash slow-statusline.sh\""
Enter
Sleep 1500ms
```
Option (b) sidesteps the editor-launch entirely and is what the
narrative intends anyway (the gif's purpose is to communicate the wrap;
the editor is incidental). Option (b) is also cheaper to re-render
deterministically.

---

## Info

### IN-01 | `dist-workspace.toml:22` repeats the wrong `/usr/local/bin/holt` path in a comment

**File:** `dist-workspace.toml:22`
**Issue:** The deferral comment block reads:
```toml
# friction. The README documents `xattr -d com.apple.quarantine /usr/local/bin/holt` as the post-
# install workaround.
```
This perpetuates WR-01's path mismatch and ensures that when a future
maintainer regenerates `dist-workspace.toml` (the `allow-dirty = ["ci"]`
edits don't extend to this comment block), the wrong path stays in two
places. Tightly couples doc + config drift.

**Fix:** Update the comment alongside the README fix:
```toml
# friction. The README documents `xattr -d com.apple.quarantine ~/.cargo/bin/holt`
# (or wherever the user installed the extracted tarball) as the post-install
# workaround.
```
Same change to the `RC1-CHECKLIST.md:120` example.

---

### IN-02 | `setup-labels.sh` defaults to `Sinderella/holt` — fork-unfriendly without rebuilding

**File:** `tools/setup-labels.sh:18`
**Issue:**
```bash
REPO="${REPO:-Sinderella/holt}"
```
Hardcoded default. A fork-maintainer (e.g. someone running this script
in their own fork's CI) must remember to set `REPO=` or edit the
script. The fallback works (the env var is honored), but the default is
opinionated.

This is a deliberate design choice (the script is for the canonical
repo's maintainer; fork-maintainers presumably have their own labels),
and the docstring at line 13–14 documents the override pattern. Not a
bug; flagged because the constant `Sinderella/holt` will need to change
if the repo ever moves namespaces (e.g. ownership transfer).

**Fix:** None required. Optional: derive the default from `git remote
get-url origin` parsed into `<owner>/<repo>` so the script auto-targets
whatever the user has cloned:
```bash
DEFAULT_REPO=$(git remote get-url origin 2>/dev/null \
  | sed -E 's#^.*github\.com[:/]([^/]+/[^/.]+)(\.git)?$#\1#' \
  || echo "Sinderella/holt")
REPO="${REPO:-$DEFAULT_REPO}"
```
This makes the script genuinely portable across forks at the cost of
two extra shell pipes. Not v0.1-blocking.

---

### IN-03 | `crates/holt-cli/Cargo.toml`: `[package.metadata.dist] dist = true` placement is correct, but easily misread as a `[dependencies]` table

**File:** `crates/holt-cli/Cargo.toml:20-21`
**Issue:**
```toml
publish = false

# D-02 (post-tap-drop, 2026-04-29): `publish = false` above blocks crates.io
# publication; dist needs an explicit opt-in to still include this package's
# binary in the release artifact build.
[package.metadata.dist]
dist = true

[[bin]]
name = "holt"
```
The `[package.metadata.dist]` table is correctly under the `[package]`
table (it sits between `publish = false` and the first non-package
table `[[bin]]`), so dist parses it as expected. The `[package.metadata.*]`
TOML idiom is "any subtable of `[package.metadata]` is a free-form
namespace for build tooling," which is exactly what dist consumes here.

But the visual layout — `[package.metadata.dist]` immediately followed
by `[[bin]]` with no blank line of separation — could read at a glance
as if `[package.metadata.dist]` is at the top level, which is what
`[dependencies]` is (a top-level table). A maintainer skimming the
manifest looking for the package metadata block might miss it. Pure
ergonomics; not a parsing bug.

**Fix:** None required. Optional cosmetic improvement: add a blank line
or a section comment to make the package-vs-non-package boundary
visible:
```toml
# ----------------------------------------------------------------------
# Binary target
# ----------------------------------------------------------------------
[[bin]]
name = "holt"
```

---

### IN-04 | `assets/demo.gif` properties verify cleanly (938KB, GIF89a, 900×400)

**File:** `assets/demo.gif`
**Issue:** Not a defect — verification record. `file assets/demo.gif`
reports `GIF image data, version 89a, 900 x 400`. Size: 960,783 bytes
(~938KB) — well under the 2MB cap from the plan's success criterion.
The 900×400 frame matches `tools/demo.tape`'s `Set Width 900 / Set
Height 400`. GitHub markdown renders inline GIF89a without a player.

**Fix:** None. Recording for traceability.

---

## Verdict

**VERDICT: PASS-WITH-WARNINGS**

Phase 4 ships the v0.1.0-rc.1 readiness contract: the dist scaffold is
clean, the release workflow is wired, the version smoke + parity tests
exist, the README leads with a working demo gif + 2-command install, the
9 labels are scriptable, and the maintainer's pre-/post-tag flow is
checklisted. Hard constraints C1..C6 hold. No critical defects, no
security vulnerabilities, no panic-on-render-path regressions.

The four warnings are diagnostic, not blocking:

- **WR-01** (xattr path mismatch) is a doc-only fix — replace
  `/usr/local/bin/holt` with `~/.cargo/bin/holt` in three files
  (README + dist-workspace.toml comment + RC1-CHECKLIST). 5 minutes of
  work, no source code change. Should happen before launch — first user
  to hit the macOS Gatekeeper path will report the broken `xattr`
  command.
- **WR-02** (binstall claim is unverified) is closed by RC1-CHECKLIST §2
  — the maintainer runs `cargo binstall holt --version 0.1.0-rc.1
  --dry-run` post-tag, and either the dist auto-generated metadata
  resolves (in which case the README's claim is true and no action is
  needed) or it fails (and the D-05 manual `[package.metadata.binstall]`
  fallback gets added in `v0.1.0-rc.2`). The risk is correctly
  enumerated in the checklist; the README claim is optimistic but
  recoverable.
- **WR-03** (D-14 binary discovery via `head -1`) is forward-looking; it
  works today on the v0.31 staging convention. A 1-line tightening of
  the `find` invocation closes it.
- **WR-04** (`$EDITOR` in demo.tape) is a reproducibility wart — the
  committed gif is fine; re-rendering by a contributor with a different
  editor would not reproduce. Worth fixing for hermeticity but doesn't
  block launch.

INFO findings are minor (comment drift, fork-friendly default,
manifest-layout cosmetics, asset-properties record).

Recommend fixing WR-01 immediately (single-commit doc-only patch) and
softening WR-02's README claim before tagging rc.1. WR-03 and WR-04 can
land post-rc.1 without changing the release contract. Phase 4 is ready
to close.

---

_Reviewed: 2026-04-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
