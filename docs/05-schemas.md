# Schemas — heartbeat & pet state, v1

**Date locked:** 2026-04-28
**Versioning:** Both schemas carry `schema_version: 1`. Breaking changes bump the version; readers handle older versions for at least one minor release before dropping.

---

## 1. Heartbeat schema (per-session)

**Path:** `$XDG_RUNTIME_DIR/holt/sessions/<session_id>.json` (Linux) or `${TMPDIR}/holt-${UID}/sessions/<session_id>.json` (macOS).

**Writer:** A holt-installed hook running inside CC (`PreToolUse`, `PostToolUse`, `Stop`, `Notification`, `SessionStart` events). Single writer per file → no locking needed.

**Reader:** Every fire of every session's statusLine binary. Reads all heartbeat files; treats `mtime > 2 × refreshInterval` as stale.

**Schema:**
```json
{
  "schema_version": 1,
  "session_id": "uuid-from-cc",
  "pid": 12345,
  "started": "2026-04-28T08:00:00+0700",
  "updated": "2026-04-28T08:34:12+0700",
  "cwd": "/Users/thanats/projects/auth-worktrees/feat",
  "cwd_label": "auth/feat",
  "mode": "default",
  "current_tool": "Bash",
  "blocked_on": null,
  "context_pct_real": 0.42,
  "burn_rate_usd_per_min": 0.034,
  "last_assistant_at": "2026-04-28T08:34:01+0700",
  "model_display": "Opus"
}
```

**Field semantics:**
- `mode` ∈ `default | plan | acceptEdits | bypassPermissions` — sourced from CC's `permission-mode` JSONL line type, falling back to stdin JSON if not yet observed.
- `current_tool` is the name of the tool currently in flight (last `tool_use` without matching `tool_result`), or `null` if idle.
- `blocked_on` ∈ `null | "permission" | "stalled" | "wedged"` — derived state. `"stalled"` means no token activity for >5s; `"wedged"` means heartbeat itself is stale (the reader infers this — writer never claims it).
- `context_pct_real` is the *autocompact-corrected* context fraction (we apply the buffer correction; the JSON field CC passes can lie).
- `burn_rate_usd_per_min` is rolling over the last ~60s of usage entries, derived from transcript tail.
- `cwd_label` is the rendered worktree label (`<repo>/<branch>` by default; override with `HOLT_LABEL` env var).

**Atomic write rule:** Writer always writes to `<file>.tmp` then renames to `<file>`. POSIX rename is atomic on the same filesystem; readers never see a half-written heartbeat.

---

## 2. Pet state schema (per-pet)

**Path:** `~/.local/state/holt/pet/<name>.json`. Only one pet per machine at v1.0; multi-pet is post-1.0.

**Writer:** holt itself, on certain events (first run, naming, rename, friendship milestones, exhaustion events, contented moments). Append-only on the `memories` array; `friendship` is keyed-update.

**Reader:** `holt pet diary`, `holt pet status`, `holt pet friends`. Statusline binary does NOT read this — it reads the heartbeat. Pet-state file is for the bond layer's UX, not the live render.

**Schema:**
```json
{
  "schema_version": 1,
  "name": "Nak",
  "born": "2026-04-28T08:21:56+0700",
  "renamed_history": [],
  "memories": [
    {
      "date": "2026-04-28",
      "event": "first build success",
      "session_id": "uuid"
    },
    {
      "date": "2026-04-29",
      "event": "exhausted at 91% context, you compacted just in time",
      "session_id": "uuid"
    },
    {
      "date": "2026-05-02",
      "event": "met auth/feat Nak for the first time"
    }
  ],
  "friendship": {
    "auth/feat": {
      "first_met": "2026-05-02T10:00:00+0700",
      "last_met": "2026-05-12T16:30:00+0700",
      "hours": 8.4,
      "merges": 2,
      "exhaustions": 1
    }
  },
  "stats": {
    "total_sessions": 47,
    "total_cost_usd": 12.34,
    "total_exhaustions": 3,
    "total_merges_observed": 4
  }
}
```

**Memory eviction:** memories are append-only with a configurable cap (default: last 200 events). Older memories trim from the front when cap is exceeded; never deleted unless the user explicitly runs `holt pet diary --forget <date>`. Trimmed memories are archived to `~/.local/state/holt/pet/<name>.archive.jsonl` for posterity (read-only after archive).

---

## 3. Friendship aggregation rules

When two heartbeat files (`A` and `B`) are both alive (mtime within 2× refreshInterval) on the same machine, their pets are "meeting."

**Tracking rules:**
- `first_met` — set the first time both are alive simultaneously, never overwritten.
- `last_met` — updated to `now` on every fire where both are alive.
- `hours` — incremented by `(now - last_render_time)` where `last_render_time` is the previous fire that observed both alive. Capped at 0.5h per increment to avoid drift if a session sleeps for hours.
- `merges` — incremented when a `git merge` event is observed in either session's transcript while both are alive. (Stretch — defer if it gets complex.)
- `exhaustions` — incremented when either pet enters the exhausted state (real context >90%) while both are alive. Diary captures the moment.

**Friendship thresholds (UX, not data):**
| Hours | Diary says | Why |
|-------|-----------|-----|
| <1 | "Met for the first time" | Single-fire awareness |
| 1–8 | "You've worked together a few sessions" | Day-scale familiarity |
| 8–40 | "Old friends" | Workweek-scale bond |
| 40+ | "Inseparable" | Multi-week consistency |

These thresholds are tunable in `~/.config/holt/config.toml` under `[pet.friendship]`. Defaults reflect a typical paid-developer workweek.

---

## 4. Pet rename consequences

When a user runs `holt pet rename Nak Otto`:

1. `name` field updates immediately to `Otto`.
2. `renamed_history` appends `{"from": "Nak", "to": "Otto", "when": "<now>"}`. Append-only.
3. **Past memories keep the original name embedded in the event text.** *"first build success"* stays the same. *"met auth/feat Nak for the first time"* stays the same. We never rewrite history — that breaks the emotional continuity the bond layer is built on.
4. **Future memories use the new name.** *"first build success after rename"* etc.
5. `holt pet diary` shows the rename moment as its own entry: *"2026-05-12 — Nak became Otto today. Same otter, new name."*
6. `friendship` keys (worktree labels) are unaffected — they reference the *peer's* name at the time of meeting; the rename of *your* pet doesn't propagate.

**Why no retcon:** The Replika/Tamagotchi research said callback continuity is what builds long-term bonds. If the user can rewrite history, the diary loses its load-bearing property of being a true log.

---

## 5. Forward compatibility

When `schema_version` bumps to 2:
- Old readers (v0.x of holt) refuse to read v2 files; print a clear "please upgrade holt" message; never silently corrupt.
- New readers (v1.x of holt) handle v1 files transparently for at least one minor release after v2 ships.
- Migration is one-way: a `holt migrate-state` subcommand upgrades v1 → v2 in place with a backup at `<file>.v1-backup`.

The schema is locked at v1 today. Changes in 0.x development before v1.0 release are free; changes after v1.0 require version bumps.
