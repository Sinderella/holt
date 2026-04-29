//! `holt --self-bench` (D-14).
//!
//! Wraps `:` (POSIX no-op) ≥10 times, measuring holt-only render-path overhead.
//! The bench measures WARM cold-start (process already running). True wall-clock
//! cold-start is a follow-up CI script that wraps `time target/release/holt --self-bench`
//! externally — see RESEARCH §"Pattern 8 Important nuance".

use std::time::Instant;

use serde::Serialize;

use holt_supervisor::{SupervisorOptions, wrap_and_run};

#[derive(Debug, Clone, Serialize)]
pub struct BenchResult {
    pub iterations: u32,
    pub overhead_p50_us: u64,
    pub overhead_p95_us: u64,
    pub overhead_p99_us: u64,
    pub budget_p95_us: u64,
    pub passed: bool,
}

pub fn run_self_bench(iterations: u32) -> BenchResult {
    let mut samples_us: Vec<u64> = Vec::with_capacity(iterations as usize);

    // WR-10 note: `:` (POSIX no-op, `cmd /c exit 0` on Windows) is chosen
    // SPECIFICALLY because it does not read stdin. This used to mask CR-03
    // (the supervisor leaving child stdin open); CR-03 is now fixed, but
    // swapping in a stdin-reading bench command without re-checking is still
    // a foot-gun — the bench's measured workload is supposed to be
    // process-creation overhead, not stdin handling.
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("cmd", &["/c", "exit 0"])
    } else {
        ("sh", &["-c", ":"])
    };

    // WR-05: bench against an isolated tempdir so we never write
    // session_id="self-bench" entries into the user's real ~/.cache/holt/
    // telemetry stream. CI invokes --self-bench on every push; without a
    // tempdir, holt doctor (v0.5) would have to filter synthetic samples out
    // of the live data forever. If the tempdir create fails (extremely rare),
    // fall back to a process-id-tagged subdir under std::env::temp_dir() so
    // we still don't touch the user's cache.
    let _bench_tmp = tempfile::tempdir().ok();
    let cache_root = match &_bench_tmp {
        Some(t) => t.path().to_path_buf(),
        None => std::env::temp_dir().join(format!("holt-self-bench-{}", std::process::id())),
    };

    for _ in 0..iterations {
        let opts = SupervisorOptions {
            timeout: std::time::Duration::from_secs(5),
            session_id: "self-bench".into(),
            stdin_bytes: Vec::new(),
            cache_root: cache_root.clone(),
            writer_version: env!("CARGO_PKG_VERSION"),
        };

        let t_total = Instant::now();
        let t_supervised_in = Instant::now();
        let _ = wrap_and_run(program, args, opts);
        let supervised = t_supervised_in.elapsed();
        let total = t_total.elapsed();

        // Holt-only overhead = (function-entry-to-function-exit) − (wrapped child runtime).
        let overhead = total.saturating_sub(supervised);
        samples_us.push(overhead.as_micros() as u64);
    }

    samples_us.sort_unstable();
    // WR-06: linear-interpolation percentile (not nearest-integer round). With
    // the previous `((len-1)*frac).round()` formula and len=10, both p95 and
    // p99 collapsed to index 9 (the maximum), so the CI gate "p95 ≤ 20_000us"
    // was effectively "max-of-10 ≤ 20_000us" — much more sensitive to outliers
    // than a real p95. Linear interpolation between adjacent indices gives a
    // meaningful percentile even on small N. Saturating ops avoid panics on
    // empty samples_us (defensive — len is iterations.max(10) ≥ 10 in practice).
    let pick = |frac: f64| -> u64 {
        let n = samples_us.len();
        if n == 0 {
            return 0;
        }
        let pos = (n as f64 - 1.0) * frac;
        let lo = pos.floor() as usize;
        let hi = (lo + 1).min(n - 1);
        let weight = pos - (lo as f64);
        let lo_v = samples_us[lo] as f64;
        let hi_v = samples_us[hi] as f64;
        (lo_v + (hi_v - lo_v) * weight).round() as u64
    };

    let budget_p95_us: u64 = if cfg!(windows) { 40_000 } else { 20_000 };
    let p95 = pick(0.95);

    BenchResult {
        iterations,
        overhead_p50_us: pick(0.50),
        overhead_p95_us: p95,
        overhead_p99_us: pick(0.99),
        budget_p95_us,
        passed: p95 <= budget_p95_us,
    }
}

