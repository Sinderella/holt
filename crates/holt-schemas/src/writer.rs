//! Atomic write helper (D-07 / H2 mitigation).
//!
//! Same-directory tmp file with PID suffix, fsync(2) on the temp fd, then rename(2).
//! POSIX rename is atomic only within one filesystem; same-dir tmp avoids EXDEV.
//! ext4 with `data=writeback` requires the fsync to close the delayed-allocation
//! window (LWN /Articles/789600/).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "atomic_write: target has no parent directory",
        )
    })?;

    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "atomic_write: target is a directory or has no file name",
        )
    })?;

    // Tmp file lives in the SAME DIRECTORY as target — avoids EXDEV (cross-mount rename).
    let pid = std::process::id();
    let tmp = dir.join(format!("{}.holt-tmp.{pid}", file_name.to_string_lossy()));

    // 0600 perms on Unix — heartbeat / LKG / breach logs are user-private.
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    // CR-05: clean up the tmp file on EVERY error path, not just rename
    // failure. If write_all/sync_all errored out (EIO, ENOSPC, EAGAIN under
    // load), the tmp file would otherwise linger and the next call from the
    // same PID would hit EEXIST on `create_new(true)` and stay broken until
    // the user manually cleared `*.holt-tmp.<pid>`. The closure isolates the
    // fallible steps so the cleanup runs unconditionally on `Err`.
    let result = (|| -> io::Result<()> {
        let mut f = opts.open(&tmp)?;
        f.write_all(contents)?;
        f.sync_all()?; // fsync(2) BEFORE rename — closes ext4 delayed-alloc window
        drop(f);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }

    // (We deliberately do NOT fsync the directory at v0.1. Heartbeat / LKG are ephemeral;
    // a power-loss between rename and dirent flush is recoverable next fire.
    // Trigger to add directory fsync: ≥1 corrupted-on-power-loss report.)
    result
}
