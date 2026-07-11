use axiom_api::ApiServerBuilder;
use axiom_runtime::{AxiomRuntime, RuntimeConfig};
use axiom_oversight::{OversightKernelAdapter, EntropyGovernorCell, HealthCollectorCell, ComplianceGuardCell};
use std::sync::Arc;

fn create_test_runtime() -> Arc<AxiomRuntime> {
    Arc::new(AxiomRuntime::new(RuntimeConfig::default()))
}

fn create_test_oversight() -> Arc<OversightKernelAdapter> {
    Arc::new(OversightKernelAdapter::new(
        Arc::new(EntropyGovernorCell::default()),
        Arc::new(HealthCollectorCell::new()),
        Arc::new(ComplianceGuardCell::new()),
    ))
}

#[tokio::test]
async fn api_server_builder_creates_server() {
    let runtime = create_test_runtime();
    let oversight = create_test_oversight();

    let server = ApiServerBuilder::new()
        .addr(([127, 0, 0, 1], 0).into())
        .build(runtime, oversight);

    let router = server.router();
    let _ = router;
}

#[tokio::test]
async fn api_server_builder_with_api_key() {
    let runtime = create_test_runtime();
    let oversight = create_test_oversight();

    let server = ApiServerBuilder::new()
        .addr(([127, 0, 0, 1], 0).into())
        .with_api_key("test-key-12345")
        .build(runtime, oversight);

    let router = server.router();
    let _ = router;
}

#[tokio::test]
async fn api_health_endpoint_returns_ok() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let runtime = create_test_runtime();
    let oversight = create_test_oversight();

    let server = ApiServerBuilder::new()
        .addr(([127, 0, 0, 1], 0).into())
        .build(runtime, oversight);

    let app = server.router();

    let response = app
        .oneshot(Request::builder().uri("/api/v1/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("status").is_some());
    assert!(json.get("cells_running").is_some());
}

#[tokio::test]
async fn api_cells_endpoint_returns_ok() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let runtime = create_test_runtime();
    let oversight = create_test_oversight();

    let server = ApiServerBuilder::new()
        .addr(([127, 0, 0, 1], 0).into())
        .build(runtime, oversight);

    let app = server.router();

    let response = app
        .oneshot(Request::builder().uri("/api/v1/cells").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

#[tokio::test]
async fn api_entropy_endpoint_returns_ok() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let runtime = create_test_runtime();
    let oversight = create_test_oversight();

    let server = ApiServerBuilder::new()
        .addr(([127, 0, 0, 1], 0).into())
        .build(runtime, oversight);

    let app = server.router();

    let response = app
        .oneshot(Request::builder().uri("/api/v1/entropy").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("value").is_some());
    assert!(json.get("level").is_some());
}

#[tokio::test]
async fn api_heatmap_endpoint_returns_ok() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let runtime = create_test_runtime();
    let oversight = create_test_oversight();

    let server = ApiServerBuilder::new()
        .addr(([127, 0, 0, 1], 0).into())
        .build(runtime, oversight);

    let app = server.router();

    let response = app
        .oneshot(Request::builder().uri("/api/v1/heatmap").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("timestamp").is_some());
}

#[tokio::test]
async fn api_cors_headers_present() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let runtime = create_test_runtime();
    let oversight = create_test_oversight();

    let server = ApiServerBuilder::new()
        .addr(([127, 0, 0, 1], 0).into())
        .build(runtime, oversight);

    let app = server.router();

    let response = app
        .oneshot(Request::builder().uri("/api/v1/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.headers().contains_key("access-control-allow-origin"));
    assert!(response.headers().contains_key("access-control-allow-methods"));
}

#[tokio::test]
async fn runtime_config_has_api_endpoint() {
    let config = RuntimeConfig::default();
    assert!(config.api_endpoint.is_none());

    let config = RuntimeConfig {
        api_endpoint: Some(([0, 0, 0, 0], 9092).into()),
        ..Default::default()
    };
    assert!(config.api_endpoint.is_some());
}