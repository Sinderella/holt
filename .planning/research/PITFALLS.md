# Pitfalls Research — holt

**Domain:** Rust CLI / cross-platform process supervisor / Claude Code statusLine tool
**Researched:** 2026-04-28
**Confidence:** HIGH on the verified GitHub issues; MEDIUM on platform-specific (APFS / fscrypt / Gatekeeper) traps where evidence is generic-rather-than-holt-specific

## Relationship to docs/

This file does **not** duplicate the pitfalls already named in `docs/`. The design wave already captured:

- `docs/01-findings.md` §1 (10 ranked pain points with GitHub links) and §4 (6 root causes for statusLine lag)
- `docs/02-scope.md` OUT table (7 deferred features with "why deferred" reasoning) and §3 (the "trigger to pivot" row if Anthropic ships native timing)
- `docs/03-orchestrator.md` §3 (the daemon-vs-files debate, resolved via shape-of-load)
- `docs/04-pet.md` §2 (the Clippy autopsy with 4 hard rules) and §5.3 (pet-decay reframe to drop emotional manipulation)
- `docs/05-schemas.md` §4 (rename consequences) and §5 (forward-compat plan)

This file **augments**: implementation-mid-flight traps a developer hits at the keyboard, plus a freshness check on five GitHub issues from `01-findings.md` to confirm they remain unfixed as of 2026-04-28. It treats the existing pitfalls as already-mitigated and focuses on what the design docs did not cover.

---

## Part A — Freshness check on GitHub issues from 01-findings.md §1

