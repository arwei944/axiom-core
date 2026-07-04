//! Prompt error types.

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum PromptError {
    #[error("missing variable: {0}")]
    MissingVariable(String),
    #[error("invalid variable type: {0}")]
    InvalidType(String),
    #[error("template not found: {0}")]
    NotFound(String),
    #[error("version conflict: {0}")]
    VersionConflict(String),
    #[error("render error: {0}")]
    RenderError(String),
    #[error("validation error: {0}")]
    ValidationError(String),
}
