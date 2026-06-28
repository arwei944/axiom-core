use thiserror::Error;

#[derive(Error, Debug)]
pub enum AxiomError {
    #[error("Mailbox is full (capacity: {capacity})")]
    MailboxFull { capacity: usize },

    #[error("Cell {cell_id} not found")]
    CellNotFound { cell_id: String },

    #[error("Axiom invariant violated: {message}")]
    InvariantViolated { message: String },

    #[error("Signal validation failed: {message}")]
    SignalValidation { message: String },

    #[error("Cell crashed: {message}")]
    CellCrashed { message: String },

    #[error("Stale state detected: expected {expected:?}, got {actual:?}")]
    StaleState { expected: u64, actual: u64 },

    #[error("Store error: {0}")]
    Store(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AxiomError>;