| # | Issue | Status as of 2026-04-28 | Implication for holt |
|---|-------|------------------------|---------------------|
| 1 | [#17020](https://github.com/anthropics/claude-code/issues/17020) — statusLine config silently ignored in 2.1.1 | **Closed as duplicate** — but *the wedge survives*: same class of "silently doesn't execute" reports has continued in [#43826](https://github.com/anthropics/claude-code/issues/43826) and [#52997](https://github.com/anthropics/claude-code/issues/52997) up through 2.1.119 (Windows). The bug class is alive even where the specific issue is closed. |
| 2 | [#18943](https://github.com/anthropics/claude-code/issues/18943) — input lag with multi-jq pipeline at full context | **Open** as of search. Still the canonical reproducer for the "your statusLine is killing input echo" wedge. |
| 3 | [#13783](https://github.com/anthropics/claude-code/issues/13783) — context_window cumulative-not-current (340k of 200k = 169%) | **Open**. Buffer-math correction (v1.0 in `PROJECT.md`) still needed. |
| 4 | [#21022](https://github.com/anthropics/claude-code/issues/21022) — 102MB JSONL freezes CC | **Closed as not-planned**. Anthropic has explicitly *declined* to fix this. holt's transcript-tail strategy in `docs/03-orchestrator.md` §2 ("bounded reverse-tail of ~200 lines") is therefore not a defensive choice, it's a load-bearing one — Anthropic is not going to bound transcript size for us. |
| 5 | [#50679](https://github.com/anthropics/claude-code/issues/50679) — statusLine not invoked during long task; activity indicator overwrites | **Open**. The 4-minute-gap reproducer in `01-findings.md` is current. holt's "render last-known-good cached" + heartbeat-driven liveness is what lets the bar tell the truth during this gap. |

**Bonus discoveries during freshness check** — additional issues that affect roadmap:

- [#28842 / #29217 / #28847 / #29143](https://github.com/anthropics/claude-code/issues/29143) — **`.claude.json` corrupts under concurrent CC writes**. This is exactly the class of bug holt's `install-hooks` must not introduce on `~/.claude/settings.json`. (Pitfall H1 below.)
- [#52089](https://github.com/anthropics/claude-code/issues/52089) — feature request: "expose session token usage to hooks and statusline commands." If Anthropic ships it before holt does, the buffer-math correction layer becomes redundant. Trigger-watch.
- [#34837](https://github.com/anthropics/claude-code/issues/34837) — `effortLevel "max"` is **not** serialized to settings.json and **not** passed to statusLine stdin. Even fields the docs say are exposed sometimes silently aren't. (Reinforces pitfall S1.)

**Bottom line:** All 5 spot-checked issues remain unmitigated as of 2026-04-28. The wedge has not eroded. Anthropic's April 2026 postmortem about CC quality decline addressed [model behavior](https://www.anthropic.com/engineering/april-23-postmortem), not statusLine plumbing. holt's runtime hygiene thesis is still load-bearing.

---

## Part B — Implementation pitfalls the design docs do not cover

### H1: `~/.claude/settings.json` mutation race (BLOCKER)

**What goes wrong:** `holt install-hooks` reads settings.json, JSON-merges its `hooks` block, writes back. If the user is *also* editing settings.json in a code editor (or another `holt install-hooks` is running from a second terminal, or CC itself is mutating `.claude.json`'s sibling), holt's read-modify-write produces a torn write. Worse: most editors write `.bak` files to the same directory; `holt`'s own `.bak` collides.

**Why it happens:** No coordination protocol exists for `~/.claude/`. CC itself doesn't lock its config files (see [#29143](https://github.com/anthropics/claude-code/issues/29143)). The user is not expecting their editor's swap files to be a problem.

**Prevention:**
- Use `fs2` crate's `FileExt::try_lock_exclusive()` on settings.json for the read-modify-write window. Fail fast with "another process is editing this file" if lock-acquire times out at 200ms.
- Write to `settings.json.holt-tmp.<pid>` then `rename(2)` — the PID suffix avoids collision with editor swap files (`.swp`, `.swo`, vim's `4913` probe file, VS Code's `.tmp.*`).
- Backup name: `settings.json.holt.bak` (NOT `.bak` — that's vim's territory).
- `--dry-run` prints the diff and exits 0 without locking.
- macOS APFS gives strong rename atomicity ([Apple APFS Features](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/Features/Features.html)); ext4 with `data=writeback` plus delayed allocation does not — call `fsync(2)` on the file descriptor *before* rename ([LWN article](https://lwn.net/Articles/789600/)).

**Warning signs:** User reports `settings.json: unexpected EOF`. CC fails to start. `holt install-hooks` exits successfully but hooks don't fire (because settings.json was rewritten by editor mid-merge).

**Phase:** v0.1 (the install-hooks subcommand is in the v0.1 IN list of `PROJECT.md`).

**Severity:** Blocker. Corrupting a user's settings.json on first install is a one-strike trust violation.

---

### H2: APFS vs ext4 atomic-rename divergence on heartbeat writes (BLOCKER)

**What goes wrong:** The heartbeat writer writes `<sid>.json.tmp` then renames. On macOS APFS, this is transactionally atomic ([Apple APFS Features](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/Features/Features.html)). On Linux ext4 with default `data=ordered`, POSIX says rename is atomic but ext4's *delayed allocation* means the new file's data may not have hit disk before the rename did — a power loss leaves a zero-byte heartbeat. The reader sees `{}` and crashes serde.

**Why it happens:** Heartbeat writers fire on every PreToolUse / PostToolUse / Stop event — many writes per second across a session lifetime. The probability of crash-during-write over a year is non-trivial. The reader must defend.

**Prevention:**
- Writer: `fsync(2)` on the temp file descriptor before `rename(2)`. POSIX-portable, costs ~1ms on SSD, and closes the ext4 delayed-alloc window ([npm/write-file-atomic#64](https://github.com/npm/write-file-atomic/issues/64)).
- Reader: deserialize with `serde_json::from_slice` returning `Result`; treat parse failure as "stale heartbeat" — do NOT panic. Reader must already handle stale heartbeats per orchestrator design (`docs/03-orchestrator.md` §3 dead-session detection); add zero-byte and parse-error to the same path.
- Test matrix: include a Linux ext4 + power-loss simulation (using `dm-flakey` in CI or just `kill -9` mid-write loop).

**Warning signs:** Reader logs `serde error: EOF while parsing object` or `unexpected end of file`. Single user complaint becomes a cluster.

**Phase:** v0.1 (heartbeat writer ships in v0.1).

**Severity:** Blocker. A statusline that panics on stale data is worse than no statusline.

---

### H3: `process-wrap` setpgid edge cases on macOS (BLOCKER)

**What goes wrong:** `process-wrap` ([docs.rs](https://docs.rs/process-wrap/latest/process_wrap/)) uses `setpgid` so that `killpg(SIGTERM)` reaches descendants of the wrapped script — the whole reason the crate is on the v0.1 IN list ([rust-lang/rust#115241](https://github.com/rust-lang/rust/issues/115241)). On macOS, three edge cases bite:

1. **Sandbox / SIP**: if holt is run from a sandboxed parent (Cursor, some VS Code variants under containerized terminals), `setpgid` succeeds but `killpg` against descendants may be denied silently. macOS `sandboxd` ([Boutnaru on Medium](https://medium.com/@boutnaru/the-macos-process-journey-sandboxd-sandbox-daemon-17c8c0efe8c9)) inherits to children but not always symmetrically.
2. **Launch Services daemonization**: scripts that exec a `.app` (e.g. a doctor-detected `osascript -e 'tell application "Slack"…'`) re-parent under `launchd`. `killpg` on holt's process group misses them.
3. **Session leader**: if CC itself is run from `nohup` or under `screen -d`, the existing process group may already be a session leader; `setpgid` then fails with EPERM. holt must not assume success.

**Why it happens:** Cross-platform process supervision is hard ([rust-lang/rust#115241](https://github.com/rust-lang/rust/issues/115241) catalogues why). The v0.1 IN list's "clean kill" promise needs caveats on macOS.

**Prevention:**
- Always check the return value of `setpgid` and surface a doctor warning if it failed (don't silently degrade to plain `kill`).
- After SIGTERM + grace period, fall back to walking `/proc/*/status` PPID chain (Linux) or `libproc` `proc_listchildpids` (macOS) for descendants we know we spawned, and SIGKILL them individually.
- Document the sandbox limitation in README under "Known limitations" — *"if you run CC inside another sandboxed process, holt's process-group kill may not reach all descendants; see `holt doctor --check-supervision`."*
- Add `holt doctor --check-supervision` test: spawn a script that forks 3 children, set timeout to 100ms, verify all 4 PIDs are gone after kill.

**Warning signs:** Breach log shows "killed parent, descendants survived." User reports growing zombie process count.

**Phase:** v0.1.

**Severity:** Blocker. The "clean Unix process-group kill" feature is a v0.1 IN bullet; if it doesn't work on macOS-with-sandboxed-CC, the wedge is leaky.

---

### H4: `$XDG_RUNTIME_DIR` missing on minority Linux distros (QUALITY)

**What goes wrong:** `PROJECT.md` and `docs/05-schemas.md` say heartbeats live at `$XDG_RUNTIME_DIR/holt/sessions/<sid>.json` on Linux. But XDG_RUNTIME_DIR is set by `pam_systemd` at login. Distros without systemd-logind (Devuan, Artix, Void without elogind, NixOS without home-manager configuring it, Raspberry Pi OS in some headless setups, [docker/ssh sessions on Manjaro](https://forum.manjaro.org/t/xdg-runtime-dir-not-set/137268), [Sway on Arch without seatd](https://github.com/swaywm/sway/issues/7202)) leave it unset.

**Why it happens:** XDG spec says fallback is the app's responsibility ([XDG Base Dir Spec](https://specifications.freedesktop.org/basedir-spec/latest/)). Tools that hard-code `$XDG_RUNTIME_DIR` panic with `Option::unwrap()` failures.

**Prevention:** Layered fallback, in order:
1. `$XDG_RUNTIME_DIR/holt` if set and writable
2. `/run/user/$UID/holt` if directory exists and is writable (logind convention even when env var was stripped — common with `sudo` and `su`, [Red Hat solution](https://access.redhat.com/solutions/6634751))
3. `${TMPDIR:-/tmp}/holt-$UID` with `0700` perms — same path holt uses on macOS, so the macOS path code becomes the universal fallback
4. **Never** `~/.cache/holt/sessions/` (cache dir may sync — Dropbox does cache by default on some configs)

Print a one-line warning to stderr the first time fallback fires. Document expected location in `holt doctor` output.

**Warning signs:** Test users on non-systemd distros report `holt: panicked at 'env var not set'`.

**Phase:** v0.1.

**Severity:** Quality. Affects a small but vocal Linux audience; their bug reports are the highest-effort to debug.

---

### H5: CC stdin JSON shape drift across versions, and the heartbeat hook can't be auto-updated (QUALITY)

**What goes wrong:** The hook script that writes heartbeats lives in user-space (`~/.claude/hooks/` or wherever holt installs it) and is small and copied across machines manually. It parses CC's stdin JSON. CC ships breaking changes:

- v2.1.119 added `effort.level` + `thinking.enabled` + `PostToolUse.duration_ms` (mentioned in `docs/03-orchestrator.md` §6).
- [#34837](https://github.com/anthropics/claude-code/issues/34837) — `effortLevel "max"` is *silently* not serialized into stdin even though the docs imply it is.
- [#52997](https://github.com/anthropics/claude-code/issues/52997) — v2.1.119 changed Windows behavior such that the command stops being executed at all.
- [#53417](https://github.com/anthropics/claude-code/issues/53417) — older session JSONLs silently stop being written after a CC version upgrade.

**Why it happens:** Anthropic's stdin schema is unversioned and undocumented at the level of "what-fields-can-disappear." Every CC release potentially breaks anyone's hook.

**Prevention:**
- The heartbeat-writer hook is a *Rust binary subcommand* (`holt write-heartbeat`), not a shell script. Same binary updates the hook logic when the user upgrades holt; no manual sync across machines.
- The hook does **defensive deserialization**: `#[serde(default)]` on every field except `session_id`. Unknown fields silently accepted (do NOT use `deny_unknown_fields`). Missing fields fall back to: `permission-mode` from JSONL tail (per `docs/05-schemas.md` §1 field semantics — already designed-in), then to "default."
- The hook writes `cc_stdin_schema_observed_at: "2026-04-28"` to the heartbeat as a debug field; `holt doctor` flags if the schema version observed is older than the holt build claims to support.
- Snapshot 5 verbatim CC stdin JSONs in `tests/fixtures/cc-stdin/v2.1.119.json` etc.; replay them in unit tests so a serde refactor doesn't accidentally break hook compat.

**Warning signs:** `holt peers` shows all sessions in `mode: default` after a CC upgrade — likely the hook is no longer reading the new mode field. Doctor reports "unfamiliar stdin shape."

**Phase:** v0.1 (hook ships in v0.1).

**Severity:** Quality. Silent degradation, not a crash. But silent degradation is exactly what `01-findings.md` §3 calls out as the wedge.

---

### H6: `permission-mode` JSONL line not yet observed in fresh sessions (QUALITY)

**What goes wrong:** `docs/05-schemas.md` §1 says `mode` is sourced from CC's `permission-mode` JSONL line type, falling back to stdin JSON. But `permission-mode` is only written when the user *changes* mode; a brand-new session that hasn't toggled plan-mode has no `permission-mode` line in its JSONL yet. Reading transcript tail returns no signal; if stdin JSON also doesn't carry it (CC v<2.1.119 didn't reliably), the heartbeat writes `mode: null`. The pet's plan-mode color flip and the bypassPermissions warning glyph never fire on first-session-of-the-day.

**Why it happens:** CC's "current mode" is a derived state with no canonical write event. Both the JSONL stream and the stdin JSON express it inconsistently across versions.

**Prevention:**
- Fallback chain (in heartbeat-writer): (1) CC stdin `permissionMode` field if present, (2) most-recent `permission-mode` JSONL line if any, (3) `default` as the hard floor — never `null`.
- The pet renderer treats absence of mode signal as "default" (no color flip), never as "wedged." The test for "is this session in plan mode" is *positive evidence required*.
- Log to breaches.log on first hook fire when fallback hits the floor — gives doctor a signal that mode detection regressed.

**Warning signs:** Plan-mode users say "the cyan tint isn't showing." Hard to reproduce because it only affects the first turn.

**Phase:** v0.1 (heartbeat hook ships in v0.1).

**Severity:** Quality. Breaks `PROJECT.md`'s "Plan-mode vs execute-mode color flip" v1.0 line if not handled at heartbeat-write time.

---

### H7: macOS Gatekeeper friction is *worse* than `02-scope.md` documents (QUALITY)

**What goes wrong:** `02-scope.md` and `PROJECT.md` defer code signing under "Gatekeeper warning is acceptable friction at MVP; document the workaround." The actual friction in macOS 15+ is **harsher than a warning**:

- macOS 15.1 (Sequoia point release) **removes** the right-click → Open → "Open Anyway" path for some unsigned binaries. Workflow is now: launch → dismiss block → System Settings → Privacy & Security → scroll to bottom → "Open Anyway" → re-launch ([MacRumors thread](https://forums.macrumors.com/threads/macos-15-1-completely-removes-ability-to-launch-unsigned-applications.2441792/)).
- For *terminal-launched* unsigned binaries (which holt is — statusLine commands fork from CC), the warning is silent: the binary fails to exec, CC's statusLine displays "command failed" with no surfaced cause. User sees a broken bar and blames CC.
- `xattr -d com.apple.quarantine /path/to/holt` removes quarantine but only for binaries with the attribute (downloaded via browser); cargo-binstall installs may or may not get the attr depending on how the user got the binary onto disk ([Open Ecoacoustics docs](https://openecoacoustics.org/resources/help-centre/software/unsigned/)).

**Why it happens:** Gatekeeper rules tighten with each macOS release. The "document the workaround" assumption from MVP scoping was based on Gatekeeper-as-of-2024; 2026 is stricter.

**Prevention:**
- **Default install path is `brew install <user>/holt/holt`** — Homebrew applies an ad-hoc signature via `codesign --sign -` ([Tuist blog](https://tuist.dev/blog/2024/12/31/signing-macos-clis)) which bypasses Gatekeeper for terminal-launched binaries. cargo-dist auto-generates the brew tap; this is the path of least friction on macOS by far.
- README's macOS install instructions lead with `brew`, not `cargo-binstall` or direct download.
- For users who do `cargo-binstall holt` or download tarballs directly: emit a one-time `holt doctor --first-run` block on first invocation that tests for quarantine attr and prints the exact `xattr -d` command if needed.
- Watch the issue tracker for ≥5 Gatekeeper-friction reports — the trigger condition for moving signing/notarization to the 1.0 promise (per `PROJECT.md` Out-of-Scope row). Until then, brew is the workaround.

**Warning signs:** macOS users report "holt installed but my statusLine is empty / shows a permissions error." Issue tracker collects these.

**Phase:** v0.1 distribution / README.

**Severity:** Quality. Doesn't crash, but kills first-run conversion on macOS.

---

### H8: Multiple holt versions on the same machine reading each other's heartbeats (QUALITY)

**What goes wrong:** User has holt v0.5 from `cargo-binstall` and v1.0 from brew, both binaries on `$PATH` in different terminals (because brew installs to `/opt/homebrew/bin` and cargo-binstall to `~/.cargo/bin`, ordering depends on shell rc). v0.5 hooks write `schema_version: 1` heartbeats. v1.0 (post-1.0) might write `schema_version: 2`. The v0.5 reader reads a v2 file. Per `docs/05-schemas.md` §5, v0.x readers should refuse to read v2 files with a clear "please upgrade" message — but the rule was written for the case of *the user's own holt being old*, not for the case of *two holts running side-by-side*.

**Why it happens:** Mac/Linux users routinely have stale binaries in older Cargo install dirs. Homebrew tap upgrades don't touch `~/.cargo/bin`. The PATH ordering question is genuinely undefined.

**Prevention:**
- The reader logs a one-line warning when it encounters an unrecognized `schema_version` and skips that heartbeat — does NOT crash, does NOT pollute the bar with errors.
- `holt doctor` reports the hook-writer binary path (resolved via `which holt` from inside CC's hook env — different from the `which` of the user's interactive shell) AND the reader binary path. If they differ, surface as a warning with the upgrade command for both.
- `holt --version` is included in every breach-log entry and every heartbeat (as `writer_version: "0.5.3"` field, per the `docs/05-schemas.md` §5 forward-compat plan).
- `holt install-hooks` writes the absolute path of the holt binary into `~/.claude/settings.json::hooks`, not bare `holt`. Removes PATH ambiguity entirely.

**Warning signs:** User says "the bar is showing stale data after I upgraded." Log shows "skipped heartbeat with schema_version=2."

**Phase:** v0.1 (the writer_version field) and post-v1.0 (the migration path itself).

**Severity:** Quality. Manifests after the v1.0→v1.x schema bump, but the prevention work has to be in v0.1.

---

### H9: holt reading its own breach log on the render path (QUALITY)

**What goes wrong:** Naive doctor implementation reads the breach log on every fire to compute "trend" or "regressing?" indicators for the bar. The breach log is the *output* of the render path. If reading it costs more than the render budget, the act of measuring slowdowns *causes* slowdowns. Storm.

**Why it happens:** It's a tempting integration — the breach log is structured, the doctor wants context, the render pipeline already opens files. The first prototype writes itself.

**Prevention:**
- **The render path never reads `breaches.log` or `timings.jsonl`.** Those are written by the render path and read by `holt doctor` only.
- If the bar wants a "session has had >N breaches today" badge, it reads a *summary* file written by `holt doctor` post-hoc (e.g., `~/.cache/holt/today.summary.json`, written when doctor runs, never on the render path).
- Write a unit test that runs the render path 100× and asserts no read of `breaches.log` happened.
- Codify in CONTRIBUTING.md "Architectural North Star" rule #1 elaboration: *"the render path is write-only on its own observability files."*

**Warning signs:** Tail-latency p99 grows over a week as the breach log grows. User reports "holt was fast at first, now it's slow."

**Phase:** v0.1 (set the constraint before any render-time observability code is written).

**Severity:** Quality. Architectural rule that prevents a class of regressions.

---

### H10: Pet diary on encrypted home directories (QUALITY)

**What goes wrong:** `docs/05-schemas.md` §2 says pet state lives at `~/.local/state/holt/pet/<name>.json` and the diary at `~/.local/share/holt/pet/<name>/diary.md`. On Linux home-dirs encrypted with **eCryptfs** (still default on some Ubuntu 18.04 LTS upgraders, Linux Mint installs through 21.x, some Manjaro layouts), the stacked-fs design imposes a real append latency penalty (Phoronix benchmarks show eCryptfs is the slowest of the three options ([Phoronix benchmark](https://www.phoronix.com/review/ext4-crypto-418))). eCryptfs is also unmaintained ([Arch Wiki](https://wiki.archlinux.org/title/ECryptfs)) and limits filenames to 143 bytes.

**Why it happens:** The diary is appended to from the heartbeat-write path on certain pet-event transitions. Even though writes are rare, when they happen and the user is on eCryptfs, latency is observable. Statusline lag during pet events is the failure mode the project exists to prevent.

**Prevention:**
- Pet-state writes happen *off the render path*: from a deferred queue inside the heartbeat-writer hook, never inside the statusline binary's render. The hook can take 50–100ms (per `docs/03-orchestrator.md` §5) — eCryptfs latency is fine there.
- The statusline binary never reads the diary; only `holt pet diary` does. Already designed-in (`docs/05-schemas.md` §2 reader list).
- `holt doctor --first-run` checks if `$HOME` is on eCryptfs (mount table inspection) and warns: *"your home is on eCryptfs (deprecated). Pet writes will work but consider migrating to fscrypt."*

**Warning signs:** Linux Mint user opens an issue: "holt is slow when pet does something." Almost certainly eCryptfs.

**Phase:** v1.0 (pet ships at v1.0 per the phased scope).

**Severity:** Quality. Niche but real.

---

### H11: schema_version=2 reader reading v1 file written by older holt (QUALITY)

**What goes wrong:** Mirror of H8 from the other side. After v1.0 ships and the user upgrades to a v1.5 that reads schema v2 heartbeats, but a never-upgraded peer terminal is still running v1.0 and writing v1 files. The v2-aware reader must accept v1 files (forward compat from `docs/05-schemas.md` §5 says one minor release of overlap), but only for *one* minor release. What happens at v1.5 when the v1.0 reader is gone but a v1.0 *writer* still runs? The v1.5 reader sees v1 files indefinitely.

**Why it happens:** `docs/05-schemas.md` §5 frames forward-compat as "new readers handle old files for one minor release after v2 ships," which is right for *new schema* but wrong for *old writers*. Old writers can persist for years.

**Prevention:**
- The reader's v1-handling code path is permanent for as long as v1.0 binaries can plausibly be in the wild — at minimum 2 years post-v2.0. Document this in `docs/05-schemas.md` §5 as a separate paragraph from "new readers handle v1 files."
- `holt doctor` reports the oldest `schema_version` and `writer_version` observed across alive heartbeats; nudges user to upgrade ancient peer sessions.
- Heartbeats include `writer_version` field (already proposed in H8 prevention) so doctor can be specific: *"session 12345 is writing schema_version=1, last seen at writer_version=0.5.3, please run `holt upgrade` in that terminal."*

**Warning signs:** Long-tail bug reports years post-v2.0 about peers showing stale or missing fields.

**Phase:** Post-v1.0 — but the foundation (writer_version field, permanent v1-read code path) is set in v0.1.

**Severity:** Quality.

---

### H12: Concurrent `holt install-hooks` from two terminals (QUALITY)

**What goes wrong:** User opens two terminals at once, runs `holt install-hooks` in both because of muscle memory or a setup script. Both read settings.json simultaneously, both compute the same merged result (idempotent so this is OK), both write back. Race window: first writer's rename happens, second writer's read is now stale, second writer's write reverts the first writer's work. In the *idempotent* case this is fine — but if the user has *also* edited settings.json in between (added a custom hook), the second writer's write removes that edit.

**Why it happens:** `install-hooks` is a one-shot setup command; users assume it's safe to re-run. The race is real; the idempotency assumption is shaky once the user has any other hooks.

**Prevention:** This is a special case of H1 — same `try_lock_exclusive()` defense applies. Specifically:
- Hold the exclusive lock for the entire read-merge-write window.
- The merge logic must be `read → diff → merge → write`, not `read → write canonical`. Idempotency of merge-with-other-hooks-present is what's load-bearing.
- Make `holt install-hooks --reinstall` an explicit flag that prints the diff and asks for confirmation (default-no on TTY) before stomping non-holt hook entries.

**Warning signs:** User opens an issue: "holt removed my custom PreToolUse hook."

**Phase:** v0.1.

**Severity:** Quality.

---

### H13: Setting `refreshInterval=1` to test, never resetting (QUALITY)

**What goes wrong:** During v0.1 testing the maintainer (or an early user) sets `refreshInterval: 1` in settings.json so the bar updates every second. They forget to revert. The shim wraps a previously-cheap statusLine command; multiplied by 1Hz the cumulative cost is 60× higher and now CC actually does feel slow — but the slowness is attributable to *holt-induced refresh frequency*, not to a real script regression. The breach log fires constantly. Issue reports follow.

**Why it happens:** The CC docs encourage `refreshInterval` for "real-time updates" but it's additive on top of turn-boundary fires. Easy to set, easy to forget.

**Prevention:**
- `holt doctor` warns when `refreshInterval ≤ 2` and the user's wrapped script's median fire takes >100ms — i.e., the configured cadence is faster than the script can keep up.
- README's installation instructions explicitly call out refreshInterval: *"holt is heartbeat-driven, you don't need to set refreshInterval. If you have it set, consider removing it."*
- Telemetry-via-issues: tag the first 5 user reports with `refreshInterval-misconfig` to validate the pattern.

**Warning signs:** Breach log shows tight regular cadence (~1s). User reports CC slowness post-holt-install.

**Phase:** v0.5 (doctor lands here; warning logic is part of doctor).

**Severity:** Quality.

---

## Part C — Quick-reference tables

### Severity rollup

| # | Pitfall | Severity | Phase |
|---|---------|----------|-------|
| H1 | settings.json mutation race | Blocker | v0.1 |
| H2 | APFS vs ext4 atomic-rename divergence | Blocker | v0.1 |
| H3 | process-wrap setpgid edge cases on macOS | Blocker | v0.1 |
| H4 | XDG_RUNTIME_DIR missing | Quality | v0.1 |
| H5 | CC stdin JSON shape drift | Quality | v0.1 |
| H6 | permission-mode JSONL not yet observed | Quality | v0.1 |
| H7 | macOS Gatekeeper friction worse than docs say | Quality | v0.1 |
| H8 | Multiple holt versions side-by-side | Quality | v0.1 (foundation) |
| H9 | Render path reading own breach log | Quality | v0.1 (constraint) |
| H10 | Pet diary on eCryptfs | Quality | v1.0 |
| H11 | v2 reader, persistent v1 writer | Quality | post-v1.0 (foundation v0.1) |
| H12 | Concurrent install-hooks | Quality | v0.1 |
| H13 | refreshInterval=1 forgotten | Quality | v0.5 |

### Pitfall-to-phase mapping (the load-bearing column for roadmap)

| Phase | Pitfalls to address | What "addressed" looks like |
|-------|---------------------|----------------------------|
| **v0.1 — runtime hygiene wedge** | H1, H2, H3, H4, H5, H6, H8 (foundation), H9 (constraint), H12 | settings.json locking; fsync-before-rename in writer; setpgid return-checking + descendant-walk fallback; layered XDG fallback; defensive serde with `#[serde(default)]`; `mode` fallback chain; `writer_version` field in heartbeat schema; render-path-write-only architectural rule documented in CONTRIBUTING.md |
| **v0.5 — doctor** | H7 (Gatekeeper detection), H13 (refreshInterval misconfig warn) | `holt doctor --first-run` quarantine + Gatekeeper test; refreshInterval-vs-script-cost cross-check warning |
| **v1.0 — orchestrator + pet** | H10 (eCryptfs) | Pet-write deferral off render path verified; eCryptfs detection + nudge in doctor |
| **post-v1.0** | H11 | v1-reader code path retained ≥2 years post-v2.0; doctor reports oldest writer_version |

---

## Part D — Sources

- **GitHub issues spot-checked (5):** [#17020](https://github.com/anthropics/claude-code/issues/17020), [#18943](https://github.com/anthropics/claude-code/issues/18943), [#13783](https://github.com/anthropics/claude-code/issues/13783), [#21022](https://github.com/anthropics/claude-code/issues/21022), [#50679](https://github.com/anthropics/claude-code/issues/50679) — all unresolved as of 2026-04-28.
- **Adjacent issues found during freshness check:** [#28842](https://github.com/anthropics/claude-code/issues/28842), [#29143](https://github.com/anthropics/claude-code/issues/29143), [#29217](https://github.com/anthropics/claude-code/issues/29217), [#34837](https://github.com/anthropics/claude-code/issues/34837), [#43826](https://github.com/anthropics/claude-code/issues/43826), [#52089](https://github.com/anthropics/claude-code/issues/52089), [#52997](https://github.com/anthropics/claude-code/issues/52997), [#53417](https://github.com/anthropics/claude-code/issues/53417).
- **Filesystem atomicity:** [Apple APFS Features](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/Features/Features.html), [LWN: A way to do atomic writes](https://lwn.net/Articles/789600/), [npm/write-file-atomic#64](https://github.com/npm/write-file-atomic/issues/64).
- **Process supervision in Rust:** [process-wrap docs](https://docs.rs/process-wrap/latest/process_wrap/), [rust-lang/rust#115241](https://github.com/rust-lang/rust/issues/115241), [Boutnaru on macOS sandboxd](https://medium.com/@boutnaru/the-macos-process-journey-sandboxd-sandbox-daemon-17c8c0efe8c9).
- **XDG_RUNTIME_DIR fallback:** [Red Hat solution 6634751](https://access.redhat.com/solutions/6634751), [swaywm/sway#7202](https://github.com/swaywm/sway/issues/7202), [Manjaro forum thread](https://forum.manjaro.org/t/xdg-runtime-dir-not-set/137268).
- **Gatekeeper:** [MacRumors macOS 15.1 Gatekeeper thread](https://forums.macrumors.com/threads/macos-15-1-completely-removes-ability-to-launch-unsigned-applications.2441792/), [Open Ecoacoustics: unsigned apps in terminal](https://openecoacoustics.org/resources/help-centre/software/unsigned/), [Tuist: signing macOS CLIs](https://tuist.dev/blog/2024/12/31/signing-macos-clis).
- **Linux home encryption:** [Phoronix EXT4 fscrypt vs eCryptfs benchmarks](https://www.phoronix.com/review/ext4-crypto-418), [Arch Wiki: eCryptfs](https://wiki.archlinux.org/title/ECryptfs).
- **Serde versioning:** [serde-rs/serde#1137](https://github.com/serde-rs/serde/issues/1137), [Rust Serde Versioning blog](https://siedentop.dev/posts/rust-serde-versioning/).
- **Anthropic April 2026 postmortem (context only — not a holt fix):** [April 23 postmortem](https://www.anthropic.com/engineering/april-23-postmortem).

---

*Pitfalls research for: holt — Rust statusLine for Claude Code with multi-session orchestration and an ASCII otter pet.*
*Researched: 2026-04-28*
