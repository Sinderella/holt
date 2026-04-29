//! `holt hook <event>` subcommand dispatcher (D-14).
//!
//! Pipeline:
//!   1. Slurp CC stdin via `stdin::slurp_and_parse` (Phase 1 helper, CR-04
//!      200ms deadline — applies to hooks too because hooks fire on the
//!      render path per D-15).
//!   2. Build `Env { writer_version: env!("CARGO_PKG_VERSION"), pid, now_iso }`.
//!      The writer_version comes from THIS binary crate per D-11 / HOOK-06.
//!   3. Call `holt_hooks::handle_event(event, stdin_raw, &env)` — the entry
//!      point from Plan 02-01.
//!   4. Ignore the `HookOutcome` variant. Exit 0 unconditionally per D-03.
//!
//! The hook NEVER bubbles errors to CC. parse_fail / unwritable failures land
//! in `breaches.log` (best-effort) inside `handle_event`; we don't even check
//! whether the breach was written.

use holt_hooks::{Env, HookEvent, HookOutcome, handle_event};

use crate::stdin::{StdinParseOutcome, slurp_and_parse};

/// Run the hook subcommand. Returns the process exit code; ALWAYS 0 per D-03.
/// The `event` parameter has already been parsed by clap (`HookEventArg::into_lib`).
pub fn run(event: HookEvent) -> i32 {
    // WR-02: empty stdin is a NORMAL condition (Phase 1 holt-cli/src/stdin.rs
    // documents StdinParseOutcome::Empty as 'Stdin was empty (or unreadable
    // — treated equivalently per H5 defensive posture)'), not a parse
    // failure. Short-circuit before handle_event so we don't write a
    // parse_fail breach record on every `echo | holt hook PreToolUse`
    // ad-hoc developer test or every empty-stdin Notification fire.
    let stdin_bytes = match slurp_and_parse() {
        StdinParseOutcome::Ok { raw, .. } => raw,
        StdinParseOutcome::ParseFail { raw, .. } => raw,
        StdinParseOutcome::Empty => return 0,
    };

    let env = Env {
        writer_version: env!("CARGO_PKG_VERSION"),
        pid: std::process::id(),
        now_iso: jiff::Timestamp::now().to_string(),
    };

    // WR-01: explicit exhaustive match (instead of `let _outcome = ...`) so
    // the compiler forces a conscious decision if `HookOutcome` ever grows a
    // new variant. The contract today is 'exit 0 unconditionally' per D-03;
    // a new variant should not silently inherit that behavior.
    match handle_event(event, &stdin_bytes, &env) {
        HookOutcome::Wrote { .. }
        | HookOutcome::FellBack { .. }
        | HookOutcome::ParseFailed
        | HookOutcome::Unwritable => {}
    }

    0
}
