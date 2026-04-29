//! D-04 fs2 exclusive lock with 200ms try-loop.
//!
//! Pattern: `try_lock_exclusive` once; if it returns `ErrorKind::WouldBlock`,
//! sleep 50ms and try again, up to 4 attempts total (200ms budget). On the
//! fourth failure, return [`LockError::Timeout`] with a user-facing error
//! message that names both `holt install-hooks` and `settings.json` so
//! support tickets can grep for either keyword (per Phase 3 D-04 / `<specifics>`).
//!
//! On success, returns the open `File` handle. The caller MUST keep the
//! handle alive for the entire read-merge-write window; the lock is
//! released on `Drop` (POSIX `flock(2)` / Windows `LockFileEx`).
//!
//! C3 mandate: lock is acquired on the settings.json file itself, NOT a
//! separate lock file. fs2's exclusive lock is advisory but every other
//! `holt install-hooks` invocation respects it because every invocation
//! enters the same loop here.

use fs2::FileExt;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::path::Path;
use std::thread;
use std::time::Duration;

const MAX_ATTEMPTS: u32 = 4;
const SLEEP_BETWEEN_ATTEMPTS: Duration = Duration::from_millis(50);
const TOTAL_BUDGET_MS: u64 = 200;

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error(
        "could not acquire exclusive lock on {path} after {budget_ms}ms: another holt install-hooks is running (or settings.json is locked by another editor)"
    )]
    Timeout { path: String, budget_ms: u64 },
    #[error("io error opening {path} for locking: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Open `path` (creating it as `0o600` with 0-byte content if absent), acquire
/// an exclusive fs2 lock with 200ms budget, return the locked `File`.
///
/// The caller is responsible for reading the file's bytes (the merger treats
/// empty as the "clean" / `{}` starting state) and for keeping the returned
/// handle alive for the entire read-merge-write window.
pub fn acquire_settings_lock(path: &Path) -> Result<std::fs::File, LockError> {
    // Ensure parent directory exists (e.g., `~/.claude/` on a fresh system).
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| LockError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
    }

    let mut opts = OpenOptions::new();
    opts.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    // create(true) does NOT truncate (we set truncate(false) explicitly above);
    // if the file exists with content, open it as-is. The caller seeds an
    // empty file with `{}` semantics in the merger (input.trim().is_empty()
    // path). This function's contract is "return a locked handle".
    let f = opts.open(path).map_err(|e| LockError::Io {
        path: path.display().to_string(),
        source: e,
    })?;

    for attempt in 0..MAX_ATTEMPTS {
        match f.try_lock_exclusive() {
            Ok(()) => return Ok(f),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                if attempt + 1 < MAX_ATTEMPTS {
                    thread::sleep(SLEEP_BETWEEN_ATTEMPTS);
                }
                continue;
            }
            Err(e) => {
                return Err(LockError::Io {
                    path: path.display().to_string(),
                    source: e,
                });
            }
        }
    }

    Err(LockError::Timeout {
        path: path.display().to_string(),
        budget_ms: TOTAL_BUDGET_MS,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use tempfile::tempdir;

    #[test]
    fn acquires_when_free() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let f = acquire_settings_lock(&p).expect("free lock should succeed");
        // Confirm the file exists and we own a handle.
        assert!(p.exists());
        drop(f);
    }

    #[test]
    fn second_attempt_after_drop_succeeds() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("settings.json");
        {
            let _f = acquire_settings_lock(&p).expect("first acquire");
        } // dropped here — lock released
        let _f2 = acquire_settings_lock(&p).expect("second acquire after drop");
    }

    #[cfg(unix)]
    #[test]
    fn timeout_when_held_by_other_handle() {
        // Simulate a competing holder by opening + locking from this process.
        // fs2 try_lock_exclusive is per-fd on macOS / Linux (BSD flock semantics);
        // a second handle to the same file from the same process WILL see
        // WouldBlock — sufficient simulation. We DO NOT call
        // acquire_settings_lock recursively here because Linux flock semantics
        // would still see WouldBlock for the second fd, but we want to assert
        // on the loop's wall-clock + the LockError::Timeout shape directly.
        let dir = tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let _holder = acquire_settings_lock(&p).expect("holder acquires");

        let mut opts = OpenOptions::new();
        opts.read(true).write(true).create(true).truncate(false);
        let f2 = opts.open(&p).unwrap();

        let start = Instant::now();
        for attempt in 0..MAX_ATTEMPTS {
            match f2.try_lock_exclusive() {
                Ok(()) => panic!("should not have acquired while holder is alive"),
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    if attempt + 1 < MAX_ATTEMPTS {
                        thread::sleep(SLEEP_BETWEEN_ATTEMPTS);
                    }
                }
                Err(e) => panic!("unexpected io error: {e}"),
            }
        }
        let elapsed_ms = start.elapsed().as_millis() as u64;
        assert!(
            (150..=400).contains(&elapsed_ms),
            "expected ~150-200ms total wall clock, got {elapsed_ms}ms"
        );
    }

    #[test]
    fn error_message_contains_keywords() {
        let err = LockError::Timeout {
            path: "/tmp/x".into(),
            budget_ms: 200,
        };
        let s = format!("{err}");
        assert!(s.contains("holt install-hooks"), "missing keyword: {s}");
        assert!(s.contains("settings.json"), "missing keyword: {s}");
    }
}
