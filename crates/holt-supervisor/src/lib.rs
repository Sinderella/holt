//! `holt-supervisor` — the v0.1 wedge.
//!
//! HARD CONSTRAINTS (load-bearing across the workspace):
//!
//!   - **C1** — pipe stdin/stdout/stderr BEFORE wrapping with the process-group
//!     leader. Enforced inside `supervisor::wrap_and_run` and audited by
//!     `tests/chokepoint_audit.rs`: there must be exactly one call site for
//!     `wrap(ProcessGroup::leader())` in this crate's `src/`.
//!   - **C5** — no `unwrap()` on the render path. The outer surface returns
//!     `SupervisorOutcome`, never panics. Internal failures (spawn, wait, kill)
//!     degrade to `Breach { kind, .. }` variants.
//!   - **C6** — the render path never *reads* `breaches.log` or `timings.jsonl`.
//!     This crate WRITES them; v0.5 `holt doctor` (deferred) is the only reader.
//!     Reading on the render path creates a storm: measuring slowdowns causes
//!     slowdowns as the log grows.
//!
//! Public surface:
//!   - [`Supervisor`] / [`wrap_and_run`] — the single supervised-spawn chokepoint (D-09).
//!   - [`SupervisorOptions`], [`SupervisorOutcome`], [`BreachKind`] — caller surface.
//!   - [`lkg::write_lkg`] / [`lkg::read_lkg`] — last-known-good cache (D-10).
//!   - [`timings::append_timings`] / [`breaches::append_breach`] — telemetry writers
//!     with 5MB / `.1` rotation (D-12, D-13).
//!   - [`kill::kill_process_group`] — `killpg` + Linux `/proc` PPID-walk fallback (H3).

#![forbid(unsafe_code)]

pub mod breaches;
pub mod kill;
pub mod lkg;
pub mod options;
pub mod paths;
pub mod supervisor;
pub mod timings;

pub use options::{BreachKind, DEFAULT_TIMEOUT, SupervisorOptions, SupervisorOutcome};
pub use supervisor::{Supervisor, wrap_and_run};
