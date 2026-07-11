pub mod v1;

use axum::{Router, serve, middleware};
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use axum::response::{IntoResponse, Json};
use axum::body::Body;
use crate::aggregator::{CellAggregator, EntropyAggregator, HealthAggregator, HeatmapAggregator};
use axiom_runtime::RuntimeDataSource;
use axiom_oversight::OversightDataSource;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct ApiState {
    health_aggregator: Arc<HealthAggregator>,
    cell_aggregator: Arc<CellAggregator>,
    heatmap_aggregator: Arc<HeatmapAggregator>,
    entropy_aggregator: Arc<EntropyAggregator>,
    metrics_registry: Option<Arc<dyn axiom_viz::MetricsRegistry>>,
}

impl ApiState {
    pub fn new(
        runtime_data: Arc<dyn RuntimeDataSource>,
        oversight_data: Arc<dyn OversightDataSource>,
    ) -> Self {
        Self {
            health_aggregator: Arc::new(HealthAggregator::new(runtime_data.clone(), oversight_data)),
            cell_aggregator: Arc::new(CellAggregator::new(runtime_data.clone())),
            heatmap_aggregator: Arc::new(HeatmapAggregator::new(runtime_data.clone())),
            entropy_aggregator: Arc::new(EntropyAggregator::new(runtime_data)),
            metrics_registry: None,
        }
    }

    pub fn with_metrics_registry(mut self, registry: Arc<dyn axiom_viz::MetricsRegistry>) -> Self {
        self.metrics_registry = Some(registry);
        self
    }
}

#[derive(Clone)]
pub struct ApiServerConfig {
    pub addr: SocketAddr,
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        Self {
            addr: ([0, 0, 0, 0], 9092).into(),
        }
    }
}

pub struct ApiServer {
    config: ApiServerConfig,
    state: ApiState,
}

impl ApiServer {
    pub fn new(config: ApiServerConfig, state: ApiState) -> Self {
        Self { config, state }
    }

    pub fn router(&self) -> Router {
        Router::new()
            .nest("/api/v1", v1::routes(self.state.clone()))
            .layer(middleware::from_fn(cors_middleware))
            .layer(middleware::from_fn(request_logging_middleware))
    }

    pub async fn serve(self) -> Result<(), std::io::Error> {
        let router = self.router();
        tracing::info!("API server listening on {}", self.config.addr);
        let listener = tokio::net::TcpListener::bind(&self.config.addr).await?;
        serve(listener, router).await
    }
}

async fn cors_middleware(req: Request<Body>, next: middleware::Next) -> Response<Body> {
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        axum::http::HeaderValue::from_static("*"),
    );
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_METHODS,
        axum::http::HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS,
        axum::http::HeaderValue::from_static("*"),
    );
    response
}

async fn request_logging_middleware(req: Request<Body>, next: middleware::Next) -> Response<Body> {
    let start = std::time::Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    let response = next.run(req).await;
    
    let duration = start.elapsed().as_millis();
    tracing::info!("{} {} {}ms", method, uri, duration);
    
    response
}

pub async fn health_handler(State(state): State<ApiState>) -> impl IntoResponse {
    match state.health_aggregator.aggregate().await {
        Ok(health) => Json(health).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(e)).into_response(),
    }
}

pub async fn cells_handler(State(state): State<ApiState>) -> impl IntoResponse {
    match state.cell_aggregator.get_cells().await {
        Ok(cells) => Json(cells).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(e)).into_response(),
    }
}

pub async fn heatmap_handler(State(state): State<ApiState>) -> impl IntoResponse {
    match state.heatmap_aggregator.get_heatmap().await {
        Ok(heatmap) => Json(heatmap).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(e)).into_response(),
    }
}

pub async fn entropy_handler(State(state): State<ApiState>) -> impl IntoResponse {
    match state.entropy_aggregator.get_entropy().await {
        Ok(entropy) => Json(entropy).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(e)).into_response(),
    }
}

pub async fn metrics_handler(State(state): State<ApiState>) -> Response<Body> {
    match &state.metrics_registry {
        Some(registry) => {
            let body = registry.encode();
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
                .body(Body::from(body))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .expect("static response")
                })
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .expect("static response"),
    }
}