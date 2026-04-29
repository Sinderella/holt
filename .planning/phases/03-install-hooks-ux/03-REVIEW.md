---
phase: 3
phase_name: install-hooks UX
status: findings_present
depth: standard
files_reviewed: 16
findings:
  critical: 0
  warning: 6
  info: 5
  total: 11
reviewed: 2026-04-28
files_reviewed_list:
  - crates/holt-cli/Cargo.toml
  - crates/holt-cli/src/install_hooks/mod.rs
  - crates/holt-cli/src/install_hooks/entries.rs
  - crates/holt-cli/src/install_hooks/lock.rs
  - crates/holt-cli/src/install_hooks/merge.rs
  - crates/holt-cli/src/install_hooks/diff.rs
  - crates/holt-cli/src/install_hooks/print.rs
  - crates/holt-cli/src/install_hooks_cmd.rs
  - crates/holt-cli/src/cli.rs
  - crates/holt-cli/src/main.rs
  - crates/holt-cli/tests/install_hooks_merge_smoke.rs
  - crates/holt-cli/tests/install_hooks_smoke.rs
  - crates/holt-cli/tests/install_hooks_concurrent.rs
  - crates/holt-cli/tests/install_hooks_sigkill.rs
  - tests/cli_dep_boundary.rs
  - Cargo.toml
---

# Phase 3: Code Review Report

**Reviewed:** 2026-04-28
**Depth:** standard
**Files Reviewed:** 16
**Status:** findings_present

## Summary

Phase 3 implements `holt install-hooks` with a JSONC CST round-trip merger,
fs2 exclusive locking, atomic-write commit pipeline, and a strong test
matrix (fixture corpus, 50× concurrent, 200× SIGKILL, workspace-root C4
boundary). The code is generally tight: hard constraints C3 and C4 hold,
no forbidden crates appear, `unsafe_code` is kept inside the SIGKILL FFI
test with a SAFETY comment, and the dispatcher uses `eprintln!` + numeric
exit codes rather than `unwrap`/`panic` on user-facing errors.

No CRITICAL findings. The merger does not corrupt settings.json on any
inspected error path; `.holt.bak` is written before the merged file;
`atomic_write` cleanup is verified end-to-end.

WARNING-level findings cluster around three themes:

1. **TOCTOU + lock-acquire side effects.** `acquire_settings_lock` opens
   with `create(true)` BEFORE attempting the lock loop, so a lock-timeout
   on a fresh `~/.claude/` leaves a zero-byte `settings.json` that the
   user did not author. The dispatcher then re-reads via `std::fs::read_to_string`
   (a fresh open), introducing a small window between lock and read.
2. **`expect()` on a CST round-trip invariant.** `merge.rs` panics if
   `jsonc-parser` ever returns a non-object node from a just-appended
   `Object(Vec::new())` — defensible but a panic on a user-facing path.
3. **`string_property` uses `raw_value().trim_matches('"')`.** Fine for
   our static canonical commands, but the same path is taken when scanning
   user entries for substring detection; a user entry with escape sequences
   in `"command"` would not decode correctly. The substring check still
   works for the literal `"holt hook "` needle, but `is_canonical_entry`
   could spuriously reject a canonical entry whose source uses an escape
   sequence, breaking byte-equal idempotency in that degenerate case.

INFO-level findings are minor (magic strings, fragile test parsers, missing
JSON escaping in `pretty_snippet`).

## Warnings

### WR-01 | `acquire_settings_lock` creates settings.json before lock contention is resolved

**File:** `crates/holt-cli/src/install_hooks/lock.rs:60-75`
**Constraint:** C3 (settings.json mutation hygiene)
**Issue:** `OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)`
runs unconditionally before the `try_lock_exclusive` loop. On a fresh system
where `~/.claude/settings.json` does not yet exist, this creates a zero-byte
file with mode `0o600`. If lock acquisition then times out (200ms budget,
return `LockError::Timeout`), the function returns Err but the zero-byte
file is left on disk. The user sees a `settings.json` they did not create,
and a subsequent `holt install-hooks` will treat the empty file as `{}`
(per `merge.rs:60` `input.trim().is_empty()` path) and write a fully-merged
file — which is fine — but the timeout-then-empty-file intermediate state
is a surprising side effect for a function whose name implies "acquire
or fail without mutation."

