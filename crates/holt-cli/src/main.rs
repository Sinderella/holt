//! holt — Rust statusLine for Claude Code.
//!
//! Entry points:
//!   - `holt run -- <wrapped>`              — wrap + supervise a user statusLine (Phase 1 D-09)
//!   - `holt --self-bench [--json]`         — measure holt-only render-path overhead (Phase 1 D-14)
//!   - `holt hook <event>`                  — write a heartbeat (Phase 2 D-14)
//!   - `holt --self-bench-hook <event>`     — measure hook render-path overhead (Phase 2 D-15)
//!   - `holt --version`                     — clap-generated from Cargo.toml

// WR-09: defence-in-depth — `deny` (not `forbid`) so the D-15 hook self-bench
// harness in `self_bench::run_self_bench_hook` can carry a single
// `#[allow(unsafe_code)]` exception for `std::env::set_var` (Rust 2024 made
// `set_var` unsafe). The bench is an opt-in CLI mode invoked from `main()`
// before any threads spawn; the env mutation is contained to a tempdir-scoped
// `XDG_RUNTIME_DIR` override required by WR-05 hermeticity. No other code in
// this binary may use unsafe — clippy will deny it.
#![deny(unsafe_code)]

mod cli;
mod hook;
mod install_hooks;
mod run;
mod self_bench;
mod stdin;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

    let exit_code = if let Some(event_arg) = cli.self_bench_hook {
        // D-15: hook self-bench. Same JSON shape as the Phase 1 self-bench.
        let event = event_arg.into_lib();
        let result = self_bench::run_self_bench_hook(event, cli.iterations.max(10));
        if cli.json {
            self_bench::print_json(&result);
        } else {
            self_bench::print_human(&result);
        }
        if result.passed { 0 } else { 1 }
    } else if cli.self_bench {
        // Phase 1 D-14: render-path self-bench.
        let result = self_bench::run_self_bench(cli.iterations.max(10));
        if cli.json {
            self_bench::print_json(&result);
        } else {
            self_bench::print_human(&result);
        }
        if result.passed { 0 } else { 1 }
    } else {
        match cli.command {
            Some(cli::Command::Run {
                timeout,
                session_id,
                wrapped,
            }) => run::run(timeout, session_id, wrapped),
            Some(cli::Command::Hook { event }) => hook::run(event.into_lib()),
            None => {
                eprintln!(
                    "holt: no subcommand. Try `holt --help`, `holt --self-bench`, or `holt hook <event>`."
                );
                2
            }
        }
    };

    std::process::exit(exit_code);
}
