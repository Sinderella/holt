---
phase: 1
phase_name: Schema + supervisor substrate
status: findings_present
depth: standard
files_reviewed: 35
findings:
  critical: 5
  warning: 11
  info: 8
  total: 24
reviewed: 2026-04-28
---

# Phase 1 — Schema + supervisor substrate · Adversarial Code Review

Three layers under audit: the keystone `holt-schemas`, the `holt-supervisor` wedge with C1/C5/C6 discipline, and the `holt-cli` skeleton with `--self-bench`. Plumbing is clean; the architecture-DAG, chokepoint, and reader-contract tests prove what they claim. The findings below are real defects, not nits — most cluster around the *interface between holt and the wrapped script's stdin/stdout pipes*, the LKG round-trip on a non-existent target, and one mandated `serde_json` feature that's missing from the workspace.

Key positive signals (no finding to write):

- C1 chokepoint (exactly one `.wrap(ProcessGroup::leader())`) holds; the audit test enforces it on every `cargo test`.
- C2 architecture-DAG test (`tests/architecture_dag.rs`) is correctly wired and would fail loudly on a `holt-render` → `holt-supervisor` edge.
- C5 reader contract is implemented and exhaustively tested (six Ok(None) cases, one Ok(Some), arbitrary-bytes fuzz).
- C6 render-path `strace` test is in place (Linux-only, by design, with the platform skip path correctly handled).
- No forbidden crates surfaced in any `Cargo.toml` or `Cargo.lock` (no `tokio`, `simd-json`, `figment`, `chrono`, `jsonc-parser`, `fs2`, `owo-colors`, `supports-color`, `terminal_size`, `atomic-write-file`, `crossterm`, `wait-timeout`, `cargo_metadata`).
- `#![forbid(unsafe_code)]` set on `holt-schemas` and `holt-supervisor`. No `unsafe` blocks in any `src/`.

---

## Critical Issues

### CR-01: `serde_json` `preserve_order` feature not enabled — violates project tech-stack mandate

**File:** `Cargo.toml:51`
**Constraint impact:** Direct violation of CLAUDE.md technology-stack directive ("`serde_json` with `preserve_order` feature").

**Description:**
The workspace dep is declared `serde_json = "1"` with no features. CLAUDE.md (project root) is explicit:

> **JSON:** Strict — `serde_json` with `preserve_order` feature; **NO** `simd-json`/`figment`

Without `preserve_order`, every round-trip through `serde_json::Value` (including the one in `holt-cli/src/run.rs` line 39 — see CR-02) re-orders keys to BTreeMap order. Wrapped scripts that key on insertion order, or that pretty-print the input JSON, will see different bytes than CC sent. Phase 2's heartbeat-write hooks will hit the same problem when round-tripping `additionalContext` blobs.

**Recommendation:**
Update workspace dep declaration:

```toml
# Cargo.toml
[workspace.dependencies]
serde_json = { version = "1", features = ["preserve_order"] }
```

Then verify in `Cargo.lock` that `serde_json` pulls `indexmap` (it already does for other reasons; preserving keys becomes the active path). Add a smoke test in `holt-schemas/tests/` round-tripping `{"b":1,"a":2}` and asserting key order is preserved.

---

### CR-02: `holt run` re-serialises CC stdin via `Value::to_string()`, dropping the original bytes

**File:** `crates/holt-cli/src/run.rs:39-40`
**Constraint impact:** Breaks the spirit of CORE-08 ("forward CC stdin to wrapped script unmodified"). Couples wrapped-script behaviour to serde_json's serializer choices (number formatting, key ordering — see CR-01).

**Description:**

```rust
StdinParseOutcome::Ok(v) => v.to_string().into_bytes(),
```

`v.to_string()` on `serde_json::Value` produces compact JSON without trailing newline, with `f64::to_string()` formatting (so CC's `1.0` becomes `1`, `0.1` becomes `0.1` — but Display rounds in surprising ways), and (without CR-01's fix) re-orders object keys. ccstatusline and other wrapped scripts that parse CC stdin will see slightly-different input than CC actually sent.

