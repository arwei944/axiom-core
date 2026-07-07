//! Kernel integration for `axiom-prompt`.
//!
//! Provides adapters so prompt templates can be observed through
//! the kernel runtime.

use crate::registry::TemplateRegistry;

/// Adapter that exposes a `TemplateRegistry` through the kernel runtime.
pub struct PromptKernelAdapter {
    registry: TemplateRegistry,
}

impl PromptKernelAdapter {
    pub fn new(registry: TemplateRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &TemplateRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_wraps_registry() {
        let registry = TemplateRegistry::new();
        let adapter = PromptKernelAdapter::new(registry);
        assert!(adapter.registry().list_templates().is_empty());
    }
}
