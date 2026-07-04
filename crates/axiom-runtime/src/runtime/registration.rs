use super::CellRegistration;
use axiom_core::cell::CellHandle;
use axiom_core::id::CellId;
use axiom_core::layer::Layer;
use axiom_core::version::Version;
use std::sync::Arc;

impl CellRegistration {
    pub fn new(id: CellId, layer: Layer) -> Self {
        Self {
            id,
            layer,
            version: Version::new(0, 1, 0),
            supervision_strategy: axiom_core::cell::SupervisionStrategy::default(),
            cell: None,
            factory: None,
        }
    }

    pub fn with_cell(mut self, cell: CellHandle) -> Self {
        self.cell = Some(cell);
        self
    }

    pub fn with_factory<F>(mut self, factory: F) -> Self
    where
        F: Fn() -> CellHandle + Send + Sync + 'static,
    {
        self.factory = Some(Arc::new(factory));
        self
    }
}
