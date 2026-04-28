//! Defensive CC stdin parse (CORE-08 / PITFALLS H5).
//!
//! At Phase 1 we don't interpret CC stdin fields — we only need to capture parse
//! failures so the breach log shows what shape was problematic. Phase 2's hook
//! crate consumes the parsed Value.
//!
//! CR-02: the parser VALIDATES that stdin is well-formed JSON, but always
//! returns the ORIGINAL bytes. Re-serializing via `Value::to_string()` would
//! re-format numbers, drop trailing newlines, and (without CR-01's
//! `preserve_order`) re-order object keys — wrapped scripts that key on the
//! exact bytes CC sent would see slightly different input. The defensive parse
//! is a *check*, not a transform.

use std::io::{self, Read};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// CR-04: stdin-slurp deadline. The render-path budget is sub-20ms (D-04); an
/// unbounded `read_to_end` on stdin would let a slow / never-closing CC pipe
/// blow that budget silently. 200ms is generous compared to the supervisor's
/// 2s wait deadline (DEFAULT_TIMEOUT) but well under any plausible refresh
/// boundary, and keeps the worst case bounded by exactly one render fire.
const STDIN_SLURP_DEADLINE: Duration = Duration::from_millis(200);

pub enum StdinParseOutcome {
    /// Stdin parsed cleanly as JSON. `raw` holds the unmodified bytes exactly
    /// as CC sent them; the parsed `value` is currently unused at v0.1 (Phase 2
    /// hooks will consume it) but is exposed so the type stays useful.
    Ok {
        #[allow(dead_code)] // Phase 2 hooks will consume this.
        value: serde_json::Value,
        raw: Vec<u8>,
    },
    /// Stdin was non-empty but did not parse. `excerpt` is a UTF-8-lossy view
    /// (size-capped at 2KB to match D-13 STDIN_EXCERPT_CAP) for breach logging;
    /// `raw` carries the original bytes for forwarding to LKG / breach context
    /// without lossy re-encoding.
    ParseFail {
        excerpt: String,
        #[allow(dead_code)] // Held for future LKG/breach-context use.
        raw: Vec<u8>,
    },
    /// Stdin was empty (or unreadable — treated equivalently per H5 defensive
    /// posture).
    Empty,
}

pub fn slurp_and_parse() -> StdinParseOutcome {
    // CR-04: bound the slurp with a deadline. We drop the read thread on
    // timeout — its OS thread keeps living until stdin EOFs, but the render
    // path returns immediately. For our use case (a short-lived statusLine
    // process) this is acceptable: the leaked thread dies with the process.
    let (tx, rx) = mpsc::channel::<(bool, Vec<u8>)>();
    thread::spawn(move || {
        let mut buf = Vec::with_capacity(4096);
        let read_ok = io::stdin().read_to_end(&mut buf).is_ok();
        let _ = tx.send((read_ok, buf));
    });

    let (read_ok, buf) = match rx.recv_timeout(STDIN_SLURP_DEADLINE) {
        Ok(pair) => pair,
        // Deadline elapsed → render path must not block. Treat as Empty so
        // the caller falls through to LKG (or empty stdout) without breaching.
        Err(_) => return StdinParseOutcome::Empty,
    };

    if !read_ok || buf.is_empty() {
        return StdinParseOutcome::Empty;
    }

    match serde_json::from_slice::<serde_json::Value>(&buf) {
        Ok(value) => StdinParseOutcome::Ok { value, raw: buf },
        Err(_) => {
            let excerpt = String::from_utf8_lossy(&buf[..buf.len().min(2048)]).into_owned();
            StdinParseOutcome::ParseFail { excerpt, raw: buf }
        }
    }
}
