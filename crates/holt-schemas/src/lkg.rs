//! Last-known-good cache schema (`schema_version: 1`).
//!
//! Source: CONTEXT.md D-10. Schema-version-tagged so future bumps are graceful.
//! Render path on cache hit reads ONLY the `stdout` field; remaining fields are
//! observability for `holt doctor` (v0.5).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LkgEntry {
    pub schema_version: u8, // always 1 at v0.1
    pub stdout: String,
    pub exit_code: i32,
    pub captured_at: String, // ISO 8601 via jiff::Timestamp::now().to_string()
    pub duration_ms: u64,
}

impl LkgEntry {
    pub const SCHEMA_VERSION: u8 = 1;
}
