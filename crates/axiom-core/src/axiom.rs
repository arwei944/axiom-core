//! Axiom - Global invariant constraints for entropy control.
//!
//! Axioms are deterministic pure functions (no async, no IO) that validate
//! state transitions. They act as "entropy reducers" that detect when the
//! system is drifting from its invariants.
//!
//! Axiom violations can trigger: Reject, Warn, CircuitBreak, or Rollback.
//! Layer-aware axiom chains can enforce different constraints per layer.

use crate::error::AxiomError;
use crate::layer::Layer;
use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationAction {
    Reject,
    Warn,
    CircuitBreak,
    Rollback,
}

pub trait Axiom: Send + Sync {
    type State;
    type Message;

    fn name(&self) -> &'static str;

    fn check(&self, current: &Self::State, new: &Self::State, msg: &Self::Message) -> Result<()>;

    fn violation_action(&self) -> ViolationAction {
        ViolationAction::Reject
    }

    fn applies_to_layer(&self, _layer: Layer) -> bool {
        true
    }
}

pub struct AxiomViolation {
    pub axiom_name: &'static str,
    pub error: AxiomError,
    pub action: ViolationAction,
}

pub struct AxiomChain<T, M> {
    axioms: Vec<Box<dyn Axiom<State = T, Message = M>>>,
}

impl<T, M> AxiomChain<T, M> {
    pub fn new() -> Self {
        Self { axioms: Vec::new() }
    }

    pub fn push<A: Axiom<State = T, Message = M> + 'static>(mut self, axiom: A) -> Self {
        self.axioms.push(Box::new(axiom));
        self
    }

    pub fn check_all(&self, current: &T, new: &T, msg: &M) -> Vec<AxiomViolation> {
        self.check_for_layer(current, new, msg, None)
    }

    pub fn check_for_layer(
        &self,
        current: &T,
        new: &T,
        msg: &M,
        layer: Option<Layer>,
    ) -> Vec<AxiomViolation> {
        self.axioms
            .iter()
            .filter(|a| layer.is_none() || layer.is_some_and(|l| a.applies_to_layer(l)))
            .filter_map(|a| {
                a.check(current, new, msg).err().map(|e| AxiomViolation {
                    axiom_name: a.name(),
                    error: e,
                    action: a.violation_action(),
                })
            })
            .collect()
    }

    pub fn has_reject_violations(&self, violations: &[AxiomViolation]) -> bool {
        violations
            .iter()
            .any(|v| v.action == ViolationAction::Reject)
    }
}

impl<T, M> Default for AxiomChain<T, M> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NonEmpty;
    impl Axiom for NonEmpty {
        type State = String;
        type Message = String;
        fn name(&self) -> &'static str {
            "non-empty"
        }
        fn check(&self, _current: &String, new: &String, _msg: &String) -> Result<()> {
            if new.is_empty() {
                Err(AxiomError::InvariantViolated {
                    message: "state is empty".into(),
                })
            } else {
                Ok(())
            }
        }
    }

    struct MaxLength(usize);
    impl Axiom for MaxLength {
        type State = String;
        type Message = String;
        fn name(&self) -> &'static str {
            "max-length"
        }
        fn check(&self, _current: &String, new: &String, _msg: &String) -> Result<()> {
            if new.len() > self.0 {
                Err(AxiomError::InvariantViolated {
                    message: format!("too long: {}", new.len()),
                })
            } else {
                Ok(())
            }
        }
        fn violation_action(&self) -> ViolationAction {
            ViolationAction::Warn
        }
    }

    #[test]
    fn test_axiom_chain_rejects() {
        let chain = AxiomChain::<String, String>::new()
            .push(NonEmpty)
            .push(MaxLength(10));
        let violations = chain.check_all(&"hello".into(), &"".into(), &"set".into());
        assert!(!violations.is_empty());
        assert!(chain.has_reject_violations(&violations));
    }

    #[test]
    fn test_axiom_chain_passes() {
        let chain = AxiomChain::<String, String>::new()
            .push(NonEmpty)
            .push(MaxLength(10));
        let violations = chain.check_all(&"".into(), &"ok".into(), &"set".into());
        assert!(violations.is_empty());
    }

    #[test]
    fn test_warn_only_no_reject() {
        let chain = AxiomChain::<String, String>::new().push(MaxLength(5));
        let violations = chain.check_all(&"".into(), &"this is way too long".into(), &"set".into());
        assert_eq!(violations.len(), 1);
        assert!(!chain.has_reject_violations(&violations));
    }
}
