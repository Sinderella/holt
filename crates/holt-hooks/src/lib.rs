//! holt-hooks — heartbeat write side (Phase 2).
//!
//! `handle_event(event, stdin_bytes, env)` is the single public entry point per
//! CONTEXT.md D-03. It parses CC stdin defensively (D-04), assembles a
//! `Heartbeat` (D-05/D-08/D-09/D-10/D-11), resolves the writer path through
//! the three-tier fallback chain (D-06/D-07), atomically writes via
//! `holt_schemas::atomic_write` and sets 0600 perms (D-12), and routes failures
//! through `holt_supervisor::breaches::append_breach` (D-04, D-06 tier 4).
//!
//! **Never panics. Never bubbles errors to the caller.** The CLI dispatcher in
//! `holt-cli` ignores the returned `HookOutcome` and always exits 0.

#![forbid(unsafe_code)] // WR-09 baseline preserved.

pub mod assemble;
pub mod event;
pub mod handle;
pub mod path;
pub mod stdin;

pub use assemble::{Env, assemble_heartbeat};
pub use event::HookEvent;
pub use handle::{FallbackReason, HookOutcome, handle_event};
pub use stdin::HookStdin;
