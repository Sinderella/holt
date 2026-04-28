//! D-09 chokepoint: [`wrap_and_run`] is the ONLY supervised-spawn site.
//!
//! C1 INVARIANT: stdin/stdout/stderr are piped BEFORE the process-group
//! wrapper is attached. The textual ordering matters for human auditing —
//! `tests/chokepoint_audit.rs` confirms exactly one wrap call site exists in
//! this crate's `src/`. Adding a second one anywhere is an audit hazard:
//! every supervised spawn must route through this function or break the test.
//!
//! Why piped × 3 first: inheriting the parent TTY in a backgrounded process
//! group causes the kernel to send `SIGTTIN`, which stops the child and looks
//! like a hang. Verified failure mode on macOS and Linux (openai/codex#8690,
//! elixir-lang/elixir#15036, cross-confirmed in PITFALLS.md H3).
//!
//! Why mpsc + thread for the deadline: `process-wrap` v9.1.0's `WrappedChild`
//! does not expose `wait_timeout`. RESEARCH §"Pitfall: wait_timeout is not on
//! process-wrap's WrappedChild" recommends `std::sync::mpsc` + a wait thread
//! over pulling in the `wait-timeout` crate (one fewer dep, equivalent
//! ergonomics).
//!
//! Source: docs.rs/process-wrap/9.1.0 (verified 2026-04-28).

use std::io::{Read, Write};
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use holt_schemas::LkgEntry;
use process_wrap::std::{CommandWrap, ProcessGroup};

use crate::breaches::append_breach;
use crate::lkg::write_lkg;
use crate::options::{BreachKind, SupervisorOptions, SupervisorOutcome};
use crate::timings::append_timings;

/// Marker zero-sized type so callers can write `Supervisor::wrap_and_run(...)`
/// when the namespacing reads more clearly than a free function. The struct
/// holds no state — the chokepoint is the function, not an instance.
pub struct Supervisor;

impl Supervisor {
    /// Convenience alias for [`wrap_and_run`].
    pub fn wrap_and_run(
        program: &str,
        args: &[&str],
        opts: SupervisorOptions,
    ) -> SupervisorOutcome {
        wrap_and_run(program, args, opts)
    }
}

