//! Caller-facing supervisor options + outcome enums.
//!
//! Source: CONTEXT.md D-09, D-11. Plan 03 (`holt-cli`) constructs `SupervisorOptions`
//! from clap-parsed args; Phase 2's hooks may also fan in here when `holt run`
//! becomes the main statusLine entry point.

use std::path::PathBuf;
use std::time::Duration;

/// D-11: default supervisor timeout. Configurable via `holt run --timeout`.
///
/// Rationale: Claude Code's default statusLine `refreshInterval` is 5s; 2s leaves
/// headroom while staying under the next refresh boundary. The user's wrapped
/// script gets the remainder of the budget after holt's ~1.5ms overhead.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);

/// Configuration passed into [`crate::supervisor::wrap_and_run`].
#[derive(Debug, Clone)]
pub struct SupervisorOptions {
    /// Hard deadline (D-11 default 2 seconds).
    pub timeout: Duration,
    /// Session id — keys the LKG cache file path (`<cache_root>/lkg/<session_id>.json`).
    pub session_id: String,
    /// CC stdin bytes to forward to the wrapped script (Phase 2 fills this; v0.1 may be empty).
    pub stdin_bytes: Vec<u8>,
    /// Cache root (defaults to `~/.cache/holt/`; tests inject a tempdir).
    pub cache_root: PathBuf,
    /// WR-08: holt-binary version stamped into breaches.log records (D-13).
    /// The supervisor crate's own `CARGO_PKG_VERSION` is meaningless to a
    /// triager — they want to know which `holt` binary wrote the record.
    /// Callers (holt-cli/main.rs) pass `env!("CARGO_PKG_VERSION")` from the
    /// binary crate. Static lifetime so we can keep `BreachRecord::writer_version`
    /// as `&'static str` without an extra allocation per breach.
    pub writer_version: &'static str,
}

impl SupervisorOptions {
    /// Build options with the D-11 default timeout and an empty stdin payload.
    /// Tests pass `tempdir().path().to_path_buf()` for `cache_root`.
    ///
    /// `writer_version` defaults to the supervisor crate's own version, which
    /// is acceptable for tests. Production callers SHOULD set it explicitly
    /// from their binary crate's `env!("CARGO_PKG_VERSION")`.
    pub fn with_defaults(session_id: String, cache_root: PathBuf) -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            session_id,
            stdin_bytes: Vec::new(),
            cache_root,
            writer_version: env!("CARGO_PKG_VERSION"),
        }
    }
}

/// Outcome of a single supervised invocation.
///
/// `Ok` triggers an LKG cache update (D-10). `Breach` writes one entry to
/// `breaches.log` (D-13). Both variants append exactly one line to
/// `timings.jsonl` (D-12 / CORE-02).
#[derive(Debug, Clone)]
pub enum SupervisorOutcome {
    Ok {
        stdout: String,
        exit_code: i32,
        duration_ms: u64,
    },
    Breach {
        kind: BreachKind,
        exit_code: Option<i32>,
        duration_ms: u64,
        stderr_excerpt: String,
    },
}

/// Why a supervised invocation breached.
///
/// `ParseFail` is reserved for the CLI-side stdin parser (CORE-08); the
/// supervisor itself only emits `Timeout` and `SpawnFail`. `Unwritable` is
/// emitted by `holt-hooks` (Phase 2) when all three D-06 fallback tiers fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreachKind {
    Timeout,
    ParseFail,
    SpawnFail,
    /// Hook write failed at all three D-06 tiers — the heartbeat could not be
    /// written and the failure is recorded in `breaches.log` (D-06 tier 4) if
    /// THAT is writable; otherwise the hook exits 0 silently. Phase 2 hooks
    /// emit this; the supervisor itself never does.
    Unwritable,
}

impl BreachKind {
    /// Stable string used in `breaches.log` JSON records.
    pub fn as_str(self) -> &'static str {
        match self {
            BreachKind::Timeout => "timeout",
            BreachKind::ParseFail => "parse_fail",
            BreachKind::SpawnFail => "spawn_fail",
            BreachKind::Unwritable => "unwritable",
        }
    }
}
