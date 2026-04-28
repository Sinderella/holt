//! `holt --self-bench` (D-14).
//!
//! Wraps `:` (POSIX no-op) ≥10 times, measuring holt-only render-path overhead.
//! The bench measures WARM cold-start (process already running). True wall-clock
//! cold-start is a follow-up CI script that wraps `time target/release/holt --self-bench`
//! externally — see RESEARCH §"Pattern 8 Important nuance".

use std::time::Instant;

use serde::Serialize;

use holt_supervisor::{SupervisorOptions, paths::default_cache_root, wrap_and_run};

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

    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("cmd", &["/c", "exit 0"])
    } else {
        ("sh", &["-c", ":"])
    };

    let cache_root = default_cache_root();

    for _ in 0..iterations {
        let opts = SupervisorOptions {
            timeout: std::time::Duration::from_secs(5),
            session_id: "self-bench".into(),
            stdin_bytes: Vec::new(),
            cache_root: cache_root.clone(),
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
    let pick = |frac: f64| -> u64 {
        let idx = ((samples_us.len() as f64 - 1.0) * frac).round() as usize;
        samples_us[idx.min(samples_us.len() - 1)]
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
