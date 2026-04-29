# 04-02 Plan Check — goal-backward review

**Reviewed:** 2026-04-29
**Plan under review:** `.planning/phases/04-distribution-launch/04-02-PLAN.md`
**Sister plan (dependency):** `.planning/phases/04-distribution-launch/04-01-PLAN.md`
**Authoritative inputs:** ROADMAP Phase 4 success criteria #1, #2, #3, #5; CONTEXT decisions D-04, D-06, D-07, D-09, D-10, D-14, D-15; REQUIREMENTS DIST-02, DIST-04, DIST-06, DIST-07.

---

## Summary

| Goal-backward verification | Status |
|---|---|
| Plan's `must_haves.truths` map cleanly to ROADMAP criteria #1/#2/#3/#5 | OK |
| Each task has a verifiable acceptance criterion runnable locally | OK |
| Hard constraints C1..C6 are read-only this plan | OK |
| Plan does NOT push `v0.1.0-rc.1` (D-10) | OK |
| Maintainer-post-plan items correctly deferred (tag push, release run, real binstall resolution) | OK |
| 04-01 cross-plan contract honored (no `dist-workspace.toml` rewrite; `release.yml` is appended-to) | OK |
| Task 8 re-runs constraint suite + self-benches at full strength | OK |
| Task 5 handles `gh auth status` precondition without making it a blocker | OK |

**VERDICT:** PASS

Two minor warnings recorded below; none affect the plan's ability to deliver criteria #1, #2, #3, #5 when fully executed alongside 04-01.

---

## Goal-backward trace per ROADMAP criterion

### Criterion #1 — rc.1 publishes 4 artifacts; `holt --version` matches the tag

**04-02's owned half:** D-14 release-workflow `--version` parity check + RC1-CHECKLIST.md tag-push readiness gate.

| Required for criterion to be achievable | Plan task | Status |
|---|---|---|
| `release.yml` asserts binary's `--version` contains `${GITHUB_REF_NAME#v}` | Task 6 (action lines 778–809) | OK — bash step with `set -euo pipefail`, `grep -qF`, `if: startsWith(github.ref, 'refs/tags/v')` defense-in-depth |
| Local sanity-check the assertion logic without pushing a tag | Task 6 Step D + verify line 863 | OK — builds release binary locally and validates `grep -qF "$EXPECTED"` against current `Cargo.toml` |
| `dist plan` still validates after the workflow edit | Task 6 Step G + Task 8 Step F | OK — both tasks re-run `dist plan` |
| Maintainer's pre-tag / tag-push / post-tag flow documented (D-10) | Task 7 (RC1-CHECKLIST.md) | OK — Cargo.toml bump, tap repo bootstrap, label apply, tag push, post-tag binstall + `--version` checks, rollback path all enumerated |
| Plan does NOT push the tag itself | Task 7 acceptance criterion (line 1063) + Task 8 diff-footprint check (verify line 1183) | OK — `Cargo.toml` not bumped; only the 6 declared files mutated |

OK — criterion #1 is achievable end-to-end once 04-01's workflow exists and the maintainer follows RC1-CHECKLIST.md.

### Criterion #2 — `brew install` and `cargo binstall` resolve to dist artifacts

**04-02's owned half:** README install instructions naming the three resolution paths.

| Required for criterion to be achievable | Plan task | Status |
|---|---|---|
| README install block names `brew install Sinderella/holt` | Task 3 (Step B + verify line 502) | OK |
| README install block names `cargo binstall holt` | Task 3 (verify line 503) | OK |
| README install block names a prebuilt-binary path (GitHub releases) | Task 3 Step B line 434 | OK |
| `dist-workspace.toml` tap = `Sinderella/homebrew-holt` (D-04 infra half) | 04-01 Task 3 Step C | OK — confirmed via `<interfaces>` block lines 122–127 |
| Real `cargo binstall holt --version 0.1.0-rc.1 --dry-run` resolution deferred to maintainer | RC1-CHECKLIST.md Step 2 (post-tag) | OK — D-05 fallback (manual `[package.metadata.binstall]`) documented at line 985 |
| Homebrew tap repo bootstrap deferred to maintainer | RC1-CHECKLIST.md Step 2 (pre-tag) | OK — `gh repo create Sinderella/homebrew-holt` documented |

