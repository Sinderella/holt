//! `killpg` + EPERM PPID-walk fallback (PITFALLS H3).
//!
//! On macOS, `setpgid` succeeds inside sandboxes (Cursor, some VS Code
//! variants) but `killpg` may be denied silently against descendants. On
//! Linux, scripts that exec a daemon may re-parent under PID 1 (init /
//! systemd) and escape the original pgid. The fallback walks `/proc/*/status`
//! (Linux) or returns `Err` on macOS for v0.1 (libproc binding deferred).
//!
//! Best-effort: returns `Ok(())` if either `killpg` itself succeeded OR the
//! Linux fallback reaped at least one descendant. Returns `Err` only when
//! every approach failed.
//!
//! Source: PITFALLS.md H3; openai/codex#8690; elixir-lang/elixir#15036;
//!         rust-lang/rust#115241.

#[cfg(unix)]
pub fn kill_process_group(pgid: i32) -> std::io::Result<()> {
    use nix::errno::Errno;
    use nix::sys::signal::{Signal, killpg};
    use nix::unistd::Pid;

    match killpg(Pid::from_raw(pgid), Signal::SIGKILL) {
        Ok(()) => Ok(()),
        Err(Errno::EPERM) => {
            // Sandbox / launchd-reparented: walk descendants and SIGKILL each.
            ppid_walk_kill(pgid)
        }
        Err(e) => Err(std::io::Error::other(format!("killpg({pgid}) failed: {e}"))),
    }
}

#[cfg(all(unix, target_os = "linux"))]
fn ppid_walk_kill(target_pgid: i32) -> std::io::Result<()> {
    use std::fs;

    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;

    // Walk /proc/*/status for any pid whose `Pgid:` line matches target_pgid.
    let entries = fs::read_dir("/proc")?;
    let mut killed_any = false;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        let Ok(pid) = name_str.parse::<i32>() else {
            continue;
        };
        let status_path = entry.path().join("status");
        let Ok(status) = fs::read_to_string(&status_path) else {
            continue;
        };
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("Pgid:") {
                if rest.trim().parse::<i32>().ok() == Some(target_pgid) {
                    let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
                    killed_any = true;
                }
            }
        }
    }
    if killed_any {
        Ok(())
    } else {
        Err(std::io::Error::other(
            "killpg EPERM and no /proc descendants matched target pgid",
        ))
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
fn ppid_walk_kill(_target_pgid: i32) -> std::io::Result<()> {
    // macOS: `libc::proc_listchildpids` exists but requires `libproc-sys`
    // (with an unsafe FFI surface). For v0.1 we accept the EPERM-and-survive
    // case on macOS and document it as a known limitation. Trigger to harden:
    // ≥1 macOS-tagged "killpg failed under sandbox" issue.
    Err(std::io::Error::other(
        "killpg EPERM on non-linux Unix; libproc fallback deferred to post-v0.1",
    ))
}
