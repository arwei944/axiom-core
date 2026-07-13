use crate::router::{
    cells_handler, entropy_handler, health_handler, heatmap_handler, metrics_handler, ApiState,
};
use axum::{routing::get, Router};

pub fn routes(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/cells", get(cells_handler))
        .route("/heatmap", get(heatmap_handler))
        .route("/entropy", get(entropy_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}
