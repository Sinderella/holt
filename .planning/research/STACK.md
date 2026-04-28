# Stack Research — holt

**Domain:** Rust statusLine + multi-session orchestrator + ASCII pet for Claude Code
**Researched:** 2026-04-28
**Confidence:** HIGH for verified crates · MEDIUM for `process-wrap` macOS quirks · LOW for cargo-dist syntax (full schema not surfaced in fetched results)

## Relationship to docs/

This file **augments** `docs/02-scope.md` (locked OUT/IN tables and stack decisions) and `docs/01-findings.md` (wedge thesis). It does **not** redo the wedge thesis or competitive analysis. Specifically:

| Locked in design docs | Confirmed here | New here |
|---|---|---|
| Rust, sync stdlib + threads, no async at v0.1 | Still the right call (statusLine doc unchanged: no timeout, no timing log, cancel-mid-flight) | MSRV recommendation; `tokio` audit checklist |
| `process-wrap` for cross-platform process-group kill | Crate alive, **v9.1.0** (not v6.0.0 as cited in `02-scope.md`); exposes `ProcessGroup::leader()` and `JobObject` per-platform | Exact wrap+kill snippet; **macOS sandbox + inherited-TTY SIGTTIN gotcha**; recommendation to gate `setpgid` on whether stdio is piped |
| `cargo-dist` for prebuilt binaries + Homebrew tap + binstall | Latest is **v0.31.0** (2026-02-23), `dist.toml` / `dist-workspace.toml` config | Concrete target list snippet for holt's matrix |
| Stock `serde` + `toml`, NOT `simd-json` / `figment` | Still correct — sub-millisecond on small inputs | `serde_json` `preserve_order` feature for `settings.json` round-trip |
| Atomic write via tmp + POSIX rename | Correct on same-volume writes | **EXDEV cross-volume failure mode**, ext4 `auto_da_alloc` heuristic, fsync-the-directory durability rule, `atomic-write-file` crate as audited alternative |
| Heartbeat path: `$XDG_RUNTIME_DIR/holt/sessions/` (Linux) / `$TMPDIR/holt-$UID/sessions/` (macOS), never `~/.claude/` | Correct | — |
| Audit transitive `tokio` pull-through | `reqwest #1233` is **closed** (PR #1263 merged); audit still applies in principle | — |

## Recommended Stack

### Core Technologies

| Technology | Min Version | Purpose | Why |
|------------|-------------|---------|-----|
| Rust toolchain | **1.85+** (Edition 2024) | Language | Edition 2024 stabilized in 1.85 (Jan 2026); MSRV-aware resolver in 1.84+. Pin via `rust-toolchain.toml` for reproducible CI. `process-wrap` 9.1.0 declares MSRV 1.87. **Recommend `rust-version = "1.87"` in `Cargo.toml`** to match the strictest dep. |
| `serde` + `serde_json` | 1.x | JSON I/O — heartbeat, pet state, `settings.json` read | Stock perf is sub-ms on small inputs (heartbeat <1KB). For `settings.json` round-trip enable `features = ["preserve_order"]` to keep user's key order. |
| `toml` | 0.8+ | Read `~/.config/holt/config.toml` (v1.0+) | Stock crate is fast enough; **NOT** `figment` (over-engineered for our scope). |
| `process-wrap` | **9.1.0+** | Cross-platform process group / JobObject supervision of the wrapped statusLine command | Successor to `command-group`; composable wrappers, std + tokio frontends. v6.0.0 cited in `docs/02-scope.md` is stale — bump to 9.x. |
| `clap` (derive) | 4.5+ (`clap_derive` 4.6.0) | CLI: `holt run`, `holt install-hooks`, `holt doctor`, `holt pet …` | De facto standard. Stable derive API since 4.0. |
| `anyhow` | 1.x | Application-layer error type for binary | Right tool for self-contained CLI. Pair `anyhow` (binary) + `thiserror` (any internal lib boundary) — current 2026 best practice. |
| `thiserror` | 1.x | Internal error enums where the binary needs to match variants (e.g., `BreachReason`) | Use only at boundaries that benefit from matchable errors. |
| `terminal_size` | 0.3+ | Detect column width for the statusLine render budget | Lighter than pulling all of `crossterm` for one function. v1.0 needs this once we add Nak's variable-companion-dot rendering. |
| `supports-color` | 3.x | Decide colored vs no-color output, `NO_COLOR` env handling | Pairs cleanly with `owo-colors`. |
| `owo-colors` | 4.2.3 | ANSI color output | Zero-allocation, drop-in replacement for `colored`, supports `NO_COLOR` / `FORCE_COLOR` natively. Strictly preferred over `colored` for our cold-start budget. |
| `jsonc-parser` | 0.26+ (with `cst` feature) | `holt install-hooks` JSON-preserving merge into `~/.claude/settings.json` | CC's `settings.json` is JSON, but power users keep comments via JSONC editors. CST API preserves comments + key order across edits — `serde_json` round-trip alone destroys both. |

### Distribution

| Tool | Version | Purpose | Notes |
|------|---------|---------|-------|
| `cargo-dist` (now `dist`) | **0.31.0** (2026-02-23) | Prebuilt binaries, Homebrew tap, GitHub release artifacts | Config moved to `dist-workspace.toml` / `dist.toml` (separate from `Cargo.toml`). `cargo-binstall` works automatically once `repository` is set in `Cargo.toml`. |

### Explicitly NOT Used (re-confirming `02-scope.md`)

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `tokio` (any version) at v0.1 | No render-path async justification; pulls in 100KB+ runtime; silent transitive pulls are why `reqwest #1233` exists | `std::thread`, `std::process::Command`, `std::sync::mpsc` for the background-recompute pattern |
| `simd-json` | Pessimization for our <1KB inputs; SIMD setup cost dominates | Stock `serde_json` |
| `figment` | Overkill — we don't need layered config sources at v0.1 | Stock `toml` + env vars + CLI flags |
| `crossterm` (full crate) | Pulls more than we need (input handling, alt-screen, cursor); cold-start matters | `terminal_size` (just `ioctl(TIOCGWINSZ)`) until `holt peers` TUI is on the table |
| `chrono` (default features) | TZ data not shipped; `chrono-tz` adds binary size | `jiff` (modern, RFC 3339 + IANA TZ built-in) **OR** keep `chrono` with explicit minimal features. Heartbeat schemas use ISO 8601 strings — either works, jiff is the future-proof bet. **Confidence: MEDIUM.** No urgency to swap; pick one and stick. |
| `reqwest` / any HTTP client | We have no network calls. If `holt doctor` ever shells out to fetch CC's GitHub status, do it via the user's `curl` — not a Rust HTTP client. | n/a |

## Implementation Patterns

### 1. `process-wrap` — wrap user's statusLine with timeout + clean kill

The locked decision in `02-scope.md` is "setpgid + killpg via `process-wrap`." The exact pattern at v9.1.0:

```rust
use std::process::Stdio;
use std::time::Duration;
use process_wrap::std::{CommandWrap, ProcessGroup};

pub fn run_supervised(cmd_line: &str, stdin_bytes: &[u8], timeout: Duration)
    -> Result<Output, BreachReason>
{
    let mut command = CommandWrap::with_new("sh", |c| {
        c.arg("-c").arg(cmd_line)
         .stdin(Stdio::piped())
         .stdout(Stdio::piped())
         .stderr(Stdio::piped());
    });

    // Unix: new process group → killpg(SIGKILL) on timeout reaches descendants.
    // Windows: equivalent via JobObject (see #[cfg(windows)] block below).
    #[cfg(unix)]    command.wrap(ProcessGroup::leader());
    #[cfg(windows)] command.wrap(process_wrap::std::JobObject);

    let mut child = command.spawn()?;
    // ... write stdin, read stdout with timeout via std::sync::mpsc + thread,
    //     on timeout call child.kill() — process-wrap routes to killpg/JobObject.
}
```

### 2. macOS gotcha — DO NOT setpgid when stdio is inherited

**Found in research (this was an open gap in the design docs):** OpenAI Codex (`openai/codex#8690`) and Elixir (`elixir-lang/elixir#15036`) both hit a hang on macOS where:

1. Sandbox or wrapping process calls `setpgid(0,0)` on the child.
2. Child inherits the parent's controlling TTY.
3. Child reads from TTY → kernel sends `SIGTTIN` to background process group → child stops, looks like a hang.

**Mitigation for holt:** Our shim **always pipes stdin/stdout/stderr** (we read child stdout to capture for breach log; statusLine doesn't print to a TTY directly — CC reads our process's stdout). Because we redirect all three before `wrap(ProcessGroup::leader())`, the SIGTTIN class doesn't apply to us. **Document this constraint** in the breach-log capture path: if a future feature ever inherits stdio (e.g., `holt doctor --interactive`), it must NOT use `ProcessGroup::leader()`.

### 3. Atomic heartbeat write — the EXDEV trap

The locked design says "atomic write via tmp + POSIX rename." Two real-world caveats the design docs don't yet capture:

```rust
// NOT this — EXDEV if /tmp and $XDG_RUNTIME_DIR are different mounts:
fs::rename("/tmp/holt-heartbeat.tmp", "/run/user/1000/holt/sessions/<sid>.json")?;

// DO this — same-directory tmp file:
let target = "/run/user/1000/holt/sessions/abc.json";
let tmp    = "/run/user/1000/holt/sessions/.abc.json.tmp";
{
    let mut f = File::create(tmp)?;
    f.write_all(payload)?;
    f.sync_all()?;          // fsync the data
}
fs::rename(tmp, target)?;   // POSIX-atomic only WITHIN one filesystem
// For full crash-durability also fsync the directory fd. Skip at v0.1
// (heartbeat is ephemeral; a crash mid-rename is recoverable next fire).
```

**Filesystem-specific notes** picked up in research:
- **ext4** has an `auto_da_alloc` heuristic that flushes dirty pages of the source file before rename — fine, but extends the latency. Acceptable for our 5-events-per-turn cadence.
- **APFS firmlinks** (macOS): user's `~/.claude/` could be on a non-default APFS volume. Heartbeats live in `$TMPDIR/holt-$UID/` so this doesn't bite us — but **`holt install-hooks` writes to `~/.claude/settings.json`**, which means **its** atomic-write tmp file MUST also live alongside `settings.json` (same directory) to avoid EXDEV.
- **iCloud / OneDrive on `~/.claude/`**: the original reason we keep heartbeats out of `~/.claude/`. Settings.json edits remain a write-and-pray (user opted into the cloud sync); `--dry-run` and `--print` flags exist for the nervous case.

**Recommended posture:**
- v0.1 — hand-rolled tmp+rename in same dir, no fsync. ~10 lines.
- v0.5+ if we hit reports of corrupted heartbeats — switch to `atomic-write-file` crate (uses `linkat`/`renameat` with directory FDs; handles APFS firmlinks correctly).

### 4. JSON-preserving merge for `holt install-hooks`

`docs/02-scope.md` notes "JSON-aware merge" but doesn't pick a library. The right pick is **`jsonc-parser` with the `cst` feature**:

```toml
# Cargo.toml
jsonc-parser = { version = "0.26", features = ["cst", "serde"] }
```

Pattern: parse user's `settings.json` as CST, find/create the `hooks` object, insert/update our keys, serialize with comments + key-order intact.

```rust
use jsonc_parser::cst::{CstRootNode, CstObject};
use jsonc_parser::ParseOptions;

let original = fs::read_to_string(&settings_path)?;
let root = CstRootNode::parse(&original, &ParseOptions::default())?;
let obj  = root.value().and_then(|v| v.as_object()).ok_or(...)?;

// Get or create "hooks" sub-object, then merge our entries...
let hooks = obj.object_value_or_create("hooks");
hooks.set("PreToolUse",  json!([{ "matcher": "*", "hooks": [{ "type": "command", "command": "holt heartbeat pre" }] }]));
hooks.set("PostToolUse", /* ... */);
// ...

fs::write(&settings_path, root.to_string())?;  // comments + ordering preserved
```

**Why not `serde_json` round-trip:**
- Drops comments (`//` and `/* */`) silently. Many users have JSONC-mode editors writing comments into `settings.json`.
- Even with `preserve_order` feature, formatting (indentation, blank lines) is normalized.
- `holt install-hooks --print` is supposed to show a clean diff — re-formatted JSON makes the diff noisy.

**Backup + dry-run safeguards:**
1. Always copy `settings.json` → `settings.json.bak` before write.
2. `--dry-run` writes to stdout instead of disk.
3. `--print` emits the merged JSON without writing — the "I'll paste it myself" escape.
4. Validate the post-merge result still parses with `serde_json::from_str::<Value>` before writing — refuse to corrupt user config.

### 5. cargo-dist matrix for holt v0.1

Putting `02-scope.md` constraints into v0.31.0 syntax. **Confidence: MEDIUM** — published examples in fetched results are partial; treat this as a starting point, run `dist init` to generate the canonical scaffold.

```toml
# dist-workspace.toml (or [workspace.metadata.dist] in workspace Cargo.toml)
[dist]
cargo-dist-version = "0.31.0"
ci = "github"
installers = ["shell", "powershell", "homebrew"]
tap = "<github-user>/homebrew-holt"
publish-jobs = ["homebrew"]
targets = [
  "x86_64-unknown-linux-gnu",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc",
]
# Linux arm64 deferred to first .1 release per docs/02-scope.md.
```

`cargo-binstall` Just Works once `repository = "https://github.com/<user>/holt"` is set in `Cargo.toml` — no extra config needed.

## Cold-start budget (sub-20ms target)

The 20ms target on macOS arm64 / Linux x64 cold-start drives library choices. Actuals to expect:

| Step | Budget | Notes |
|------|--------|-------|
| Process spawn + Rust runtime init | ~2-4ms | irreducible; arm64 macOS faster than x64 Linux due to dyld |
| `clap` parsing | <1ms | derive-mode is zero-cost-at-parse |
| `~/.claude/settings.json` read + parse | ~1-2ms | small file, `serde_json` |
| Heartbeat reads (≤8 sessions × stat + read) | ~3-5ms | per `docs/03-orchestrator.md` |
| Render | <1ms | string formatting, no I/O |
| **Total at v1.0** | ~10-12ms | leaves headroom for Windows Defender (~30ms additional) |

**Things that would blow the budget:**
- Pulling `tokio` for any reason on the render path → adds ~3-5ms init
- Using `crossterm` instead of `terminal_size` → adds ~1-2ms
- Reading the CC transcript JSONL on every fire → unbounded; do this in the heartbeat hook only, not the render

## Version Compatibility

| Constraint | Notes |
|------------|-------|
| `process-wrap 9.1.0` ↔ `rustc 1.87+` | Pinning MSRV at 1.87 in `Cargo.toml` matches process-wrap exactly. |
| `process-wrap` features | Only enable `std` frontend (no `tokio1`); `process-group` and `process-session` and `job-object` and `creation-flags` are default — fine. |
| `serde_json preserve_order` | Adds `indexmap` dep transitively; ~5KB binary cost. Worth it for `settings.json` round-trip. |
| `jsonc-parser` + `serde_json` | Compatible — `jsonc-parser` can parse-to-`serde_json::Value` if needed. |

## URL Freshness Spot-Checks (4 of cited)

| URL | Status as of 2026-04-28 | Drift from `docs/01-findings.md` / `02-scope.md` |
|-----|-------------------------|---------------------------------------------------|
| [`anthropics/claude-code#18943`](https://github.com/anthropics/claude-code/issues/18943) (input lag / slow typing echo) | **Open**; opened 2026-01-18; no fix linked | None — pain still real, wedge intact |
| [`anthropics/claude-code#21022`](https://github.com/anthropics/claude-code/issues/21022) (102MB JSONL freezes CC) | **Closed as not planned** | `02-scope.md` cites it as evidence; recommend updating wording from "issue active" to "issue closed wontfix — pain persists per the closure rationale, holt's heartbeat pattern sidesteps it" |
| [`Haleclipse/CCometixLine#118`](https://github.com/Haleclipse/CCometixLine/issues/118) (Opus 4.6 crash) | **Open**; opened 2026-04-18 — **newer than `01-findings.md` claimed**; no PR | Opening date drift; project-stalled thesis still holds |
| [`seanmonstar/reqwest#1233`](https://github.com/seanmonstar/reqwest/issues/1233) (blocking-in-tokio panic) | **Closed** via PR #1263 | `02-scope.md` cites this as live evidence for transitive-tokio caution. The audit principle still applies (any dep that secretly pulls tokio is a problem on a sync codebase) — but the specific issue is resolved. **Recommend rewording**: cite `cargo tree -i tokio` discipline rather than the closed reqwest issue. |
| [`rust-lang/rust#115241`](https://github.com/rust-lang/rust/issues/115241) (Child::kill descendants) | **Open**; opened 2023-08-26 | None — confirms `process-wrap` is still the right answer at the std level |

## Sources

**HIGH confidence (Context7 / docs.rs / official):**
- [process-wrap on docs.rs (v9.1.0)](https://docs.rs/process-wrap/9.1.0/process_wrap/) — wrap pattern, feature flags
- [process-wrap on lib.rs](https://lib.rs/crates/process-wrap) — version, MSRV (1.87), release date 2026-03-08
- [cargo-dist GitHub releases](https://github.com/axodotdev/cargo-dist/releases) — v0.31.0 (2026-02-23), platform list
- [Claude Code statusLine docs](https://code.claude.com/docs/en/statusline) — execution model still cancel-on-new-event, no built-in timeout
- [serde_json `preserve_order` discussion](https://github.com/serde-rs/json/issues/54)
- [jsonc-parser CST](https://docs.rs/jsonc-parser/latest/jsonc_parser/) — comment-preserving manipulation, CST feature

**MEDIUM confidence (web, multi-source verified):**
- [openai/codex#8690](https://github.com/openai/codex/issues/8690) — macOS sandbox + setpgid + inherited-TTY SIGTTIN hang (cross-confirmed by `elixir-lang/elixir#15036`)
- [Rust forum atomic file write thread](https://users.rust-lang.org/t/how-to-write-replace-files-atomically/42821) — EXDEV, ext4 auto_da_alloc, fsync-the-directory pattern
- [atomic-write-file crate](https://crates.io/crates/atomic-write-file) — production-grade alternative if hand-rolled rename hits real-world bugs
- [Rust 1.84 MSRV-aware resolver announcement](https://blog.rust-lang.org/2025/01/09/Rust-1.84.0/)

**LOW confidence (single-source or fetched-empty):**
- Exact `dist-workspace.toml` schema for v0.31.0 — fetched results were partial. Run `dist init` to generate canonical config rather than copying my snippet wholesale.
- [`jiff` vs `chrono`](https://docs.rs/jiff/latest/jiff/) — both viable for heartbeat ISO 8601; recommendation to pick one is a project-style choice, not a hard requirement.

## Open Gaps for Phase Planners

1. **`process-wrap` 6 → 9 bump** — `docs/02-scope.md` cites v6.0.0; the v0.1 phase plan should call out the version bump explicitly so contributors don't copy stale syntax.
2. **macOS `setpgid` discipline** — every place we spawn must redirect all three stdio streams. Consider adding a `cargo clippy` lint or a wrapper helper to enforce.
3. **`atomic-write-file` adoption trigger** — define when we promote from hand-rolled rename to the crate (proposed: ≥2 reports of corrupted heartbeats post-launch).
4. **`jsonc-parser` round-trip safety net** — `holt install-hooks` should always validate the post-edit text re-parses cleanly with both `jsonc_parser` AND `serde_json`, before overwriting `settings.json`. Catches CST-feature bugs.
5. **`jiff` vs `chrono` decision** — defer to phase planner; both are <50KB binary cost; pick at first commit and document.
