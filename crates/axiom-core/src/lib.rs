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
pub mod id;
pub mod layer;
pub mod lens;
pub mod schema;
pub mod signal;
pub mod version;
pub mod witness;

pub use entropy::EntropyScore;
pub use error::{AxiomError, Result};
pub use id::{AxiomId, CellId, CorrelationId, LensId, MsgId, TraceId, WitnessId};
pub use layer::Layer;
pub use schema::Schema;
pub use version::{
    Compatibility, CrateVersion, IdentityVersion, Migration, MigrationRegistry, ProtocolVersion,
    SchemaVersion, Version, VersionInfo, Versioned,
};
