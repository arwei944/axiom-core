//! Kernel integration for `axiom-identity`.
//!
//! Provides adapters so identity and skill state can be observed through
//! the kernel runtime.

use crate::identity::AgentIdentity;

/// Adapter that exposes an `AgentIdentity` through the kernel runtime.
pub struct IdentityKernelAdapter {
    identity: AgentIdentity,
}

impl IdentityKernelAdapter {
    pub fn new(identity: AgentIdentity) -> Self {
        Self { identity }
    }

    pub fn identity(&self) -> &AgentIdentity {
        &self.identity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_wraps_identity() {
        let identity = AgentIdentity::new("test", "Test Agent");
        let adapter = IdentityKernelAdapter::new(identity);
        assert_eq!(adapter.identity().id, "test");
    }
}
