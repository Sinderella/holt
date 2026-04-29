# tools/demo.tape — D-06 demo gif source for README.
# Render: `vhs tools/demo.tape` (writes assets/demo.gif).
# Maintainer prerequisite: `brew install vhs` (charmbracelet/vhs).
#
# 4-step sequence (D-06):
#   1. cat slow-statusline.sh — show the user's existing slow statusLine.
#   2. time bash slow-statusline.sh — Claude Code blocks for ~5s.
#   3. edit ~/.claude/settings.json — wrap the script with `holt run --`.
#   4. time holt run -- bash slow-statusline.sh — bar returns instantly via LKG.

Output assets/demo.gif

Set Theme "Catppuccin Mocha"
Set FontSize 14
Set Width 900
Set Height 400
Set Padding 10
Set TypingSpeed 50ms
Set PlaybackSpeed 1.0

# ─── Step 1: introduce the slow statusLine the user has today ──────────────
Type "# Your existing statusLine — innocuous-looking, but it sleeps 5s on every fire."
Enter
Type "cat slow-statusline.sh"
Enter
Sleep 800ms

# ─── Step 2: simulate Claude Code firing it — bar blocks for 5 seconds ─────
Type "# Claude Code fires this on every event. Watch what happens to the bar."
Enter
Type "time bash slow-statusline.sh"
Enter
Sleep 5500ms

# ─── Step 3: apply the fix — wrap with `holt run --` ───────────────────────
Type "# Wrap it with 'holt run --'. One edit to ~/.claude/settings.json."
Enter
Type "vi ~/.claude/settings.json   # change command to: holt run -- bash slow-statusline.sh"
Enter
Sleep 1500ms

# ─── Step 4: re-fire — bar returns instantly via the LKG cache ─────────────
Type "# Re-fire. Holt supervises, times the script, falls through to the LKG cache instantly."
Enter
Type "time holt run -- bash slow-statusline.sh"
Enter
Sleep 1500ms

# ─── Closer ────────────────────────────────────────────────────────────────
Type "# Bar never blocks. Slow scripts diagnosed via 'holt doctor' (v0.5)."
Enter
Sleep 1200ms
