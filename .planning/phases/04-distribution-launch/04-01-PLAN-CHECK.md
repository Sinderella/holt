# Plan-check: 04-01 (dist scaffold + binstall + MSRV)

**Reviewer:** plan-checker
**Phase:** 4 — Distribution + launch
**Plan:** 04-01 — dist v0.31.0 scaffold + Cargo.toml audit + binstall verification + MSRV CI matrix + version smoke
**Mode:** goal-backward against ROADMAP §Phase 4 success criteria #1, #2 (binstall structural prereq half), #4 (MSRV half), per CONTEXT.md "ROADMAP success criteria mapping" table
**Inputs read:** 04-01-PLAN.md (1071 LOC), 04-CONTEXT.md (D-01..D-16), ROADMAP.md §Phase 4, REQUIREMENTS.md (DIST-01/03/05), Cargo.toml, .github/workflows/ci.yml, crates/holt-cli/tests/version_smoke.rs (current), `grep ^publish` audit across all 6 source crates (none currently set — audit work is real)

---

## Goal-backward coverage map

| ROADMAP success criterion | 04-01 share (per CONTEXT.md) | Owning task(s) | Status |
|---|---|---|---|
| #1 — `v0.1.0-rc.1` publishes 4 artifacts; `--version` matches tag | Workflow scaffold + local --version smoke | T2 (verbatim `dist init`) + T3 (target trim, tap, Win allow-fail, SHA-256) + T5 (sanity gate + route-finder comment) + T7 (CARGO_PKG_VERSION assertion) | OK with citation — end-to-end (artifact download + tag-derived parity) is correctly deferred to 04-02 D-14 |
| #2 — `brew install` + `cargo binstall` resolve to dist artifacts | Binstall structural prereq (URL pattern + repo reachability); tap config | T3 (tap = `Sinderella/homebrew-holt`) + T8 (informational dry-run + `repository` field 200/301 probe) | OK with citation — real binstall verification post-rc.1 is correctly deferred to 04-02 per orchestrator's brief and D-05 plan-time action |
| #4 (MSRV half) — `Cargo.toml` MSRV/Edition pinned + CI 1.87 matrix gate | Fully owned by 04-01 | T6 (msrv-linux/macos/windows jobs, build-only, allow-fail Windows) | OK — Cargo.toml MSRV pin already landed Phase 1; T6 closes the CI half |
| #4 (binstall metadata half — `pkg-url`/`pkg-fmt` present) | CONTEXT.md D-05 resolves: rely on dist auto-generation in release artifacts, NOT a manual `[package.metadata.binstall]` block in Cargo.toml | T3 (dist scaffold) + T8 (dry-run probe) | OK with citation — D-05 supersedes the literal ROADMAP "in `Cargo.toml`" wording; manual fallback documented for 04-02 if rc.1 dry-run mismatches |

All four 04-01-owned criterion halves trace to specific tasks; the must_haves.truths block (11 items) maps 1:1 onto these halves plus the C1..C6 read-only invariants.

---

## Dimension-by-dimension findings

### Dimension 1: Requirement coverage — OK

DIST-01 (dist scaffold) → T1+T2+T3. DIST-03 (binstall structural prereq) → T8 (informational; real verification 04-02). DIST-05 (MSRV pin + CI gate) → T6. All three frontmatter `requirements:` entries trace to ≥1 task. DIST-02 (Homebrew tap user-facing), DIST-04 (README demo), DIST-06 (tier statement), DIST-07 (labels + CONTRIBUTING) are correctly NOT in 04-01's `requirements:` list — they belong to 04-02 per D-16.

### Dimension 2: Task completeness — OK

All 9 tasks declare `<read_first>`, `<action>`, `<verify>` (all `<automated>`), `<acceptance_criteria>`, `<done>`. Tasks T1 and T8 are correctly typed as auto-but-no-files-modified (preflight tool gate + informational probe). Task T6 also carries a `<behavior>` block with 5 numbered tests, exceeding the auto-task minimum.

### Dimension 3: Dependency correctness — OK

`depends_on: []`, `wave: 1` in frontmatter. Plan 04-01 is the Wave-1 infra sibling; 04-02 will declare `depends_on: ["04-01"]`. No forward references; the plan never reads files that 04-02 will create.

