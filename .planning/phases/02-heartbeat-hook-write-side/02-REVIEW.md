---
phase: 2
phase_name: Heartbeat hook (write side)
status: findings_present
depth: standard
files_reviewed: 21
findings:
  critical: 2
  warning: 9
  info: 7
  total: 18
reviewed: 2026-04-28
files_reviewed_list:
  - crates/holt-hooks/Cargo.toml
  - crates/holt-hooks/src/lib.rs
  - crates/holt-hooks/src/event.rs
  - crates/holt-hooks/src/stdin.rs
  - crates/holt-hooks/src/path.rs
  - crates/holt-hooks/src/assemble.rs
  - crates/holt-hooks/src/handle.rs
  - crates/holt-hooks/tests/assemble_field_policy.rs
  - crates/holt-hooks/tests/handle_event_smoke.rs
  - crates/holt-hooks/tests/sigkill_atomicity.rs
  - crates/holt-hooks/tests/sigkill_test_driver.rs
  - crates/holt-cli/Cargo.toml
  - crates/holt-cli/src/cli.rs
  - crates/holt-cli/src/main.rs
  - crates/holt-cli/src/hook.rs
  - crates/holt-cli/src/self_bench.rs
  - crates/holt-cli/tests/hook_subcommand_smoke.rs
  - crates/holt-cli/tests/hook_self_bench_smoke.rs
  - crates/holt-schemas/src/heartbeat.rs
  - crates/holt-supervisor/src/options.rs
  - .github/workflows/ci.yml
---

# Phase 2: Code Review Report — Heartbeat hook (write side)

**Reviewed:** 2026-04-28
**Depth:** standard
**Files Reviewed:** 21
**Status:** findings_present

## Summary

Phase 2 ships a clean, defensively-postured hook crate. The hard constraints (C1, C2, C5, C6) all hold by inspection: no second `wrap(ProcessGroup::leader())`, no `holt-render -> holt-supervisor` edge introduced, no read calls against `breaches.log` or `timings.jsonl` from `handle.rs` or `holt-cli/src/hook.rs`, and no forbidden crates appear in any new `Cargo.toml`. The defensive-parse posture (`#[serde(default)]` on every `HookStdin` field, no `deny_unknown_fields`) is correct per D-04.

That said, the "**Never panics**" promise on `holt-hooks/src/lib.rs` is broken by a single `unreachable!()` macro on the success path of `handle_event`, and the path-resolver's "first writable wins" semantics are subtly wrong: `create_dir_all` succeeding on an existing-but-not-writable directory commits the resolver to a dead tier with no fallback retry. Both are the kind of latent bugs the verifier doesn't surface.

The remaining findings are quality issues — TOCTOU on env-var manipulation in tests, stale env after the bench harness, redundant `chmod` after `atomic_write` already enforced 0o600, and a small handful of edge cases around empty-cwd, empty-stdin, and stuck-NFS mounts.

## Critical Issues

### CR-01: `unreachable!()` on render path violates "never panics" contract

**File:** `crates/holt-hooks/src/handle.rs:124`
**Constraint impact:** Violates the module-level invariant on `crates/holt-hooks/src/lib.rs:10–11` ("**Never panics. Never bubbles errors to the caller.**") and the C5 spirit (render path must not panic). `unreachable!()` is a `panic!()` primitive — it WILL panic if reached.
**Description:**

```rust
let reason = match resolved.tier {
    ResolvedTier::TmpDir => FallbackReason::XdgUnavailable,
    ResolvedTier::Cache => FallbackReason::XdgAndTmpUnavailable,
    ResolvedTier::XdgRuntimeDir => unreachable!("is_fallback() filtered this"),
};
```

The current control flow does guarantee `XdgRuntimeDir` is filtered by `is_fallback()` two lines up. But:

