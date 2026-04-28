# Research Summary — holt

**Project:** holt — Rust statusLine for Claude Code with multi-session orchestration and ASCII otter pet (Nak)
**Domain:** Rust CLI / process supervisor / statusLine runtime
**Researched:** 2026-04-28
**Confidence:** HIGH (architecture locked; stack versions verified; pitfalls GitHub-confirmed)

---

## 1. Relationship to docs/

The five design documents in `docs/` (`01-findings.md` through `05-schemas.md`) are locked and authoritative. They own the wedge thesis, competitive analysis, MVP scope IN/OUT tables, orchestrator architecture, Nak pet design, and v1 schemas. The four research files this summary synthesizes are **augmentation only** — they confirm those decisions still hold as of 2026-04-28 and surface three categories of new information the design docs deliberately deferred: version drift since the docs were written, implementation-specific gotchas a developer would hit at the keyboard (never covered in design docs), and a small set of new CC ecosystem features the docs missed. **Do not re-read docs/ to understand the project; read this summary to understand what has changed since docs/ was written.**

---

## 2. Drift since 2026-04-28

### Version bumps

| What changed | Old reference | Correct now | Source | Implication |
|---|---|---|---|---|
| `process-wrap` crate version | `docs/02-scope.md` cites v6.0.0 | **v9.1.0** (2026-03-08; MSRV 1.87) | STACK.md | Pin `Cargo.toml` to `process-wrap = "9.1.0"` and `rust-version = "1.87"`; v9.x API same structure but version is stale in docs |
| `cargo-dist` tool name + version | `docs/02-scope.md` cites `cargo-dist` | Renamed to **`dist`**, latest **v0.31.0** (2026-02-23); config in `dist.toml` / `dist-workspace.toml` (NOT `Cargo.toml`) | STACK.md | Phase 3 (distribution) must run `dist init` for scaffold; don't copy old cargo-dist config syntax |

### Issues closed or resolved

| Issue | What docs said | Reality as of 2026-04-28 | Implication |
|---|---|---|---|
| `anthropics/claude-code#21022` (102MB JSONL freezes CC) | `docs/02-scope.md` cites as active evidence | **Closed as not-planned** — Anthropic declined to fix | holt's bounded-tail strategy is load-bearing, not defensive; reframe citation in docs |
| `seanmonstar/reqwest#1233` (blocking-in-tokio panic) | `docs/02-scope.md` cites as live rationale for avoiding tokio | **Closed** via PR #1263 | Replace issue citation with "`cargo tree -i tokio` discipline" — audit principle still applies |

### Triggers fired — move to v1.0 IN

Three `docs/02-scope.md §3` deferred-with-triggers entries have fired (CC shipped the underlying capability):

| Feature | Trigger that fired | Action |
|---|---|---|
| **Plan-mode color flip** | `permissionMode` confirmed in CC v2.1.119 stdin; already in `PROJECT.md` v1.0 list | Already in scope. Confirm at Phase 6. |
| **Effort/thinking pill** (`effort.level` from CC v2.1.119; `xhigh` added v2.1.111) | Field shipped | Move to v1.0 IN only if/when native rendering ships (Phase 5+); keep deferred otherwise |
| **Stuck-loop detector** (`PostToolUse.duration_ms` from CC v2.1.119, April 23, 2026) | Confirmed shipped | Move to v1.0 IN — Nak's wedged-state (`docs/04-pet.md §3` state #5) depends on this |

### 6 new features the docs missed

| # | Feature | Phase | Implication | Source |
|---|---|---|---|---|
| 1 | **Sub-agent cost rollup** — heartbeat-per-session architecture inherently solves CC #48040 (sub-agents spend silently) | v1.0 | Add half a paragraph to v1.0 pitch; no new code | FEATURES.md |
| 2 | **`workspace.git_worktree`** field (CC v2.1.98) — first-class field, more robust than cwd-parse | v0.1 | One-line heartbeat change: read field if present, fall back to cwd-parse | FEATURES.md |
| 3 | **`PreCompact` hook** (CC v2.1.105) — precise compact-start/end detection for Nak's groggy transition | v1.0 | Add to v1.0 hook subscription list alongside PreToolUse/PostToolUse/Stop/Notification/SessionStart | FEATURES.md |
| 4 | **Defensive stdin JSON parse** — CC v2.1.119 Windows regression broke statusLine execution; `effort.level` xhigh broke ccstatusline | v0.1 | Capture parse failures in breach log separately; never bubble errors to user; explicit acceptance criterion | FEATURES.md |
| 5 | **ccstatusline competitive pressure** — shipped Token Speed, Skills, Vim Mode widgets during doc-lock period | Anti-feature note | Reinforces wrap-don't-compete; native rendering trigger should reference ccstatusline's current widget floor | FEATURES.md |
| 6 | **`/statusline-setup` is now first-party** (Anthropic shipped April 22, 2026) | README/pitch | README must assume user already has a statusline; pitch is runtime hygiene + multi-session + Nak | FEATURES.md |