### Dimension 4: Key links planned — OK

5 key_links declared: dist-workspace.toml → release.yml (via dist generate), release.yml → GitHub Releases (via v* tag push), dist-workspace.toml → tap (via tap publisher), ci.yml → MSRV gate (via dtolnay@1.87.0), version_smoke.rs → CARGO_PKG_VERSION (via env!() macro). Each link is realised by a specific task action (T2/T3/T6/T7) with a concrete verify command. No "create artifact in isolation" failure mode.

### Dimension 5: Scope sanity — WARNING (acceptable)

9 tasks at ~120 LOC of action prose each = a heavier-than-typical plan. Mitigations present:
- T1 + T8 are tool-availability gates with near-zero source impact.
- T9 is a verification gate, no source change.
- T2-T7 are the 6 actual implementation tasks — within the 5-task warning band when you net out the gates.
- File modification surface is bounded: 2 new files (`dist-workspace.toml`, `release.yml`), 1 modified workflow (`ci.yml`), 6 crate Cargo.toml audits (single-line additive change each), 1 test file replacement (≤25 LOC).

The plan is borderline but the task-shape correctly partitions preflight + impl + audit; no single task crosses the 10-file or 5-subtask threshold. **Acceptable; do not split.**

### Dimension 6: must_haves derivation — OK

11 truths, all user/maintainer-observable (e.g., "release.yml trigger is `on.push.tags: ['v*']`", "every non-holt-cli crate carries `publish = false`", "msrv job pinned to 1.87.0 build-only"). No implementation-detail truths. Artifacts list (8 entries) maps cleanly onto truths. Key_links (5 entries) cover the wiring that is otherwise easy to forget (tap config → release workflow, ci.yml → MSRV toolchain).

### Dimension 7: Context compliance — OK

Every locked decision in CONTEXT.md D-01..D-16 has at least one implementing task or an explicit deferral to 04-02:

- D-01 (run `dist init`, verbatim baseline) → T1+T2 (with bisect-friendly commit message `chore(dist): scaffold via dist init v0.31.x`)
- D-02 (publish = false; scope to holt-cli) → T3 (dist-workspace.toml scope) + T4 (per-crate audit)
- D-03 (4-target matrix + Windows allow-fail) → T3 + T5
- D-04 (tap = `Sinderella/homebrew-holt`) → T3
- D-05 (binstall via dist auto-gen; manual fallback documented but not added) → T8
- D-08 (MSRV 1.87.0 build-only CI job) → T6
- D-11 (`v*` tag-push trigger) → T2 (verbatim) + T5 (normalize if `dist init` chose narrower)
- D-12 (SHA-256 default) → T3
- D-13 (CARGO_PKG_VERSION smoke) → T7
- D-04 manual repo-create, D-06 demo gif, D-07 tier statement, D-09 setup-labels.sh, D-10 rc.1 tag, D-14 release-workflow `--version` parity, D-15 install-hooks budget reaffirmation → all explicitly deferred to 04-02 in `<objective>` block ("Plan 04-01 explicitly does NOT do") and `<verification>` mapping table; matches CONTEXT.md D-16.

### Dimension 7b: Scope reduction — OK

Searched the plan for `v1`, `simplified`, `static for now`, `placeholder`, `stub`, `not wired`, `future enhancement`, `too complex`. The single hit — T8's "informational at 04-01" — is **not** scope reduction; it is a CONTEXT.md-blessed deferral (D-05's URL-probe-against-rc.1-tag is impossible at 04-01 because rc.1 is pushed in 04-02). The orchestrator's brief explicitly names this characterization as correct. No silent simplification of any locked decision detected.

### Dimension 7c: Architectural tier compliance — N/A

Phase 4 makes no source-crate dependency-graph or tier changes; the entire plan is workspace-level config + manifest metadata + CI scripting. The Architectural Responsibility Map (C1..C6) is read-only this phase, which CONTEXT.md frontmatter explicitly states (`hard_constraints_in_scope: ... read-only invariants`).

### Dimension 8: Nyquist compliance — OK

