//! Witness - Immutable audit record for every state transition.
//!
//! Every state transition automatically produces a Witness, forming an
//! append-only SHA-256 hash chain.

pub mod batch;
pub mod builder;
pub mod def;
pub mod hash;
pub mod methods;

pub use batch::WitnessBatch;
pub use builder::WitnessBuilder;
pub use def::{
    TransitionOutcome, Witness, WitnessEvent, WitnessGenerator, WitnessHash, WitnessKind,
    WitnessMetrics,
};
pub use hash::{compute_signal_fingerprint, truncate};
