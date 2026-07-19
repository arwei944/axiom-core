use thiserror::Error;

pub type IsaResult<T> = Result<T, IsaError>;

#[derive(Debug, Error, Clone)]
pub enum IsaError {
    #[error("atom `{name}` failed: {message}")]
    Atom { name: String, message: String },

    #[error("port `{name}` failed: {message}")]
    Port { name: String, message: String },

    #[error("adapter `{name}` failed: {message}")]
    Adapter { name: String, message: String },

    #[error("composer `{name}` failed: {message}")]
    Composer { name: String, message: String },

    #[error("governor rejected: {reason}")]
    Rejected { reason: String },

    #[error("circuit open on port `{name}`")]
    CircuitOpen { name: String },

    #[error("witness journal: {0}")]
    Journal(String),
}

impl IsaError {
    pub fn atom(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Atom {
            name: name.into(),
            message: message.into(),
        }
    }

    pub fn port(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Port {
            name: name.into(),
            message: message.into(),
        }
    }

    pub fn adapter(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Adapter {
            name: name.into(),
            message: message.into(),
        }
    }

    pub fn composer(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Composer {
            name: name.into(),
            message: message.into(),
        }
    }

    pub fn rejected(reason: impl Into<String>) -> Self {
        Self::Rejected {
            reason: reason.into(),
        }
    }
}
