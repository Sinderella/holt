#!/usr/bin/env bash
# tools/setup-labels.sh — D-09 idempotent label apply for the holt repo.
#
# Applies the 9 labels documented in CONTRIBUTING.md to the GitHub repo.
# Idempotent: `gh label create --force` updates existing labels and creates
# missing ones; identical labels are no-ops. Run once before launch; re-run
# whenever CONTRIBUTING.md's label list changes.
#
# Prerequisite: `gh auth login` against an account with admin/maintain access
# to the target repo.
#
# Usage:
#   ./tools/setup-labels.sh                 # defaults to Sinderella/holt
#   REPO=other/repo ./tools/setup-labels.sh # override the target repo

set -euo pipefail

REPO="${REPO:-Sinderella/holt}"

if ! command -v gh >/dev/null 2>&1; then
  echo "error: gh (GitHub CLI) is not installed. https://cli.github.com" >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "error: gh is not authenticated. Run \`gh auth login\` first." >&2
  exit 1
fi

# Format: label_name|color_hex|description (colors mirror GH defaults).
labels=(
  "bug|d73a4a|Something is broken"
  "feature|a2eeef|Something we don't have but should"
  "question|d876e3|Clarification or design discussion"
  "windows|0e8a16|Windows-specific report (counts toward Windows tier-1 trigger)"
  "pet|fbca04|Nak — sprites, diary, bond mechanics"
  "runtime|1d76db|Shim, doctor, breach log, supervision"
  "orchestrator|5319e7|Heartbeat, peer awareness, cross-session render"
  "good first issue|7057ff|Small, well-scoped, no domain context required"
  "help wanted|008672|Domain expertise or testing on a platform we lack"
)

for entry in "${labels[@]}"; do
  IFS='|' read -r name color description <<<"$entry"
  echo "applying label: ${name}"
  gh label create "$name" --repo "$REPO" --color "$color" --description "$description" --force
done

echo "done. ${#labels[@]} labels applied to ${REPO}."
