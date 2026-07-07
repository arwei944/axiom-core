//! Kernel integration for `axiom-tool`.
//!
//! Provides adapters so tool invocations can be recorded as witnesses
//! and dispatched through the kernel runtime.

use crate::registry::ToolRegistry;

/// Adapter that exposes a `ToolRegistry` through the kernel runtime.
pub struct ToolKernelAdapter {
    registry: ToolRegistry,
}

impl ToolKernelAdapter {
    pub fn new(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_wraps_registry() {
        let registry = ToolRegistry::new();
        let adapter = ToolKernelAdapter::new(registry);
        assert!(adapter.registry().list().is_empty());
    }
}
