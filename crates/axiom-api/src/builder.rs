use crate::auth::AuthConfig;
use crate::logging::{init_logging, LoggingConfig};
use crate::middleware::{CorsConfig, RateLimitConfig, SecurityMiddlewareConfig};
use crate::router::{ApiServer, ApiServerConfig, ApiState};
use axiom_oversight::{OversightDataSource, OversightKernelAdapter};
use axiom_runtime::{AxiomRuntime, RuntimeDataSource};
use axiom_viz::{
    metrics::{self, PrometheusRegistry},
    MetricsRegistry,
};
use std::net::SocketAddr;
use std::sync::Arc;

pub struct ApiServerBuilder {
    addr: SocketAddr,
    auth_config: AuthConfig,
    metrics_registry: Option<Arc<dyn axiom_viz::MetricsRegistry>>,
    logging_config: LoggingConfig,
    security_config: SecurityMiddlewareConfig,
    body_limit: usize,
}

impl Default for ApiServerBuilder {
    fn default() -> Self {
        Self {
            addr: ([0, 0, 0, 0], 9092).into(),
            auth_config: AuthConfig::disabled(),
            metrics_registry: None,
            logging_config: LoggingConfig::default(),
            security_config: SecurityMiddlewareConfig::default(),
            body_limit: 10 * 1024 * 1024,
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
        self.auth_config = AuthConfig::api_key(key);
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

    pub fn logging_config(mut self, config: LoggingConfig) -> Self {
        self.logging_config = config;
        self
    }

    /// 设置速率限制配置
    pub fn rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.security_config.rate_limit = Some(config);
        self
    }

    /// 设置 CORS 配置
    pub fn cors(mut self, config: CorsConfig) -> Self {
        self.security_config.cors = Some(config);
        self
    }

    /// 设置请求体大小限制（字节）
    pub fn body_limit(mut self, limit: usize) -> Self {
        self.body_limit = limit;
        self
    }

    /// 使用开发环境默认配置
    pub fn development(mut self) -> Self {
        self.security_config = SecurityMiddlewareConfig {
            rate_limit: Some(RateLimitConfig { max_requests: 1000, window_secs: 60 }),
            cors: Some(CorsConfig::default()),
        };
        self
    }

    /// 使用生产环境默认配置
    pub fn production(mut self, allowed_origins: Vec<String>) -> Self {
        self.security_config = SecurityMiddlewareConfig {
            rate_limit: Some(RateLimitConfig { max_requests: 100, window_secs: 60 }),
            cors: Some(CorsConfig {
                allowed_origins,
                allow_credentials: true,
                ..Default::default()
            }),
        };
        self
    }

    pub fn build(
        self,
        runtime: Arc<AxiomRuntime>,
        oversight: Arc<OversightKernelAdapter>,
    ) -> ApiServer {
        init_logging(self.logging_config);

        let mut state = ApiState::new(
            runtime as Arc<dyn RuntimeDataSource>,
            oversight as Arc<dyn OversightDataSource>,
        );

        let registry = self.metrics_registry.unwrap_or_else(|| {
            let mut reg = PrometheusRegistry::new();
            let _ = reg.register_counter(metrics::message_total());
            let _ = reg.register_histogram(
                metrics::message_duration_seconds(),
                &[0.001, 0.01, 0.1, 1.0, 10.0],
            );
            let _ = reg.register_counter(metrics::cell_restarts_total());
            let _ = reg.register_gauge(metrics::entropy_score());
            let _ = reg.register_counter(metrics::witness_chain_errors());
            let _ = reg.register_counter(metrics::dead_letters_total());
            let _ = reg.register_gauge(metrics::active_cells());
            Arc::new(reg)
        });
        state = state.with_metrics_registry(registry);

        ApiServer::new(
            ApiServerConfig {
                addr: self.addr,
                body_limit: self.body_limit,
                security: self.security_config,
            },
            state,
        )
    }
}

pub async fn start_api_server(
    addr: SocketAddr,
    runtime: Arc<AxiomRuntime>,
    oversight: Arc<OversightKernelAdapter>,
) -> Result<(), std::io::Error> {
    let server = ApiServerBuilder::new().addr(addr).build(runtime, oversight);
    server.serve().await
}
