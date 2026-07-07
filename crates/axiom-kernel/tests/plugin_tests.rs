use axiom_kernel::plugin::abi::PluginContext;

#[tokio::test]
async fn test_plugin_context_creation() {
    let ctx = PluginContext::new(
        axiom_kernel::CellKernel::new(),
        axiom_kernel::SignalKernel::new(),
        axiom_kernel::LensKernel::new(),
        axiom_kernel::AxiomKernel::new(),
        axiom_kernel::WitnessKernel::new(),
        axiom_kernel::PluginRegistry::new(),
        std::sync::Arc::new(tokio::sync::RwLock::new(axiom_kernel::HeatmapCollector::new())),
    );
    assert_eq!(ctx.cells.count().await, 0);
}
