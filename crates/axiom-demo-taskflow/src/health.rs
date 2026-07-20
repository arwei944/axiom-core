//! Production entry health surface for the commercial taskflow path.

use axiom_runtime::RuntimeHealth;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// Minimal HTTP health server (no heavy API stack required).
///
/// Routes:
/// - `GET /health`
/// - `GET /api/v1/health`
///
/// Auth boundary is documented separately (`AXIOM_API_KEY`); this floor endpoint
/// is intentionally unauthenticated like a k8s liveness probe.
pub struct HealthServer {
    pub addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    join: Option<tokio::task::JoinHandle<()>>,
}

impl HealthServer {
    pub async fn start(
        bind: SocketAddr,
        health_fn: Arc<dyn Fn() -> RuntimeHealth + Send + Sync>,
    ) -> Result<Self, String> {
        let listener = TcpListener::bind(bind)
            .await
            .map_err(|e| format!("bind health: {e}"))?;
        let addr = listener.local_addr().map_err(|e| e.to_string())?;
        let (tx, mut rx) = oneshot::channel::<()>();

        let join = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut rx => break,
                    acc = listener.accept() => {
                        match acc {
                            Ok((mut sock, _)) => {
                                let hf = health_fn.clone();
                                tokio::spawn(async move {
                                    let mut buf = [0u8; 2048];
                                    let _ = sock.read(&mut buf).await;
                                    let req = String::from_utf8_lossy(&buf);
                                    let path_ok = req.contains("GET /health")
                                        || req.contains("GET /api/v1/health");
                                    let body = if path_ok {
                                        let h = hf();
                                        json!({
                                            "status": if h.started { "ok" } else { "starting" },
                                            "started": h.started,
                                            "preflight_passed": h.preflight_passed,
                                            "cells_running": h.cells_running,
                                            "entropy_score": h.entropy_score,
                                            "messages_delivered": h.messages_delivered,
                                            "product": "ule-taskflow",
                                            "history": "witness-only",
                                            "admit": "governor"
                                        })
                                        .to_string()
                                    } else {
                                        json!({"error":"not found"}).to_string()
                                    };
                                    let status = if path_ok { "200 OK" } else { "404 Not Found" };
                                    let resp = format!(
                                        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                        body.len()
                                    );
                                    let _ = sock.write_all(resp.as_bytes()).await;
                                });
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        });

        Ok(Self {
            addr,
            shutdown: Some(tx),
            join: Some(join),
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn stop(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(j) = self.join.take() {
            let _ = j.await;
        }
    }
}

/// One-shot client GET against a health URL (raw TCP).
pub async fn fetch_health(url_path_host: &str) -> Result<(u16, String), String> {
    http_exchange("GET", url_path_host, None).await
}

/// Raw HTTP client for path tests (GET/POST).
///
/// `url_path_host` like `127.0.0.1:19092/api/v1/tasks`.
pub async fn http_exchange(
    method: &str,
    url_path_host: &str,
    body: Option<&str>,
) -> Result<(u16, String), String> {
    let (hostport, path) = if let Some(i) = url_path_host.find('/') {
        (&url_path_host[..i], &url_path_host[i..])
    } else {
        (url_path_host, "/")
    };
    let addr: SocketAddr = hostport
        .parse()
        .map_err(|e| format!("parse addr: {e}"))?;
    let mut sock = tokio::net::TcpStream::connect(addr)
        .await
        .map_err(|e| format!("connect: {e}"))?;
    let body = body.unwrap_or("");
    let req = if method.eq_ignore_ascii_case("POST") {
        format!(
            "POST {path} HTTP/1.1\r\nHost: {hostport}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    } else {
        format!("GET {path} HTTP/1.1\r\nHost: {hostport}\r\nConnection: close\r\n\r\n")
    };
    sock.write_all(req.as_bytes())
        .await
        .map_err(|e| format!("write: {e}"))?;
    let mut buf = Vec::new();
    sock.read_to_end(&mut buf)
        .await
        .map_err(|e| format!("read: {e}"))?;
    let text = String::from_utf8_lossy(&buf).to_string();
    let status_line = text.lines().next().unwrap_or("");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = text
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .trim_matches('\0')
        .to_string();
    Ok((status, body))
}
