//! `breaches.log` writer + record schema (D-13).
//!
//! Each breach lands as one JSON object on its own line:
//!
//! ```text
//! { ts, kind, env_capture, stdin_excerpt, stderr_excerpt, exit_code, writer_version }
//! ```
//!
//! `env_capture` is **allowlisted**, never the full env — pasting a breach
//! into a GitHub issue must not leak secrets (D-13; CONTRIBUTING.md North
//! Star #2). Excerpts are size-capped (≤2KB stdin, ≤4KB stderr).

use std::path::Path;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::options::BreachKind;
use crate::paths::breaches_path;
use crate::timings::append_jsonl;

/// Allowlist for `env_capture` (D-13). Adding a key here means it will appear
/// in any breach record forever — review the threat model before extending.
///
/// Intentionally excludes anything matching `*_TOKEN`, `*_KEY`, `*_SECRET`,
/// `AWS_*`, `GH_*`, `OPENAI_*`, etc. We allow only what's needed to debug a
/// statusLine spawn failure: shell + locale + Holt-internal flags.
const ENV_ALLOWLIST: &[&str] = &[
    // POSIX/shell baseline:
    "PATH",
    "HOME",
    "USER",
    "SHELL",
    "TERM",
    "LANG",
    "LC_ALL",
    // FS roots:
    "XDG_RUNTIME_DIR",
    "TMPDIR",
    // Holt-internal flags (introduced by future plans; safe to capture now):
    "HOLT_LABEL",
    "HOLT_NESTED",
    "HOLT_TRACE",
    // Claude Code project context (path only — never API keys):
    "CLAUDE_PROJECT_DIR",
];

/// Stdin excerpt cap per D-13.
const STDIN_EXCERPT_CAP: usize = 2 * 1024;
/// Stderr excerpt cap per D-13.
const STDERR_EXCERPT_CAP: usize = 4 * 1024;

#[derive(Debug, Serialize)]
struct BreachRecord {
    ts: String,
    kind: &'static str,
    env_capture: Map<String, Value>,
    stdin_excerpt: String,
    stderr_excerpt: String,
    exit_code: Option<i32>,
    writer_version: &'static str,
}

/// Append a breach record to `<cache_root>/breaches.log`. Rotation policy is
/// shared with `timings.jsonl` (5MB → `.1`, see [`crate::timings::MAX_BYTES`]).
///
/// Best-effort: returns the underlying I/O error on failure but supervisor
/// callers ignore the result — telemetry must never fail the render path.
pub fn append_breach(
    cache_root: &Path,
    kind: BreachKind,
    stdin_bytes: &[u8],
    stderr_bytes: &[u8],
    exit_code: Option<i32>,
) -> std::io::Result<()> {
    let mut env_capture = Map::new();
    for &k in ENV_ALLOWLIST {
        if let Ok(v) = std::env::var(k) {
            env_capture.insert(k.to_string(), Value::String(v));
        }
    }

    let stdin_excerpt =
        String::from_utf8_lossy(&stdin_bytes[..stdin_bytes.len().min(STDIN_EXCERPT_CAP)])
            .into_owned();
    let stderr_excerpt =
        String::from_utf8_lossy(&stderr_bytes[..stderr_bytes.len().min(STDERR_EXCERPT_CAP)])
            .into_owned();

    let rec = BreachRecord {
        ts: jiff::Timestamp::now().to_string(),
        kind: kind.as_str(),
        env_capture,
        stdin_excerpt,
        stderr_excerpt,
        exit_code,
        writer_version: env!("CARGO_PKG_VERSION"),
    };

    let path = breaches_path(cache_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut text = serde_json::to_string(&rec)
        .map_err(|e| std::io::Error::other(format!("breach serialize: {e}")))?;
    text.push('\n');
    append_jsonl(&path, &text)
}