This also weakens the contract caller-side: `install_hooks_cmd.rs:99-106`
re-reads with `std::fs::read_to_string` and silently treats empty input
as `{}`; a corrupted-by-truncation file from another tool between the
lock-create and the read would be silently overwritten with the canonical
shape. Probability is low (we hold the lock), but `truncate(false)` means
content existing at lock-acquire time is fully preserved — the issue is
narrowly the create-before-lock side effect.

**Fix:** Either (a) check `path.exists()` before opening and skip
`create(true)` so a lock-timeout-on-fresh-system leaves the directory
genuinely untouched, or (b) document this side effect in the lock.rs
module doc and `acquire_settings_lock` doc comment so callers know the
file may exist post-failure. Option (a) is preferred:

```rust
let mut opts = OpenOptions::new();
opts.read(true).write(true).truncate(false);
if !path.exists() {
    opts.create(true);
}
#[cfg(unix)]
{
    use std::os::unix::fs::OpenOptionsExt;
    opts.mode(0o600);
}
```
Note: this introduces its own TOCTOU (file could appear between `exists()`
and `open`), but `create(true)` without `create_new(true)` is idempotent
on race so the result is still correct. Alternative: separate the "ensure
file" step into its own short-lived open-and-close that runs only after
lock acquisition on the existing file; that requires lock-on-parent-dir
or a sentinel lockfile, which is heavier than the current single-flock
contract and probably not worth it.

---

### WR-02 | `merge.rs::merge_settings` panics if `append("hooks", Object(...))` returns a non-object

**File:** `crates/holt-cli/src/install_hooks/merge.rs:80-87`
**Constraint:** No-panic-on-user-input UX expectation (matches CLAUDE.md
"no `unwrap()` on the render path"; install-hooks is not the render path,
but malformed-JSONC panics are still bad UX)
**Issue:**
```rust
let prop = root_obj.append("hooks", CstInputValue::Object(Vec::new()));
prop.value()
    .and_then(|v| match v {
        CstNode::Container(CstContainerNode::Object(o)) => Some(o),
        _ => None,
    })
    .expect("just-appended hooks value is an object")
```
The `expect` will fire if jsonc-parser ever returns `None` from `prop.value()`
or returns a non-Object container. The comment is correct that the input
shape (`CstInputValue::Object`) ought to deterministically produce an
Object node, but the API contract is not statically enforced — a
`jsonc-parser` minor-version bump (the file already documents
`jsonc-parser-0.26.3` as the source-of-truth via comments) could in
principle change this, and the panic message would surface as an
unrecoverable abort to the user mid-install.

**Fix:** Convert to a typed error variant rather than `expect`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum MergeError {
    // ...
    #[error("internal: jsonc-parser CST returned unexpected shape after append; please file a bug")]
    CstShape,
}

// in merge_settings:
let prop = root_obj.append("hooks", CstInputValue::Object(Vec::new()));
let v = prop.value().ok_or(MergeError::CstShape)?;
let CstNode::Container(CstContainerNode::Object(o)) = v else {
    return Err(MergeError::CstShape);
};
o
```
The dispatcher in `install_hooks_cmd.rs:108-123` already routes
`MergeError::Parse` to a clean stderr-with-hint exit; route `CstShape`
the same way.

---

### WR-03 | `string_property` uses `trim_matches('"')` instead of decoding the JSON string literal

**File:** `crates/holt-cli/src/install_hooks/merge.rs:212-220`
**Constraint:** D-08 idempotency (byte-equal re-merge)
**Issue:** `raw_value()` returns the source bytes verbatim (including
surrounding `"` quotes and any escape sequences). `trim_matches('"')`
strips the outer quotes but leaves escape sequences in their raw form.
For the static canonical commands (`"holt hook PreToolUse"` etc.) there
are no escapes, so the comparison `string_property(...) == Some(expected_command)`
in `is_canonical_entry` works fine on re-merge of holt's own output.

