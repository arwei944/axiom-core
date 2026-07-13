//! Runtime tier labels for architectural self-constraint.
//!
//! Every Cell and Signal carries a RuntimeTier tag, enabling compile-time
//! and runtime enforcement of the call-direction rule:
//! Oversight -> Agent -> Validate -> Exec (no reverse, no skip).
//!
//! Note: This is distinct from "Crate Layer" (defined in .axiom/architecture.toml),
//! which is a compile-time crate dependency hierarchy. RuntimeTier is a runtime
//! concept for Cell and Signal routing.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum RuntimeTier {
    /// Tier 0: Oversight (supervises everything, executes no business logic)
    Oversight = 0,
    /// Tier 3: Deliberative (LLM/non-deterministic reasoning)
    Agent = 3,
    /// Tier 2: Validation (schema checks, rule engines, Axiom validation)
    Validate = 2,
    /// Tier 1: Executive (deterministic execution, DB, API, IO)
    Exec = 1,
}

impl RuntimeTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeTier::Oversight => "oversight",
            RuntimeTier::Agent => "agent",
            RuntimeTier::Validate => "validate",
            RuntimeTier::Exec => "exec",
        }
    }

    /// Check if a signal can be sent from this tier to the target tier.
    ///
    /// Rules:
    /// - Oversight can send to any tier (supervises everything)
    /// - Agent can send to Agent or Validate (deliberative tier)
    /// - Validate can send to Validate, Exec, or Agent (validation tier)
    /// - Exec can only send to Exec (execution tier, no reverse)
    pub fn can_send_to(&self, target: RuntimeTier) -> bool {
        match self {
            RuntimeTier::Oversight => true,
            RuntimeTier::Agent => matches!(target, RuntimeTier::Agent | RuntimeTier::Validate),
            RuntimeTier::Validate => {
                matches!(target, RuntimeTier::Validate | RuntimeTier::Exec | RuntimeTier::Agent)
            }
            RuntimeTier::Exec => matches!(target, RuntimeTier::Exec),
        }
    }
}

impl std::fmt::Display for RuntimeTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Backwards compatibility alias - deprecated, use RuntimeTier instead
#[deprecated(
    since = "0.4.0",
    note = "Use RuntimeTier instead. Layer was renamed to avoid confusion with Crate Layer."
)]
pub type Layer = RuntimeTier;
