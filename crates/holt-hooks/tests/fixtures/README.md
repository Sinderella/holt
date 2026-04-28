# Claude Code stdin fixtures

Captured stdin envelopes for the five hook events `holt-hooks` subscribes to at
v0.1: `PreToolUse`, `PostToolUse`, `Stop`, `Notification`, `SessionStart`.

These fixtures back every test that drives `holt_hooks::handle_event` and
`assemble_heartbeat`. They are **golden** — committed to the repo, reviewed at
PR time, and never silently mutated. They lock the defensive-parse posture
(`#[serde(default)]` on every `HookStdin` field) against CC stdin-shape drift
(see `.planning/research/PITFALLS.md` H5 — the v2.1.119 `effort.level: "xhigh"`
regression that broke other tools).

## Why golden? (D-02)

1. The defensive-parse contract is unfalsifiable without real stdin shapes. If
   we generate fixtures on the fly during tests, a regression that adds a
   `deny_unknown_fields` attribute or removes a `#[serde(default)]` would slip
   through silently.
2. PRs that change a fixture must explain WHY in the PR description. A reviewer
   cannot meaningfully review a hand-rolled JSON shift; the changelog of CC
   versions documents the expected diffs.
3. Forward-compat regression evidence: defensive parse must succeed on EVERY
   prior version's fixture, not just the current one.

## Versioning policy (D-02)

- One subdirectory per CC version (`v2.1.119/`, `v2.1.120/`, `pre-2.1.98/`).
- **Never delete** a version directory — old shapes are part of the regression
  net. New CC versions land alongside, not on top.
- The `pre-2.1.98/` directory is a deliberate synthetic fixture exercising the
  D-08 `cwd_label` fallback branch (no `workspace.git_worktree` field — that
  field landed in CC v2.1.98).

## How to refresh from a real CC install

When CC ships a new version that changes the stdin envelope:

1. Confirm `claude --version` reports the new version.
2. Install a one-off statusLine that dumps stdin to disk for ONE fire of each
   hook event. Example for PreToolUse:
   ```bash
   # ~/.claude/statusLine.sh
   #!/bin/sh
   cat > "/tmp/cc-stdin-${HOOK_EVENT_NAME:-unknown}.json"
   echo ""  # silent statusLine output
   ```
3. Fire each event by triggering the appropriate CC behavior (run a tool for
   PreToolUse + PostToolUse; let CC finish for Stop; trigger a permission
   prompt for Notification; restart CC for SessionStart).
4. Copy the resulting files into a new `v<NEW_VERSION>/` directory.
5. Run `cargo test -p holt-hooks` — the defensive-parse posture means all
   existing assertions should still hold even with new fields present. If a
   test fails it's because we made a load-bearing assumption about a field
   shape that changed; investigate before "fixing" the test.
6. Commit the new directory in a dedicated `test(holt-hooks): capture
   v<NEW_VERSION> stdin fixtures` PR.

## Synthetic fixture caveat

The fixtures in this repo were generated synthetically based on the documented
v2.1.119 stdin shape (CC docs at `code.claude.com/docs/en/changelog`). They
match the field set the defensive-parse posture relies on: `session_id`,
`cwd`, `transcript_path`, `hook_event_name`, `tool_name` (PreToolUse +
PostToolUse), `tool_input` / `tool_response`, `model.display_name`,
`workspace.git_worktree` (CC v2.1.98+ only), `last_assistant_at`,
`permission_mode`, `source` (SessionStart only), and the `effort.level: "xhigh"`
regression precedent (PITFALLS.md H5) on at least one v2.1.119 fixture.

When a real-capture refresh lands per the procedure above, replace the
synthetic v2.1.119 directory with the real one (commit message documents the
swap). All later versions then land as additional sibling directories.
