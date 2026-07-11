use crate::auth::AuthConfig;
use crate::router::{ApiServer, ApiServerConfig, ApiState};
use axiom_runtime::{AxiomRuntime, RuntimeDataSource};
use axiom_oversight::{OversightDataSource, OversightKernelAdapter};
use std::net::SocketAddr;
use std::sync::Arc;

pub struct ApiServerBuilder {
    addr: SocketAddr,
    auth_config: AuthConfig,
    metrics_registry: Option<Arc<dyn axiom_viz::MetricsRegistry>>,
}

impl Default for ApiServerBuilder {
    fn default() -> Self {
        Self {
            addr: ([0, 0, 0, 0], 9092).into(),
            auth_config: AuthConfig::disabled(),
            metrics_registry: None,
        }
    }
}

impl ApiServerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn addr(mut self, addr: SocketAddr) -> Self {
        self.addr = addr;
        self
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.auth_config = AuthConfig::with_api_key(key);
        self
    }

    pub fn auth_config(mut self, config: AuthConfig) -> Self {
        self.auth_config = config;
        self
    }

    pub fn with_metrics_registry(mut self, registry: Arc<dyn axiom_viz::MetricsRegistry>) -> Self {
        self.metrics_registry = Some(registry);
        self
    }

    pub fn build(
        self,
        runtime: Arc<AxiomRuntime>,
        oversight: Arc<OversightKernelAdapter>,
    ) -> ApiServer {
        let mut state = ApiState::new(runtime as Arc<dyn RuntimeDataSource>, oversight as Arc<dyn OversightDataSource>);
        if let Some(registry) = self.metrics_registry {
            state = state.with_metrics_registry(registry);
        }
        ApiServer::new(
            ApiServerConfig { addr: self.addr },
            state,
        )
    }
}

pub async fn start_api_server(
    addr: SocketAddr,
    runtime: Arc<AxiomRuntime>,
    oversight: Arc<OversightKernelAdapter>,
) -> Result<(), std::io::Error> {
    let server = ApiServerBuilder::new()
        .addr(addr)
        .build(runtime, oversight);
    server.serve().await
}