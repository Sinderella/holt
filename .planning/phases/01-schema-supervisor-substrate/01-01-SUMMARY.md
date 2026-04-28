---
phase: 01-schema-supervisor-substrate
plan: 01
plan_id: 01-01
subsystem: holt-schemas / workspace foundation
tags: [keystone, workspace, schema, atomic-write, reader-contract, hook-11]
requires: []
provides:
  - holt-schemas::Heartbeat
  - holt-schemas::LkgEntry
  - holt-schemas::ReaderError
  - holt-schemas::read_heartbeat
  - holt-schemas::atomic_write
  - workspace skeleton (six crates)
  - tuned [profile.release] (D-04)
affects:
  - "All future Phase 1 / Phase 2 / v1.0 crates depend on holt-schemas."
  - "Plan 02 (holt-supervisor) consumes atomic_write + LkgEntry."
  - "Plan 03 (holt-cli + tests/architecture_dag.rs) references all six crates."
tech-stack:
  added:
    - "rust 1.87 / edition 2024 (rust-toolchain.toml + workspace.package)"
    - "process-wrap = \"=9.1.0\" (workspace.dependencies; consumed in plan 02)"
    - "serde 1 + serde_json 1 (defensive parse posture for Heartbeat)"
    - "jiff 0.2 (timestamps; replaces chrono per D-02)"
    - "thiserror 1 (only at internal lib boundaries per D-03)"
    - "tempfile 3 (dev-only; reader & atomic-write integration tests)"
  patterns:
    - "Defensive serde: schema_version first, #[serde(default)], #[non_exhaustive], NO deny_unknown_fields."
    - "Hand-rolled atomic_write: same-dir tmp + PID suffix + fsync(2) + rename(2). Audited fallback (atomic-write-file crate) deferred."
    - "C5 contract: read_heartbeat returns Ok(None) for missing file / zero-byte / truncated JSON / unrecognized schema_version / missing required field. Never panics."
    - "#![forbid(unsafe_code)] at holt-schemas crate root."
key-files:
  created:
    - "Cargo.toml (workspace manifest, six members, MSRV 1.87 / Edition 2024, tuned release profile)"
    - "Cargo.lock (binary workspace — committed for reproducibility)"
    - "rust-toolchain.toml (channel = \"1.87\", rustfmt + clippy components)"
    - ".gitignore (/target, Cargo.lock.bak, *.holt-tmp.*)"
    - "crates/holt-schemas/Cargo.toml"
    - "crates/holt-schemas/src/lib.rs"
    - "crates/holt-schemas/src/heartbeat.rs"
    - "crates/holt-schemas/src/lkg.rs"
    - "crates/holt-schemas/src/error.rs"
    - "crates/holt-schemas/src/reader.rs"
    - "crates/holt-schemas/src/writer.rs"
    - "crates/holt-schemas/tests/reader_contract.rs (7 tests)"
    - "crates/holt-schemas/tests/atomic_write_smoke.rs (5 tests)"
    - "crates/holt-supervisor/Cargo.toml + src/lib.rs (placeholder)"
    - "crates/holt-hooks/Cargo.toml + src/lib.rs (placeholder)"
    - "crates/holt-orchestrator/Cargo.toml + src/lib.rs (placeholder)"
    - "crates/holt-render/Cargo.toml + src/lib.rs (placeholder; NO holt-supervisor edge per C2)"
    - "crates/holt-cli/Cargo.toml + src/main.rs (binary stub)"
  modified: []
decisions:
  - "D-01: Single Cargo workspace at repo root, all six members declared from day one (so C2 has packages to enforce against)."
  - "D-02: jiff used for timestamps; chrono explicitly forbidden."
  - "D-03: thiserror at internal lib boundary (ReaderError); anyhow reserved for plan 02 / 03 binary surfaces."
  - "D-04: [profile.release] = lto thin + codegen-units 1 + strip symbols + panic abort. Locked in workspace Cargo.toml."
  - "D-05: Heartbeat uses #[serde(default)] on optional fields, schema_version first, NO deny_unknown_fields. session_id is the only required non-version field."
  - "D-06: read_heartbeat C5 contract — Ok(None) for 5 corruption modes; Err only for non-NotFound I/O; never panics, never .unwrap()s."
  - "D-07: Hand-rolled atomic_write — same-dir tmp + PID suffix + fsync + rename. Unix 0600 perms via OpenOptionsExt. Directory fsync deferred per RESEARCH Pattern 3."
  - "D-08: Heartbeat and LkgEntry marked #[non_exhaustive] for forward-compat schema bumps."
metrics:
  start: "2026-04-28T09:55:00Z"
  end: "2026-04-28T10:03:49Z"
  duration: "~9 minutes (executor wall-clock; cargo first-run pulled toolchain 1.87 + 57 crate downloads)"
  tasks_completed: 3
  files_created: 21
  files_modified: 0
  tests_added: 12
  tests_passing: 12
