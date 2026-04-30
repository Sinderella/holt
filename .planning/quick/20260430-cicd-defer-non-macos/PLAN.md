---
slug: cicd-defer-non-macos
created: 2026-04-30
type: quick
---

# CI/CD: defer non-macOS builds until just before 1.0

## Problem

The CI/CD pipeline does too much for v0.5-cycle work. Many of the non-macOS builds are broken on the freshly-bootstrapped GitHub repo (Windows, Linux variants, stable-toolchain informational jobs). Maintainer wants only macOS active for now; non-macOS work is deferred to "just before 1.0" — when binary distribution becomes load-bearing again.

## Decisions

1. **`if: false` over commenting-out.** Preserves the YAML shape, makes re-enablement a 1-line edit per job, and keeps GitHub Actions parsing the file (so a syntax break in a deferred section is still caught at parse time).
2. **Include `lint` in the deferral.** User said "Enable only macos for now" — strict reading. Either move lint to `macos-14` or disable it. Moving to macOS is fine (lint is cheap there too).
3. **`release.yml` trigger reduced to `workflow_dispatch` only.** The workflow stays in the repo (so it can be re-enabled by uncommenting the original triggers) but never auto-fires. The dist 4-target matrix internals are left untouched.
4. **Add a clear header banner** to both files explaining what's deferred and how to re-enable.
5. **Defer marker:** "v0.5+1.0-prep" (re-enable when the v0.5 milestone closes and 1.0 prep starts).

## Tasks

1. Edit `.github/workflows/ci.yml` — move `lint` to `macos-14`, add `if: false` to every non-macOS job, add header banner with re-enable instructions.
2. Edit `.github/workflows/release.yml` — replace `on:` with `workflow_dispatch:` only, preserve original triggers as commented-out, add header banner.
3. `cargo check --workspace` — nothing source-affecting changed; sanity check.
4. `actionlint` if available, else `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` to confirm YAML parses.
5. Commit atomically.
6. Update STATE.md "Quick Tasks Completed" table.