Every task has `<automated>` verify commands. T1 (3 dist-version probes), T2 (6 grep/file/git assertions), T3 (10 grep + dist plan), T4 (7 audits + cargo build/test/2 hard-constraint tests), T5 (5 grep + dist plan), T6 (7 grep + python yaml + matrix audits), T7 (7 cargo test/grep/fmt/clippy), T8 (3 binstall/curl/dry-run informational), T9 (13 build/test/self-bench/clippy/dist plan). Sampling continuity ≥2-of-3 in every wave. Feedback latency: cargo build/test runs are the longest at ~30s on cold cache; no full-E2E playwright/cypress in scope. **PASS.**

### Dimension 8e: VALIDATION.md — N/A

Phase 4 is `--skip-research` per CONTEXT.md frontmatter; no RESEARCH.md or VALIDATION.md authored by design. CONTEXT.md is the authoritative input.

### Dimension 9: Cross-plan data contracts — OK

04-01 and 04-02 share two surfaces: (a) `dist-workspace.toml` — 04-01 creates, 04-02 reads (no transformation); (b) `.github/workflows/release.yml` — 04-01 creates, 04-02 amends with the D-14 `--version` parity step. The plan's T5 route-finder comment block proactively documents the customization layering so 04-02's edit doesn't surprise. No data-format conflict between the two plans.

### Dimension 10: CLAUDE.md compliance — OK