1. The guarantee is non-local — it relies on `ResolvedTier::is_fallback` always returning `false` only for `XdgRuntimeDir`. A future maintainer adding a `ResolvedTier::Memfd` (Linux memfd_create) variant must remember to update both `is_fallback` AND this match, or the render path panics.
2. `unreachable!()` is a hostile primitive in a "best-effort, exits 0 unconditionally" pipeline. The whole point of the dispatcher exiting 0 is that a panic NEVER reaches CC. A panic here aborts the process AFTER the heartbeat write succeeded, so the caller (`holt-cli/src/hook.rs::run`) never returns 0, and CC sees a non-zero exit code. That's a CC-visible regression for a cosmetic post-write step (the stderr warning).
3. `#![forbid(unsafe_code)]` is paired with "no panics on render path" in CONTRIBUTING.md. The relaxation to `#![deny(unsafe_code)]` was already taken in `holt-cli`; the equivalent panic-tolerance relaxation has NOT been justified for `holt-hooks`.

**Fix:** Replace the `unreachable!()` arm with a defensive fallthrough so the worst case is a slightly-less-precise warning, not a panic.

```rust
let reason = match resolved.tier {
    ResolvedTier::TmpDir => FallbackReason::XdgUnavailable,
    ResolvedTier::Cache => FallbackReason::XdgAndTmpUnavailable,
    // Defensive: if a future tier is added and forgot to update is_fallback(),
    // we still don't panic — degrade to the "both upstream tiers unavailable"
    // reason because we can't say more without lying.
    ResolvedTier::XdgRuntimeDir => FallbackReason::XdgUnavailable,
};
```

Or, better, restructure so the reason is computed alongside the tier resolution in `path.rs` and `is_fallback()` returns `Option<FallbackReason>`.

---

### CR-02: Path resolver commits to a dead tier when `create_dir_all` succeeds on an unwritable directory

**File:** `crates/holt-hooks/src/path.rs:79–106`
**Constraint impact:** Breaks the documented "first writable wins" semantics on lines 14–18 of the same file, and the success criterion #4 ("the hook falls back through the chain"). This is a correctness bug, not a style issue.
**Description:**

`std::fs::create_dir_all(path)` returns `Ok(())` if the path **already exists as a directory**, regardless of whether the current process has write permission to it. The resolver treats `Ok` as "tier wins" and never retries.

Reproducer scenario (entirely plausible):

1. User runs `holt` once under `sudo` for testing → `~/.cache/holt/sessions/` is created with root ownership and 0700 perms.
2. User reverts to normal UID; `XDG_RUNTIME_DIR` is also unset (typical on stock macOS without systemd-logind).
3. Tier 1 returns `None` (XDG_RUNTIME_DIR unset).
4. Tier 2 succeeds — `$TMPDIR/holt-<UID>/sessions/` is fresh.
5. **But suppose `$TMPDIR` is also unset** (rare but happens in `env -i` shells, cron, some container runtimes). Tier 2 returns `None`.
6. Tier 3: `create_dir_all(/home/user/.cache/holt/sessions)` → `Ok(())` (directory exists), but the dir is owned by root with 0700. We commit to tier 3 with `ResolvedTier::Cache`, then `atomic_write` fails with EACCES, then we breach as `Unwritable` — even though there are no other tiers, so this is the "correct" outcome here.

The more common failure path that DOES cost us: imagine `$XDG_RUNTIME_DIR=/run/user/1000` exists with the wrong owner (e.g., laptop suspend/resume across user-switch corrupted the runtime dir). `create_dir_all` succeeds (dir exists), we commit to tier 1, `atomic_write` fails, `Unwritable` outcome — but tier 2 (`$TMPDIR/holt-<UID>/sessions/`) would have worked perfectly. The user gets `Unwritable` instead of `FellBack { tier: TmpDir }`.

**Fix:** Probe writability with a same-dir tmp file. The lightest probe is to attempt the actual atomic_write at each tier, but a cheaper probe is to try `OpenOptions::new().create_new(true).write(true)` on a probe-named file inside the candidate parent.

