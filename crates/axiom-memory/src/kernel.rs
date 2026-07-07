//! Kernel integration for `axiom-memory`.
//!
//! Provides adapters so memory operations can be recorded as witnesses
//! and routed through the kernel runtime.

use crate::memory::WorkingMemory;

/// Adapter that exposes a `WorkingMemory` through the kernel runtime.
pub struct MemoryKernelAdapter {
    memory: WorkingMemory,
}

impl MemoryKernelAdapter {
    pub fn new(memory: WorkingMemory) -> Self {
        Self { memory }
    }

    pub fn memory(&self) -> &WorkingMemory {
        &self.memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_wraps_memory() {
        let memory = WorkingMemory::new(1000);
        let adapter = MemoryKernelAdapter::new(memory);
        assert_eq!(adapter.memory().item_count(), 0);
    }
}
