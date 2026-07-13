//! API 安全中间件模块
//!
//! 包含速率限制、CORS、请求日志等中间件

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// P2-J1: 速率限制中间件
// ---------------------------------------------------------------------------

/// 令牌桶速率限制器
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 每个窗口期内允许的最大请求数
    pub max_requests: u32,
    /// 时间窗口大小（秒）
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self { max_requests: 100, window_secs: 60 }
    }
}

/// 每个 IP 的令牌桶状态
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: u32,
    last_refill: Instant,
}

/// 速率限制器共享状态
#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: Arc<RwLock<HashMap<IpAddr, TokenBucket>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self { config, buckets: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// 尝试消费一个令牌，返回是否允许通过
    async fn try_acquire(&self, ip: IpAddr) -> bool {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_secs);

        let bucket = buckets
            .entry(ip)
            .or_insert(TokenBucket { tokens: self.config.max_requests, last_refill: now });

        // 检查是否需要补充令牌
        if now.duration_since(bucket.last_refill) >= window {
            bucket.tokens = self.config.max_requests;
            bucket.last_refill = now;
        }

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// 清理过期的桶条目（防止内存泄漏）
    #[allow(dead_code)]
    pub async fn cleanup(&self) {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_secs);
        buckets.retain(|_, bucket| now.duration_since(bucket.last_refill) < window * 2);
    }
}

/// 速率限制中间件
pub async fn rate_limit_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let limiter = match req.extensions().get::<Arc<RateLimiter>>() {
        Some(l) => l.clone(),
        None => return next.run(req).await,
    };

    let ip = extract_client_ip(&req);

    if !limiter.try_acquire(ip).await {
        tracing::warn!(client_ip = %ip, "rate limit exceeded");
        return Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Retry-After", limiter.config.window_secs.to_string())
            .body(Body::from(
                serde_json::json!({
                    "error": "Too Many Requests",
                    "message": "Rate limit exceeded. Please try again later.",
                    "retry_after_secs": limiter.config.window_secs
                })
                .to_string(),
            ))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .body(Body::empty())
                    .expect("static response")
            });
    }

    next.run(req).await
}

/// 从请求中提取客户端 IP 地址
fn extract_client_ip(req: &Request<Body>) -> IpAddr {
    // 优先从 X-Forwarded-For 头获取（经过代理时）
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(s) = forwarded.to_str() {
            if let Some(first) = s.split(',').next() {
                if let Ok(ip) = first.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }

    // 从 X-Real-IP 头获取
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(s) = real_ip.to_str() {
            if let Ok(ip) = s.parse::<IpAddr>() {
                return ip;
            }
        }
    }

    // 回退到连接地址（axum 中可能不可用，回退到 0.0.0.0）
    IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)
}

// ---------------------------------------------------------------------------
// P2-J3: 可配置 CORS 中间件
// ---------------------------------------------------------------------------

/// CORS 配置
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// 允许的源列表，空表示允许所有（*）
    pub allowed_origins: Vec<String>,
    /// 允许的 HTTP 方法
    pub allowed_methods: Vec<String>,
    /// 允许的请求头
    pub allowed_headers: Vec<String>,
    /// 是否允许携带凭证
    pub allow_credentials: bool,
    /// 预检请求缓存时间（秒）
    pub max_age_secs: u64,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Requested-With".to_string(),
            ],
            allow_credentials: false,
            max_age_secs: 3600,
        }
    }
}

impl CorsConfig {
    fn build_origin_header(&self) -> String {
        if self.allowed_origins.is_empty() || self.allowed_origins.contains(&"*".to_string()) {
            "*".to_string()
        } else {
            self.allowed_origins.join(", ")
        }
    }

    fn build_methods_header(&self) -> String {
        self.allowed_methods.join(", ")
    }

    fn build_headers_header(&self) -> String {
        self.allowed_headers.join(", ")
    }
}

/// 可配置的 CORS 中间件
pub async fn configurable_cors_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    // 在请求被消费前提取 CORS 配置
    let cors =
        req.extensions().get::<Arc<CorsConfig>>().map(|c| c.as_ref().clone()).unwrap_or_default();

    // 处理预检请求
    if req.method() == axum::http::Method::OPTIONS {
        return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, cors.build_origin_header())
            .header(axum::http::header::ACCESS_CONTROL_ALLOW_METHODS, cors.build_methods_header())
            .header(axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS, cors.build_headers_header())
            .header(axum::http::header::ACCESS_CONTROL_MAX_AGE, cors.max_age_secs.to_string())
            .header(
                axum::http::header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                cors.allow_credentials.to_string(),
            )
            .body(Body::empty())
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(Body::empty())
                    .expect("static response")
            });
    }

    let mut response = next.run(req).await;

    let headers = response.headers_mut();
    headers.insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        axum::http::HeaderValue::from_str(&cors.build_origin_header())
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("*")),
    );
    headers.insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_METHODS,
        axum::http::HeaderValue::from_str(&cors.build_methods_header())
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("GET, POST, OPTIONS")),
    );
    headers.insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS,
        axum::http::HeaderValue::from_str(&cors.build_headers_header())
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("*")),
    );

    if cors.allow_credentials {
        headers.insert(
            axum::http::header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
            axum::http::HeaderValue::from_static("true"),
        );
    }

    response
}

// ---------------------------------------------------------------------------
// P2-J4: 增强请求日志中间件
// ---------------------------------------------------------------------------

/// 增强的请求日志中间件，记录方法、路径、状态码、延迟、用户代理等
pub async fn enhanced_request_logging_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let user_agent = req
        .headers()
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();
    let client_ip = extract_client_ip(&req).to_string();
    let content_length = req
        .headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("0")
        .to_string();

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();
    let duration_ms = duration.as_millis();

    tracing::info!(
        method = %method,
        uri = %uri,
        status = %status.as_u16(),
        duration_ms = duration_ms,
        client_ip = %client_ip,
        user_agent = %user_agent,
        content_length = %content_length,
        "request completed"
    );

    response
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 创建安全中间件层配置
#[derive(Debug, Clone, Default)]
pub struct SecurityMiddlewareConfig {
    pub rate_limit: Option<RateLimitConfig>,
    pub cors: Option<CorsConfig>,
}

/// 将安全配置注入到 Router 的扩展中
pub fn inject_security_extensions(
    router: axum::Router,
    config: &SecurityMiddlewareConfig,
) -> axum::Router {
    let mut router = router;

    if let Some(rl_config) = &config.rate_limit {
        router = router.layer(axum::Extension(Arc::new(RateLimiter::new(rl_config.clone()))));
    }

    if let Some(cors_config) = &config.cors {
        router = router.layer(axum::Extension(Arc::new(cors_config.clone())));
    }

    router
}