---

# Phase 1 Plan 01: Workspace + holt-schemas Keystone Summary

**One-liner:** Cargo workspace with six members (MSRV 1.87 / Edition 2024) + the keystone `holt-schemas` crate shipping the C5 non-panicking reader contract and a hand-rolled atomic_write that closes the ext4 delayed-allocation window — every other Phase 1 / Phase 2 / v1.0 crate now compiles against this surface.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Workspace root manifest, toolchain pin, .gitignore | `3c846a5` | `Cargo.toml`, `rust-toolchain.toml`, `.gitignore` |
| 2 | Six crate skeletons (5 placeholders + holt-schemas keystone) | `88f0e62` | 18 files across `crates/holt-{schemas,supervisor,hooks,orchestrator,render,cli}/` + `Cargo.lock` |
| 3 | reader_contract.rs (7 tests) + atomic_write_smoke.rs (5 tests) | `115e984` | `crates/holt-schemas/tests/{reader_contract,atomic_write_smoke}.rs` |

## Files Created

### Workspace root
- `Cargo.toml` — workspace manifest. Six members. `rust-version = "1.87"`, `edition = "2024"`, `resolver = "2"`. `workspace.dependencies` pin `process-wrap = "=9.1.0"` and provide `serde`, `serde_json`, `jiff`, `clap`, `anyhow`, `thiserror`, `humantime`, `nix`, `tempfile`. `[profile.release]` set to `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`, `panic = "abort"` per D-04 (load-bearing for the sub-20ms cold-start budget verified in plan 03).
- `Cargo.lock` — committed for binary-workspace reproducibility (the `holt-cli` crate has a `[[bin]]` target).
- `rust-toolchain.toml` — `channel = "1.87"`, `components = ["rustfmt", "clippy"]`, `profile = "minimal"`. Triggers `rustup` to install 1.87.0 on first cargo invocation.
- `.gitignore` — `/target`, `Cargo.lock.bak`, `*.holt-tmp.*` (the atomic-write tmp pattern, in case any test crashes mid-write).

### holt-schemas (keystone — full implementation)
- `crates/holt-schemas/Cargo.toml` — depends on `serde.workspace`, `serde_json.workspace`, `jiff.workspace`, `thiserror.workspace`. `[dev-dependencies]` `tempfile.workspace`.
- `crates/holt-schemas/src/lib.rs` — `#![forbid(unsafe_code)]`; declares 5 modules; re-exports `Heartbeat`, `LkgEntry`, `ReaderError`, `read_heartbeat`, `atomic_write`.
- `crates/holt-schemas/src/heartbeat.rs` — `pub struct Heartbeat` with `#[non_exhaustive]`, `schema_version: u8` first, `session_id: String` required, 12 other fields all `#[serde(default)]`. `pub const SCHEMA_VERSION: u8 = 1`.
- `crates/holt-schemas/src/lkg.rs` — `pub struct LkgEntry` with `#[non_exhaustive]`, schema_version-tagged; fields = `stdout`, `exit_code`, `captured_at` (ISO 8601), `duration_ms`.
- `crates/holt-schemas/src/error.rs` — `pub enum ReaderError` with single `Io(#[from] std::io::Error)` variant.
- `crates/holt-schemas/src/reader.rs` — `pub fn read_heartbeat(path: &Path) -> Result<Option<Heartbeat>, ReaderError>` implementing the four-step C5 contract: read → ENOENT short-circuit; empty-file short-circuit; serde_json parse → Ok(None) on any error; schema_version check → Ok(None) on mismatch.
- `crates/holt-schemas/src/writer.rs` — `pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()>` implementing same-dir tmp file with `.holt-tmp.<pid>` suffix, `OpenOptions::create_new(true).mode(0o600)` (Unix), `f.sync_all()` before rename, orphan-tmp cleanup on rename failure.

### holt-schemas tests
- `crates/holt-schemas/tests/reader_contract.rs` — 7 integration tests verifying the C5 contract: missing file, zero-byte, truncated JSON, unrecognized schema_version, missing required field, valid round-trip, arbitrary garbage bytes.
- `crates/holt-schemas/tests/atomic_write_smoke.rs` — 5 integration tests verifying D-07 invariants: contents written, no orphan tmp left, overwrites existing target, Unix 0600 perms (cfg-gated), error on path-without-parent.

