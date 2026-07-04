//! Tool error types.

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("execution failed: {0}")]
    Execution(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("timeout")]
    Timeout,
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("unknown error")]
    Unknown,
}

impl From<serde_json::Error> for ToolError {
    fn from(e: serde_json::Error) -> Self {
        ToolError::Serialization(e.to_string())
    }
}