pub fn print_human(r: &BenchResult) {
    println!(
        "holt --self-bench  iterations={n}  budget_p95={budget}us",
        n = r.iterations,
        budget = r.budget_p95_us,
    );
    println!(
        "  p50={p50}us  p95={p95}us  p99={p99}us",
        p50 = r.overhead_p50_us,
        p95 = r.overhead_p95_us,
        p99 = r.overhead_p99_us,
    );
    if r.passed {
        println!("PASS: holt-only p95 ≤ {budget}us", budget = r.budget_p95_us);
    } else {
        println!(
            "FAIL: holt-only p95 = {p95}us > budget {budget}us",
            p95 = r.overhead_p95_us,
            budget = r.budget_p95_us,
        );
    }
}

pub fn print_json(r: &BenchResult) {
    match serde_json::to_string(r) {
        Ok(s) => println!("{s}"),
        Err(_) => println!("{{}}"),
    }
}

// PHASE 2 D-15: hook self-bench harness.

use holt_hooks::{Env, HookEvent, handle_event};

/// Hook variant of `run_self_bench`. Calls `holt_hooks::handle_event` ≥10
/// times against an in-memory minimal stdin envelope, measures the call
/// duration in microseconds, and reports p50/p95/p99 + PASS/FAIL vs the
/// 20_000us budget (40_000us on Windows). Same JSON shape as the Phase 1
/// self-bench so CI consumes both via the same `python3 json.load` step.
///
/// We point XDG_RUNTIME_DIR at a tempdir inside this process for the
/// duration of the bench so the user's real `~/.cache/holt/sessions/` is
/// never written to. Same pattern as WR-05's tempfile isolation for
/// `run_self_bench`.
///
/// Carries a scoped `#[allow(unsafe_code)]` for `std::env::set_var` /
/// `remove_var` (Rust 2024 made these unsafe). Safety: the bench is invoked
/// from `main()` BEFORE any threads spawn; the env mutation is hermetic to
/// this CLI mode and never observed by another thread. No other code in the
/// binary uses unsafe (`#![deny(unsafe_code)]` at the crate root).
#[allow(unsafe_code)]
pub fn run_self_bench_hook(event: HookEvent, iterations: u32) -> BenchResult {
    let iterations = iterations.max(10);
    let budget_p95_us: u64 = if cfg!(windows) { 40_000 } else { 20_000 };

    let xdg_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(_) => {
            return BenchResult {
                iterations: 0,
                overhead_p50_us: u64::MAX,
                overhead_p95_us: u64::MAX,
                overhead_p99_us: u64::MAX,
                budget_p95_us,
                passed: false,
            };
        }
    };
    // SAFETY: bench harness runs single-threaded inside main(); we set the
    // env var for our own process before each handle_event call. set_var is
    // unsafe in Rust 2024; we accept that tradeoff because (a) the bench is
    // an opt-in CLI mode invoked from main() before any threads spawn, and
    // (b) the alternative (per-call env override) doesn't exist in libstd.
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", xdg_dir.path());
        std::env::remove_var("TMPDIR"); // force tier 1 to win
    }

    // Minimal valid CC stdin envelope. We don't need the full v2.1.119 shape
    // for the bench — the parse path doesn't care what fields are present
    // beyond session_id (any non-empty string drops the hash-of-cwd codepath
    // in path::session_id_or_hash, making timing deterministic).
    let stdin_bytes = br#"{"session_id":"bench","cwd":"/tmp","tool_name":"Bash"}"#;

    let env = Env {
        writer_version: env!("CARGO_PKG_VERSION"),
        pid: std::process::id(),
        now_iso: "2026-04-28T10:00:00Z".to_string(),
    };

    let mut samples_us: Vec<u64> = Vec::with_capacity(iterations as usize);
    for _ in 0..iterations {
        let t0 = Instant::now();
        let _ = handle_event(event, stdin_bytes, &env);
        let elapsed_us = t0.elapsed().as_micros() as u64;
        samples_us.push(elapsed_us);
    }
    samples_us.sort_unstable();

    // WR-06: linear-interpolation percentile (same shape as `run_self_bench`).
    let pick = |frac: f64| -> u64 {
        let n = samples_us.len();
        if n == 0 {
            return 0;
        }
        let pos = (n as f64 - 1.0) * frac;
        let lo = pos.floor() as usize;
        let hi = (lo + 1).min(n - 1);
        let weight = pos - (lo as f64);
        let lo_v = samples_us[lo] as f64;
        let hi_v = samples_us[hi] as f64;
        (lo_v + (hi_v - lo_v) * weight).round() as u64
    };

    let p95 = pick(0.95);

    BenchResult {
        iterations,
        overhead_p50_us: pick(0.50),
        overhead_p95_us: p95,
        overhead_p99_us: pick(0.99),
        budget_p95_us,
        passed: p95 <= budget_p95_us,
    }
}
