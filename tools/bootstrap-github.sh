#!/usr/bin/env bash
# tools/bootstrap-github.sh — Phase 4.1 one-shot GitHub repo bootstrap.
#
# Provisions the holt GitHub repo from a clean local clone:
#   1. Verifies preconditions (gh auth, working tree clean, no origin remote yet).
#   2. `gh repo create <REPO> --public --source . --remote origin --push`.
#   3. Calls tools/setup-labels.sh against the new repo (idempotent label apply).
#
# Does NOT push the v0.1.0-rc.1 tag — that's a deliberate human decision per
# RC1-CHECKLIST.md §4 (tag pushes are irreversible-in-spirit; Homebrew/binstall
# mirrors cache them immediately).
#
# Usage:
#   ./tools/bootstrap-github.sh                    # actually bootstrap
#   ./tools/bootstrap-github.sh --dry-run          # print actions, do nothing
#   REPO=fork-user/holt ./tools/bootstrap-github.sh   # override default repo
#
# Exits non-zero on any precondition failure.

set -euo pipefail

REPO="${REPO:-Sinderella/holt}"
DRY_RUN=0
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=1
fi

run_or_print() {
    if [[ $DRY_RUN -eq 1 ]]; then
        printf "  [dry-run] %s\n" "$*"
    else
        printf "  → %s\n" "$*"
        "$@"
    fi
}

echo "==> bootstrap-github.sh — repo: ${REPO}, dry-run: ${DRY_RUN}"

# 1. Preconditions
echo "==> Preconditions"

# 1a. gh installed + authenticated
if ! command -v gh >/dev/null 2>&1; then
    echo "FAIL: gh CLI not found. Install via 'brew install gh' (macOS) or see https://cli.github.com/" >&2
    exit 2
fi
if ! gh auth status >/dev/null 2>&1; then
    echo "FAIL: gh is not authenticated. Run 'gh auth login' first." >&2
    exit 2
fi
echo "  ok: gh CLI authenticated"

# 1b. working tree clean (untracked files block)
if [[ -n "$(git status --porcelain)" ]]; then
    echo "FAIL: working tree is not clean. Commit or stash before bootstrap." >&2
    git status --short >&2
    exit 2
fi
echo "  ok: working tree clean"

# 1c. no origin remote yet (otherwise gh repo create errors confusingly)
if git remote get-url origin >/dev/null 2>&1; then
    EXISTING="$(git remote get-url origin)"
    echo "FAIL: 'origin' remote already exists (${EXISTING}). Bootstrap is a one-shot." >&2
    echo "      To re-bootstrap a fork, run: git remote remove origin" >&2
    exit 2
fi
echo "  ok: no origin remote configured"

# 1d. setup-labels.sh present + executable
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ ! -x "${SCRIPT_DIR}/setup-labels.sh" ]]; then
    echo "FAIL: ${SCRIPT_DIR}/setup-labels.sh missing or not executable." >&2
    exit 2
fi
echo "  ok: setup-labels.sh present and executable"

# 2. Create the GitHub repo + push main
echo "==> Creating GitHub repo + pushing main"
run_or_print gh repo create "${REPO}" --public --source . --remote origin --push --description "Rust statusLine for Claude Code (with an otter named Nak)"

# 3. Apply the 9 issue labels
echo "==> Applying CONTRIBUTING.md labels"
if [[ $DRY_RUN -eq 1 ]]; then
    printf "  [dry-run] REPO=%s %s/setup-labels.sh\n" "${REPO}" "${SCRIPT_DIR}"
else
    REPO="${REPO}" "${SCRIPT_DIR}/setup-labels.sh"
fi

echo "==> Bootstrap complete."
echo "    Repo:   https://github.com/${REPO}"
echo "    Next:   bump Cargo.toml versions to 0.1.0-rc.1, then 'git tag v0.1.0-rc.1 && git push origin v0.1.0-rc.1'"
echo "    Detail: .planning/phases/04-distribution-launch/RC1-CHECKLIST.md §4 (tag-push step)"
