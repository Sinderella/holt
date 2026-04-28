//! ROADMAP success criterion #3 second clause + CORE-09 + C6:
//! Render path opens NEITHER `breaches.log` NOR `timings.jsonl` for reading.
//!
//! Strategy: Linux-only via strace -e openat,access. macOS / Windows: deferred to
//! a stub-replaced fs interface (RESEARCH §"Open Questions" — flagged for v0.5).

#[cfg(target_os = "linux")]
#[test]
fn render_path_does_not_open_observability_logs_for_reading() {
    use std::process::Command;
    use tempfile::tempdir;

    let strace_check = Command::new("which").arg("strace").output();
    if !matches!(strace_check, Ok(o) if o.status.success()) {
        eprintln!("strace not available — skipping render_path_no_read");
        return;
    }

    let exe = env!("CARGO_BIN_EXE_holt");
    let cache = tempdir().unwrap();
    let trace_path = cache.path().join("holt-strace.txt");

    // First fire: build up some telemetry so the files definitely exist on disk.
    for _ in 0..3 {
        let _ = Command::new(exe)
            .args(["run", "--", "bash", "-c", "echo prime"])
            .env("XDG_CACHE_HOME", cache.path())
            .output();
    }

    // Now run under strace and capture openat + access.
    let out = Command::new("strace")
        .args([
            "-f",
            "-e",
            "trace=openat,access",
            "-o",
            trace_path.to_str().unwrap(),
        ])
        .arg(exe)
        .args(["run", "--", "bash", "-c", "echo hello"])
        .env("XDG_CACHE_HOME", cache.path())
        .output()
        .expect("strace must run");

    assert!(out.status.success() || out.status.code() == Some(0));

    let trace = std::fs::read_to_string(&trace_path).unwrap_or_default();
    for line in trace.lines() {
        // O_RDONLY / O_RDWR are reads; O_WRONLY / O_APPEND are writes.
        if line.contains("breaches.log") && (line.contains("O_RDONLY") || line.contains("O_RDWR")) {
            panic!(
                "C6 VIOLATED: render path opened breaches.log for reading.\n  trace line: {line}"
            );
        }
        if line.contains("timings.jsonl") && (line.contains("O_RDONLY") || line.contains("O_RDWR"))
        {
            panic!(
                "C6 VIOLATED: render path opened timings.jsonl for reading.\n  trace line: {line}"
            );
        }
    }
}

#[cfg(not(target_os = "linux"))]
#[test]
fn render_path_does_not_open_observability_logs_for_reading() {
    // macOS / Windows: deferred. Tracked in STATE.md as a follow-up — the Linux
    // path is the v0.1 enforcement boundary; the success-criterion language
    // explicitly names strace.
}
