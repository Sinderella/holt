//! Filled in Task 4 (JSONC CST round-trip merge, D-02 / D-08 / D-09 / D-10).

#[derive(Debug, thiserror::Error)]
pub enum MergeError {
    #[error("settings.json is not valid JSONC: {0}")]
    Parse(String),
    #[error("settings.json root is not a JSON object (got {got})")]
    NotAnObject { got: &'static str },
}

pub struct MergeOutput {
    /// Post-merge bytes (UTF-8). Pass to `holt_schemas::atomic_write`.
    pub bytes: String,
    /// True if any byte changed vs the input. False = no-op (idempotent re-run).
    pub changed: bool,
}

pub fn merge_settings(_input: &str) -> Result<MergeOutput, MergeError> {
    unimplemented!("Task 4");
}
