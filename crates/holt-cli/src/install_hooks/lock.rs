//! Filled in Task 3 (fs2 exclusive lock with 200ms try-loop, D-04).

use std::path::Path;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error(
        "could not acquire exclusive lock on {path} after {budget_ms}ms: another holt install-hooks is running (or settings.json is locked by another editor)"
    )]
    Timeout { path: String, budget_ms: u64 },
    #[error("io error opening {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

pub fn acquire_settings_lock(_path: &Path) -> Result<std::fs::File, LockError> {
    unimplemented!("Task 3");
}

// keep import live until Task 3 fills the loop
const _: Duration = Duration::from_millis(50);
