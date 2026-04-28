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
}
