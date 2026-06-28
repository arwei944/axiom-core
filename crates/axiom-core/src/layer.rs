//! Layer labels for architectural self-constraint.
//!
//! Every Cell and Signal carries a Layer tag, enabling compile-time
//! and runtime enforcement of the call-direction rule:
//! Oversight → Agent → Validate → Exec (no reverse, no skip).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Layer {
    /// Layer 0: Oversight (supervises everything, executes no business logic)
    Oversight = 0,
    /// Layer 3: Deliberative (LLM/non-deterministic reasoning)
    Agent = 3,
    /// Layer 2: Validation (schema checks, rule engines, Axiom validation)
    Validate = 2,
    /// Layer 1: Executive (deterministic execution, DB, API, IO)
    Exec = 1,
}

impl Layer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Layer::Oversight => "oversight",
            Layer::Agent => "agent",
            Layer::Validate => "validate",
            Layer::Exec => "exec",
        }
    }

    pub fn can_send_to(&self, target: Layer) -> bool {
        match self {
            Layer::Oversight => true,
            Layer::Agent => matches!(target, Layer::Agent | Layer::Validate),
            Layer::Validate => matches!(target, Layer::Validate | Layer::Exec),
            Layer::Exec => matches!(target, Layer::Exec),
        }
    }
}

impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