But the same function is also called via `element_command_contains` to
scan USER-AUTHORED entries (line 175). If a user wrote
`"command": "holt hook PreToolUse  trailing"` (a contrived but valid
JSON example), `raw_value()` returns the literal `"holt hook PreToolUse  trailing"`
including the backslash. The `.contains("holt hook ")` substring check
on that raw string still matches (the substring is present byte-for-byte),
so D-10 detection still fires correctly. But `is_canonical_entry` would
then compare the same raw-with-escapes string against the decoded
`"holt hook PreToolUse"` and report non-canonical — meaning even an
otherwise-canonical entry whose `command` happens to use an escape
sequence (e.g., `"holt hook PreToolUse"` written as `"holt hook PreToolUse"`)
would be `replace_with`-ed on every install-hooks run, breaking byte-equal
idempotency for that user.

Probability of this occurring in practice: very low — no user writes
unicode-escaped ASCII in `settings.json`. But the comment on `string_property`
asserts "the canonical command/matcher strings have no escapes and no
embedded quotes, so trim_matches('"') is faithful here," which is true
only for holt's own output, not for the user inputs the same function
also reads.

**Fix:** Use jsonc-parser's decoded-value accessor if one exists, or
parse the raw string manually:

```rust
// Prefer jsonc-parser's own decoder; fall back to a small JSON-string
// decoder if the surface is unstable. Inspect ~/.cargo/registry/.../jsonc-parser-0.26.3/src/cst/mod.rs
// for `StringLit::decoded_value()` or similar.
fn string_property(obj: &CstObject, name: &str) -> Option<String> {
    let prop = obj.get(name)?;
    let value = prop.value()?;
    let CstNode::Leaf(CstLeafNode::StringLit(s)) = value else {
        return None;
    };
    // jsonc-parser 0.26.x exposes `value()` on StringLit returning the decoded form;
    // verify the exact method name on the version pinned in Cargo.toml.
    s.value().ok().map(|v| v.to_string())
}
```
If no decoded accessor exists, hand-roll JSON-string unescape (control
the canonical set of escape pairs: `\"`, `\\`, `\/`, `\b`, `\f`, `\n`,
`\r`, `\t`, `\uXXXX`). At minimum, document the limitation in the
`string_property` doc comment and add a fixture covering an escape-laden
user entry to lock the chosen behavior.

---

### WR-04 | `--dry-run` reads settings.json without holding the fs2 lock

**File:** `crates/holt-cli/src/install_hooks_cmd.rs:58-78`
**Constraint:** C3 (settings.json mutation hygiene; lock window covers the
read-merge-write sequence)
**Issue:** The dry-run path uses `std::fs::read_to_string(&path)` directly,
no lock. If a concurrent `holt install-hooks` (default mode) is mid-write,
the dry-run reader observes whichever inode is currently published —
either pre-merge or post-merge bytes (atomic_write guarantees no torn
state). The diff then shows a change set that is either correct or
already-applied; no data corruption results.

This is not a data-loss bug. But the dry-run output can mislead a user
who runs `--dry-run` while another holt invocation is concurrently
mutating: they could see a diff that shows pending changes which have
already been applied by the time they look at stderr. The 200ms lock
budget makes this extremely narrow in practice (sub-second), but the
deeper implication is that the displayed diff is not bound to the state
that a follow-up default-mode invocation will see.

**Fix:** Acquire the lock for `--dry-run` too, treating dry-run as a
read-only operation that still wants serialised consistency. Cost: 200ms
lock-budget added to dry-run wall clock. Alternative: document in the
`--dry-run` help text + module doc that dry-run is a read-only snapshot
without exclusive access, so users running install-hooks concurrently
should not rely on the displayed diff being bound to the next default-mode
write. The latter is cheaper and matches the typical mental model of
"diff is a preview"; if you adopt it, mention it in `cli.rs:73` doc
comment as well.

---

### WR-05 | Lock release on panic depends solely on stack unwinding

**File:** `crates/holt-cli/src/install_hooks_cmd.rs` (entire function `run`)
**Constraint:** C3 (lock contract)
**Issue:** The fs2 exclusive lock is released on `Drop` of the `File`
handle (POSIX `flock(2)` / Windows `LockFileEx`). The dispatcher binds
the handle to `lock_handle` and calls `drop(lock_handle)` explicitly on
every return path — good. But if any code between lock-acquire and
explicit drop panics (e.g., the `WR-02` `expect` fires, or a future
refactor introduces `unwrap` somewhere in the merge or commit), unwinding
would still drop the handle and release the lock — IF the binary is
compiled with `panic = unwind`. The workspace `Cargo.toml:78` sets
`panic = "abort"` for the release profile.

