//! Kernel integration for `axiom-oversight`.
//!
//! Provides adapters so oversight governance can consume kernel witness
//! chains and entropy state.

use crate::entropy_governor::{EntropyGovernorCell, GovernanceAction};

/// Adapter that exposes `EntropyGovernorCell` through the kernel runtime.
pub struct OversightKernelAdapter {
    governor: EntropyGovernorCell,
}

impl OversightKernelAdapter {
    pub fn new(governor: EntropyGovernorCell) -> Self {
        Self { governor }
    }

    pub fn governor(&self) -> &EntropyGovernorCell {
        &self.governor
    }

    pub fn evaluate(&self) -> Vec<GovernanceAction> {
        let action = self.governor.take_action();
        vec![action]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_wraps_governor() {
        let governor = EntropyGovernorCell::default();
        let adapter = OversightKernelAdapter::new(governor);
        let actions = adapter.evaluate();
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], GovernanceAction::None));
    }
}
