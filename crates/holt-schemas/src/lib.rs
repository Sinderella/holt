//! holt-schemas — keystone crate.
//!
//! Public surface:
//!   - `Heartbeat` (per-session JSON written by Phase 2 hooks; read by v1.0 orchestrator).
//!   - `LkgEntry` (last-known-good cache; written by plan 02 supervisor).
//!   - `read_heartbeat` (the C5 / HOOK-11 non-panicking reader contract).
//!   - `atomic_write` (D-07 same-dir tmp + fsync + rename — used by both supervisor and hooks).
//!   - `ReaderError` (only Err variant: I/O that is not "file missing").

#![forbid(unsafe_code)] // D-07 atomic_write uses safe std fs APIs only.

pub mod error;
pub mod heartbeat;
pub mod lkg;
pub mod reader;
pub mod writer;

pub use error::ReaderError;
pub use heartbeat::Heartbeat;
pub use lkg::LkgEntry;
pub use reader::read_heartbeat;
pub use writer::atomic_write;
