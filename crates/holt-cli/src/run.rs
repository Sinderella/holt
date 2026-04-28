//! `holt run -- <wrapped>` handler.
//!
//! Flow (RESEARCH §"System Architecture Diagram"):
//!   1. Slurp + defensively parse CC stdin. ParseFail → record breach + LKG fall-through.
//!   2. Dispatch to holt_supervisor::wrap_and_run.
//!   3. Match outcome:
//!      - Ok(stdout)  → emit stdout, exit 0
//!      - Breach      → emit LKG (or empty if no LKG yet), exit 0
//!   4. NEVER bubble errors to CC.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use holt_supervisor::breaches::append_breach;
use holt_supervisor::lkg::read_lkg;
use holt_supervisor::options::BreachKind;
use holt_supervisor::paths::default_cache_root;
use holt_supervisor::{SupervisorOptions, SupervisorOutcome, wrap_and_run};

use crate::stdin::{StdinParseOutcome, slurp_and_parse};

/// Top-level handler for `holt run -- <wrapped>`.
///
/// NEVER bubbles errors to CC; degrades to LKG (or empty stdout) on any failure.
/// Always exits 0 unless arguments are missing (exit 2 in that case so CI / shell
/// scripts can detect the misuse — but live CC stdin always supplies args).
pub fn run(timeout: Option<String>, session_id: Option<String>, wrapped: Vec<String>) -> i32 {
    if wrapped.is_empty() {
        eprintln!("holt run: missing wrapped command (use `--` to separate args).");
        return 2;
    }

    let cache_root = default_cache_root();
    let session_id = session_id.unwrap_or_else(|| "default".to_string());

    // Step 1: defensive CC stdin parse (CORE-08).
    //
    // CR-02: we VALIDATE that CC stdin is JSON (so we can record a parse_fail
    // breach if it isn't), but we forward the ORIGINAL bytes to the wrapped
    // script — never `Value::to_string()`, which would re-format numbers and
    // (pre-`preserve_order`) re-order keys.
    let stdin_outcome = slurp_and_parse();
    let stdin_bytes = match stdin_outcome {
        StdinParseOutcome::Ok { raw, .. } => raw,
        StdinParseOutcome::ParseFail { excerpt, .. } => {
            // Record parse_fail breach and fall through to LKG (or empty).
            let _ = append_breach(
                &cache_root,
                BreachKind::ParseFail,
                excerpt.as_bytes(),
                b"",
                None,
            );
            emit_lkg_or_empty(&cache_root, &session_id);
            return 0;
        }
        StdinParseOutcome::Empty => Vec::new(),
    };

    // Step 2: parse timeout.
    let parsed_timeout = timeout
        .as_deref()
        .and_then(|s| humantime::parse_duration(s).ok())
        .unwrap_or(Duration::from_secs(2));

    // Step 3: dispatch through the chokepoint.
    let opts = SupervisorOptions {
        timeout: parsed_timeout,
        session_id: session_id.clone(),
        stdin_bytes,
        cache_root: cache_root.clone(),
    };
    let program = wrapped[0].as_str();
    let args: Vec<&str> = wrapped[1..].iter().map(String::as_str).collect();
    let outcome = wrap_and_run(program, &args, opts);

    // Step 4: emit per outcome.
    match outcome {
        SupervisorOutcome::Ok { stdout, .. } => {
            let _ = std::io::stdout().write_all(stdout.as_bytes());
            let _ = std::io::stdout().flush();
        }
        SupervisorOutcome::Breach { .. } => {
            emit_lkg_or_empty(&cache_root, &session_id);
        }
    }

    0
}

fn emit_lkg_or_empty(cache_root: &Path, session_id: &str) {
    if let Some(entry) = read_lkg(cache_root, session_id) {
        let _ = std::io::stdout().write_all(entry.stdout.as_bytes());
    }
    // No LKG → empty stdout (CC sees a blank statusLine, never an error).
    let _ = std::io::stdout().flush();
}
