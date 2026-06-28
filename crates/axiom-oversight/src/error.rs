use std::fmt;

#[derive(Debug)]
pub enum OversightError {
    LayerViolation {
        from: String,
        to: String,
        message: String,
    },
    ResourceExhausted {
        resource: String,
        message: String,
    },
    ComplianceViolation {
        pattern: String,
        message: String,
    },
    StartupFailed {
        checks_failed: Vec<String>,
    },
    Internal(String),
}

impl fmt::Display for OversightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LayerViolation { from, to, message } => {
                write!(f, "layer violation {} -> {}: {}", from, to, message)
            }
            Self::ResourceExhausted { resource, message } => {
                write!(f, "resource {} exhausted: {}", resource, message)
            }
            Self::ComplianceViolation { pattern, message } => {
                write!(f, "compliance [{}] {}", pattern, message)
            }
            Self::StartupFailed { checks_failed } => write!(
                f,
                "startup failed: {} checks failed: {:?}",
                checks_failed.len(),
                checks_failed
            ),
            Self::Internal(msg) => write!(f, "oversight internal: {}", msg),
        }
    }
}

impl std::error::Error for OversightError {}

pub type OversightResult<T> = Result<T, OversightError>;