```rust
fn dir_is_writable(parent: &Path) -> bool {
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    let probe = parent.join(format!(".holt-probe.{}", std::process::id()));
    let writable = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .is_ok();
    let _ = std::fs::remove_file(&probe); // best-effort cleanup
    writable
}

pub fn resolve_writer_path(stdin: &HookStdin) -> Option<ResolvedPath> {
    let sid = session_id_or_hash(stdin);
    let filename = format!("{sid}.json");

    if let Some(parent) = tier_xdg_runtime_dir() {
        if dir_is_writable(&parent) {
            return Some(ResolvedPath { path: parent.join(&filename), tier: ResolvedTier::XdgRuntimeDir });
        }
    }
    // ... same pattern for tier 2 and tier 3
}
```

The probe adds one syscall pair (open + unlink) per tier per fire — well under the 20ms budget. Note this introduces a deliberate TOCTOU window between the probe and the real `atomic_write`, but that's strictly an improvement: today's code has the same window AND fails to retry.

## Warnings

### WR-01: `_outcome` discard in CLI dispatcher loses exhaustiveness checking

**File:** `crates/holt-cli/src/hook.rs:38`
**Description:** `let _outcome = handle_event(event, &stdin_bytes, &env);` deliberately discards the variant. If `holt-hooks` ever adds a 5th `HookOutcome` variant (e.g., `RateLimited`, `SchemaUnknown`), the compiler won't flag this dispatcher. The contract says "exit 0 unconditionally", but the contract may evolve. Worse, since `HookOutcome` lacks `#[non_exhaustive]`, a downstream consumer outside this crate could match exhaustively today and silently break tomorrow.

**Fix:** Either (a) make the match explicit so new variants force a build break and a conscious decision:

```rust
match handle_event(event, &stdin_bytes, &env) {
    HookOutcome::Wrote { .. }
    | HookOutcome::FellBack { .. }
    | HookOutcome::ParseFailed
    | HookOutcome::Unwritable => {}
}
```

Or (b) add `#[non_exhaustive]` to `HookOutcome` in `holt-hooks/src/handle.rs:33` and do the explicit match here.

---

### WR-02: Empty-stdin path emits a `parse_fail` breach instead of being treated as a benign empty case

**File:** `crates/holt-hooks/src/stdin.rs:57–62` and `crates/holt-cli/src/hook.rs:24–28`
**Description:** `holt-cli/src/hook.rs::run` collapses `StdinParseOutcome::Empty` to `Vec::new()`, then passes that into `handle_event`, which calls `parse(&[])`, which returns `None` per `stdin.rs:58` (`if bytes.is_empty() { return None; }`), which routes to `HookOutcome::ParseFailed` and emits a breach record as `BreachKind::ParseFail`.

In Phase 1 (`crates/holt-cli/src/stdin.rs`), `StdinParseOutcome::Empty` is documented as "Stdin was empty (or unreadable — treated equivalently per H5 defensive posture)" — i.e., empty is a NORMAL condition, not a parse failure. But Phase 2 promotes empty stdin to a logged breach. Every developer who runs `echo | holt hook PreToolUse` for ad-hoc testing now writes a `parse_fail` line to `breaches.log`.

**Fix:** Distinguish at the dispatcher level — empty stdin should result in NO breach record, just a no-op exit. Either short-circuit before `handle_event` is called, or add a `HookOutcome::EmptyStdin` variant:

```rust
pub fn run(event: HookEvent) -> i32 {
    let stdin_bytes = match slurp_and_parse() {
        StdinParseOutcome::Ok { raw, .. } => raw,
        StdinParseOutcome::ParseFail { raw, .. } => raw,
        StdinParseOutcome::Empty => return 0, // no work, no breach
    };
    // ... rest unchanged
}
```

---

### WR-03: Redundant `chmod 0o600` after `atomic_write` adds a render-path syscall pair for no real benefit

**File:** `crates/holt-hooks/src/handle.rs:103–111`
**Description:** `holt_schemas::atomic_write` already opens the tmp file with `OpenOptionsExt::mode(0o600)` (see `crates/holt-schemas/src/writer.rs:35–39`). The `mode(0o600)` argument bypasses the umask only for the bits passed; since 0o600 has no group/other bits, the umask cannot widen it. The renamed-to file inherits 0o600 from the tmp inode. The `set_permissions(... 0o600)` after rename is therefore strictly redundant under normal conditions.

