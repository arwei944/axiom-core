//! Kernel integration for `axiom-planner`.
//!
//! Provides adapters so planning results can be emitted as kernel signals
//! and recorded as witnesses.

use crate::planner::Planner;

/// Adapter that exposes a `dyn Planner` through the kernel runtime.
pub struct PlannerKernelAdapter {
    planner: Box<dyn Planner>,
}

impl PlannerKernelAdapter {
    pub fn new(planner: Box<dyn Planner>) -> Self {
        Self { planner }
    }

    pub fn planner(&self) -> &dyn Planner {
        self.planner.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_wraps_planner() {
        let planner: Box<dyn Planner> = Box::new(crate::react::ReActPlanner::new());
        let adapter = PlannerKernelAdapter::new(planner);
        assert!(!adapter.planner().name().is_empty());
    }
}