Under `panic = abort`, a panic in `merge_settings` or `commit` does NOT
unwind; the process aborts immediately. POSIX `flock(2)` locks are
released on file descriptor close, which the kernel does on process
exit, so the lock is still released (no permanent leak). On Windows,
LockFileEx behavior on abnormal termination is well-defined — the kernel
releases the lock when the process handle closes.

So technically: no permanent lock leak under `panic = abort` even on
panic, because the OS reaps file handles. But: if `panic = abort` is
ever changed to `panic = unwind` AND a panic happens between lock-acquire
and `drop(lock_handle)`, the unwind path is the only release; if a
catch-the-unwind harness ever wraps `run()` (e.g., for a future
JSON-output mode), the lock could be held longer than expected. This is
a forward-looking concern — not a bug today.

**Fix:** Document the panic-release contract in the lock.rs module doc:

```rust
//! Release semantics: the lock is released on `Drop` of the returned
//! `File`. Under workspace's `panic = abort` (Cargo.toml [profile.release]),
//! a panic in the read-merge-write window aborts the process immediately
//! and the OS reaps the file descriptor (and the lock) on exit. If the
//! profile is ever changed to `panic = unwind`, callers must ensure the
//! `File` handle is `drop`'d on every path — the existing dispatcher in
//! `install_hooks_cmd.rs` does this explicitly via `drop(lock_handle)` at
//! every return.
```

No code change required if the fix is doc-only.

---

### WR-06 | SIGKILL test does not assert clean tmp-file state at the end

**File:** `crates/holt-cli/tests/install_hooks_sigkill.rs:96-101, 153-159`
**Constraint:** C3 atomic-write contract (`atomic_write` cleans up tmp files
on every error path)
**Issue:** Each iteration removes leftover `.holt-tmp.*` files at the
*start* of the iteration (lines 97-101) so they don't accumulate. But
the test never asserts at the end of the loop that `.holt-tmp.*` are
actually absent — it only relies on the per-iteration cleanup masking
any leak. If `atomic_write`'s tmp-cleanup ever regresses (e.g., a future
change drops the `let _ = fs::remove_file(&tmp);` on the error path in
`crates/holt-schemas/src/writer.rs:55`), this test would still pass:
each iteration's leftovers are wiped at the next iteration's start, and
the final state is whatever the 200th iteration left behind, which is
bounded by ~30ms of post-rename time and so almost always clean.

The 50× concurrent test (`install_hooks_concurrent.rs:128-138`) DOES
assert no orphans remain at the end — that's the correct shape. SIGKILL
should match.

**Fix:** Add a final orphan check after the iteration loop ends:

```rust
// (d) After 200 SIGKILL iterations, no orphan .holt-tmp.<pid> files
// should remain. The atomic_write cleanup must hold up under SIGKILL
// (where the cleanup closure cannot run in the killed child — but the
// renamed file becomes the new settings.json or the tmp is reaped at
// process death by the kernel? No — tmp files survive process death.
// The contract is that holt's *next* invocation sees no orphans.)
let orphans: Vec<_> = fs::read_dir(&claude)
    .unwrap()
    .filter_map(|e| e.ok())
    .filter(|e| e.file_name().to_string_lossy().contains(".holt-tmp."))
    .collect();
assert!(
    orphans.is_empty(),
    "orphan .holt-tmp.* files after 200x SIGKILL: {:?}",
    orphans.iter().map(|e| e.file_name()).collect::<Vec<_>>()
);
```
Note: SIGKILL between fopen-tmp and rename will leave a `.holt-tmp.<pid>`
file the killed child cannot clean up. So this test, if added, would
likely expose that orphan tmp files DO accumulate under SIGKILL — which
is the realistic situation, and either (a) the test should accept that
and only verify the FINAL `settings.json` is parseable, or (b) the
production code should add a "clean up stale `.holt-tmp.*` files at the
start of `holt install-hooks`" sweep so subsequent invocations recover.
Recommend (b) as a hardening item; for the test, at minimum document the
expectation.

---

## Info

### IN-01 | `MergeError::NotAnObject { got }` field is always the same string