The defensive parse should *validate* CC stdin is JSON, but should *forward the original bytes* to the wrapped script.

**Recommendation:**
Capture the original bytes from `slurp_and_parse` and forward those:

```rust
// stdin.rs
pub enum StdinParseOutcome {
    Ok { value: serde_json::Value, raw: Vec<u8> },
    ParseFail { excerpt: String, raw: Vec<u8> },
    Empty,
}

pub fn slurp_and_parse() -> StdinParseOutcome {
    let mut buf = Vec::with_capacity(4096);
    if io::stdin().read_to_end(&mut buf).is_err() { return StdinParseOutcome::Empty; }
    if buf.is_empty() { return StdinParseOutcome::Empty; }
    match serde_json::from_slice::<serde_json::Value>(&buf) {
        Ok(v) => StdinParseOutcome::Ok { value: v, raw: buf },
        Err(_) => StdinParseOutcome::ParseFail {
            excerpt: String::from_utf8_lossy(&buf[..buf.len().min(2048)]).into_owned(),
            raw: buf,
        },
    }
}
```

Then in `run.rs`, set `stdin_bytes = raw` on the `Ok` arm.

---

### CR-03: Supervised child stdin is never closed when `stdin_bytes` is empty — wrapped scripts that read stdin always breach

**File:** `crates/holt-supervisor/src/supervisor.rs:108-115`
**Constraint impact:** Render-path budget violation (forces 2s timeout breach for any wrapped script that does `read_to_end(stdin)` when CC sends empty stdin). C5 (degrade gracefully) breaks because `holt run`'s "empty stdin" code path becomes a guaranteed timeout for stdin-reading scripts.

**Description:**

```rust
if !opts.stdin_bytes.is_empty() {
    if let Some(mut stdin) = child.stdin().take() {
        let bytes = opts.stdin_bytes.clone();
        thread::spawn(move || { let _ = stdin.write_all(&bytes); });
    }
}
```

When `stdin_bytes` is empty, the `if` is false, the child's stdin handle is never `take()`-d, and the writable end of the pipe stays attached to the `WrappedChild`. That handle moves into the wait-thread (line 138-142) and only drops when `child.wait()` returns. A wrapped script that does `cat`, `jq`, or any `read_to_end(stdin)` will block forever waiting for EOF — until the 2s `DEFAULT_TIMEOUT` expires, at which point we kill it with SIGKILL and emit a `BreachKind::Timeout` to `breaches.log`.

This is the single most likely bug to surface as "holt is breaking my statusLine" in the wild: ccstatusline reads CC stdin to produce its output. With CC's "empty stdin" early in a session, holt synthesises a permanent timeout breach for it.

**Recommendation:**
Always close the child's stdin handle if we're not going to write to it:

```rust
if let Some(mut stdin) = child.stdin().take() {
    if !opts.stdin_bytes.is_empty() {
        let bytes = opts.stdin_bytes.clone();
        thread::spawn(move || { let _ = stdin.write_all(&bytes); });
    }
    // If stdin_bytes is empty, dropping `stdin` here closes the pipe → child gets EOF.
}
```

(`drop(stdin)` is implicit at end of `if let` scope; the explicit close is just being clear about intent.) Add a regression test: `wrap_and_run("bash", &["-c", "cat"], opts)` with `stdin_bytes: Vec::new()` must complete cleanly within 100ms, not breach at the 2s deadline.

---

### CR-04: `slurp_and_parse` blocks on stdin with no timeout — render-path budget can exceed 20ms indefinitely

**File:** `crates/holt-cli/src/stdin.rs:15-31`
**Constraint impact:** Violates the sub-20ms cold-start budget (D-04). The render path is supposed to never block on input, but `io::stdin().read_to_end(&mut buf)` waits until CC closes its stdin or EOFs.