/// Spawn `program args...` under process-group supervision with a hard deadline.
///
/// # Outcomes
/// * [`SupervisorOutcome::Ok`] — child exited within the deadline. Captured
///   stdout (UTF-8 lossy) is returned, exit code is preserved, and on `exit
///   == 0` the LKG cache is refreshed via `holt_schemas::atomic_write`.
/// * [`SupervisorOutcome::Breach`] — `Timeout` if the deadline elapsed before
///   the child exited (whole process group is `SIGKILL`'d via
///   [`crate::kill::kill_process_group`]); `SpawnFail` if `spawn()` or
///   `wait()` failed at the OS layer.
///
/// Every invocation appends one line to `timings.jsonl` (CORE-02). Every
/// breach also appends one line to `breaches.log` (CORE-06). All telemetry
/// failures are swallowed — render path must not block on disk hiccups.
pub fn wrap_and_run(program: &str, args: &[&str], opts: SupervisorOptions) -> SupervisorOutcome {
    let started = Instant::now();

    // Build the command. C1: stdin/stdout/stderr are piped INSIDE the closure
    // BEFORE we attach the process-group wrapper below. macOS SIGTTIN
    // avoidance — see module docs.
    let mut wrap = CommandWrap::with_new(program, |c| {
        for a in args {
            c.arg(a);
        }
        c.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    });

    // The ONE call site in the crate. Every supervised spawn must come through
    // this function (chokepoint_audit asserts the count is exactly 1).
    #[cfg(unix)]
    {
        wrap.wrap(ProcessGroup::leader());
    }
    #[cfg(windows)]
    {
        wrap.wrap(process_wrap::std::JobObject);
    }

    let mut child = match wrap.spawn() {
        Ok(c) => c,
        Err(e) => {
            return finalize_spawn_fail(&opts, started, &e.to_string());
        }
    };

    // Capture pgid (= child PID since we spawned as group leader) for the
    // kill-on-timeout fallback. `child` itself moves into the wait thread
    // below, so we have to read this before the move.
    #[cfg(unix)]
    let pgid: i32 = child.id() as i32;

    // Pump CC stdin into the child stdin in a thread to avoid deadlocking on
    // a slow reader. Best-effort: broken-pipe is normal if the child exits
    // before reading.
    //
    // CR-03: ALWAYS take the stdin handle so it gets closed. If we don't take
    // it, the writable end stays attached to the WrappedChild (and moves into
    // the wait thread), so any wrapped script that does `read_to_end(stdin)`
    // — `cat`, `jq`, ccstatusline — blocks forever waiting for EOF and
    // synthesizes a guaranteed timeout breach. Taking the handle drops it at
    // the end of this scope when there's nothing to write, which closes the
    // pipe and gives the child immediate EOF.
    if let Some(mut stdin) = child.stdin().take() {
        if !opts.stdin_bytes.is_empty() {
            let bytes = opts.stdin_bytes.clone();
            thread::spawn(move || {
                let _ = stdin.write_all(&bytes);
            });
        }
        // Else: stdin drops at end of `if let` scope → pipe closed → child sees EOF.
    }

    // Drain stdout/stderr in their own threads so the child can't block on a
    // full pipe. We MUST take these handles before moving `child` into the
    // wait thread.
    let stdout_thread = child.stdout().take().map(|mut s| {
        thread::spawn(move || {
            let mut buf = Vec::with_capacity(4096);
            let _ = s.read_to_end(&mut buf);
            buf
        })
    });
    let stderr_thread = child.stderr().take().map(|mut s| {
        thread::spawn(move || {
            let mut buf = Vec::with_capacity(4096);
            let _ = s.read_to_end(&mut buf);
            buf
        })
    });

    // Wait with a deadline via mpsc + thread — `WrappedChild::wait_timeout`
    // doesn't exist in process-wrap v9.1.0.
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut child = child;
        let res = child.wait();
        let _ = tx.send(res);
    });

    match rx.recv_timeout(opts.timeout) {
        Ok(Ok(status)) => {
            // Clean exit within deadline.
            let stdout_bytes = stdout_thread
                .and_then(|t| t.join().ok())
                .unwrap_or_default();
            let stderr_bytes = stderr_thread
                .and_then(|t| t.join().ok())
                .unwrap_or_default();

            let dur_ms = started.elapsed().as_millis() as u64;
            let exit_code = status.code().unwrap_or(-1);
            let stdout_str = String::from_utf8_lossy(&stdout_bytes).into_owned();

            // Refresh the LKG cache ONLY on a clean exit (D-10).
            if exit_code == 0 {
                let entry = LkgEntry::new(
                    stdout_str.clone(),
                    exit_code,
                    jiff::Timestamp::now().to_string(),
                    dur_ms,
                );
                let _ = write_lkg(&opts.cache_root, &opts.session_id, &entry);
            }

            // Always append a timings line (CORE-02).
            let _ = append_timings_for(&opts, dur_ms, Some(exit_code), &stderr_bytes);

            SupervisorOutcome::Ok {
                stdout: stdout_str,
                exit_code,
                duration_ms: dur_ms,
            }
        }
        Ok(Err(e)) => {
            // wait() itself failed — surface as SpawnFail (no useful exit code).
            finalize_spawn_fail(&opts, started, &format!("wait error: {e}"))
        }
        Err(_) => {
            // TIMEOUT — kill the whole process group.
            #[cfg(unix)]
            {
                let _ = crate::kill::kill_process_group(pgid);
            }
            // Windows JobObject auto-reaps when this process exits; for v0.1
            // we accept a small race on Windows where wedged children may
            // linger briefly. Trigger to harden: ≥1 Windows-tagged "leftover
            // process" report.

            // WR-07: also join the stdout thread on the timeout branch. The
            // captured stdout is irrelevant on a Breach (we won't return it),
            // but joining ensures the thread terminates and its buffer is
            // freed before wrap_and_run returns. Without the join the
            // JoinHandle is dropped (the OS thread detaches) and continues
            // running until read_to_end on the closed stdout pipe finishes
            // — accumulating live threads + Vec<u8> buffers in long-running
            // CC sessions that hit hundreds of timeouts.
            let _ = stdout_thread.and_then(|t| t.join().ok());

            // Drain whatever made it through before the kill (best-effort).
            let stderr_bytes = stderr_thread
                .and_then(|t| t.join().ok())
                .unwrap_or_default();
            let stderr_excerpt =
                String::from_utf8_lossy(&stderr_bytes[..stderr_bytes.len().min(4096)]).into_owned();
            let dur_ms = started.elapsed().as_millis() as u64;

            let _ = append_breach(
                &opts.cache_root,
                BreachKind::Timeout,
                &opts.stdin_bytes,
                &stderr_bytes,
                None,
                opts.writer_version,
            );
            let _ = append_timings_for(&opts, dur_ms, None, &stderr_bytes);

            SupervisorOutcome::Breach {
                kind: BreachKind::Timeout,
                exit_code: None,
                duration_ms: dur_ms,
                stderr_excerpt,
            }
        }
    }
}

/// Shared finalizer for `spawn()` and `wait()` errors. Writes a SpawnFail
/// breach + timings line and returns the corresponding outcome.
fn finalize_spawn_fail(
    opts: &SupervisorOptions,
    started: Instant,
    detail: &str,
) -> SupervisorOutcome {
    let dur_ms = started.elapsed().as_millis() as u64;
    let stderr = format!("spawn failed: {detail}");
    let _ = append_breach(
        &opts.cache_root,
        BreachKind::SpawnFail,
        &opts.stdin_bytes,
        stderr.as_bytes(),
        None,
        opts.writer_version,
    );
    let _ = append_timings_for(opts, dur_ms, None, stderr.as_bytes());
    SupervisorOutcome::Breach {
        kind: BreachKind::SpawnFail,
        exit_code: None,
        duration_ms: dur_ms,
        stderr_excerpt: stderr,
    }
}

/// Build & append one timings.jsonl line. Best-effort.
///
/// CORE-02 schema: `{ ts, session_id, duration_ms, fork_count, exit_code, stderr_capture }`.
/// `fork_count` is hard-coded to `1` at v0.1 — true fork-count attribution is a
/// v0.5 `holt doctor` concern (we'd need to walk descendants, which costs
/// budget the render path doesn't have).
fn append_timings_for(
    opts: &SupervisorOptions,
    duration_ms: u64,
    exit_code: Option<i32>,
    stderr_bytes: &[u8],
) -> std::io::Result<()> {
    let stderr_capture =
        String::from_utf8_lossy(&stderr_bytes[..stderr_bytes.len().min(4096)]).into_owned();
    let line = serde_json::json!({
        "ts": jiff::Timestamp::now().to_string(),
        "session_id": &opts.session_id,
        "duration_ms": duration_ms,
        "fork_count": 1,
        "exit_code": exit_code,
        "stderr_capture": stderr_capture,
    });
    let mut text = serde_json::to_string(&line).unwrap_or_else(|_| "{}".into());
    text.push('\n');
    append_timings(&opts.cache_root, &text)
}