### Placeholder crates (filled in by plans 02 / 03 / Phase 2 / v1.0)
- `crates/holt-supervisor/{Cargo.toml,src/lib.rs}` — depends on `holt-schemas`; stub `pub fn placeholder() {}`.
- `crates/holt-hooks/{Cargo.toml,src/lib.rs}` — depends on `holt-schemas`; stub.
- `crates/holt-orchestrator/{Cargo.toml,src/lib.rs}` — depends on `holt-schemas`; stub.
- `crates/holt-render/{Cargo.toml,src/lib.rs}` — depends on `holt-schemas` + `holt-orchestrator` only. Cargo.toml carries an inline comment forbidding the `holt-supervisor` edge (C2 — to be CI-enforced by `tests/architecture_dag.rs` in plan 03).
- `crates/holt-cli/{Cargo.toml,src/main.rs}` — single `[[bin]] name = "holt"`; depends on every workspace crate; current `main()` is a stub returning exit 2 (plan 03 wires the clap derive subcommands).

## Test Results

```
$ cargo test -p holt-schemas
running 0 tests          # unit tests
test result: ok. 0 passed; 0 failed; 0 ignored

running 5 tests          # atomic_write_smoke
test errors_on_invalid_target_no_parent ... ok
test unix_perms_are_0600 ... ok
test writes_target_file_with_expected_contents ... ok
test leaves_no_orphan_tmp_file_on_success ... ok
test overwrites_existing_target ... ok
test result: ok. 5 passed; 0 failed; 0 ignored

running 7 tests          # reader_contract
test returns_ok_none_for_missing_file ... ok
test returns_ok_none_for_zero_byte_file ... ok
test returns_ok_none_for_unrecognized_schema_version ... ok
test returns_ok_some_for_valid_heartbeat ... ok
test does_not_panic_on_arbitrary_bytes ... ok
test returns_ok_none_for_truncated_json ... ok
test returns_ok_none_for_missing_required_fields ... ok
test result: ok. 7 passed; 0 failed; 0 ignored
```

**12/12 tests pass.** `cargo build --workspace --release`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` all exit 0. `cargo tree -i tokio` returns "no matches" — confirming no transitive async runtime per CLAUDE.md.

## Decisions Implemented

| ID | Decision | Where it landed |
|----|----------|-----------------|
| D-01 | Single workspace at repo root with six members | `Cargo.toml [workspace] members` |
| D-02 | jiff for timestamps (no chrono) | `workspace.dependencies` (jiff = "0.2"); LkgEntry.captured_at typed as ISO 8601 String |
| D-03 | thiserror at internal lib boundaries; anyhow reserved for binary surfaces | `holt-schemas/src/error.rs` uses thiserror; anyhow not yet pulled |
| D-04 | Tuned release profile | `Cargo.toml [profile.release]` |
| D-05 | Defensive serde on Heartbeat (schema_version first, #[serde(default)] on optionals, NO deny_unknown_fields) | `holt-schemas/src/heartbeat.rs` |
| D-06 | C5 read_heartbeat contract: Ok(None) for 5 corruption modes; never panics | `holt-schemas/src/reader.rs` + 7 contract tests |
| D-07 | Hand-rolled atomic_write — same-dir tmp + PID suffix + fsync + rename + 0600 perms | `holt-schemas/src/writer.rs` + 5 smoke tests |
| D-08 | Heartbeat and LkgEntry marked #[non_exhaustive] | `heartbeat.rs` + `lkg.rs` |

## Hygiene Verification (the load-bearing constraints)

- `grep -n '\.unwrap()' crates/holt-schemas/src/` — single match on `reader.rs:11`, which is a doc-comment line forbidding it (`//! Never panics. Never .unwrap()s. Never .expect()s.`). Zero `.unwrap()` calls in actual code.
- `grep -n 'panic!' crates/holt-schemas/src/` — zero matches.
- `grep -n 'deny_unknown_fields' crates/holt-schemas/src/heartbeat.rs` — single match on line 4, a doc-comment forbidding it. Zero attribute uses.
- `grep -n 'holt-supervisor' crates/holt-render/Cargo.toml` — single match on a `# NOTE:` comment forbidding the edge. No active dep.
- `cargo tree -i tokio` — `error: package ID specification 'tokio' did not match any packages` (no transitive async runtime).
- `cargo metadata --format-version 1 | … startswith('holt-')` — six packages.

## Patterns Established (consumed by plan 02 / 03 / Phase 2)

1. **`atomic_write` available to every crate via `holt-schemas::atomic_write`.** Plan 02's LKG cache writes go through it. Phase 2's heartbeat writes go through it.
2. **`LkgEntry` schema locked.** Plan 02 instantiates it in the supervisor; the render path reads only the `stdout` field on cache hit (D-10 / C6 — no breaches.log or timings.jsonl reads on the render path).
3. **C5 reader posture.** Any future reader on the render path must follow the same Ok(None)-on-corruption posture; the failing-test surface in `reader_contract.rs` is the regression net.
4. **`#![forbid(unsafe_code)]` at holt-schemas root.** If `nix` or `libc` are needed at the spawn site, that `unsafe` lives in `holt-supervisor` (plan 02), never in the keystone.
5. **No-tokio invariant.** `cargo tree -i tokio` is the CI gate plan 03 should add.

