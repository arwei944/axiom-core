use crate::AxiomRuntime;
use crate::RuntimeBuilder;
use crate::RuntimeConfig;

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self { config: RuntimeConfig::default(), auto_register_builtin_interceptors: true }
    }

    pub fn with_config(mut self, config: RuntimeConfig) -> Self {
        self.config = config;
        self
    }

    pub fn mailbox_capacity(mut self, cap: usize) -> Self {
        self.config.mailbox_capacity = cap;
        self
    }

    pub fn auto_register_builtins(mut self, b: bool) -> Self {
        self.auto_register_builtin_interceptors = b;
        self
    }

    pub fn build(self) -> AxiomRuntime {
        let rt = AxiomRuntime::new(self.config);
        rt.auto_interceptors
            .store(self.auto_register_builtin_interceptors, std::sync::atomic::Ordering::Relaxed);
        rt
    }
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
