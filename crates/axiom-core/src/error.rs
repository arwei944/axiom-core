use crate::layer::Layer;
use crate::version::{Compatibility, Version};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AxiomError {
    #[error("Mailbox is full (capacity: {capacity})")]
    MailboxFull { capacity: usize },

    #[error("Cell {cell_id} not found")]
    CellNotFound { cell_id: String },

    #[error("Cell {cell_id} is not running (state: {state})")]
    CellNotRunning { cell_id: String, state: String },

    #[error("Axiom invariant violated: {message}")]
    InvariantViolated { message: String },

    #[error("Schema validation failed: {message}")]
    SchemaValidation { message: String },

    #[error("Signal validation failed: {message}")]
    SignalValidation { message: String },

    #[error("Layer violation: {from} cannot send to {to} (signal: {signal_type})")]
    LayerViolation {
        from: Layer,
        to: Layer,
        signal_type: String,
    },

    #[error("Handoff limit exceeded: message {msg_id} hopped {hops} times (max 8)")]
    HandoffLimitExceeded { msg_id: String, hops: u32 },

    #[error("Cell {cell_id} heartbeat timeout (last seen {last_seen_ms}ms ago)")]
    HeartbeatTimeout { cell_id: String, last_seen_ms: u64 },

    #[error("Cell crashed: {message}")]
    CellCrashed { message: String },

    #[error("Circuit breaker open for cell {cell_id}")]
    CircuitBreak { cell_id: String },

    #[error("Stale state detected: expected version {expected:?}, got {actual:?}")]
    StaleState { expected: u64, actual: u64 },

    #[error("Duplicate message {msg_id} (idempotency violation)")]
    DuplicateMessage { msg_id: String },

    #[error("Version incompatibility: {compatibility:?} (required: {required}, found: {found})")]
    VersionMismatch {
        compatibility: Compatibility,
        required: Version,
        found: Version,
    },

    #[error("Schema version too new: found v{found}, max supported v{max_supported}")]
    SchemaVersionTooNew { found: u16, max_supported: u16 },

    #[error("Schema version too old: found v{found}, no migration path to v{current}")]
    MigrationPathNotFound { found: u16, current: u16 },

    #[error("Migration chain incomplete: missing migration v{from} to v{to}")]
    MigrationChainGap { from: u16, to: u16 },

    #[error("Protocol version mismatch: expected v{expected}, got v{got}")]
    ProtocolMismatch { expected: u16, got: u16 },

    #[error("Migration failed from v{from} to v{to}: {reason}")]
    MigrationFailed { from: u16, to: u16, reason: String },

    #[error("Permission denied: {action} requires {required:?} permission")]
    PermissionDenied { action: String, required: String },

    #[error("Correlation chain broken: {message}")]
    CorrelationBroken { message: String },

    #[error("Entropy threshold exceeded: {score} > {threshold}")]
    EntropyExceeded { score: f64, threshold: f64 },

    #[error("Token budget exceeded: used {used}, budget {budget}")]
    TokenBudgetExceeded { used: u64, budget: u64 },

    #[error("Message loop detected: {message}")]
    LoopDetected { message: String },

    #[error("Timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Store error: {0}")]
    Store(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: &'static str,
        actual: &'static str,
    },

    #[error("Witness serialization failed: {0}")]
    WitnessSerialization(String),

    #[error("Signal serialization failed: {0}")]
    SignalSerialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, AxiomError>;
