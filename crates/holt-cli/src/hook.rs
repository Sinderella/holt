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

use holt_hooks::{Env, HookEvent, handle_event};

use crate::stdin::{StdinParseOutcome, slurp_and_parse};

/// Run the hook subcommand. Returns the process exit code; ALWAYS 0 per D-03.
/// The `event` parameter has already been parsed by clap (`HookEventArg::into_lib`).
pub fn run(event: HookEvent) -> i32 {
    let stdin_bytes = match slurp_and_parse() {
        StdinParseOutcome::Ok { raw, .. } => raw,
        StdinParseOutcome::ParseFail { raw, .. } => raw,
        StdinParseOutcome::Empty => Vec::new(),
    };

    let env = Env {
        writer_version: env!("CARGO_PKG_VERSION"),
        pid: std::process::id(),
        now_iso: jiff::Timestamp::now().to_string(),
    };

    // D-03: ignore the outcome variant. parse_fail / unwritable already routed
    // through `holt_supervisor::breaches::append_breach` inside `handle_event`.
    let _outcome = handle_event(event, &stdin_bytes, &env);

    0
}
