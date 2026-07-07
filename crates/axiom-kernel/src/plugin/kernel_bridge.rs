//! Kernel bridge for consumers that want a single entrypoint to `axiom-kernel` primitives.
//!
//! This lives in `axiom-kernel` so that higher-level crates like `axiom-cli`
//! can assemble the full kernel stack without depending on `axiom-runtime`.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    AxiomKernel, CellKernel, HeatmapCollector, LensKernel, PluginRegistry, SignalKernel,
    WitnessKernel,
};

/// Bundled `axiom-kernel` primitives plus a shared heatmap.
pub struct RuntimeKernelBridge {
    pub cell_kernel: Arc<CellKernel>,
    pub signal_kernel: Arc<SignalKernel>,
    pub lens_kernel: Arc<LensKernel>,
    pub axiom_kernel: Arc<AxiomKernel>,
    pub witness_kernel: Arc<WitnessKernel>,
    pub plugin_registry: Arc<PluginRegistry>,
    pub heatmap: Arc<RwLock<HeatmapCollector>>,
}

impl RuntimeKernelBridge {
    pub fn new() -> Self {
        let heatmap = Arc::new(RwLock::new(HeatmapCollector::new()));
        Self {
            cell_kernel: Arc::new(CellKernel::with_heatmap(heatmap.clone())),
            signal_kernel: Arc::new(SignalKernel::with_heatmap(heatmap.clone())),
            lens_kernel: Arc::new(LensKernel::with_heatmap(heatmap.clone())),
            axiom_kernel: Arc::new(AxiomKernel::with_heatmap(heatmap.clone())),
            witness_kernel: Arc::new(WitnessKernel::with_heatmap(heatmap.clone())),
            plugin_registry: Arc::new(PluginRegistry::new()),
            heatmap,
        }
    }

    pub async fn heatmap_snapshot(&self) -> HeatmapCollector {
        self.heatmap.read().await.clone()
    }
}

impl Default for RuntimeKernelBridge {
    fn default() -> Self {
        Self::new()
    }
}
