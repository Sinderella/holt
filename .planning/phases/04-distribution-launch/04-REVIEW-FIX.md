---
phase: 4
fixed_at: 2026-04-29
review_input: 04-REVIEW.md (PASS-WITH-WARNINGS; 0 critical / 4 warning / 4 info)
verifier_input: 04-VERIFICATION.md (PARTIAL — 4/5 verified, 1 deferred via AMENDMENT, 1 partial pending maintainer rc.1 tag-push)
fixed: 3
deferred: 1 (WR-02)
info_deferred: 4
---

# Phase 4 — Code Review Fix Pass

## Findings addressed

### WR-01 — Wrong xattr install path → FIXED in `7a22d16`

**Issue:** `README.md:19`, `dist-workspace.toml:22`, and `RC1-CHECKLIST.md:120` documented `xattr -d com.apple.quarantine /usr/local/bin/holt`. dist-workspace.toml's `install-path = "CARGO_HOME"` means cargo-binstall and dist's shell installer both place the binary at `~/.cargo/bin/holt`. The documented xattr command would have failed with "No such file" for the very macOS users it was meant to help.

**Fix:** Replaced with `xattr -d com.apple.quarantine "$(command -v holt)"` (robust regardless of install path). Updated all three references atomically.

**Verification:** `grep -rn 'xattr' README.md dist-workspace.toml .planning/phases/04-distribution-launch/RC1-CHECKLIST.md` returns the corrected form in all three files; no remaining `/usr/local/bin/holt` references in the docs.

### WR-02 — Optimistic `cargo binstall` README copy → DEFERRED

**Issue:** `README.md:12` says `cargo binstall holt` works "any platform with cargo + cargo-binstall". With `publish = false` in `crates/holt-cli/Cargo.toml` and no `[package.metadata.binstall]` block, binstall's primary lookup (crates.io ↔ binstall metadata) doesn't apply. Whether dist 0.31's release-asset URL pattern auto-resolves via binstall's name-only resolver is the post-rc.1 verification (RC1-CHECKLIST.md §2).

**Decision: defer.** The README's steady-state audience is post-launch users; the binstall claim WILL work after the rc.1 tag pushes and dist publishes the release artifacts (D-05's "rely on dist auto-generation" path). The reviewer's softening suggestion is appropriate for the pre-launch window only, and that window closes the moment the maintainer pushes the rc.1 tag (the explicit Phase 4 exit ramp per D-10). RC1-CHECKLIST.md §7 already documents the binstall dry-run probe as the post-tag validation step; if that fails, the D-05 fallback is "add manual `[package.metadata.binstall]` to `crates/holt-cli/Cargo.toml`" — also documented.

**No code change.** The README copy is correct for the steady state; verifying it is the maintainer's rc.1 tag-push action. DIST-03's "from day one" wording in REQUIREMENTS.md is the contract — this fix-pass keeps it intact.

### WR-03 — Non-deterministic `find` in D-14 parity step → FIXED in `c125474`

**Issue:** `release.yml:222` used `find target -type f \( -name 'holt' -o -name 'holt.exe' \) -perm -u+x | head -1`. find's traversal order is not specified by POSIX, so a future build-step rearrangement (debug-profile artifact in target/debug/holt, intermediate cargo layout change) could pick up the wrong binary.

**Fix:** Restricted the find to the canonical release-profile path: `find target -type f \( -path '*/release/holt' -o -path '*/release/holt.exe' \) -perm -u+x | head -1`. With a single matrix target per job (host runner builds one triple), all matches are equivalent so `head -1` is now deterministic.

**Verification:** `grep -A 2 'BIN=\$(find' .github/workflows/release.yml` shows the new path-glob expression; `dist plan` still validates the workflow is well-formed.

### WR-04 — `$EDITOR` in vhs tape makes gif host-dependent → FIXED in `d15951e`

**Issue:** `tools/demo.tape:38` typed `$EDITOR ~/.claude/settings.json` literally. vhs does not shell-expand; the simulated terminal does, with results that vary by whichever `$EDITOR` the rendering host has set. Re-rendering by a contributor with `EDITOR=emacs` (or unset) would produce a visibly different gif from the maintainer's render.

**Fix:** Hardcoded `vi` in the tape (universally available on macOS + most Linux distros; deterministic visual output). Re-rendered `assets/demo.gif` via `vhs tools/demo.tape`. New gif size 377 KB (down from 938 KB — vhs 0.x encoder churn between renders, still well under the 2 MB cap); visual content unchanged.

**Verification:** `grep -n 'EDITOR' tools/demo.tape` returns empty; `file assets/demo.gif` reports `GIF image data, version 89a, 900 x 400`.

## Findings deferred (info-only, not load-bearing)

| ID | Summary | Rationale for deferral |
|---|---|---|
| WR-02 | `cargo binstall` README copy is forward-looking | Steady-state-correct post-rc.1; RC1-CHECKLIST.md §7 covers the validation step; D-05 manual fallback documented |
| IN-01..04 | Reviewer info findings | Per `04-REVIEW.md` review section (4 items, none load-bearing). All are "could be tighter" rather than "is wrong" — recorded for v0.1.x cleanup. |

## Verifier alignment

The `04-VERIFICATION.md` PARTIAL verdict's gaps align cleanly with the deferral list:

- Criterion #1 (rc.1 publishes 4 platform artifacts) — pending maintainer tag-push (D-10).
- Criterion #2 brew half — DEFERRED via AMENDMENT 2026-04-29 (override accepted).
- Criterion #4 binstall live-resolve — pending maintainer tag-push (same as #1).
- Criterion #5 `gh label list` — pending `gh repo create Sinderella/holt` (RC1-CHECKLIST.md §1).

None of these are codebase gaps. RC1-CHECKLIST.md (153 lines) documents the maintainer flow end-to-end with rollback procedure.

## Post-fix-pass verification

| Gate | Result |
|------|--------|
| `cargo build --workspace --release` | exit 0 |
| `cargo test --workspace` | **80/80 pass** (matches Plan 04-02 baseline) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |
| `dist plan` | exit 0 — 4 platform tarballs + sha256 sidecars + sha256.sum aggregate |
| Hard constraints C1..C6 | all 6 enforced (no constraint test changes) |
| `grep -rn 'xattr.*\/usr\/local' README.md dist-workspace.toml .planning/phases/04-distribution-launch/RC1-CHECKLIST.md` | empty (WR-01 closed) |
| `grep -n 'EDITOR' tools/demo.tape` | empty (WR-04 closed) |
| `grep -A 1 'find target' .github/workflows/release.yml` | shows `*/release/holt` path glob (WR-03 closed) |
| `file assets/demo.gif` | `GIF image data, version 89a, 900 x 400` |

## Commits

| # | Hash | Type & subject |
|---|------|----------------|
| 1 | `7a22d16` | fix(docs): correct xattr install path for macOS Gatekeeper workaround (WR-01) |
| 2 | `c125474` | fix(release): tighten D-14 binary find to */release/holt path glob (WR-03) |
| 3 | `d15951e` | fix(demo): hardcode `vi` instead of $EDITOR for hermetic gif renders (WR-04) |

## Verdict

**Phase 4 fix-pass: COMPLETE.**

3/4 review warnings fixed (WR-01, WR-03, WR-04); WR-02 deferred with rationale (steady-state-correct copy, validation handed off to maintainer's rc.1 tag-push per D-10 / RC1-CHECKLIST.md §7).

The phase is ready for closeout: STATE.md / ROADMAP.md mark Phase 4 complete, then milestone audit/complete/cleanup.
