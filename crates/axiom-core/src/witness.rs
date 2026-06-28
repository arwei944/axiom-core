//! Witness - Immutable audit record for every state transition.
//!
//! Every state transition automatically produces a Witness, forming an
//! append-only audit chain. This enables post-hoc analysis: "Why did we
//! enter this state?"

use serde::{Deserialize, Serialize};
use crate::signal::VectorClock;

/// Hash for witness chain integrity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessHash(pub [u8; 32]);

/// An immutable record of a state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    /// Unique witness ID.
    pub witness_id: String,
    /// Cell that produced this transition.
    pub cell_id: String,
    /// Correlation ID from the triggering signal.
    pub correlation_id: String,
    /// Vector clock after this transition.
    pub vector_clock: VectorClock,
    /// Timestamp (nanoseconds since UNIX epoch).
    pub timestamp_ns: u64,
    /// Hash of previous witness (chain integrity).
    pub prev_hash: Option<WitnessHash>,
    /// Hash of this witness.
    pub hash: WitnessHash,
    /// Human-readable description of what happened (no secrets!).
    pub summary: String,
    /// Whether this transition was successful or failed.
    pub outcome: TransitionOutcome,
}

/// Outcome of a state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionOutcome {
    Success,
    Failed { reason: String },
    AxiomViolated { axiom_name: String, message: String },
}
