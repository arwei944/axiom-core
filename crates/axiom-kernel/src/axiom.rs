use crate::heatmap::HeatmapCollector;
use crate::signal::SignalEnvelope;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;
use thiserror::Error;
use tokio::sync::RwLock;

pub type BoxedAxiom = Box<dyn DynAxiom + Send + Sync>;
pub type BoxedLens = Box<dyn DynLens + Send + Sync>;
pub type BoxedSignalHandler = Box<dyn SignalHandler + Send + Sync>;

// ============================================================
// Validation types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

impl ValidationError {
    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Error,
        }
    }

    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Warning,
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}: {}", self.severity, self.field, self.message)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn is_valid(&self) -> bool {
        self.errors.iter().all(|e| e.severity != ValidationSeverity::Error)
    }

    pub fn has_errors(&self) -> bool {
        self.errors.iter().any(|e| e.severity == ValidationSeverity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.errors.iter().any(|e| e.severity == ValidationSeverity::Warning)
    }

    pub fn extend(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
    }
}

impl std::fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let messages: Vec<String> = self.errors.iter().map(|e| format!("{}: {}", e.field, e.message)).collect();
        write!(f, "{}", messages.join("; "))
    }
}

// ============================================================
// Error types
// ============================================================

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("cell not found: {0}")]
    CellNotFound(String),

    #[error("signal validation failed: {0}")]
    SignalValidationFailed(String),

    #[error("axiom violated: {0}")]
    AxiomViolated(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("internal error: {0}")]
    InternalError(String),

    #[error("Mailbox for cell {cell_id} is full (capacity: {capacity})")]
    MailboxFull { cell_id: String, capacity: usize },

    #[error("Cell {cell_id} already exists")]
    CellAlreadyExists { cell_id: String },

    #[error("Cell {cell_id} is not running (state: {state})")]
    CellNotRunning { cell_id: String, state: String },

    #[error("Axiom invariant violated: {message}")]
    InvariantViolated { message: String },

    #[error("Schema validation failed for {signal_type}: {message}")]
    SchemaValidation {
        signal_type: String,
        message: String,
    },

    #[error("Signal validation failed for {signal_type}: {message}")]
    SignalValidation {
        signal_type: String,
        message: String,
    },

    #[error("Layer violation: {from} cannot send to {to} (signal: {signal_type}, source_cell: {source_cell})")]
    LayerViolation {
        from: crate::Layer,
        to: crate::Layer,
        signal_type: String,
        source_cell: String,
    },

    #[error("Handoff limit exceeded: message {msg_id} hopped {hops} times (max 8, correlation: {correlation_id})")]
    HandoffLimitExceeded {
        msg_id: String,
        hops: u32,
        correlation_id: String,
    },

    #[error("Cell {cell_id} heartbeat timeout (last seen {last_seen_ms}ms ago)")]
    HeartbeatTimeout { cell_id: String, last_seen_ms: u64 },

    #[error("Cell {cell_id} crashed: {message}")]
    CellCrashed { cell_id: String, message: String },

    #[error("Cell {cell_id} panicked: {message}")]
    CellPanic { cell_id: String, message: String },

    #[error("Circuit breaker open for cell {cell_id} (failures: {failures})")]
    CircuitBreak { cell_id: String, failures: u32 },

    #[error("Stale state detected for cell {cell_id}: expected version {expected}, got {actual}")]
    StaleState {
        cell_id: String,
        expected: u64,
        actual: u64,
    },

    #[error("Duplicate message {msg_id} (idempotency violation, correlation: {correlation_id})")]
    DuplicateMessage {
        msg_id: String,
        correlation_id: String,
    },

    #[error("Version incompatibility: {compatibility:?} (required: {required}, found: {found})")]
    VersionMismatch {
        compatibility: crate::Compatibility,
        required: crate::Version,
        found: crate::Version,
    },

    #[error(
        "Schema version too new for {signal_type}: found v{found}, max supported v{max_supported}"
    )]
    SchemaVersionTooNew {
        signal_type: String,
        found: u16,
        max_supported: u16,
    },

    #[error(
        "Schema version too old for {signal_type}: found v{found}, no migration path to v{current}"
    )]
    MigrationPathNotFound {
        signal_type: String,
        found: u16,
        current: u16,
    },

    #[error("Migration chain incomplete for {signal_type}: missing migration v{from} to v{to}")]
    MigrationChainGap {
        signal_type: String,
        from: u16,
        to: u16,
    },

    #[error("Protocol version mismatch: expected v{expected}, got v{got}")]
    ProtocolMismatch { expected: u16, got: u16 },

    #[error("Migration failed from v{from} to v{to} for {signal_type}: {reason}")]
    MigrationFailed {
        signal_type: String,
        from: u16,
        to: u16,
        reason: String,
    },

    #[error("Permission denied: {action} requires {required} permission")]
    PermissionDenied { action: String, required: String },

    #[error("Correlation chain broken: {message} (correlation_id: {correlation_id})")]
    CorrelationBroken {
        message: String,
        correlation_id: String,
    },

    #[error("Witness chain broken: {message} (cell_id: {cell_id}, witness_id: {witness_id})")]
    WitnessChainBroken {
        message: String,
        cell_id: String,
        witness_id: String,
    },

    #[error("Entropy threshold exceeded: {score} > {threshold} (cell_id: {cell_id})")]
    EntropyExceeded {
        score: f64,
        threshold: f64,
        cell_id: String,
    },

    #[error("Token budget exceeded for {cell_id}: used {used}, budget {budget}")]
    TokenBudgetExceeded { cell_id: String, used: u64, budget: u64 },

    #[error("Message loop detected: {message} (correlation: {correlation_id})")]
    LoopDetected {
        message: String,
        correlation_id: String,
    },

    #[error("Timeout after {timeout_ms}ms (cell_id: {cell_id}, operation: {operation})")]
    Timeout {
        timeout_ms: u64,
        cell_id: String,
        operation: String,
    },

    #[error("Resource exhausted: {resource} (cell_id: {cell_id})")]
    ResourceExhausted { resource: String, cell_id: String },

    #[error("Invalid signal type: {signal_type} (expected one of: {expected_types})")]
    InvalidSignalType {
        signal_type: String,
        expected_types: String,
    },

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
    SignalSerialization {
        signal_type: String,
        message: String,
    },

    #[error("Lens not found: {lens_id}")]
    LensNotFound { lens_id: String },

    #[error("Lens projection error for {lens_id}: {message}")]
    LensProjectionError { lens_id: String, message: String },

    #[error("Lens access denied: cell {cell_id} cannot access lens {lens_id}")]
    LensAccessDenied { cell_id: String, lens_id: String },
}