The cost: one `metadata()` (stat) + one `set_permissions()` (chmod) syscall on the render path on EVERY successful write — i.e., on every PreToolUse fire (very frequent). Two syscalls is sub-microsecond on warm cache, but it adds up. The justification in the comment ("defence-in-depth and matches the success-criterion language") is fair — but the criterion language could be satisfied by a doc comment on `atomic_write` instead of a runtime chmod.

There's also a subtle correctness wrinkle: between the `metadata()` read and `set_permissions()` write, if some external process replaces the file (only possible if our atomic_write contract is violated, which we control), permissions reset. Negligible threat model.

**Fix:** Either remove the chmod entirely and document the inheritance:

```rust
// 0o600 is set by atomic_write on Unix via OpenOptionsExt::mode(0o600);
// the rename(2) inherits it. No post-rename chmod needed.
```

Or, if defence-in-depth is required by policy, gate it behind a `#[cfg(test)]` flag and assert in tests instead of paying for it on the render path.

---

### WR-04: Bench harness leaves `XDG_RUNTIME_DIR` set to a deleted tempdir on exit

**File:** `crates/holt-cli/src/self_bench.rs:162–183, 197–230`
**Description:** `run_self_bench_hook` opens a tempdir, sets `XDG_RUNTIME_DIR` to it, runs the bench, returns. The `xdg_dir` `TempDir` value is implicitly dropped on function return, deleting the directory. But the env var `XDG_RUNTIME_DIR` is NOT restored — it now points at a deleted path. Subsequent code in `main()` that reads `XDG_RUNTIME_DIR` (none currently, but `holt --self-bench-hook` mode currently exits before reaching any) would see a stale path.

The process exits within microseconds of the bench returning, so there is no in-process consumer today. But:
1. If anyone later adds a "run bench then continue" mode, they hit the stale env.
2. The pattern of "set unsafe global env, drop tempdir, leave env stale" is a footgun worth flagging.

**Fix:** Restore the env var (or remove it) before returning, mirroring the test pattern in `handle_event_smoke.rs`. Better: capture the prior value and restore.

```rust
let prev_xdg = std::env::var_os("XDG_RUNTIME_DIR");
// SAFETY: ...
unsafe {
    std::env::set_var("XDG_RUNTIME_DIR", xdg_dir.path());
    std::env::remove_var("TMPDIR");
}

// ... bench loop ...

unsafe {
    match prev_xdg {
        Some(v) => std::env::set_var("XDG_RUNTIME_DIR", v),
        None => std::env::remove_var("XDG_RUNTIME_DIR"),
    }
}
```

---

### WR-05: `cwd_label` for empty `cwd` returns empty string, silently violating must_have-5

**File:** `crates/holt-hooks/src/assemble.rs:90–102` and `crates/holt-hooks/tests/assemble_field_policy.rs:35`
**Description:** `derive_cwd_label("", &None)` returns `""`. The integration test `cwd_label_uses_workspace_git_worktree_when_present` asserts `assert!(!hb.cwd_label.is_empty(), "must_have-5: cwd_label non-empty")`, but it's only tested on fixtures with non-empty `cwd`. If CC ever sends a stdin with `cwd: ""` (e.g., in a Notification event when no cwd is meaningful), the must_have-5 invariant silently breaks at runtime — and there's no test fixture exercising this.

**Fix:** Add a final fallback in `derive_cwd_label` so the return is never empty:

```rust
pub fn derive_cwd_label(git_worktree: &Option<String>, cwd: &str) -> String {
    if let Some(label) = git_worktree {
        if !label.is_empty() { return label.clone(); }
    }
    if let Some(base) = Path::new(cwd).file_name().and_then(|s| s.to_str()) {
        if !base.is_empty() { return base.to_string(); }
    }
    if !cwd.is_empty() {
        return cwd.to_string();
    }
    "<unknown>".to_string() // must_have-5 invariant: never empty
}
```

And add a unit test fixture with empty `cwd` and absent `workspace.git_worktree`.

---

### WR-06: Test `must_have_4_unwritable_returns_unwritable_outcome` accepts `FellBack` as a substitute for `Unwritable`

