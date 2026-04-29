//! `clap` derive surface for `holt`.
//!
//! Locked invariants:
//!   - top-level `--self-bench`, `--json`, `--iterations` flags (Phase 1 D-14)
//!   - top-level `--self-bench-hook <EVENT>` flag (Phase 2 D-15)
//!   - `run` subcommand with `--timeout`, `--session-id`, trailing wrapped command (Phase 1 D-09)
//!   - `hook <EVENT>` subcommand (Phase 2 D-14)

use clap::{Parser, Subcommand, ValueEnum};

use holt_hooks::HookEvent;

/// holt — Rust statusLine for Claude Code.
#[derive(Parser, Debug)]
#[command(name = "holt", version, about)]
pub struct Cli {
    /// Run the bench harness ≥10 iterations and print p50/p95/p99 + PASS/FAIL.
    #[arg(long)]
    pub self_bench: bool,

    /// D-15: Run the hook bench harness ≥10 iterations against the named
    /// event and print p50/p95/p99 + PASS/FAIL. Same 20ms p95 budget as
    /// `--self-bench` because hooks fire on the render path.
    #[arg(long, value_name = "EVENT")]
    pub self_bench_hook: Option<HookEventArg>,

    /// Emit machine-readable JSON (only meaningful with --self-bench / --self-bench-hook).
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
    /// Phase 2 D-14: write a heartbeat for the given CC event. Reads CC stdin
    /// from stdin, ALWAYS exits 0 (never bubbles errors to CC).
    Hook {
        /// Which CC hook event fired. Must be one of the five v0.1 subscribed
        /// events (PreToolUse / PostToolUse / Stop / Notification / SessionStart).
        /// PreCompact is reserved for v1.0 per docs/02-scope.md.
        event: HookEventArg,
    },
    /// Phase 3 D-16: idempotently merge holt's 5 hook entries into ~/.claude/settings.json.
    ///
    /// Default mode acquires an fs2 exclusive lock, writes a `.holt.bak` backup, then
    /// atomically writes the merged file (fsync-before-rename). `--dry-run` prints a
    /// unified diff to stdout without touching settings.json. `--print` emits just the
    /// JSON snippet for manual paste. `--dry-run` and `--print` are mutually exclusive.
    /// The single backup at `<settings>.holt.bak` is overwritten on each run (not a
    /// versioned chain).
    InstallHooks {
        /// Print a unified diff of what would change; do not modify settings.json (D-11).
        #[arg(long, conflicts_with = "print")]
        dry_run: bool,
        /// Print just the holt hook-entry JSON snippet for manual paste; do not modify
        /// settings.json (D-12).
        #[arg(long)]
        print: bool,
    },
}

/// Clap-side mirror of `holt_hooks::HookEvent` so we can derive `ValueEnum`.
/// Same five variants in the same order; converted to `HookEvent` via
/// `HookEventArg::into_lib()`. We can't `derive(ValueEnum)` directly on
/// `HookEvent` because it lives in another crate.
#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "PascalCase")]
pub enum HookEventArg {
    PreToolUse,
    PostToolUse,
    Stop,
    Notification,
    SessionStart,
}

impl HookEventArg {
    pub fn into_lib(self) -> HookEvent {
        match self {
            HookEventArg::PreToolUse => HookEvent::PreToolUse,
            HookEventArg::PostToolUse => HookEvent::PostToolUse,
            HookEventArg::Stop => HookEvent::Stop,
            HookEventArg::Notification => HookEvent::Notification,
            HookEventArg::SessionStart => HookEvent::SessionStart,
        }
    }
}