**File:** `crates/holt-cli/src/install_hooks/merge.rs:46-49, 71-73`
**Issue:** `got: &'static str` is named like a discriminator (e.g., would
distinguish "array" / "string" / "boolean" roots), but the call site
unconditionally passes `"non-object root"`. The field adds no information.

**Fix:** Either populate `got` with the actual variant name from
`CstNode` (`"array"`, `"string"`, etc.) when downcasting fails, or drop
the field and let the error message stand alone:

```rust
#[error("settings.json root is not a JSON object")]
NotAnObject,
```

---

### IN-02 | `pretty_snippet` does not escape `command` strings for JSON safety

**File:** `crates/holt-cli/src/install_hooks/print.rs:21`
**Issue:**
```rust
out.push_str(&format!("          \"command\": \"{}\"\n", e.command));
```
`e.command` comes from the static `HOLT_HOOK_ENTRIES` table, where every
value is a known-safe ASCII string with no quotes / backslashes / control
chars. Today this is fine. But the contract on `pretty_snippet(entries: &[HoltHookEntry])`
takes a slice — a future caller passing user-derived entries (e.g., a
plan that lets users register custom hook commands) would generate
invalid JSON if any command contains `"` or `\`.

**Fix:** Either constrain the function signature to only the canonical
const (`pretty_snippet()` with no parameter, hard-coded to `HOLT_HOOK_ENTRIES`),
or escape the command string. For now, a comment + a debug_assert is the
cheapest guard:

```rust
debug_assert!(
    !e.command.contains('"') && !e.command.contains('\\'),
    "pretty_snippet does not escape; command must be JSON-safe ASCII"
);
out.push_str(&format!("          \"command\": \"{}\"\n", e.command));
```

---

### IN-03 | `extract_quoted_keys` test helper misclassifies values-followed-by-colon

**File:** `crates/holt-cli/tests/install_hooks_smoke.rs:115-149`
**Issue:** `extract_quoted_keys` walks bytes and treats any `"..."` token
followed by whitespace and `:` as a key. In settings.json that's correct
for keys, but a string VALUE positioned just before another property's
key (rare in practice given JSON structure but possible in malformed
inputs) would be misclassified. The test is robust against the current
fixture corpus; if a future fixture adds a string value containing a
colon, the helper could over-count.

**Fix:** Document the helper's limited scope in a comment:

```rust
// Loose key extraction sufficient for monotonicity check on small
// JSONC fixtures. NOT a general JSON tokenizer — does not track
// array/object nesting and may misclassify string values that happen
// to be followed by `:` in malformed inputs.
fn extract_quoted_keys(s: &str) -> Vec<String> {
```

Or harden it with a bracket-depth counter so it only emits inside-an-object
quoted tokens. The test passes today either way.

---

### IN-04 | `tests/cli_dep_boundary.rs::parse_package_name` duplicated from `tests/architecture_dag.rs`

**File:** `tests/cli_dep_boundary.rs:22-40`
**Issue:** The function is verbatim-copied from `tests/architecture_dag.rs`
with a comment justifying the duplication (kept independent so a refactor
of one helper module doesn't invalidate either test). Defensible — both
tests are CI-critical and should not share a test-helper module that
could regress silently. But the comment is correct that this is a
deliberate trade-off, not a bug.

**Fix:** None required. If the duplication ever drifts (e.g., one parser
gets a fix the other lacks), reconcile then.

---

### IN-05 | `pseudo_random_delay_ms` returns `0..30` (mod 31) — comment says "0..30ms"

**File:** `crates/holt-cli/tests/install_hooks_sigkill.rs:43-50`
**Issue:** Comment says `// xorshift PRNG — sufficient randomness for
0..30ms scatter` and the function returns `x % 31`. `% 31` yields
`0..=30` inclusive, which matches "0..30ms". `0` means no sleep —
SIGKILL fires before the child has done anything, which is a valid
test point but means a meaningful fraction of iterations test the
"killed before any work" case rather than the "killed mid-write" case
the test is actually trying to exercise. With 200 iterations, ~6 will
land on 0ms.

**Fix:** Optional. Bias the distribution to `1..=30` so every iteration
exercises some racing window:

```rust
1 + (x % 30)
```

Not strictly required — SIGKILL-at-zero-delay still tests the contract
that `settings.json` is observably the pre-merge state, which the
existing assertion already covers.

---

_Reviewed: 2026-04-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
