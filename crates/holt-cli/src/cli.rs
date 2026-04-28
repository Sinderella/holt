//! `clap` derive surface for `holt`.
//!
//! Locked invariants (PLAN 01-03 `<interfaces>`):
//!   - top-level `--self-bench`, `--json`, `--iterations` flags
//!   - `run` subcommand with `--timeout`, `--session-id`, and a trailing wrapped command

use clap::{Parser, Subcommand};

/// holt — Rust statusLine for Claude Code.
#[derive(Parser, Debug)]
#[command(name = "holt", version, about)]
pub struct Cli {
    /// Run the bench harness ≥10 iterations and print p50/p95/p99 + PASS/FAIL.
    #[arg(long)]
    pub self_bench: bool,

    /// Emit machine-readable JSON (only meaningful with --self-bench).
    #[arg(long)]
    pub json: bool,

    /// Number of self-bench iterations (≥10).
    #[arg(long, default_value_t = 10)]
    pub iterations: u32,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Wrap and supervise a user statusLine command (D-09 chokepoint).
    Run {
        /// Hard timeout (default 2s — D-11). Examples: `2s`, `1500ms`, `1h30m`.
        #[arg(long)]
        timeout: Option<String>,

        /// Session id override (used to key LKG cache).
        #[arg(long)]
        session_id: Option<String>,

        /// `--` then the wrapped command + args.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        wrapped: Vec<String>,
    },
}
