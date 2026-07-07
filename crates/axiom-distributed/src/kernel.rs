//! Kernel integration for `axiom-distributed`.
//!
//! Provides adapters so distributed state can be observed through
//! the kernel runtime.

use crate::cluster::ClusterView;

/// Adapter that exposes a `ClusterView` through the kernel runtime.
pub struct DistributedKernelAdapter {
    view: ClusterView,
}

impl DistributedKernelAdapter {
    pub fn new(view: ClusterView) -> Self {
        Self { view }
    }

    pub fn view(&self) -> &ClusterView {
        &self.view
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_wraps_cluster_view() {
        let view = ClusterView::default();
        let adapter = DistributedKernelAdapter::new(view);
        assert!(adapter.view().is_empty());
    }
}
