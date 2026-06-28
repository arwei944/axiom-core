//! Axiom - Global invariant constraints for entropy control.
//!
//! Axioms are deterministic pure functions that validate state transitions.
//! They act as "entropy reducers" that detect when the system is drifting.

use crate::Result;
use serde::{Deserialize, Serialize};

/// Violation action when an axiom is broken.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ViolationAction {
    /// Reject the state transition.
    Reject,
    /// Log a warning but allow the transition.
    Warn,
    /// Trigger circuit breaker (pause the cell/supervisor).
    CircuitBreak,
    /// Roll back to last valid state.
    Rollback,
}

/// An Axiom is an invariant constraint on state transitions.
pub trait Axiom: Send + Sync {
    /// Type of state this axiom validates.
    type State;
    /// Type of signal/command being applied.
    type Message;

    /// Axiom name (for logging/metrics).
    fn name(&self) -> &'static str;

    /// Validate whether applying `message` to `current_state` (producing `new_state`) is valid.
    fn check(&self, current_state: &Self::State, new_state: &Self::State, message: &Self::Message) -> Result<()>;

    /// Action to take when this axiom is violated.
    fn violation_action(&self) -> ViolationAction {
        ViolationAction::Reject
    }
}

/// Chain multiple axioms together - all must pass.
pub struct AxiomChain<T, M> {
    axioms: Vec<Box<dyn Axiom<State = T, Message = M>>>,
}

impl<T, M> AxiomChain<T, M> {
    pub fn new() -> Self {
        Self { axioms: Vec::new() }
    }

    pub fn add<A: Axiom<State = T, Message = M> + 'static>(mut self, axiom: A) -> Self {
        self.axioms.push(Box::new(axiom));
        self
    }

    pub fn check_all(&self, current: &T, new: &T, msg: &M) -> Vec<(&'static str, crate::AxiomError)> {
        self.axioms
            .iter()
            .filter_map(|a| {
                a.check(current, new, msg)
                    .err()
                    .map(|e| (a.name(), e))
            })
            .collect()
    }
}