OK — `brew install Sinderella/holt` is the concise form Homebrew expands to `Sinderella/homebrew-holt/holt` (matches D-04's "Homebrew strips the prefix"). ROADMAP's `<user>/tap/holt` shape is satisfied by the equivalent `Sinderella/holt` tap shorthand.

### Criterion #3 — README leads with demo + first-30-line install + platform tier with link

**04-02's full ownership.**

| Required for criterion to be achievable | Plan task | Status |
|---|---|---|
| Reproducible demo source (vhs tape) | Task 1 | OK — ~30–80 LOC tape with `Output assets/demo.gif`, canonical `Type`/`Enter`/`Sleep` directives, D-06 4-step sequence |
| Rendered demo gif committed in-repo (D-06 — not asciinema-cast) | Task 2 | OK — `vhs tools/demo.tape` produces `assets/demo.gif`; size bound 1KB–2MB; `file` validates GIF89a |
| Gif embedded above the fold (`![demo](assets/demo.gif)` in first 30 lines) | Task 3 verify line 501 | OK |
| Three-line install block in first 30 lines | Task 3 verify lines 502–503 | OK |
| Platform tier paragraph linking `docs/02-scope.md` (D-07) within first 50 lines | Task 3 verify line 504 | OK — link target exists (verified `docs/02-scope.md` present at line 7 of repo) |
| Windows-tier trigger phrasing matches `docs/02-scope.md` verbatim | Task 3 verify line 505 | OK — `head -50` grep for "Windows-tagged issues" |
| First-run subsection per D-15 (`--dry-run` then apply; `.holt.bak` mention) | Task 3 verify lines 506–507 | OK |
| Below-the-fold v1.0 vision preserved verbatim | Task 3 verify lines 508–513 | OK — explicit greps for each section header + signature line |

OK — criterion #3 is fully owned and verifiable.

### Criterion #5 — repo labels configured + CONTRIBUTING.md routes labels

**04-02's full ownership.**

| Required for criterion to be achievable | Plan task | Status |
|---|---|---|
| Idempotent script applying CONTRIBUTING.md's 9 labels | Task 4 | OK — `gh label create --force` per label, `set -euo pipefail`, REPO env override, pre-flight `command -v gh` + `gh auth status` |
| All 9 labels match CONTRIBUTING.md exactly | Task 4 verify line 642 | OK — covers `bug`, `feature`, `question`, `windows`, `pet`, `runtime`, `orchestrator`, `good first issue`, `help wanted` (CONTRIBUTING.md lines 38–46 confirm) |
| Labels applied to live `Sinderella/holt` (or deferral documented) | Task 5 | OK — see "Task 5 auth handling" below |
| README bug-report section enumerates all 9 labels | Task 3 verify line 514 | OK — 7-label loop + 2 explicit URL-encoded checks (`labels/good%20first%20issue`, `labels/help%20wanted`) |
| CONTRIBUTING.md present at repo root | CONTEXT.md "What's already in place" line 40 | OK — confirmed by Read tool, file exists |

OK — criterion #5 is achievable; live apply has a documented fallback path (Task 5 Step A → RC1-CHECKLIST.md).

---

## Acceptance criteria detail

### Hard constraints C1..C6 — read-only

OK. Task 8 (verify lines 1170–1183) explicitly re-runs:
- `cargo test --test architecture_dag` (C2)
- `cargo test --test cli_dep_boundary` (C4)
- `cargo test -p holt-cli --test render_path_no_read` (C6)
- `cargo test -p holt-hooks --test sigkill_atomicity` (Phase 2 H2)
- `cargo test -p holt-cli --test install_hooks_concurrent` (Phase 3 H1)
- `cargo test -p holt-cli --test install_hooks_sigkill` (C3)
- `cargo tree -i tokio` (no async runtime leak)
- `cargo tree -p holt-render | grep holt-supervisor` (C2)
- per-crate forbidden-dep audit (`jsonc-parser` / `fs2` cli-only — C4)
- both self-benches (`--self-bench` D-13 / Phase 1; `--self-bench-hook` D-15 / Phase 2)
- diff-footprint check (verify line 1183) asserts no `crates/*.rs`, `crates/*Cargo.toml`, or `tests/*.rs` modified

The plan's `files_modified` frontmatter (lines 15–21) lists only 6 paths, none in `crates/` or `tests/`. C1 (stdio piping) is implicitly preserved because no spawn-site is touched; the self-bench gates exercise the spawn path empirically.

### `v0.1.0-rc.1` tag NOT pushed during plan execution

OK. Task 7 acceptance line 1063 explicitly requires Cargo.toml NOT be bumped. The verify at line 1053 documents this with an `INFO` echo if the maintainer pre-flight already happened, otherwise an `OK` confirming the bump remains a documented maintainer step. No `git tag` or `git push` invocation appears in any task action (Tasks 1–8); Task 7 only AUTHORS the checklist text containing those commands.

### Maintainer-post-plan deferrals correctly scoped

OK. Plan defers to maintainer:
- Tag push (`git tag v0.1.0-rc.1 && git push origin v0.1.0-rc.1`) — RC1-CHECKLIST.md "Tag and push" section (action lines 956–960)
- Actual release-workflow run — implicit on the tag push; the workflow itself was created by 04-01
- Cargo.toml version bump from `0.1.0` → `0.1.0-rc.1` — RC1-CHECKLIST.md pre-tag step 1 (action lines 906–913)
- Real `cargo binstall holt --version 0.1.0-rc.1 --dry-run` against the published rc.1 — RC1-CHECKLIST.md post-tag step 2 (action lines 980–985)
- Homebrew tap repo bootstrap (`gh repo create Sinderella/homebrew-holt`) — RC1-CHECKLIST.md pre-tag step 2 (action lines 917–923)
- D-05 manual `[package.metadata.binstall]` fallback if dist auto-generated metadata fails — documented at action line 985

D-10's "the verifier flags the readiness, the maintainer pushes the tag manually after audit" is correctly operationalized.

### 04-01 cross-plan contract

OK. The plan reads but does not modify `dist-workspace.toml`:
- Task 6 (action lines 758–795) only edits `.github/workflows/release.yml`, never `dist-workspace.toml`
- Task 8 verify line 1182 re-runs `dist plan` to confirm 04-01's `dist-workspace.toml` still validates

The plan APPENDS to `.github/workflows/release.yml` (does not replace):
- Task 6 action line 773: "Append a `--version` parity check step to the matrix build job"
- Task 6 Step F (action lines 837–844): updates the route-finder comment block from 04-01 Task 5 (adds a 5th line; does not delete the existing 4)
- `<verification>` block lines 1222–1223: "the release.yml edit (Task 6) APPENDS a step; it does not modify any pre-existing 04-01 step"

The verify commands at Task 6 (lines 856–861) only check for the *presence* of the new step — they do not assert anything about the absence of existing 04-01 steps, which is correct for an append-only edit.

### Task 8 green-light gate at full strength

OK. Task 8 mirrors 04-01's Task 9 verification matrix exactly:
- Build + test sweep (action Step A; verify lines 1170–1172)
- 6 hard-constraint tests (Step B; verify lines 1173–1174 cover 2 of 6, but the action invokes all 6 explicitly at lines 1097–1101 — see WARNING #1 below)
- Both self-benches (Step C; verify lines 1175–1176)
- Forbidden-crate audit (Step D; verify lines 1177–1179)
- fmt + clippy (Step E; verify lines 1180–1181)
- `dist plan` (Step F; verify line 1182)
- Diff footprint sanity (Step H; verify line 1183)

No regressions allowed; "test count must equal 04-01's exit count" is the explicit acceptance contract (line 1187).

### Task 5 `gh auth status` handling

OK. Task 5 Step A (action lines 666–680) explicitly handles three states:
1. `gh auth status` passes → run script + verify all 9 labels via `gh label list`
2. `gh auth status` fails → flag in SUMMARY, defer apply to maintainer, RC1-CHECKLIST.md captures the deferred step
3. Either path satisfies the acceptance criterion (line 742–745)

The verify commands (lines 736–738) gate the live-apply behind an `if gh auth status >/dev/null 2>&1; then ... else echo INFO ... fi` so an unauthenticated executor doesn't fail the task spuriously. RC1-CHECKLIST.md pre-tag step 3 (action lines 926–932) documents the apply as a maintainer pre-flight, satisfying the safety-net requirement.

This is the correct shape per the acceptance brief: "the plan should document the prerequisite without making it a blocker."

---

## Issues found

### WARNING #1 — Task 8 verify block under-covers the 6 hard-constraint tests it runs

**Severity:** WARNING
**Dimension:** task_completeness (verification specificity)
**Description:** Task 8 action Step B (lines 1094–1102) invokes 6 hard-constraint tests:
1. `cargo test --test architecture_dag`
2. `cargo test --test cli_dep_boundary`
3. `cargo test -p holt-cli --test render_path_no_read`
4. `cargo test -p holt-hooks --test sigkill_atomicity`
5. `cargo test -p holt-cli --test install_hooks_concurrent --release`
6. `cargo test -p holt-cli --test install_hooks_sigkill --release`

But Task 8's `<automated>` verify block (lines 1170–1183) only spot-checks **2 of 6** with explicit assertions:
- `cargo test --test architecture_dag` (line 1173)
- `cargo test --test cli_dep_boundary` (line 1174)

The other 4 (render_path_no_read, sigkill_atomicity, install_hooks_concurrent, install_hooks_sigkill) are subsumed by the umbrella `cargo test --workspace 2>&1 | grep -E "test result.*FAILED" | grep -c .` returning 0 (line 1171), but a regression in any of those individual tests would be reported as part of the workspace failure count without the named-test specificity. The acceptance criterion at line 1188 names all 6 explicitly; the verify block should mirror that for executor clarity.

**Fix hint:** Add 4 explicit `<automated>` lines mirroring the named-test pattern from lines 1173–1174 (e.g., `cargo test -p holt-cli --test render_path_no_read 2>&1 | grep -F "test result: ok" | grep -c . returns at least 1`). Cost: 4 lines of YAML; benefit: regression diagnostics.

**Blocking?** No. The umbrella `cargo test --workspace` invocation (line 1171) does fail the verify if any of the 6 fail, so the criterion is achievable; the warning is a diagnostic-quality issue, not a coverage gap.

### WARNING #2 — RC1-CHECKLIST.md's Cargo.toml bump script may break some non-leaf crates

**Severity:** WARNING
**Dimension:** key_links_planned (cross-artifact wiring)
**Description:** RC1-CHECKLIST.md pre-tag step 1 (Task 7 action lines 906–913) instructs:
```bash
for c in holt-schemas holt-supervisor holt-hooks holt-orchestrator holt-render holt-cli; do
  sed -i.bak 's/^version = "0\.1\.0"$/version = "0.1.0-rc.1"/' "crates/${c}/Cargo.toml"
done
```

This `sed` pattern matches **only** lines that are exactly `version = "0.1.0"` with no leading/trailing whitespace and no inline comment. If any crate has inherited the workspace version via `version.workspace = true` (which 04-01 may have introduced for consistency, though the current per-crate manifests use literal `version = "0.1.0"` per CLAUDE.md / 04-01 interfaces lines 199–207), or if a maintainer hand-formats with a trailing comment, the regex misses and that crate keeps `0.1.0` — silently breaking the D-13 version smoke test once the rc.1 tag is pushed.

**Fix hint:** RC1-CHECKLIST.md should either (a) recommend a single workspace-level version bump (`Cargo.toml [workspace.package] version = "0.1.0-rc.1"`) once 04-01 confirms inheritance is wired in, OR (b) follow the sed loop with a verification step:
```bash
for c in holt-schemas holt-supervisor holt-hooks holt-orchestrator holt-render holt-cli; do
  grep -q '^version = "0.1.0-rc.1"' "crates/${c}/Cargo.toml" \
    || (echo "FAIL: ${c} did not bump"; exit 1)
done
```

This is a documentation-quality warning, not an execution blocker — the maintainer running the checklist will see the failed `cargo test version_smoke` before pushing the tag, and the rollback path is documented.

**Blocking?** No. The criterion is achievable end-to-end; the warning surfaces a foot-gun in a maintainer-side script that is itself defended by the D-14 in-workflow parity check.

---

## Hard constraints check

| Constraint | Status | Evidence |
|---|---|---|
| C1 (stdio piping) | Read-only | No spawn-site touched; Task 8 self-benches exercise the path |
| C2 (`holt-render` ⊥ `holt-supervisor`) | Read-only | Task 8 verify line 1178 |
| C3 (settings.json mutation: lock + fsync + tmp + `.holt.bak`) | Read-only | Task 8 runs `install_hooks_sigkill` + `install_hooks_concurrent` |
| C4 (JSONC handling cli-only) | Read-only | Task 8 verify line 1179 (per-crate grep audit) |
| C5 (reader treats stale-or-corrupt as missing) | Read-only | No reader-path source touched |
| C6 (render path never reads breaches/timings) | Read-only | Task 8 runs `render_path_no_read` |

All 6 stay green.

---

## Decision compliance (locked CONTEXT decisions in scope for 04-02)

| Decision | Implementing task(s) | Status |
|---|---|---|
| D-04 (Homebrew tap user-facing) | Task 3 (README install block); RC1-CHECKLIST tap repo bootstrap | OK |
| D-06 (vhs gif, not asciinema-cast) | Tasks 1 + 2 | OK — tape source + rendered binary committed |
| D-07 (platform tier in README, link to `02-scope.md`) | Task 3 | OK — verbatim trigger phrasing per `docs/02-scope.md` line 7 |
| D-09 (idempotent `gh label create --force` script) | Tasks 4 + 5 | OK — `--force` flag, REPO env override, 9 labels |
| D-10 (rc.1 readiness; tag NOT pushed by plan) | Task 7 | OK — RC1-CHECKLIST.md authored; no tag push in plan |
| D-14 (release-workflow `--version` parity) | Task 6 | OK — `${GITHUB_REF_NAME#v}` + `grep -qF` |
| D-15 (README first-run reaffirmation) | Task 3 first-run subsection | OK — `--dry-run` + `.holt.bak` documented |

No locked decision contradicted; no deferred ideas (GPG signing, crates.io publish, notarization) leak into the plan.

---

## Requirement coverage

| Requirement | Plan coverage |
|---|---|
| DIST-02 (Homebrew tap user-facing instructions) | Task 3 README install block + RC1-CHECKLIST tap bootstrap |
| DIST-04 (README leads with demo + 3-line install in 10s) | Tasks 1, 2, 3 |
| DIST-06 (README platform tier + trigger criteria link) | Task 3 (D-07 paragraph) |
| DIST-07 (CONTRIBUTING.md present + labels configured) | Tasks 4 + 5 (script + apply) |

All 4 requirements declared in `requirements:` frontmatter (lines 11–14) have implementing tasks. None silently dropped.

---

## VERDICT: PASS

The plan, executed as written alongside its already-validated 04-01 sibling, leaves the codebase in a state where ROADMAP success criteria #1, #2, #3, and #5 are achievable. The two warnings recorded above are diagnostic-quality polish items, not coverage gaps; both are recoverable from at execution time without re-planning. The plan correctly stops at the maintainer's tag-push handoff per D-10 and ships a self-contained RC1-CHECKLIST.md so the handoff is mechanically reproducible.
