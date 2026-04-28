//! Defensive CC stdin parse (CORE-08 / PITFALLS H5).
//!
//! At Phase 1 we don't interpret CC stdin fields — we only need to capture parse
//! failures so the breach log shows what shape was problematic. Phase 2's hook
//! crate consumes the parsed Value.

use std::io::{self, Read};

pub enum StdinParseOutcome {
    Ok(serde_json::Value),
    ParseFail { excerpt: String },
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
        Ok(v) => StdinParseOutcome::Ok(v),
        Err(_) => {
            let excerpt = String::from_utf8_lossy(&buf[..buf.len().min(2048)]).into_owned();
            StdinParseOutcome::ParseFail { excerpt }
        }
    }
}
