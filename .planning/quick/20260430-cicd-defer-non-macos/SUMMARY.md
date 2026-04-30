---
slug: cicd-defer-non-macos
status: complete
completed_at: 2026-04-30
---

# CI/CD: defer non-macOS builds until just before 1.0

## What changed

| File | Change |
|---|---|
| `.github/workflows/ci.yml` | `lint` job moved from `ubuntu-latest` → `macos-14`. Added `if: false` to 6 non-macOS jobs (`test-linux`, `test-windows`, `stable-linux`, `stable-macos`, `msrv-linux`, `msrv-windows`). Added a 19-line header banner explaining what's deferred + how to re-enable. |
| `.github/workflows/release.yml` | Replaced `on:` block (`pull_request` + `push.tags: ['v*']`) with `on: workflow_dispatch:` only, so the workflow no longer auto-fires. Added a 13-line header banner with the original triggers preserved as a comment block + manual-trigger instructions (`gh workflow run Release`). |

## Jobs now running on every push/PR

- `lint` — fmt + clippy on macOS arm64 (was Linux)
- `test-macos` — full workspace test + self-bench gate + hook self-bench gate (D-15)
- `msrv-macos` — MSRV 1.87.0 build-only

3 jobs total. All on `macos-14` runners.

## Jobs disabled (`if: false`)

- `test-linux` (was REQUIRED)
- `test-windows` (was best-effort)
- `stable-linux` (was informational)
- `stable-macos` (was informational — disabled per "only macos" → strict reading)
- `msrv-linux` (was REQUIRED)
- `msrv-windows` (was best-effort)

The job blocks are preserved verbatim — re-enabling is a 1-line edit per job (`if: false` → `if: true` or remove the line entirely).

## Release workflow

Trigger reduced from `[pull_request, push.tags: ['v*']]` to `[workflow_dispatch]`. The dist-generated job structure (4-target matrix, sha256 sidecars, `--version` parity check, etc.) is preserved untouched. To trigger a release manually:

```bash
gh workflow run Release
```

To restore auto-fire when 1.0 prep starts: edit `release.yml`'s `on:` block back to the original triggers (preserved as a comment block in the file header).

## Verification

| Check | Result |
|---|---|
| `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` | ok |
| `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"` | ok |
| `grep -c '^    if: false' .github/workflows/ci.yml` | 6 (expected: 6) |
| `cargo check --workspace` | exit 0 (no source change) |

## Defer marker

`v0.5+1.0-prep` — re-enable when the v0.5 milestone (`holt doctor`) closes and 1.0 prep begins. The header banners in both workflow files reference this marker so future-Claude (or future-maintainer) sees the deferral context without needing to read this summary.

## Why this was scoped as a quick task

Single concern (CI/CD scope reduction), no source-code changes, atomic-edit nature, acceptance criteria are mechanical (YAML parse + job inventory). The /gsd-quick default mode (no research, no discussion, no plan-checker) is appropriate.