**File:** `crates/holt-hooks/tests/handle_event_smoke.rs:138–170`
**Description:** The test sets `HOME` to a regular file (a `NamedTempFile`), aiming to make tier 3 unwritable. The match arm accepts EITHER `Unwritable` OR `FellBack` because of a hypothetical `WR-01 last resort uses temp_dir which is ALWAYS writable` — but reading `default_cache_root` in `crates/holt-supervisor/src/paths.rs:17–32`, the temp_dir last-resort branch is taken ONLY when `HOME` (and `USERPROFILE`) is unset. The test SETS `HOME` (to a file path), so the `Some(h) => PathBuf::from(h).join(".cache").join("holt")` branch is taken, and `create_dir_all(<file>/.cache/holt/sessions)` MUST fail (you can't mkdir under a regular file).

So the `FellBack` arm is dead code that masks a real bug if `default_cache_root` ever changes its fallback semantics. The test is too permissive.

**Fix:** Tighten the assertion. Only `HookOutcome::Unwritable` should pass:

```rust
match outcome {
    HookOutcome::Unwritable => { /* expected */ }
    other => panic!("expected Unwritable with HOME=<file>, got {other:?}"),
}
```

If the test then becomes flaky on some CI runner where `create_dir_all` mysteriously succeeds under a regular file, that's a real platform bug worth knowing about.

---

### WR-07: 1000-iteration SIGKILL test fails on slow CI via `panic!` rather than degrading gracefully

**File:** `crates/holt-hooks/tests/sigkill_atomicity.rs:62–66`
**Description:** A 45-second wall-clock cap with `panic!("sigkill_atomicity: budget exceeded after {i} iterations")` means a slow GitHub Actions runner — `runs-on: ubuntu-latest` shared CI workers under load, or `macos-14` arm64 with thermal throttling — converts a budget overrun into a CI failure. The test goal is "no corruption observable", not "spawns 1000 children in 45s". A spurious failure here blocks the whole pipeline.

**Fix:** Convert the budget cap into a soft warning + early termination after a partial sample:

```rust
if started.elapsed() > Duration::from_secs(45) {
    eprintln!(
        "sigkill_atomicity: stopped early after {i} iterations \
         due to slow CI (budget {:.1?}); no corruption observed",
        started.elapsed()
    );
    return;
}
```

200 iterations on a slow runner with no corruption is just as strong evidence of atomicity as 1000 on a fast one.

---

### WR-08: `tempfile` declared as a runtime (non-dev) dependency in `holt-cli` for an opt-in CLI mode

**File:** `crates/holt-cli/Cargo.toml:30`
**Description:** `tempfile = workspace` is in `[dependencies]`, not `[dev-dependencies]`. The justification (lines 26–30) is that `--self-bench` and `--self-bench-hook` use it. But these are opt-in CLI modes; `tempfile` then ships in every `holt` binary, pulling in `cfg-if`, `fastrand`, and `libc` (on Unix), inflating cold-start budget for all users (D-04 sub-20ms target).

`cargo bloat --release --bin holt` would quantify the impact, but on principle: a release binary should not pay for an opt-in dev-style harness.

**Fix:** Either gate the bench harnesses behind a Cargo feature (`#[cfg(feature = "bench")]`) and exclude `tempfile` from the default binary, or replace `tempfile::tempdir()` with a hand-rolled `std::env::temp_dir().join(format!("holt-bench-{pid}"))` + `fs::create_dir_all` + manual cleanup at function exit. The `WR-05` justification only requires "an isolated cache root", not the full `tempfile` crate.

---

### WR-09: `ENV_LOCK` Mutex serializes test bodies but not helper threads

**File:** `crates/holt-hooks/tests/handle_event_smoke.rs:18, 38–43, 79–82, 116–121, 145–150, 179–184, 205–209`
**Description:** Each test acquires `ENV_LOCK` for its full duration. Inside the locked region, the test calls `unsafe { std::env::set_var(...) }`. The `set_var` documentation in Rust 2024 says it is unsafe specifically because it may race with **any other thread reading env vars in the same process**. The lock guards against concurrent test bodies but does NOT guard against:

1. Helper threads spawned by `tempfile` (rare but possible — `tempfile::tempdir()` may spawn cleanup threads in certain configurations).
2. The deadline-bounded stdin slurp thread in `crates/holt-cli/src/stdin.rs:55–60` reading env (it does not today, but it could in the future).
3. Child processes spawned by other tests in the same binary that inherit env at fork time — a fork during another test's `set_var` is a race.

This is sound TODAY (the `holt-hooks` test binary is effectively single-threaded inside any locked region), but it's brittle. The `unsafe` annotation is doing a lot of unstated work.

**Fix:** Add a comment under the `static ENV_LOCK` block explaining what it does and does NOT protect against, AND consider running these tests serially via `cargo test -- --test-threads=1` enforced by a `#[ignore]` + named runner pattern, OR migrate to a test-isolation crate like `serial_test`. Since adding `serial_test` introduces a new dependency, the lower-friction fix is documentation:

```rust
// `cargo test` runs integration tests in parallel by default. We mutate
// process-global env vars (XDG_RUNTIME_DIR, TMPDIR, HOME), so any test in
// this file MUST take this lock for its full duration.
//
// IMPORTANT: this lock guards against parallel test BODIES. It does NOT
// guard against helper threads spawned by `tempfile`, `Command::spawn`, or
// any future stdin-slurp deadline thread reading env vars concurrently.
// If we add such helpers, migrate to `serial_test` or single-threaded
// test execution.
static ENV_LOCK: Mutex<()> = Mutex::new(());
```

## Info

### IN-01: `#[allow(unsafe_code)]` placement on `run_self_bench_hook` is broader than necessary

**File:** `crates/holt-cli/src/self_bench.rs:157`
**Description:** The attribute covers the entire function (`pub fn run_self_bench_hook(...)`), but only the `unsafe { ... }` block on lines 180–183 needs it. Tightening the scope to the block reduces the surface area where `clippy::deny(unsafe_code)` is bypassed.

**Fix:** Move `#[allow(unsafe_code)]` directly above the unsafe block:

```rust
pub fn run_self_bench_hook(event: HookEvent, iterations: u32) -> BenchResult {
    // ...
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", xdg_dir.path());
        std::env::remove_var("TMPDIR");
    }
    // ...
}
```

---

### IN-02: `_bench_tmp` leading-underscore naming masks RAII intent

**File:** `crates/holt-cli/src/self_bench.rs:46`
**Description:** Rust convention treats `_var` as "intentionally unused". But `_bench_tmp` IS used — its `Drop` impl deletes the tempdir at end of scope. A reader scanning the function for live bindings will mentally skip it. The `Option<TempDir>` shape also suggests it's optional, which is just defensive (tempdir creation may fail).

**Fix:** Rename to `bench_tmp_keepalive` or `_bench_tmp_keepalive` and add a one-line comment documenting the RAII pattern:

```rust
// Keep the tempdir alive for the bench's lifetime; dropped on function exit.
let bench_tmp_keepalive = tempfile::tempdir().ok();
let cache_root = match &bench_tmp_keepalive {
    Some(t) => t.path().to_path_buf(),
    None => std::env::temp_dir().join(format!("holt-self-bench-{}", std::process::id())),
};
```

---

### IN-03: LCG seed `0xdeadbeef` is a magic number with no clear rationale

**File:** `crates/holt-hooks/tests/sigkill_atomicity.rs:54`
**Description:** The seed is fixed for reproducibility (good — a flaky SIGKILL race test would be a nightmare to triage), but `0xdeadbeef` carries no semantic meaning. A short comment would help future maintainers understand why the seed is fixed and why it's safe to use a non-cryptographic LCG.

**Fix:**

```rust
// Fixed-seed LCG for reproducibility — a flaky kill-delay distribution
// would make this test impossible to triage. Non-cryptographic; we only
// need a uniform-ish 0..=15ms delay sequence, not unpredictability.
let mut rng_state: u64 = 0xdeadbeef;
```

---

### IN-04: `current_uid_string` returns hardcoded `"0"` on non-Unix, conflating root and Windows

**File:** `crates/holt-hooks/src/path.rs:132–138`
**Description:** On Windows or other non-Unix platforms, every user's tier-2 path collapses to `$TMPDIR/holt-0/sessions/`. If two users on the same Windows machine ever shared a `TMPDIR` (extremely rare, but possible with shared `%TEMP%`), they'd collide. The comment acknowledges the fallback but flags the cosmetic UID `"0"`.

**Fix:** On Windows, use `std::env::var("USERNAME")` or fall through to the tier-3 cache path. The lowest-friction fix:

```rust
#[cfg(not(unix))]
fn current_uid_string() -> String {
    // On Windows, use USERNAME for per-user separation. Falls back to "user"
    // (not "0") so the path component reads sensibly in error messages.
    std::env::var("USERNAME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "user".to_string())
}
```

---

### IN-05: `now_iso` is captured ONCE per fire and used for both `started` and `updated`

**File:** `crates/holt-cli/src/hook.rs:33` and `crates/holt-hooks/src/assemble.rs:60–61`
**Description:** This matches D-05 / criterion #2 ("`updated` always advances per criterion #2") because every fire builds a fresh `now_iso`. But the field name `started` suggests "session start time", not "this hook fire's start time". The first PreToolUse and the last Stop have the SAME `started` value if the session only fires once, but DIFFERENT `started` values across fires. A reader inspecting two heartbeats from the same session can't tell which is older from `started` alone.

This is by design (the orchestrator at v1.0 uses `mtime` for staleness), but the field name is misleading.

**Fix:** Either rename `started` → `fired_at` in `holt-schemas::Heartbeat` (breaking change, defer to v1.0), or document the semantic in `assemble.rs` so future readers don't get confused:

```rust
// `started` in the v0.1 schema is "this fire's start time", not "session
// start time". The orchestrator (v1.0) uses mtime for staleness; `started`
// matches `updated` for v0.1 because we have no per-session genesis tracker.
```

---

### IN-06: `serde_json::to_vec(&heartbeat)` failure path is dead code in practice

**File:** `crates/holt-hooks/src/handle.rs:83–92`
**Description:** `Heartbeat` only contains `String`, `u8`, `u32`, `Option<String>`, and `Option<f64>`. `serde_json::to_vec` cannot fail on any of these — `String` is always valid UTF-8, numbers always serialize. The error arm is genuinely defensive but will never execute. Comment acknowledges this ("Should never happen").

**Fix:** Either downgrade to a `.expect("Heartbeat is always serializable")` (panic-on-impossible — but violates "never panics"), or leave the defensive arm and add a `#[cfg(test)]` test that exercises it via a `Heartbeat` with deliberately-broken UTF-8 (impossible — see above) to document the dead-code intent. Lowest-friction: replace the comment with a one-line note that the arm is structurally unreachable but coded defensively because `Heartbeat` is `#[non_exhaustive]` and v1.0 might add a `f64::NAN`-shaped field that DOES fail to serialize.

---

### IN-07: `holt-hooks/Cargo.toml` declares `nix` with the `user` feature only, but the comment hints at a wider use

**File:** `crates/holt-hooks/Cargo.toml:31–32`
**Description:** The comment on lines 28–30 says "Pulled in for symmetry with holt-supervisor and to keep the door open for the SIGKILL test driver to call `nix::sys::signal::kill` if needed." The `user` feature alone does not enable `signal`. If the SIGKILL driver ever needs `signal::kill`, the feature list must change. Today the driver uses `child.kill()` (Tokio-free, std-only), so the comment is preemptive — fine — but the door-open claim is misleading.

**Fix:** Drop the speculative half of the comment, since today's code doesn't need it:

```toml
# Unix-only: nix is currently used for getuid()-based path salting in path.rs
# (D-06 tier-2 builds $TMPDIR/holt-$UID/sessions/). We do NOT rely on nix for
# the 0o600 chmod (that uses std::os::unix::fs::PermissionsExt).
[target.'cfg(unix)'.dependencies]
nix = { workspace = true, features = ["user"] }
```

---

_Reviewed: 2026-04-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