pub type KernelResult<T> = Result<T, KernelError>;

// ============================================================
// State / Message
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub data: Vec<u8>,
}

impl State {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub payload: Vec<u8>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl Message {
    pub fn new(payload: Vec<u8>) -> Self {
        Self {
            payload,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

// ============================================================
// Axiom traits
// ============================================================

pub trait Axiom: Send + Sync {
    type State: 'static;
    type Message: 'static;

    fn name(&self) -> &'static str;

    fn check(&self, current: &Self::State, new: &Self::State, msg: &Self::Message) -> KernelResult<()>;

    fn violation_action(&self) -> ViolationAction {
        ViolationAction::Reject
    }

    fn applies_to_layer(&self, _layer: crate::Layer) -> bool {
        true
    }
}

pub trait DynAxiom: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn applies_to_layer(&self, layer: crate::Layer) -> bool;
    fn violation_action(&self) -> ViolationAction;
    fn check_dyn(
        &self,
        current: &dyn std::any::Any,
        new: &dyn std::any::Any,
        msg: &dyn std::any::Any,
    ) -> KernelResult<()>;
    fn as_any(&self) -> &dyn Any;
}

impl<T: Axiom + 'static> DynAxiom for T {
    fn name(&self) -> &'static str {
        Axiom::name(self)
    }

    fn applies_to_layer(&self, layer: crate::Layer) -> bool {
        <T as Axiom>::applies_to_layer(self, layer)
    }

    fn violation_action(&self) -> ViolationAction {
        Axiom::violation_action(self)
    }

