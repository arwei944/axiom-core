use axum::{Router, routing::get};
use crate::router::{ApiState, health_handler, cells_handler, heatmap_handler, entropy_handler, metrics_handler};

pub fn routes(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/cells", get(cells_handler))
        .route("/heatmap", get(heatmap_handler))
        .route("/entropy", get(entropy_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}