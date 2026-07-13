use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    Disabled,
    ApiKey,
    Jwt,
    OAuth2,
}

impl fmt::Display for AuthMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthMode::Disabled => write!(f, "disabled"),
            AuthMode::ApiKey => write!(f, "api_key"),
            AuthMode::Jwt => write!(f, "jwt"),
            AuthMode::OAuth2 => write!(f, "oauth2"),
        }
    }
}

#[derive(Clone)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub api_key: Option<String>,
    pub jwt_secret: Option<String>,
    pub jwt_expiry_minutes: i64,
    pub oauth2_client_id: Option<String>,
    pub oauth2_client_secret: Option<String>,
    pub oauth2_redirect_uri: Option<String>,
    pub oauth2_token_url: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::Disabled,
            api_key: None,
            jwt_secret: None,
            jwt_expiry_minutes: 60,
            oauth2_client_id: None,
            oauth2_client_secret: None,
            oauth2_redirect_uri: None,
            oauth2_token_url: None,
        }
    }
}

impl AuthConfig {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn api_key(key: impl Into<String>) -> Self {
        Self { mode: AuthMode::ApiKey, api_key: Some(key.into()), ..Default::default() }
    }

    pub fn jwt(secret: impl Into<String>) -> Self {
        Self { mode: AuthMode::Jwt, jwt_secret: Some(secret.into()), ..Default::default() }
    }

    pub fn oauth2(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
        token_url: impl Into<String>,
    ) -> Self {
        Self {
            mode: AuthMode::OAuth2,
            oauth2_client_id: Some(client_id.into()),
            oauth2_client_secret: Some(client_secret.into()),
            oauth2_redirect_uri: Some(redirect_uri.into()),
            oauth2_token_url: Some(token_url.into()),
            ..Default::default()
        }
    }

    pub fn with_jwt_expiry(mut self, minutes: i64) -> Self {
        self.jwt_expiry_minutes = minutes;
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub iss: String,
    pub roles: Vec<String>,
}

impl JwtClaims {
    pub fn new(subject: &str, roles: Vec<String>, expiry_minutes: i64) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        Self {
            sub: subject.to_string(),
            iat: now,
            exp: now + (expiry_minutes as u64) * 60,
            iss: "axiom-core".to_string(),
            roles,
        }
    }
}

pub struct JwtToken(pub String);

pub struct AuthService {
    config: Arc<AuthConfig>,
}

impl AuthService {
    pub fn new(config: AuthConfig) -> Self {
        Self { config: Arc::new(config) }
    }

    pub fn generate_jwt(&self, subject: &str, roles: Vec<String>) -> Result<String, AuthError> {
        if self.config.mode != AuthMode::Jwt {
            return Err(AuthError::AuthModeNotEnabled("JWT".to_string()));
        }

        let secret = self.config.jwt_secret.as_ref().ok_or(AuthError::MissingSecret)?;

        let claims = JwtClaims::new(subject, roles, self.config.jwt_expiry_minutes);

        encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
            .map_err(|e| AuthError::JwtEncoding(e.to_string()))
    }

    pub fn verify_jwt(&self, token: &str) -> Result<JwtClaims, AuthError> {
        if self.config.mode != AuthMode::Jwt {
            return Err(AuthError::AuthModeNotEnabled("JWT".to_string()));
        }

        let secret = self.config.jwt_secret.as_ref().ok_or(AuthError::MissingSecret)?;

        decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map(|t| t.claims)
        .map_err(|e| AuthError::JwtDecoding(e.to_string()))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("auth mode {0} not enabled")]
    AuthModeNotEnabled(String),
    #[error("missing secret")]
    MissingSecret,
    #[error("JWT encoding failed: {0}")]
    JwtEncoding(String),
    #[error("JWT decoding failed: {0}")]
    JwtDecoding(String),
    #[error("API key missing")]
    ApiKeyMissing,
    #[error("invalid API key")]
    InvalidApiKey,
}

pub async fn auth_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let auth_config = req.extensions().get::<AuthConfig>().cloned().unwrap_or_default();

    if auth_config.mode == AuthMode::Disabled {
        return next.run(req).await;
    }

    match auth_config.mode {
        AuthMode::ApiKey => api_key_auth(req, next, &auth_config).await,
        AuthMode::Jwt => jwt_auth(req, next, &auth_config).await,
        AuthMode::OAuth2 => oauth2_auth(req, next).await,
        AuthMode::Disabled => next.run(req).await,
    }
}

async fn api_key_auth(req: Request<Body>, next: Next, config: &AuthConfig) -> Response<Body> {
    let expected_key = match &config.api_key {
        Some(key) => key,
        None => return unauthorized_response(),
    };

    let api_key = req.headers().get("x-api-key").and_then(|v| v.to_str().ok());

    if api_key.map(|k| k == expected_key).unwrap_or(false) {
        next.run(req).await
    } else {
        unauthorized_response()
    }
}

async fn jwt_auth(req: Request<Body>, next: Next, config: &AuthConfig) -> Response<Body> {
    let secret = match &config.jwt_secret {
        Some(secret) => secret,
        None => return server_error_response("JWT secret not configured"),
    };

    let auth_header = req.headers().get("authorization").and_then(|v| v.to_str().ok());

    let token = match auth_header.and_then(|h| h.strip_prefix("Bearer ")) {
        Some(token) => token,
        None => return unauthorized_response(),
    };

    match decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(_) => next.run(req).await,
        Err(_) => unauthorized_response(),
    }
}

async fn oauth2_auth(req: Request<Body>, next: Next) -> Response<Body> {
    let auth_header = req.headers().get("authorization").and_then(|v| v.to_str().ok());

    if auth_header.is_some() {
        next.run(req).await
    } else {
        unauthorized_response()
    }
}

fn unauthorized_response() -> Response<Body> {
    let mut response =
        Response::new(Body::from(serde_json::json!({"error": "unauthorized"}).to_string()));
    *response.status_mut() = StatusCode::UNAUTHORIZED;
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json"),
    );
    response
}

fn server_error_response(message: &str) -> Response<Body> {
    let mut response = Response::new(Body::from(serde_json::json!({"error": message}).to_string()));
    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json"),
    );
    response
}