    fn check_dyn(
        &self,
        current: &dyn std::any::Any,
        new: &dyn std::any::Any,
        msg: &dyn std::any::Any,
    ) -> KernelResult<()> {
        let current = current.downcast_ref::<T::State>().ok_or_else(|| {
            KernelError::TypeMismatch {
                expected: std::any::type_name::<T::State>(),
                actual: "unknown",
            }
        })?;
        let new = new.downcast_ref::<T::State>().ok_or_else(|| {
            KernelError::TypeMismatch {
                expected: std::any::type_name::<T::State>(),
                actual: "unknown",
            }
        })?;
        let msg = msg.downcast_ref::<T::Message>().ok_or_else(|| {
            KernelError::TypeMismatch {
                expected: std::any::type_name::<T::Message>(),
                actual: "unknown",
            }
        })?;
        <T as Axiom>::check(self, current, new, msg)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationAction {
    Reject,
    Warn,
    CircuitBreak,
    Rollback,
}

// ============================================================
// Axiom chain
// ============================================================

pub struct AxiomViolation {
    pub axiom_name: &'static str,
    pub error: KernelError,
    pub action: ViolationAction,
}

pub struct DynAxiomChain {
    axioms: Vec<&'static dyn DynAxiom>,
}

impl DynAxiomChain {
    pub fn from_registry_for_layer(_layer: crate::Layer) -> Self {
        Self { axioms: Vec::new() }
    }

    pub fn from_registry_all() -> Self {
        Self { axioms: Vec::new() }
    }

    pub fn check_all(
        &self,
        _current: &dyn std::any::Any,
        _new: &dyn std::any::Any,
        _msg: &dyn std::any::Any,
    ) -> Vec<AxiomViolation> {
        Vec::new()
    }

    pub fn count(&self) -> usize {
        self.axioms.len()
    }
}

// ============================================================
// Lens traits
// ============================================================

pub trait Lens: Send + Sync {
    fn id(&self) -> &'static str;
    fn project(&self, state: &State) -> KernelResult<Projection>;
}

pub trait DynLens: Send + Sync + 'static {
    fn id(&self) -> &'static str;
    fn project(&self, state: &State) -> KernelResult<Projection>;
    fn as_any(&self) -> &dyn Any;
}

impl<T: Lens + 'static> DynLens for T {
    fn id(&self) -> &'static str {
        Lens::id(self)
    }
    fn project(&self, state: &State) -> KernelResult<Projection> {
        Lens::project(self, state)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projection {
    pub data: Vec<u8>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl Projection {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

// ============================================================
// AxiomKernel runtime
// ============================================================

pub struct AxiomKernel {
    axioms: RwLock<Vec<BoxedAxiom>>,
    heatmap: std::sync::Arc<RwLock<HeatmapCollector>>,
}

impl AxiomKernel {
    pub fn new() -> Self {
        Self {
            axioms: RwLock::new(Vec::new()),
            heatmap: std::sync::Arc::new(RwLock::new(HeatmapCollector::new())),
        }
    }

    pub fn with_heatmap(heatmap: std::sync::Arc<RwLock<HeatmapCollector>>) -> Self {
        Self {
            axioms: RwLock::new(Vec::new()),
            heatmap,
        }
    }

    pub fn heatmap(&self) -> std::sync::Arc<RwLock<HeatmapCollector>> {
        self.heatmap.clone()
    }

    pub async fn register(&self, axiom: BoxedAxiom) {
        let mut axioms = self.axioms.write().await;
        axioms.push(axiom);
    }

    pub async fn check(&self, current: &State, new: &State, msg: &Message) -> KernelResult<()> {
        let axioms = self.axioms.read().await;
        for axiom in axioms.iter() {
            axiom.check_dyn(current, new, msg)?;
        }
        drop(axioms);
        self.heatmap.write().await.record_axiom_check("axiom-chain");
        Ok(())
    }

    pub async fn count(&self) -> usize {
        self.axioms.read().await.len()
    }
}

impl Default for AxiomKernel {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Signal handler
// ============================================================

pub trait SignalHandler: Send + Sync {
    fn handle(&mut self, signal: &mut SignalEnvelope) -> KernelResult<()>;
}
