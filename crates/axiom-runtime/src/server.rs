//! HTTP server for metrics and health endpoints.
//!
//! Provides:
//! - `GET /metrics` — Prometheus metrics text format
//! - `GET /health` — JSON health status

use std::sync::Arc;

#[cfg(feature = "metrics")]
use axum::{
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use serde::Serialize;

/// Metric type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

/// Metric description.
#[derive(Debug, Clone)]
pub struct MetricDesc {
    pub name: String,
    pub help: String,
    pub metric_type: MetricType,
    pub labels: Vec<String>,
}

/// Metrics registry abstraction.
pub trait MetricsRegistry: Send + Sync {
    fn encode(&self) -> String;
    fn describe(&self) -> Vec<MetricDesc>;
}

/// No-op metrics registry.
#[derive(Debug, Default)]
pub struct NoopRegistry;

impl MetricsRegistry for NoopRegistry {
    fn encode(&self) -> String {
        String::new()
    }

    fn describe(&self) -> Vec<MetricDesc> {
        Vec::new()
    }
}

#[cfg(feature = "metrics")]
pub struct PrometheusRegistry {
    registry: prometheus::Registry,
}

#[cfg(feature = "metrics")]
impl PrometheusRegistry {
    pub fn new() -> Self {
        Self {
            registry: prometheus::Registry::new(),
        }
    }
}

#[cfg(feature = "metrics")]
impl MetricsRegistry for PrometheusRegistry {
    fn encode(&self) -> String {
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).ok();
        String::from_utf8(buffer).unwrap_or_default()
    }

    fn describe(&self) -> Vec<MetricDesc> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub cells_running: u64,
    pub preflight_passed: bool,
}

impl Default for HealthResponse {
    fn default() -> Self {
        Self {
            status: "ok",
            cells_running: 0,
            preflight_passed: false,
        }
    }
}

/// Metrics server handle.
pub struct MetricsServer {
    #[cfg(feature = "metrics")]
    registry: Arc<dyn MetricsRegistry>,
    #[cfg(feature = "metrics")]
    health: Arc<parking_lot::RwLock<HealthResponse>>,
    #[cfg(feature = "metrics")]
    _handle: Option<tokio::task::JoinHandle<()>>,
}

#[cfg(feature = "metrics")]
impl MetricsServer {
    pub fn new(
        registry: Arc<dyn MetricsRegistry>,
        health: Arc<parking_lot::RwLock<HealthResponse>>,
    ) -> Self {
        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/health", get(health_handler))
            .with_state(Arc::new(MetricsState {
                registry: registry.clone(),
                health: health.clone(),
            }));

        let listener = tokio::net::TcpListener::bind("0.0.0.0:9090")
            .await
            .expect("bind metrics server");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });

        Self {
            registry,
            health,
            _handle: Some(handle),
        }
    }

    pub fn update_health(&self, response: HealthResponse) {
        *self.health.write() = response;
    }
}

#[cfg(not(feature = "metrics"))]
impl MetricsServer {
    pub fn new(
        _registry: Arc<dyn MetricsRegistry>,
        _health: Arc<parking_lot::RwLock<HealthResponse>>,
    ) -> Self {
        Self {}
    }

    pub fn update_health(&self, _response: HealthResponse) {}
}

impl Default for MetricsServer {
    fn default() -> Self {
        Self::new(
            Arc::new(NoopRegistry),
            Arc::new(parking_lot::RwLock::new(HealthResponse::default())),
        )
    }
}

#[cfg(feature = "metrics")]
#[derive(Clone)]
struct MetricsState {
    registry: Arc<dyn MetricsRegistry>,
    health: Arc<parking_lot::RwLock<HealthResponse>>,
}

#[cfg(feature = "metrics")]
async fn metrics_handler(
    state: axum::extract::State<Arc<MetricsState>>,
) -> Response {
    let body = state.registry.encode();
    Response::builder()
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(body.into_response().into_body())
        .unwrap_or_else(|_| body.into_response())
}

#[cfg(feature = "metrics")]
async fn health_handler(
    state: axum::extract::State<Arc<MetricsState>>,
) -> Json<HealthResponse> {
    let health = state.health.read().clone();
    Json(health)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_health_response_is_ok() {
        let response = HealthResponse::default();
        assert_eq!(response.status, "ok");
        assert_eq!(response.cells_running, 0);
        assert!(!response.preflight_passed);
    }

    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn metrics_server_responds() {
        let registry = Arc::new(PrometheusRegistry::new());
        let health = Arc::new(parking_lot::RwLock::new(HealthResponse::default()));
        let _server = MetricsServer::new(registry, health);
    }
}