**Description:**
`slurp_and_parse` does an unbounded `read_to_end` on stdin. If CC ever leaves the stdin pipe open without writing (or writes slowly under load — exactly the conditions hooks-of-death are designed to surface), holt waits forever before even reaching `wrap_and_run`. The 20ms render-path budget is blown silently.

This is also why CR-03 is so dangerous: even if CC closes its stdin promptly, the supervised child can be left waiting for EOF on its own stdin pipe.

**Recommendation:**
Add a stdin deadline. The cheapest approach is a thread + mpsc with a short timeout (well under the 2s supervisor deadline; 200ms is generous):

```rust
pub fn slurp_and_parse() -> StdinParseOutcome {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::with_capacity(4096);
        let read_ok = io::stdin().read_to_end(&mut buf).is_ok();
        let _ = tx.send((read_ok, buf));
    });
    let (read_ok, buf) = match rx.recv_timeout(Duration::from_millis(200)) {
        Ok(pair) => pair,
        Err(_) => return StdinParseOutcome::Empty, // or a new ParseFail variant tagged "stdin_timeout"
    };
    // …rest unchanged
}
```

Better long term: also surface the timeout case as a distinct breach kind so doctor (v0.5) can flag it. Not blocking for v0.1 — Empty fall-through is acceptable as a minimum bar.

---

### CR-05: Atomic-write tmp file leaks on every `write_all` / `sync_all` failure, then poisons the next call via `create_new(true)`

**File:** `crates/holt-schemas/src/writer.rs:41-50`
**Constraint impact:** Breaks D-07 atomic-write durability story. The cleanup is *only* triggered when `fs::rename` fails (line 47-50). If `f.write_all(contents)?` or `f.sync_all()?` errors out (line 42-43, EIO / ENOSPC / EAGAIN under load), the `?` returns immediately without removing the half-written tmp file. The next call from the same PID hits `EEXIST` on `OpenOptions::create_new(true)` and the entire write path stays broken until the user manually clears `*.holt-tmp.<pid>`.

**Description:**
The current code:

```rust
let mut f = opts.open(&tmp)?;
f.write_all(contents)?;        // ← if this fails, tmp file is not removed
f.sync_all()?;                 // ← same here
drop(f);
if let Err(e) = fs::rename(&tmp, path) {
    let _ = fs::remove_file(&tmp);   // only this branch cleans up
    return Err(e);
}
```

Tmp filename is `dir.join(format!("{}.holt-tmp.{pid}", file_name.to_string_lossy()))` — same PID writing twice in a row will collide.

**Recommendation:**
Wrap the inner steps so the tmp file is removed on any error path. Idiomatic Rust:

```rust
let result = (|| -> io::Result<()> {
    let mut f = opts.open(&tmp)?;
    f.write_all(contents)?;
    f.sync_all()?;
    drop(f);
    fs::rename(&tmp, path)
})();
if result.is_err() {
    let _ = fs::remove_file(&tmp);
}
result
```

