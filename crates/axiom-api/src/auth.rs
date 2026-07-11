use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;

#[derive(Clone)]
pub struct AuthConfig {
    pub api_key: Option<String>,
    pub enabled: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            enabled: false,
        }
    }
}

impl AuthConfig {
    pub fn with_api_key(key: impl Into<String>) -> Self {
        Self {
            api_key: Some(key.into()),
            enabled: true,
        }
    }

    pub fn disabled() -> Self {
        Self::default()
    }
}

pub async fn auth_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let auth_config = req
        .extensions()
        .get::<AuthConfig>()
        .cloned()
        .unwrap_or_default();

    if !auth_config.enabled {
        return next.run(req).await;
    }

    let api_key = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok());

    match (&auth_config.api_key, api_key) {
        (Some(expected), Some(provided)) if expected == provided => next.run(req).await,
        _ => {
            let mut response = Response::new(Body::from(
                serde_json::json!({"error": "unauthorized"}).to_string(),
            ));
            *response.status_mut() = StatusCode::UNAUTHORIZED;
            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/json"),
            );
            response
        }
    }
}