## Deviations from Plan

**None — plan executed exactly as written.**

Two minor positive notes (not deviations, just things the executor noticed and the plan's verbatim contents already addressed):

- The plan's `verify` block instructed `grep -c '#\[serde(default)\]' crates/holt-schemas/src/heartbeat.rs` returns at least 12. Actual count is **14** (every Option<…> field plus `pid`, `started`, `updated`, `cwd`, `cwd_label`, `writer_version`). Above the floor.
- The plan's verify expected `grep -c 'deny_unknown_fields'` and `grep -c '\.unwrap()'` to return 0. Both return 1 — but in **doc-comments forbidding the patterns**, not in active code. Confirmed by `grep -n` on each path; not a violation. (If the orchestrator's verifier prefers literally zero matches even for forbidden-pattern callouts, those doc-comment lines could be re-phrased; current wording is more useful for future readers.)

## Notes / Gotchas

- **Toolchain auto-install:** the host machine had nightly + 1.54 + 1.76 + stable but no 1.87. The `rust-toolchain.toml` triggered `rustup` to download and install 1.87.0 (May 2025) on first `cargo build`. CI hosts will see the same one-shot pull on first run.
- **`thiserror v1` not v2:** workspace pin is `thiserror = "1"`. Cargo flagged `available: v2.0.18` during the resolution. The major bump should be considered after Phase 1 lands; v1 is sufficient for our `#[from] std::io::Error` usage and matches RESEARCH §"Standard Stack".
- **`Cargo.lock` committed** because `holt-cli` is a binary target. Subsequent plans should not delete it.
- **`tempfile` is workspace-dev-only.** It does not appear in any `[dependencies]` block of any crate — only in `holt-schemas/[dev-dependencies]`. This keeps the release-build dep tree minimal and is what `cargo tree` reports above.
- **Doc-comment matches for forbidden patterns are intentional.** Strings like `Never .unwrap()s` and `NO deny_unknown_fields` and `# NOTE: adding holt-supervisor here MUST fail …` are part of the guard rail — they make the constraint visible at the call site to a future contributor reading the file. If the verifier's grep is too coarse, prefer an AST-based check (`cargo expand` or a clippy lint via `clippy::unwrap_used`/`clippy::panic`) over rephrasing the doc-comments.

## Follow-ups (for plan 02 / plan 03 / future)

- **Plan 02 (holt-supervisor):** consume `holt_schemas::atomic_write` for the LKG cache writer; consume `holt_schemas::LkgEntry` for the cache record; pin `process-wrap = "=9.1.0"` from workspace.dependencies; ensure C1 (`Stdio::piped()` × 3 before `wrap(ProcessGroup::leader())`) at the single chokepoint.
- **Plan 03 (holt-cli + tests/architecture_dag.rs + CI):** add `tests/architecture_dag.rs` walking `cargo metadata` to assert no path from `holt-render` → `holt-supervisor` (C2). Add `cargo tree -i tokio` as a CI step (no-tokio invariant). Add `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` as required gates per D-16.
- **Phase 2 (HOOK-06 etc.):** `holt-hooks` writes Heartbeat instances via `atomic_write` to `$XDG_RUNTIME_DIR/holt/sessions/<sid>.json` (Linux) or `$TMPDIR/holt-$UID/sessions/<sid>.json` (macOS) with `~/.cache/holt/sessions/` fallback. The `writer_version` field on Heartbeat is reserved for that crate's identification.
- **Future trigger:** if ≥1 corrupted-on-power-loss heartbeat report arrives post-launch, add directory fsync to `atomic_write`. If ≥2 corruption reports, swap to `atomic-write-file` crate (audited fallback per STACK.md §3).

## Self-Check: PASSED

Files claimed → verified:

- `Cargo.toml`: FOUND
- `rust-toolchain.toml`: FOUND
- `.gitignore`: FOUND
- `crates/holt-schemas/src/{lib,heartbeat,lkg,error,reader,writer}.rs`: ALL FOUND (6/6)
- `crates/holt-schemas/tests/{reader_contract,atomic_write_smoke}.rs`: BOTH FOUND
- `crates/holt-{supervisor,hooks,orchestrator,render}/src/lib.rs`: ALL FOUND (4/4)
- `crates/holt-cli/src/main.rs`: FOUND

Commits claimed → verified:
- `3c846a5` (task 1): FOUND in `git log`
- `88f0e62` (task 2): FOUND in `git log`
- `115e984` (task 3): FOUND in `git log`
