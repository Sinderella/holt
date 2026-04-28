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
    let mut buf = Vec::with_capacity(4096);
    if io::stdin().read_to_end(&mut buf).is_err() {
        return StdinParseOutcome::Empty;
    }
    if buf.is_empty() {
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
