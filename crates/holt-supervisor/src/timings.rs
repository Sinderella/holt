//! `timings.jsonl` writer with 5MB / `.1` rotation (D-12).
//!
//! Rotation happens INSIDE the writer at write boundary — the render path
//! never reads this file (C6). Append-and-lose-on-crash is the right tradeoff
//! for telemetry: deliberately no `fsync` per line.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use crate::paths::timings_path;

/// 5MB cap per D-12.
pub const MAX_BYTES: u64 = 5 * 1024 * 1024;

/// Append one JSONL line (caller must include trailing `\n`) to
/// `<cache_root>/timings.jsonl`. Rotates the current file to `.1` (overwriting
/// any existing `.1`) when the next append would push size past 5MB.
pub fn append_timings(cache_root: &Path, line: &str) -> io::Result<()> {
    let path = timings_path(cache_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    append_jsonl(&path, line)
}

/// Internal JSONL appender shared by [`append_timings`] and the breach writer.
///
/// On 5MB overflow, renames `<file>` → `<file>.1` (overwriting any existing
/// `.1`), then opens the target fresh. Rotation is best-effort: if the rename
/// fails (e.g., target read-only), the writer still appends — the cap is a
/// soft policy, not a hard guarantee.
///
/// **Known limitation (WR-03 — accepted at v0.1):** when two holt processes
/// (two different CC sessions on the same machine, the explicit multi-session
/// use case) both observe `size > MAX_BYTES` and both rename `<file>` →
/// `<file>.1`, the second clobbers the first's `.1` (lost log data) and the
/// second's about-to-write line may land in the renamed file. On Linux this
/// is benign-ish; on Windows the rename can fail because the file is open
/// for append by the other process. We accept this tradeoff because:
///   1. The render path runs N times per session. Worst-case data loss on a
///      `.1` collision is bounded — the live file always carries the most
///      recent ~5MB and only `holt doctor` (v0.5) reads `.1` for trends.
///   2. The fix is an advisory `flock` on a sibling `.lock` file, which would
///      need a `fs2`-equivalent dep — and `fs2` is forbidden at v0.1
///      (CLAUDE.md `<forbidden_crates>`). Hand-rolling `flock(2)` would also
///      pull `unsafe` into a `#![forbid(unsafe_code)]` crate.
///   3. **Trigger to harden**: ≥1 user report of corrupted `.1` rotation, OR
///      Phase 4 introduces a hooks-managed lock-file convention we can reuse.
pub(crate) fn append_jsonl(path: &Path, line: &str) -> io::Result<()> {
    debug_assert!(line.ends_with('\n'), "caller must include trailing newline");

    if let Ok(meta) = fs::metadata(path) {
        if meta.len() + line.len() as u64 > MAX_BYTES {
            // Compose `<existing-extension>.1` so `timings.jsonl` rotates to
            // `timings.jsonl.1` and `breaches.log` rotates to `breaches.log.1`.
            let new_ext = match path.extension().and_then(|s| s.to_str()) {
                Some(ext) => format!("{ext}.1"),
                None => "1".into(),
            };
            let mut backup = path.to_path_buf();
            backup.set_extension(new_ext);
            // Best-effort: ignore rename failure — writer must not block on
            // rotation hiccups. Worst case: one oversized file lingers.
            let _ = fs::rename(path, &backup);
        }
    }

    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(line.as_bytes())?;
    // Deliberately NO fsync per line: append-and-lose-on-crash is acceptable
    // for observability output. D-07 fsync-before-rename applies only to LKG
    // and heartbeat (durability tier), not telemetry.
    Ok(())
}
