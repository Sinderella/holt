//! Reader error type — narrow on purpose.
//!
//! D-06 says read_heartbeat returns Err only for I/O that is NOT "file missing".
//! Every other failure mode (zero-byte file, truncated JSON, unrecognized
//! schema_version, missing required field) returns Ok(None).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReaderError {
    #[error("io error reading heartbeat: {0}")]
    Io(#[from] std::io::Error),
}
