//! Axiom Core - 5 fundamental primitives for reliable agentic systems.
//!
//! # Primitives
//! - **Cell**: Isolated stateful unit with private state + message mailbox
//! - **Signal**: Typed immutable message with causal tracking
//! - **Lens**: On-demand state projection from event log
//! - **Axiom**: Global invariant constraints for entropy control
//! - **Witness**: Immutable audit record for every state transition
//!
//! # Architecture
//! - **Layer**: Four-layer architecture (Oversight/Agent/Validate/Exec) with enforced call direction
//! - **Entropy**: First-class entropy metrics for system disorder quantification
//! - **Version**: Semantic versioning, schema versioning, and witness chain compatibility
//! - **Schema**: Compile-time message validation
//! - **Context**: CellContext with correlation ID propagation

#![allow(async_fn_in_trait)]

pub mod axiom;
pub mod cell;
pub mod context;
pub mod entropy;
pub mod error;
pub mod gate;
pub mod id;
pub mod layer;
pub mod registry;
pub mod schema;
pub mod sealed;
pub mod signal;
pub mod version;
pub mod witness;

pub use axiom::{Axiom, DynAxiom, DynAxiomChain, ViolationAction};
pub use entropy::{
    CellEntropy, EntropyLevel, EntropyScore, EntropySnapshot, EntropyWeights, CRITICAL_THRESHOLD,
    GREEN_THRESHOLD, RED_THRESHOLD, YELLOW_THRESHOLD,
};
pub use error::{AxiomError, Result};
pub use id::{AxiomId, CellId, CorrelationId, LensId, MsgId, TraceId, WitnessId};
pub use layer::Layer;
pub use registry::{
    count_registered_axioms, registered_axioms, registered_migration_chains,
    verify_migration_chain_completeness,
};
pub use schema::{Schema, ValidationResult};
pub use sealed::{
    can_send_at_runtime, AgentLayer, CanSendTo, ExecLayer, LayerMarker, OversightLayer,
    ValidateLayer,
};
pub use signal::{Signal, SignalEnvelope, SignalKind, VectorClock};
pub use version::{
    Compatibility, IdentityVersion, Migration, ProtocolVersion, SchemaMigrator, SchemaVersion,
    Version, VersionInfo, Versioned,
};
pub use witness::{
    TransitionOutcome, Witness, WitnessBatch, WitnessBuilder, WitnessHash, WitnessMetrics,
};

pub use axiom_macros::{axiom, cell, migration, schema_version, SignalPayload};
pub use linkme;

#[cfg(feature = "unstable")]
pub use cell::{
    BoxHandleFuture, CellHandle, CellHealth, CellMeta, DynCell, DynHandleCell, ExecCell,
    LayerOf, OversightCell, AgentCell, ValidateCell, SupervisionStrategy,
};
