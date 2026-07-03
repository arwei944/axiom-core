use crate::layer::Layer;
use crate::version::{Compatibility, Version};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AxiomError {
    #[error("Mailbox for cell {cell_id} is full (capacity: {capacity})")]
    MailboxFull { cell_id: String, capacity: usize },

    #[error("Cell {cell_id} not found")]
    CellNotFound { cell_id: String },

    #[error("Cell {cell_id} already exists")]
    CellAlreadyExists { cell_id: String },

    #[error("Cell {cell_id} is not running (state: {state})")]
    CellNotRunning { cell_id: String, state: String },

    #[error("Axiom invariant violated: {message}")]
    InvariantViolated { message: String },

    #[error("Schema validation failed for {signal_type}: {message}")]
    SchemaValidation { signal_type: String, message: String },

    #[error("Signal validation failed for {signal_type}: {message}")]
    SignalValidation { signal_type: String, message: String },

    #[error("Layer violation: {from} cannot send to {to} (signal: {signal_type}, source_cell: {source_cell})")]
    LayerViolation {
        from: Layer,
        to: Layer,
        signal_type: String,
        source_cell: String,
    },

    #[error("Handoff limit exceeded: message {msg_id} hopped {hops} times (max 8, correlation: {correlation_id})")]
    HandoffLimitExceeded { msg_id: String, hops: u32, correlation_id: String },

    #[error("Cell {cell_id} heartbeat timeout (last seen {last_seen_ms}ms ago)")]
    HeartbeatTimeout { cell_id: String, last_seen_ms: u64 },

    #[error("Cell {cell_id} crashed: {message}")]
    CellCrashed { cell_id: String, message: String },

    #[error("Cell {cell_id} panicked: {message}")]
    CellPanic { cell_id: String, message: String },

    #[error("Circuit breaker open for cell {cell_id} (failures: {failures})")]
    CircuitBreak { cell_id: String, failures: u32 },

    #[error("Stale state detected for cell {cell_id}: expected version {expected}, got {actual}")]
    StaleState { cell_id: String, expected: u64, actual: u64 },

    #[error("Duplicate message {msg_id} (idempotency violation, correlation: {correlation_id})")]
    DuplicateMessage { msg_id: String, correlation_id: String },

    #[error("Version incompatibility: {compatibility:?} (required: {required}, found: {found})")]
    VersionMismatch {
        compatibility: Compatibility,
        required: Version,
        found: Version,
    },

    #[error("Schema version too new for {signal_type}: found v{found}, max supported v{max_supported}")]
    SchemaVersionTooNew { signal_type: String, found: u16, max_supported: u16 },

    #[error("Schema version too old for {signal_type}: found v{found}, no migration path to v{current}")]
    MigrationPathNotFound { signal_type: String, found: u16, current: u16 },

    #[error("Migration chain incomplete for {signal_type}: missing migration v{from} to v{to}")]
    MigrationChainGap { signal_type: String, from: u16, to: u16 },

    #[error("Protocol version mismatch: expected v{expected}, got v{got}")]
    ProtocolMismatch { expected: u16, got: u16 },

    #[error("Migration failed from v{from} to v{to} for {signal_type}: {reason}")]
    MigrationFailed { signal_type: String, from: u16, to: u16, reason: String },

    #[error("Permission denied: {action} requires {required} permission")]
    PermissionDenied { action: String, required: String },

    #[error("Correlation chain broken: {message} (correlation_id: {correlation_id})")]
    CorrelationBroken { message: String, correlation_id: String },

    #[error("Witness chain broken: {message} (cell_id: {cell_id}, witness_id: {witness_id})")]
    WitnessChainBroken { message: String, cell_id: String, witness_id: String },

    #[error("Entropy threshold exceeded: {score} > {threshold} (cell_id: {cell_id})")]
    EntropyExceeded { score: f64, threshold: f64, cell_id: String },

    #[error("Token budget exceeded for {cell_id}: used {used}, budget {budget}")]
    TokenBudgetExceeded { cell_id: String, used: u64, budget: u64 },

    #[error("Message loop detected: {message} (correlation: {correlation_id})")]
    LoopDetected { message: String, correlation_id: String },

    #[error("Timeout after {timeout_ms}ms (cell_id: {cell_id}, operation: {operation})")]
    Timeout { cell_id: String, timeout_ms: u64, operation: String },

    #[error("Resource exhausted: {resource} (cell_id: {cell_id})")]
    ResourceExhausted { resource: String, cell_id: String },

    #[error("Invalid signal type: {signal_type} (expected one of: {expected_types})")]
    InvalidSignalType { signal_type: String, expected_types: String },

    #[error("Shutdown in progress: {message}")]
    ShutdownInProgress { message: String },

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

    #[error("Witness serialization failed for {cell_id}: {message}")]
    WitnessSerialization { cell_id: String, message: String },

    #[error("Signal serialization failed for {signal_type}: {message}")]
    SignalSerialization { signal_type: String, message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },

    #[error("Lens not found: {lens_id}")]
    LensNotFound { lens_id: String },

    #[error("Lens projection error for {lens_id}: {message}")]
    LensProjectionError { lens_id: String, message: String },

    #[error("Lens access denied: cell {cell_id} cannot access lens {lens_id}")]
    LensAccessDenied { cell_id: String, lens_id: String },
}

pub type Result<T> = std::result::Result<T, AxiomError>;
