//! Axiom Runtime - main entry point.

pub struct AxiomRuntime {
    _initialized: bool,
}

impl AxiomRuntime {
    pub fn new() -> Self {
        Self { _initialized: true }
    }

    pub async fn start(&self) {
        tracing::info!("AxiomRuntime starting...");
    }
}

impl Default for AxiomRuntime {
    fn default() -> Self {
        Self::new()
    }
}
