use super::CellRegistration;
use axiom_kernel::cell::RuntimeCellHandle;
use axiom_kernel::cell::SupervisionStrategy;
use axiom_kernel::id::CellId;
use axiom_kernel::layer::Layer;
use axiom_kernel::version::Version;
use std::sync::Arc;

impl CellRegistration {
    pub fn new(id: CellId, layer: Layer) -> Self {
        Self {
            id,
            layer,
            version: Version::new(0, 1, 0),
            supervision_strategy: SupervisionStrategy::default(),
            cell: None,
            factory: None,
        }
    }

    pub fn with_cell(mut self, cell: RuntimeCellHandle) -> Self {
        self.cell = Some(cell);
        self
    }

    pub fn with_factory<F>(mut self, factory: F) -> Self
    where
        F: Fn() -> RuntimeCellHandle + Send + Sync + 'static,
    {
        self.factory = Some(Arc::new(factory));
        self
    }
}