Add a regression test that simulates an `ENOSPC` (writing to a tmpfs sized 4KB and fsync'ing 5KB), asserts `atomic_write` returns `Err`, and asserts the directory contains zero `*.holt-tmp.*` entries afterwards.

---

## Warnings

### WR-01: `paths::default_cache_root` falls back to `.` when `HOME` is unset — silently writes `./cache/holt/...` into CWD

**File:** `crates/holt-supervisor/src/paths.rs:16`
**Constraint impact:** None hard, but a real footgun for CI/sandbox environments where HOME is sometimes unset.

**Description:**
`std::env::var("HOME").unwrap_or_else(|_| ".".into())` — if both `XDG_CACHE_HOME` and `HOME` are unset (rare but real: minimal Docker images, FreeBSD jails, some sandbox profiles), holt writes `lkg/`, `timings.jsonl`, and `breaches.log` into the user's *current working directory*. For a Claude Code session started in a git repo, that means polluting the repo with `holt/breaches.log`.

**Recommendation:**
On both unset, fall back to the system temp dir or `/tmp/holt-<uid>`, and emit nothing if even that fails. A clean approach:

```rust
let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok();
match home {
    Some(h) if !h.is_empty() => PathBuf::from(h).join(".cache").join("holt"),
    _ => std::env::temp_dir().join(format!("holt-{}", std::process::id())),
}
```

---

### WR-02: `read_heartbeat` returns `Err` for `PermissionDenied` and `IsADirectory` — C5 says these should be Ok(None)

**File:** `crates/holt-schemas/src/reader.rs:25`
**Constraint impact:** C5 (reader treats stale-or-corrupt heartbeat as missing).

**Description:**
The contract in the docs and tests is "every 'session unreadable' outcome → Ok(None); only Err for I/O that is NOT 'file missing'". But the current implementation only collapses `NotFound` into Ok(None). A heartbeat file with wrong perms (mode 0000 from a stale prior install) returns `Err(ReaderError::Io(PermissionDenied))`. A directory at the heartbeat path returns `Err(...)`. Both should arguably be Ok(None) — the render path can't read them, that's a "session unreadable" outcome by definition.

**Recommendation:**
Broaden the soft-fail set:

```rust
let bytes = match fs::read(path) {
    Ok(b) => b,
    Err(e) => match e.kind() {
        io::ErrorKind::NotFound
        | io::ErrorKind::PermissionDenied
        | io::ErrorKind::IsADirectory => return Ok(None),
        _ => return Err(ReaderError::Io(e)),
    },
};
```

(Note: `IsADirectory` is stable on `ErrorKind` since Rust 1.83 — within MSRV 1.87.) Add tests for both new branches in `tests/reader_contract.rs`.

---

### WR-03: TOCTOU race in jsonl rotation — multiple holt processes for different sessions can corrupt `.1`

**File:** `crates/holt-supervisor/src/timings.rs:36-49`
**Constraint impact:** C6 (render path writes to telemetry without coordination).

**Description:**
The write path is:

1. Stat the file; if size + line.len() > 5MB, rename current → `.1`.
2. `OpenOptions::append(true).create(true)` and write.

Two concurrent holt fires (two CC sessions on the same machine — explicitly within the multi-session use case) can both see "size > MAX_BYTES", both rename, the second clobbers the first's `.1` (lost log data), and one of them races between rename and open such that the new line lands in the OLD file (already moved to `.1`). On Linux this is benign-ish; on Windows the rename can fail because the file is open for append by the other process.

**Recommendation:**
Acceptable for v0.1 if explicitly accepted as a known limitation. Best mitigation if you want to fix it: put rotation behind a `flock`-style advisory lock on a sibling `.lock` file (one syscall, ~50µs uncontended). Trigger to harden: ≥1 user report of corrupted `.1` rotation. At minimum, add a comment block explaining the race is known and document the worst case.

---

### WR-04: TOCTOU PID-reuse race in Linux `ppid_walk_kill`

**File:** `crates/holt-supervisor/src/kill.rs:42-62`
**Constraint impact:** None hard; correctness under PID-reuse pressure.

**Description:**
The walk reads `/proc` to enumerate PIDs, then for each PID reads `/proc/<pid>/status` and matches `Pgid:`. Between enumeration and the `kill(2)` call, the kernel can reuse a PID. The PID we matched on `Pgid:` may have already exited and been replaced by an unrelated process that happens to share the pgid number. We `SIGKILL` the wrong process.

In practice: very rare on a normal host. On a heavily-forking system (CI runners, shell scripts in `for` loops) it's measurable.

**Recommendation:**
After matching on `Pgid:`, verify by reading `/proc/<pid>/stat` for the original `pgrp` field again immediately before sending SIGKILL. If it still matches, kill; otherwise skip. This isn't fully airtight (the race can still happen between read-stat and kill) but shrinks the window from milliseconds to microseconds. Document this as a known v0.1 limitation if not fixed.

---

### WR-05: Self-bench writes to the user's real `~/.cache/holt/` — pollutes telemetry with synthetic "self-bench" entries

**File:** `crates/holt-cli/src/self_bench.rs:33-46`
**Constraint impact:** None hard; user-experience and observability hygiene.

**Description:**
`run_self_bench` calls `default_cache_root()` and feeds it to `wrap_and_run` as `cache_root`. Each iteration appends a `timings.jsonl` line with `session_id: "self-bench"` and an LKG entry at `<cache>/lkg/self-bench.json`. CI runs this on every push (per `.github/workflows/ci.yml`); for users running `holt --self-bench` locally, it permanently mixes 30+ synthetic data points into their real telemetry stream. `holt doctor` (v0.5) would then have to filter them out.

**Recommendation:**
Bench against a tempdir:

```rust
let tmp = tempfile::tempdir().expect("self-bench tempdir");
let cache_root = tmp.path().to_path_buf();
```

Add `tempfile = "3"` to `holt-cli`'s `[dependencies]` (currently only in `dev-dependencies`). Or, simpler, expose `cache_root: Option<PathBuf>` on `BenchOptions` and have CI inject a tempdir.

---

### WR-06: Self-bench `pick(0.95)` on 10 samples returns the max sample — claimed "p95" is actually p100

**File:** `crates/holt-cli/src/self_bench.rs:55-58`
**Constraint impact:** None hard; CI gate is misleading.

**Description:**
`pick(frac)` computes `idx = ((len - 1) * frac).round()`. For `len=10`, `frac=0.95`: `idx = round(9 * 0.95) = round(8.55) = 9`, which is the maximum element (0-indexed). For `len=10`, `frac=0.99`: `idx = round(9 * 0.99) = round(8.91) = 9` — same maximum. So with the default `iterations = 10`, the p95 and p99 the bench reports are *both* simply the max overhead observed.

The CI gate (`p95 ≤ 20_000us`) ends up being "max-of-10 ≤ 20_000us", which is much more sensitive to outliers than a real p95. False-FAIL risk on a slow Linux runner is non-trivial.

**Recommendation:**
Either (a) bump the default iteration floor to ≥20 so p95 is meaningful, or (b) document that the gate is "max-of-N" rather than "p95-of-N", or (c) use a proper percentile calculation (linear interpolation between adjacent indices for floating-point fractions). The CI gate already passes `--iterations 30`, but that's not the local default.

---

### WR-07: Stdout-drain thread is orphaned on the timeout path — thread handles leak under sustained breach load

**File:** `crates/holt-supervisor/src/supervisor.rs:182-217`
**Constraint impact:** None hard; resource hygiene.

**Description:**
On the timeout branch (line 182), `stderr_thread` is joined (line 194-196), but `stdout_thread` is never joined and never dropped — the JoinHandle is bound at line 120 and goes out of scope here without `.join()`. The OS thread is detached and continues running until `read_to_end` on the (now-closed-by-SIGKILL-of-child) stdout pipe returns. In the meantime its `Vec<u8>` buffer (up to whatever the child wrote) leaks for the duration.

In a long-running CC session that hits 100s of timeout breaches, this accumulates.

**Recommendation:**
Either join (and discard) stdout_thread on the timeout branch the same way stderr_thread is joined, or attach a `.join()` outside the match in a single `let _stdout_dropped = stdout_thread.and_then(|t| t.join().ok());` for both arms. The captured stdout is irrelevant on a timeout (we won't return it to the caller), but joining ensures the thread terminates before `wrap_and_run` returns.

---

### WR-08: `breaches.rs` `writer_version` is hardcoded to the supervisor crate's version, not the holt binary's

**File:** `crates/holt-supervisor/src/breaches.rs:97`
**Constraint impact:** None hard; observability accuracy. Per D-13 the `writer_version` is meant to identify the holt binary version when triaging a breach.

**Description:**

```rust
writer_version: env!("CARGO_PKG_VERSION"),
```

This expands at compile time to the version in `holt-supervisor/Cargo.toml` (currently 0.1.0). If the workspace ever ships a holt binary at version 0.1.5 while `holt-supervisor` is still 0.1.3 (legitimate during refactors), the breach record reports the wrong version.

**Recommendation:**
Pass `writer_version` in via `SupervisorOptions` (filled by `holt-cli/main.rs` from its own `CARGO_PKG_VERSION`):

```rust
// SupervisorOptions
pub writer_version: &'static str,
// holt-cli/src/run.rs
writer_version: env!("CARGO_PKG_VERSION"),
```

---

### WR-09: `holt-cli`, `holt-render`, `holt-orchestrator`, `holt-hooks` lack `#![forbid(unsafe_code)]`

**File:** `crates/holt-cli/src/main.rs:1`, `crates/holt-render/src/lib.rs:1`, `crates/holt-orchestrator/src/lib.rs:1`, `crates/holt-hooks/src/lib.rs:1`
**Constraint impact:** None hard; defence in depth.

**Description:**
`holt-schemas` and `holt-supervisor` set `#![forbid(unsafe_code)]`. The other four don't. Project conventions around the render path being audit-quality argue all four should set the same crate-wide forbid.

**Recommendation:**
Add `#![forbid(unsafe_code)]` to the top of every `src/lib.rs` (and `src/main.rs` for `holt-cli`). One-line change per crate.

---

### WR-10: Self-bench never closes child stdin (same pattern as CR-03) — the `:` no-op happens to not read stdin so it hides the bug

**File:** `crates/holt-cli/src/self_bench.rs:35-46`
**Constraint impact:** None at the bench's measured workload (`:` doesn't read stdin), but the bench is supposed to model the real render path. It currently models a path with the CR-03 bug masked.

**Description:**
The bench builds `SupervisorOptions { stdin_bytes: Vec::new(), .. }` and calls `wrap_and_run`. Per CR-03, the supervisor leaves the child stdin open. `:` ignores stdin so this works. If `:` is ever swapped for something more representative (e.g., a script that does an `if [ -t 0 ]` check), behavior flips.

**Recommendation:**
Becomes a non-issue once CR-03 is fixed. Until then, add a comment in `self_bench.rs` noting the bench command is chosen specifically because it doesn't read stdin.

---

### WR-11: Cargo `resolver = "2"` set explicitly with Edition 2024 (which defaults to resolver 3)

**File:** `Cargo.toml:2`
**Constraint impact:** None hard; mild future-compat surprise.

**Description:**
Edition 2024 implies `resolver = "3"` by default (with workspace-wide `MSRV-aware feature unification`). Pinning to `"2"` is legal but suppresses an MSRV-aware-resolution feature you might want. If this was intentional, add a comment; if it was leftover from a 2021 template, drop it.

**Recommendation:**
Either remove the line (let edition default kick in) or add a `# locked to resolver 2 because <reason>` comment so the next reader doesn't second-guess it.

---

## Info

### IN-01: `Supervisor` struct is a dead-code shim around the free `wrap_and_run` function

**File:** `crates/holt-supervisor/src/supervisor.rs:38-50`
**Description:** The `Supervisor` ZST wraps the free function with no added value (and no internal state). Comment says "convenience alias" but no caller in Phase 1 uses `Supervisor::wrap_and_run`. Either pick one (drop the struct, or have `wrap_and_run` be a private impl detail and the only public surface is `Supervisor::wrap_and_run`) — exporting both is API confusion.

**Recommendation:** Drop the `Supervisor` ZST until a caller actually needs it. Or keep the struct, mark the free function `pub(crate)`, and only re-export `Supervisor`.

---

### IN-02: `holt-schemas/src/lib.rs` re-exports `error::ReaderError` AND keeps `pub mod error;`

**File:** `crates/holt-schemas/src/lib.rs:12,18`
**Description:** Both the module and the re-export are public. Callers can write either `holt_schemas::ReaderError` or `holt_schemas::error::ReaderError`. Pick one to keep the public surface minimal.

**Recommendation:** Make `mod error` private (`pub(crate)`) and rely on the re-export at the crate root.

---

### IN-03: `status.code().unwrap_or(-1)` uses `-1` as a magic sentinel for "killed by signal"

**File:** `crates/holt-supervisor/src/supervisor.rs:155`
**Description:** On Unix, `code()` returns `None` when the child was terminated by a signal. `-1` is a sentinel — not a real exit code — and downstream `holt doctor` parsers will need to handle it. Document it or use `Option<i32>` end-to-end.

**Recommendation:** Either widen `SupervisorOutcome::Ok::exit_code` to `Option<i32>` (matching the `Breach` variant) or document `exit_code: -1 ⇒ killed by signal` in the rustdoc on `SupervisorOutcome::Ok`.

---

### IN-04: `default_cache_root` does not honour `LOCALAPPDATA` on Windows

**File:** `crates/holt-supervisor/src/paths.rs:10-18`
**Description:** Falls back to `HOME` on Windows; idiomatic Windows would prefer `%LOCALAPPDATA%`. Not blocking — Phase 1 says Windows is best-effort/allowed-failure.

**Recommendation:** Defer to a follow-up; document the Windows fallback as a known v0.1 simplification.

---

### IN-05: `Heartbeat.mode` accepts any `Option<String>` — the documented enum values are not validated

**File:** `crates/holt-schemas/src/heartbeat.rs:33`
**Description:** Comment lists `"default" | "plan" | "acceptEdits" | "bypassPermissions"` but no validation enforces this. Defensive parse posture (per H5) means accepting unknown strings is intentional, but a typed enum with `#[serde(other)]` for forward-compat would be more honest.

**Recommendation:** Defer to Phase 2 when `mode` becomes load-bearing for pet-state transitions. Add a `// TODO: typed enum at Phase 2` comment so the deferral is explicit.

---

### IN-06: `breaches.rs` `Map<String, Value>` for `env_capture` is heavy for what's effectively `BTreeMap<String, String>`

**File:** `crates/holt-supervisor/src/breaches.rs:57`
**Description:** `serde_json::Map<String, Value>` is fine but encodes a flexibility the schema doesn't need (env values are always strings). A `BTreeMap<String, String>` would be smaller and clearer.

**Recommendation:** Cosmetic; defer.

---

### IN-07: `holt-render/src/lib.rs` doc-comment about C2 is good; consider mirroring it in `holt-orchestrator/src/lib.rs`

**File:** `crates/holt-orchestrator/src/lib.rs:1`
**Description:** Render explicitly documents "MUST NOT depend on holt-supervisor". Orchestrator is one hop closer to render and should carry the same warning so a future contributor adding `holt-supervisor` as a dep there doesn't accidentally re-route.

**Recommendation:** Add a one-line C2 note: `//! HARD CONSTRAINT C2: this crate must not depend on holt-supervisor (transitive ban via holt-render).`

---

### IN-08: Test `lkg_roundtrip.rs` mixes `let _ =` and ignored outcome — but only checks the file landed

**File:** `crates/holt-supervisor/tests/lkg_roundtrip.rs:25-44`
**Description:** Test discards the `SupervisorOutcome` from `wrap_and_run` and only checks the LKG file. If `wrap_and_run` returned a `Breach`, the LKG file wouldn't have been written, and the test's first assertion (`lkg_path.exists()`) would catch it — but with a less helpful message than asserting on the outcome shape.

**Recommendation:** Add `assert!(matches!(outcome, SupervisorOutcome::Ok { .. }))` at the top, then proceed to file checks.

---

_Reviewed: 2026-04-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
