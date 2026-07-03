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
//!
//! # Quick Start
//!
//! ```rust
//! use axiom_core::{Axiom, Result, CellId, Layer};
//! use axiom_core::signal::{Signal, SignalEnvelope, SignalKind, VectorClock};
//!
//! // Define a custom axiom
//! struct NonEmpty;
//! impl Axiom for NonEmpty {
//!     type State = String;
//!     type Message = String;
//!     fn name(&self) -> &'static str { "non-empty" }
//!     fn check(&self, _current: &String, new: &String, _msg: &String) -> Result<()> {
//!         if new.is_empty() {
//!             Err(axiom_core::AxiomError::InvariantViolated {
//!                 message: "state is empty".into(),
//!             })
//!         } else {
//!             Ok(())
//!         }
//!     }
//! }
//! ```
//!
//! # Crate Features
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `unstable` | No | Enable unstable APIs |
//! | `sha2-id` | No | Enable SHA-2 witness hashing |
//!
//! # Version
//!
//! Current version: **v0.2.0**
//!
//! See [VERSIONING.md](../VERSIONING.md) for versioning policy.
//! See [API_BOUNDARY.md](../API_BOUNDARY.md) for stable API surface.

#![allow(async_fn_in_trait)]

pub mod axiom;
pub mod capability;
pub mod cell;
pub mod context;
pub mod entropy;
pub mod error;
#[doc(hidden)]
pub mod gate;
pub mod id;
pub mod layer;
pub mod lens;
pub mod registry;
pub mod schema;
pub mod sealed;
pub mod signal;
pub mod version;
pub mod witness;

pub use axiom::{Axiom, DynAxiom, DynAxiomChain, Guard, ViolationAction};
pub use capability::{
    CapabilityDescriptor, CapabilityDimension, CapabilityVersionRegistry, CAPABILITY_REGISTRY,
    CAPABILITY_VERSION_REGISTRY,
};
pub use entropy::{
    CellEntropy, EntropyLevel, EntropyScore, EntropySnapshot, EntropyWeights, CRITICAL_THRESHOLD,
    GREEN_THRESHOLD, RED_THRESHOLD, YELLOW_THRESHOLD,
};
pub use error::{AxiomError, Result};
pub use id::{AxiomId, CellId, CorrelationId, LensId, MsgId, TraceId, WitnessId};
pub use layer::Layer;
pub use registry::{
    count_registered_axioms, registered_axioms, registered_migration_chains,
    verify_migration_chain_completeness, WITNESS_REGISTRY,
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
    TransitionOutcome, Witness, WitnessBatch, WitnessBuilder, WitnessEvent, WitnessGenerator,
    WitnessHash, WitnessKind, WitnessMetrics,
};
pub use lens::{
    CacheMetrics, DependencyCycleError, IncrementalProjectionCache, InMemoryProjectionCache, Lens,
    LensAccessor, LensAccessError, LensEvent, LensError, LensRegistry, LENS_REGISTRY, Projectable,
    Projection, ProjectionCache, ProjectionDowncastError,
};

pub use axiom_macros::{axiom, capability, cell, guard, lens, migration, schema_version, signal, SignalPayload, tool};

#[cfg(feature = "unstable")]
pub use cell::{
    BoxHandleFuture, CellHandle, CellHealth, CellMeta, DynCell, DynHandleCell, ExecCell,
    LayerOf, OversightCell, AgentCell, ValidateCell, SupervisionStrategy,
};
