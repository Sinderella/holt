//! Per-session heartbeat schema (`schema_version: 1`).
//!
//! Source: docs/05-schemas.md §1 (locked field set).
//!         CONTEXT.md D-05 (#[serde(default)], NO deny_unknown_fields, schema_version first).
//!         CONTEXT.md D-08 (#[non_exhaustive] for forward-compat).
//!
//! Defensive parse posture is mandatory per PITFALLS.md H5 (CC v2.1.119 stdin-shape
//! regression precedent). New fields land silently; missing fields fall back to
//! `Default::default()`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Heartbeat {
    pub schema_version: u8, // first field, always 1 at v0.1

    // Required: session_id is CC-provided and uniquely keys the file.
    pub session_id: String,

    // All other fields default-on-missing per D-05.
    #[serde(default)]
    pub pid: u32,
    #[serde(default)]
    pub started: String, // ISO 8601 / RFC 3339 via jiff
    #[serde(default)]
    pub updated: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub cwd_label: String,
    #[serde(default)]
    pub mode: Option<String>, // "default" | "plan" | "acceptEdits" | "bypassPermissions"
    #[serde(default)]
    pub current_tool: Option<String>,
    #[serde(default)]
    pub blocked_on: Option<String>, // null at v0.1 per HOOK-05
    #[serde(default)]
    pub context_pct_real: Option<f64>,
    #[serde(default)]
    pub burn_rate_usd_per_min: Option<f64>,
    #[serde(default)]
    pub last_assistant_at: Option<String>,
    #[serde(default)]
    pub model_display: Option<String>,
    #[serde(default)]
    pub writer_version: String, // populated by Phase 2 (HOOK-06); empty at v0.1
}

impl Heartbeat {
    pub const SCHEMA_VERSION: u8 = 1;

    /// Construct a `Heartbeat` with `schema_version = SCHEMA_VERSION` automatically.
    /// Required because `Heartbeat` is `#[non_exhaustive]` (D-08) — external crates
    /// (`holt-hooks`, the future v1.0 orchestrator writers) cannot use struct-literal
    /// syntax. Same pattern as `LkgEntry::new` (added in Plan 01-02).
    ///
    /// All v1.0 fields that v0.1 leaves empty (`mode`, `context_pct_real`,
    /// `burn_rate_usd_per_min`) are exposed here so the constructor stays stable
    /// when v1.0 lands without the API breaking. Phase 2 callers pass `None` for
    /// fields they don't populate.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: String,
        pid: u32,
        started: String,
        updated: String,
        cwd: String,
        cwd_label: String,
        mode: Option<String>,
        current_tool: Option<String>,
        blocked_on: Option<String>,
        context_pct_real: Option<f64>,
        burn_rate_usd_per_min: Option<f64>,
        last_assistant_at: Option<String>,
        model_display: Option<String>,
        writer_version: String,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            session_id,
            pid,
            started,
            updated,
            cwd,
            cwd_label,
            mode,
            current_tool,
            blocked_on,
            context_pct_real,
            burn_rate_usd_per_min,
            last_assistant_at,
            model_display,
            writer_version,
        }
    }
}
