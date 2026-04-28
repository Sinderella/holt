//! holt — Rust statusLine for Claude Code.
//!
//! Three entry points (D-14 / RESEARCH §Pattern 8):
//!   - `holt run -- <wrapped>`      — wrap + supervise a user statusLine
//!   - `holt --self-bench [--json]` — measure holt-only render-path overhead
//!   - `holt --version`             — clap-generated from Cargo.toml

#![forbid(unsafe_code)] // WR-09: defence-in-depth for the binary crate.

mod cli;
mod run;
mod self_bench;
mod stdin;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

    let exit_code = if cli.self_bench {
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
            None => {
                eprintln!("holt: no subcommand. Try `holt --help` or `holt --self-bench`.");
                2
            }
        }
    };

    std::process::exit(exit_code);
}
