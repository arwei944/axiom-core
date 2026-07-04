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
use crate::signal::Signal;
use crate::Result;
use serde::{Deserialize, Serialize};

pub trait Guard: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn layer(&self) -> Option<Layer>;
    fn check(&self, signal: &dyn Signal) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationAction {
    Reject,
    Warn,
    CircuitBreak,
    Rollback,
}

pub trait Axiom: Send + Sync {
    type State: 'static;
    type Message: 'static;

    fn name(&self) -> &'static str;

    fn check(&self, current: &Self::State, new: &Self::State, msg: &Self::Message) -> Result<()>;

    fn violation_action(&self) -> ViolationAction {
        ViolationAction::Reject
    }

    fn applies_to_layer(&self, _layer: Layer) -> bool {
        true
    }
}

/// Object-safe trait for runtime axiom dispatch.
/// This is automatically implemented by the `#[axiom]` macro for all Axiom types.
pub trait DynAxiom: Send + Sync {
    fn name(&self) -> &'static str;
    fn applies_to_layer(&self, layer: Layer) -> bool;
    fn violation_action(&self) -> ViolationAction;
    fn check_dyn(
        &self,
        current: &dyn std::any::Any,
        new: &dyn std::any::Any,
        msg: &dyn std::any::Any,
    ) -> Result<()>;
}

pub struct AxiomViolation {
    pub axiom_name: &'static str,
    pub error: AxiomError,
    pub action: ViolationAction,
}

/// Dynamic axiom chain built from the distributed registry at runtime.
/// Used by ArchitectureGuardian to check all registered axioms for a given layer.
pub struct DynAxiomChain {
    axioms: Vec<&'static dyn DynAxiom>,
}

impl DynAxiomChain {
    pub fn from_registry_for_layer(layer: Layer) -> Self {
        let axioms: Vec<&'static dyn DynAxiom> = crate::registry::AXIOM_REGISTRY
            .iter()
            .copied()
            .filter(|a| a.applies_to_layer(layer))
            .collect();
        Self { axioms }
    }

    pub fn from_registry_all() -> Self {
        let axioms: Vec<&'static dyn DynAxiom> =
            crate::registry::AXIOM_REGISTRY.iter().copied().collect();
        Self { axioms }
    }

    pub fn check_all(
        &self,
        current: &dyn std::any::Any,
        new: &dyn std::any::Any,
        msg: &dyn std::any::Any,
    ) -> Vec<AxiomViolation> {
        self.axioms
            .iter()
            .filter_map(|a| match a.check_dyn(current, new, msg) {
                Ok(()) => None,
                Err(crate::AxiomError::TypeMismatch { .. }) => None,
                Err(e) => Some(AxiomViolation {
                    axiom_name: a.name(),
                    error: e,
                    action: a.violation_action(),
                }),
            })
            .collect()
    }

    pub fn count(&self) -> usize {
        self.axioms.len()
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
        // Test individual axioms directly (no typed chain needed)
        let non_empty = NonEmpty;
        let max_len = MaxLength(10);

        // NonEmpty rejects empty input
        assert!(non_empty
            .check(&"hello".into(), &"".into(), &"set".into())
            .is_err());
        assert_eq!(non_empty.violation_action(), ViolationAction::Reject);

        // MaxLength rejects input that's too long
        assert!(max_len
            .check(&"".into(), &"this is way too long".into(), &"set".into())
            .is_err());
        assert_eq!(max_len.violation_action(), ViolationAction::Warn);
    }

    #[test]
    fn test_axiom_chain_passes() {
        let non_empty = NonEmpty;
        let max_len = MaxLength(10);

        assert!(non_empty
            .check(&"".into(), &"ok".into(), &"set".into())
            .is_ok());
        assert!(max_len
            .check(&"".into(), &"ok".into(), &"set".into())
            .is_ok());
    }

    #[test]
    fn test_warn_only_no_reject() {
        let max_len = MaxLength(5);
        let result = max_len.check(&"".into(), &"this is way too long".into(), &"set".into());
        assert!(result.is_err());
        // Warn actions should not reject (just warn)
        assert_eq!(max_len.violation_action(), ViolationAction::Warn);
    }

    #[test]
    fn test_dyn_axiom_chain_from_registry() {
        let chain = DynAxiomChain::from_registry_all();
        let _ = chain.count();
    }
}