CLAUDE.md hard rules — `cargo fmt` + `cargo clippy --all-targets -- -D warnings` clean before commit, tests for runtime-affecting changes, one concept per PR, no `unwrap()` on render path, render path no-read of breach/timings, JSONC only in holt-cli, holt-render must not depend on holt-supervisor, always pipe stdio. Plan 04-01:
- T9 enforces fmt + clippy clean as a final gate (Step E).
- T9 re-runs architecture_dag (C2), cli_dep_boundary (C4), render_path_no_read (C6), sigkill_atomicity, install_hooks_concurrent, install_hooks_sigkill — covers C1..C6 + Phase 2 + Phase 3 atomicity invariants.
- T3 + T4 are explicitly partitioned into 4 atomic sub-commits each per "one concept per PR."
- T7 stays under 25 LOC and uses `concat!` + `env!` (no unwrap on render path; this isn't render path anyway).
- No render-path source files modified — Plan 04-01's files_modified list is purely manifests + workflows + one test.

Project mode YOLO + coarse + parallel + all gates on — plan-check is gate #2 of three; this plan-check authors that gate.

### Dimension 11: Research resolution — N/A

CONTEXT.md authoritative; no RESEARCH.md authored. CONTEXT.md "Open questions resolved (no carry-forward)" section enumerates 7 resolved-and-locked decisions (D-01, D-03, D-05, D-06, D-08, D-09, D-10). The 4 deferred-to-v0.1.x questions are correctly out of v0.1 scope.

### Dimension 12: Pattern compliance — N/A

Phase 4 introduces no new source-code patterns; it ships scaffold + CI + manifest metadata. PATTERNS.md is not authored for `--skip-research` plans. Existing repo patterns (CI YAML shape, atomic commit per concern) are followed verbatim — T6's MSRV job mirrors `test-windows` for the `continue-on-error: true` syntax.

---

## BLOCKER findings

**None.**

## WARNING findings

**W-1 — task count borderline (Dimension 5).** 9 tasks is one above the 4-task warning band. Net-of-preflight-gates (T1, T8, T9) the impl tasks are 6, which is itself above the 5-task threshold. Mitigation: each of T2-T7 is single-concern, ≤2 files modified, ≤30 LOC of source change. **Recommendation: do not split; the plan's natural grain is correct and a split would force 04-01 to span 3 plans, exceeding CONTEXT.md D-16's 2-plan partition.** Filed as warning for posterity.

**W-2 — `dist plan` validation depends on a tool not pinned in Cargo.lock (Dimension 8b).** T3/T5/T9 all run `dist plan`. If a future contributor runs the plan with a different `dist` binary version, validation may diverge from this plan's authored shape. Mitigation: T1 captures the exact `dist --version` output for the SUMMARY, and T2 commits the verbatim `dist init` output as a bisect-friendly baseline. **Recommendation: accept; the bisect baseline is the right escape hatch.**

## INFO findings

**I-1 — `holt-cli` carries `publish = false` (scope expansion vs literal D-02).** CONTEXT.md D-02 only requires non-`holt-cli` crates carry `publish = false`. Plan 04-01 T4 also adds `publish = false` to `holt-cli` itself, justified by "v0.1 ships via dist artifacts not crates.io." This is consistent with CONTEXT.md's "Open questions deferred to v0.1.x or v0.5" (which explicitly defers `cargo install holt` to v0.5+) and tightens the publish surface, but is technically a slight scope expansion. The plan documents the rationale in T4's commit message. **Acceptable; the closure is conservative and reversible.**

**I-2 — comment block in `release.yml` will be wiped by `dist generate` (Dimension 8b).** T5 adds a 12-line route-finder comment to `release.yml`. Future maintainers who run `dist generate` to refresh the workflow will lose the comment. The plan acknowledges this implicitly ("prefer regenerating via `dist generate`") but does not document a re-application playbook. **Recommendation: accept; this is 04-02's concern if/when dist drift forces a regenerate.**

**I-3 — D-05's plan-time action references "test pre-release tag (D-15)" but D-15 is the install-hooks README budget, not a tag.** This is an internal CONTEXT.md inconsistency — the reference should be D-10 (`v0.1.0-rc.1`). Plan 04-01 correctly resolves the ambiguity by deferring real verification to 04-02 post-rc.1-push, matching the orchestrator's brief and D-10's "the tag is **not** pushed during execution — the verifier flags the readiness, the maintainer pushes the tag manually after audit." **Not a plan-04-01 defect; CONTEXT.md typo to record but not block.**

---

## Hard-constraint preservation evidence (read-only this phase)

| Constraint | Test | Where re-run | Status |
|---|---|---|---|
| C1 — stdio piping | `--self-bench` exercises spawn path | T9 Step C | re-runs |
| C2 — render ⊥ supervisor | `tests/architecture_dag.rs` | T4 verify, T9 Step B | re-runs |
| C3 — settings.json mutation atomicity | `install_hooks_concurrent` + `install_hooks_sigkill` | T9 Step B | re-runs |
| C4 — JSONC only in holt-cli | `tests/cli_dep_boundary.rs` + per-crate grep audit | T4 verify, T9 Step B+D | re-runs |
| C5 — reader contract | indirectly via heartbeat round-trip | T9 Step A (workspace tests) | re-runs |
| C6 — render path no-read | `render_path_no_read` test + `--self-bench` strace gate | T9 Step B+C | re-runs |

Plus: `cargo tree -i tokio` empty audit (Phase 1 + Phase 2 + Phase 3 invariant), Phase 2 `sigkill_atomicity`, Phase 3 `install_hooks_concurrent` + `install_hooks_sigkill`. **All re-run in T9; no constraint regressions possible without T9 failing.**

---

## Files-modified surface confirmation

Plan declares 10 files in `files_modified`. Actual surface from task actions:
1. `dist-workspace.toml` (or `dist.toml`) — created by T2, customized T3
2. `.github/workflows/release.yml` — created by T2, customized T3+T5
3. `.github/workflows/ci.yml` — modified T6 (3 jobs appended; existing 6 byte-identical)
4. `crates/holt-schemas/Cargo.toml` — T4 (publish = false + repository.workspace = true)
5. `crates/holt-supervisor/Cargo.toml` — T4 (same)
6. `crates/holt-hooks/Cargo.toml` — T4 (same)
7. `crates/holt-orchestrator/Cargo.toml` — T4 (same)
8. `crates/holt-render/Cargo.toml` — T4 (same)
9. `crates/holt-cli/Cargo.toml` — T4 (same)
10. `crates/holt-cli/tests/version_smoke.rs` — T7 (replace 1:1, ≤25 LOC)

Plus possibly aux files dist generates (`.cargo/config.toml`, `Cross.toml`) — explicitly captured in T2 Step D as "commit them as part of this baseline." Frontmatter `files_modified` list is conservative; reality may be +1-2 dist-aux files. **Not a blocker; T2 handles the case.**

---

## VERDICT: PASS
