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

pub mod cell;
pub mod signal;
pub mod lens;
pub mod axiom;
pub mod witness;
pub mod layer;
pub mod entropy;
pub mod error;

pub use error::{AxiomError, Result};
pub use layer::Layer;
pub use entropy::EntropyScore;