### Trigger-watch (not yet fired — monitor)

- **CC #52089** — "expose session token usage to hooks and statusline": if shipped before holt v1.0, the buffer-math correction layer becomes redundant. Architecture survives; degrade gracefully.

---

## 3. Hard Constraints

Implementation rules that MUST hold across the entire codebase.

**C1 — Always pipe all three stdio streams when spawning supervised processes** (STACK.md §2, PITFALLS.md H3)
Set `.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())` before `wrap(ProcessGroup::leader())`. On macOS, a child inheriting the parent's controlling TTY in a background process group receives `SIGTTIN` and stops — looks like a hang. Any future code path that inherits stdio (e.g., `holt doctor --interactive`) MUST NOT use `ProcessGroup::leader()`.

**C2 — `holt-render` MUST NOT depend on `holt-supervisor`** (ARCHITECTURE.md §2 crate DAG)
Render path (20ms budget) and supervision path (user's script runtime — unbounded) are separate crates. Enforced DAG: `holt-schemas ← holt-supervisor ← holt-cli`; separately `holt-schemas ← holt-orchestrator ← holt-render ← holt-cli`. Enforce in CI; any PR adding a `holt-render → holt-supervisor` dep edge breaks the latency contract.

**C3 — `settings.json` mutation: `fs2::FileExt::try_lock_exclusive()` + fsync-before-rename + PID-suffix tmp** (PITFALLS.md H1)
`holt install-hooks` must lock `settings.json` for the entire read-merge-write window. Write to `settings.json.holt-tmp.<pid>` (not `.bak` — that's vim's namespace). Call `fsync(2)` on the temp fd before `rename(2)` (ext4 `data=writeback` delayed-alloc is not otherwise atomic). Backup as `settings.json.holt.bak`. One-strike trust violation if this corrupts user config on first install.

**C4 — JSONC handling lives ONLY in `holt-cli`, never on the render path** (ARCHITECTURE.md §5.3)
`json_comments` and `jsonc-parser` are `holt-cli` dependencies only — never in `holt-schemas`, `holt-hooks`, `holt-orchestrator`, or `holt-render`. The render path parses known-schema heartbeat JSON (strict); install-hooks parses user-edited settings.json (JSONC-tolerant). Different parsers, different paths, no cross-contamination.

**C5 — Reader treats stale-or-corrupt heartbeat as missing, never panics** (PITFALLS.md H2, ARCHITECTURE.md §5.1)
`holt-orchestrator::read_heartbeat` catches all `serde_json::from_slice` errors and treats them as "session unreadable": exclude from attention queue, log once per session per process, continue rendering. Writer must `fsync(2)` on temp file before `rename(2)`. Never `unwrap()` on heartbeat deserialization anywhere on the render path.

**C6 — The render path never reads `breaches.log` or `timings.jsonl`** (PITFALLS.md H9)
These files are write-only outputs of the render path. Reading them on the render path creates a storm: measuring slowdowns causes slowdowns as the log grows. Any "session has N breaches" badge reads a summary file written by `holt doctor` post-hoc, never the raw log. Document in `CONTRIBUTING.md`.

---

## 4. Recommended Phase Split

Translation of ARCHITECTURE.md §3 build-order P0–P12 into 5 phases. Each phase is a standalone shippable.

### Phase 1 — Schema + Supervisor substrate (v0.1 foundation)

**Crates:** `holt-schemas`, `holt-supervisor`
**ARCHITECTURE.md steps:** P0, P1
**Delivers:** heartbeat type + atomic-rename helper + `process-wrap` integration + LKG cache + timeout/killpg + timings.jsonl + breach detector
**Pitfalls to address:** C1 (stdio piping), C5 (fsync-before-rename in writer), H3 (setpgid return-check + descendant-walk fallback)
**Research needed:** No — patterns are clear from STACK.md + PITFALLS.md

### Phase 2 — Hook write + install-hooks UX (v0.1 core)

**Crates:** `holt-hooks`, `holt-cli`
**ARCHITECTURE.md steps:** P2, P3
**Delivers:** minimal heartbeat write per CC event + `holt install-hooks` with JSONC-safe merge, `--dry-run`, `--print`, `.holt.bak`
**Pitfalls to address:** C3 (settings.json locking), C4 (JSONC in CLI only), H4 (XDG fallback chain), H5 (defensive serde with `#[serde(default)]`), H6 (mode fallback chain), H12 (concurrent install-hooks)
**Research needed:** YES — JSONC strategy spike needed before code starts (see Open Questions)

### Phase 3 — Distribution + v0.1 launch

**Crates/tools:** `dist` tool, brew tap, binstall
**ARCHITECTURE.md steps:** P4
**Delivers:** cargo-dist binaries for Linux x64, macOS x64+arm64, Windows x64; Homebrew tap; binstall support; README with asciinema/gif
**Pitfalls to address:** H7 (Gatekeeper — lead with brew in README; `dist` generates tap), H8 (writer_version field in heartbeat schema)
**Research needed:** No — run `dist init` for canonical scaffold; do not use STACK.md snippet verbatim (MEDIUM confidence)

### Phase 4 — Doctor profiler (v0.5)

**Crates:** `holt-supervisor` extension, `holt-cli`
**ARCHITECTURE.md steps:** P5, P6 (partial)
**Delivers:** `holt doctor` 20-fire profiler with ranked culprit table; `holt doctor --share` redacted bundle; `holt doctor --first-run` quarantine check
**Pitfalls to address:** H13 (refreshInterval misconfig warn), H7 (Gatekeeper detection)
**Research needed:** No — additive extension of v0.1 supervisor; no new crates

### Phase 5 — Rich heartbeat + orchestrator + Nak (v1.0 core)

**Crates:** `holt-hooks` extension, `holt-orchestrator`, `holt-render`
**ARCHITECTURE.md steps:** P6 (rich heartbeat), P7, P8, P9
**Delivers:** full heartbeat fields (current_tool, mode, context_pct_real, burn_rate_usd_per_min, PreCompact hook subscribe) + cross-session fanout reader + attention queue + Nak 12-state sprite + companion dots + rotating attention slot
**Pitfalls to address:** C2 (render MUST NOT depend on supervisor), H10 (pet writes off render path)
**Research needed:** YES — CC v2.1.119 stdin fixtures must be captured before rich-heartbeat code starts (see Open Questions)

### Phase 6 — Pet bond layer + v1.0 polish

**Crates:** `holt-schemas` (pet types), `holt-cli` (pet subcommands)
**ARCHITECTURE.md steps:** P10, P11, P12
**Delivers:** pet state schema v1 + naming + diary + friendship aggregation + `holt peers` TUI + plan-mode color flip + autocompact buffer-math correction
**Pitfalls to address:** H10 (eCryptfs pet writes), H11 (v1-reader permanent code path)
**Research needed:** YES — friendship merges-detection scope and PreCompact interaction (see Open Questions)

---

## 5. Open Questions for Phase-Start Research

**Phase 2 / P3 — JSONC strategy spike:**
Does `json_comments` (strip-then-parse) compose safely with `jsonc-parser` CST (in-place edit)? Neither ARCHITECTURE.md §5.3 nor STACK.md §4 has a tested integration snippet. Risk: `json_comments` strips a comment that `jsonc-parser` CST re-inserts at the wrong offset. Spike before Phase 2 code starts. What is the minimal `settings.json` fixture with inline comments that exercises the merge path end-to-end? (ARCHITECTURE.md §5.3, STACK.md §4)

**Phase 5 / P6 — CC v2.1.119 stdin shape fixtures:**
`PITFALLS.md H5` and `FEATURES.md §"New since 04-28" #4` both flag that `effort.level` xhigh broke ccstatusline and the v2.1.119 Windows regression stopped executing the statusLine command. Capture verbatim CC stdin JSONs for `PreToolUse`, `PostToolUse`, and `Stop` events in CC v2.1.119+ as `tests/fixtures/cc-stdin/v2.1.119.json` before rich-heartbeat code starts. Confirm field names and optionality. (FEATURES.md §triggers fired, PITFALLS.md H5)

**Phase 6 / P11 — Friendship merges-detection scope:**
`ARCHITECTURE.md §3` P11 needs a decision: does holt count merge events via the `PreCompact` hook (CC v2.1.105, confirmed shipped), or by inferring from context% drops in sequential heartbeats? `docs/05-schemas.md §3` pre-dates PreCompact. Research whether the friendship-accumulation model needs a schema bump or whether PreCompact is purely a hook event with no schema change. (FEATURES.md §"New since 04-28" #3, ARCHITECTURE.md P11)

---

## 6. Roadmap-Irrelevant Findings

Important context; skip when sequencing phases.

- **`jiff` vs `chrono`** (STACK.md): Both work for heartbeat ISO 8601. Pick one at first commit and document. No phase dependency.
- **eCryptfs latency** (PITFALLS.md H10): Real on ancient Ubuntu installs; pet writes are already off the render path by C6. No code change beyond `holt doctor --first-run` warning.
- **Session-count scaling ceiling** (ARCHITECTURE.md §5.2, §8): Soft cap at 16 pre-designed. Daemon optimization is a 1.x trigger gated on ≥3 issues reporting ≥10-session lag.
- **`atomic-write-file` crate** (STACK.md §3): Cleaner than hand-rolled tmp+rename. Adopt only if ≥2 corrupted-heartbeat reports post-launch.
- **CCometixLine Opus 4.6 crash** (FEATURES.md): Competitive context only; no action item for holt's roadmap.
- **Anthropic April 2026 postmortem** (PITFALLS.md §Part A): Addressed model behavior regressions, not statusLine plumbing; holt's wedge is unaffected.

---

## 7. Confidence Assessment

| Area | Confidence | Notes |
|---|---|---|
| Stack | HIGH | `process-wrap` v9.1.0 verified docs.rs + lib.rs; `dist` v0.31.0 via GitHub releases; all core crates confirmed. Only LOW item: exact `dist.toml` schema — use `dist init` |
| Features | HIGH | CC changelog v2.1.83–2.1.120 verified; trigger statuses confirmed; 6 new features sourced to specific issues/changelog entries |
| Architecture | HIGH | Crate DAG and build order derived from locked docs/; failure-mode topology verified against GitHub issues |
| Pitfalls | HIGH (Blockers), MEDIUM (platform-specific) | H1–H3 Blockers: GitHub issue evidence + official platform docs. H7/H10 platform pitfalls: multi-source verified, holt-specific behavior is inference |

**Overall:** HIGH — architecture is locked and research confirmed rather than overturned it. Unknowns are operational (dist.toml syntax, JSONC spike, stdin fixtures, friendship model) and appropriate for phase-start research, not architecture changes.

### Gaps to Address

- **`dist.toml` syntax** — run `dist init` at Phase 3 start; STACK.md snippet is MEDIUM confidence
- **JSONC compose safety** (`json_comments` + `jsonc-parser` CST) — spike at Phase 2 start
- **CC stdin fixtures** — capture live `v2.1.119.json` examples before Phase 5 code starts
- **Friendship model + PreCompact interaction** — research before Phase 6 P11 starts

---

## Sources

### Primary (HIGH confidence)

- `docs/01-findings.md` through `docs/05-schemas.md` — locked design; all architecture decisions trace here
- [process-wrap on docs.rs (v9.1.0)](https://docs.rs/process-wrap/9.1.0/process_wrap/)
- [cargo-dist GitHub releases](https://github.com/axodotdev/cargo-dist/releases) — v0.31.0, dist.toml
- [Claude Code statusLine docs](https://code.claude.com/docs/en/statusline)
- [Claude Code changelog](https://code.claude.com/docs/en/changelog) — v2.1.83–2.1.120 verified
- [Apple APFS Features](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/Features/Features.html) — rename atomicity
- [anthropics/claude-code#21022](https://github.com/anthropics/claude-code/issues/21022) — closed wontfix
- [anthropics/claude-code#52089](https://github.com/anthropics/claude-code/issues/52089) — trigger-watch

### Secondary (MEDIUM confidence)

- [openai/codex#8690](https://github.com/openai/codex/issues/8690) + [elixir-lang/elixir#15036](https://github.com/elixir-lang/elixir/issues/15036) — macOS setpgid + SIGTTIN cross-confirmation
- [LWN: fsync-before-rename](https://lwn.net/Articles/789600/) — ext4 delayed-alloc
- [MacRumors macOS 15.1 Gatekeeper thread](https://forums.macrumors.com/threads/macos-15-1-completely-removes-ability-to-launch-unsigned-applications.2441792/)
- [Tuist: signing macOS CLIs](https://tuist.dev/blog/2024/12/31/signing-macos-clis) — brew ad-hoc signature bypass

### Tertiary (LOW confidence)

- Exact `dist-workspace.toml` schema for v0.31.0 — partial; use `dist init`
- `jiff` vs `chrono` — project-style choice, both viable

---

*Research completed: 2026-04-28*
*Ready for roadmap: yes*
