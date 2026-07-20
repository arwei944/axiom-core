//! Commercial product HTTP gateway: read surface + write tasks + SSE + ops shell.
//!
//! **U7 adapter only** — business remains Signal → TaskCell → Governor → Composer.

use crate::alert_bridge::{alerts_json, link_governor_decision, record_run_failure_alert, SharedAlerts};
use crate::events::{DomainEvent, SharedEventBus};
use crate::lenses::{list_lens_ids, project_lens};
use crate::metrics::SharedMetrics;
use crate::run_log::SharedRunLog;
use crate::runtime_host::RuntimeHost;
use crate::surface::{build_surface_body_full, GovernorSnapshot};
use crate::task_cell::SIGNAL_SUBMIT;
use axiom_isa::{Governor, GovernorConfig};
use axiom_kernel::layer::RuntimeTier;
use axiom_runtime::RuntimeHealth;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

const OPS_HTML: &str = include_str!("../static/ops.html");

/// Live write host shared by gateway handlers.
pub struct WriteRuntime {
    pub host: RuntimeHost,
    pub metrics: SharedMetrics,
    pub events: SharedEventBus,
    pub alerts: SharedAlerts,
    pub runs: SharedRunLog,
}

pub struct ProductGateway {
    pub addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    join: Option<tokio::task::JoinHandle<()>>,
}

pub struct GatewayConfig {
    pub bind: SocketAddr,
    pub health: RuntimeHealth,
    pub gov: GovernorSnapshot,
    pub cells: Vec<String>,
    pub runs: SharedRunLog,
    pub metrics: SharedMetrics,
    pub plugins: Vec<String>,
    pub events: SharedEventBus,
    pub alerts: SharedAlerts,
    /// When set, POST /api/v1/tasks publishes SubmitTask on this host.
    pub write: Option<Arc<tokio::sync::Mutex<WriteRuntime>>>,
}

impl ProductGateway {
    pub async fn start(cfg: GatewayConfig) -> Result<Self, String> {
        let listener = TcpListener::bind(cfg.bind)
            .await
            .map_err(|e| format!("bind gateway: {e}"))?;
        let addr = listener.local_addr().map_err(|e| e.to_string())?;
        let (tx, mut rx) = oneshot::channel::<()>();

        let join = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut rx => break,
                    acc = listener.accept() => {
                        match acc {
                            Ok((mut sock, _)) => {
                                let health = cfg.health.clone();
                                let gov = cfg.gov.clone();
                                let cells = cfg.cells.clone();
                                let runs = cfg.runs.clone();
                                let metrics = cfg.metrics.clone();
                                let plugins = cfg.plugins.clone();
                                let events = cfg.events.clone();
                                let alerts = cfg.alerts.clone();
                                let write = cfg.write.clone();
                                tokio::spawn(async move {
                                    let mut buf = vec![0u8; 65536];
                                    let n = sock.read(&mut buf).await.unwrap_or(0);
                                    let req = String::from_utf8_lossy(&buf[..n]);
                                    let first = req.lines().next().unwrap_or("").to_string();
                                    handle_connection(
                                        &mut sock,
                                        &first,
                                        &req,
                                        &health,
                                        &gov,
                                        &cells,
                                        &runs,
                                        &metrics,
                                        &plugins,
                                        &events,
                                        &alerts,
                                        write,
                                    ).await;
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

async fn handle_connection(
    sock: &mut tokio::net::TcpStream,
    first: &str,
    raw: &str,
    health: &RuntimeHealth,
    gov: &GovernorSnapshot,
    cells: &[String],
    runs: &SharedRunLog,
    metrics: &SharedMetrics,
    plugins: &[String],
    events: &SharedEventBus,
    alerts: &SharedAlerts,
    write: Option<Arc<tokio::sync::Mutex<WriteRuntime>>>,
) {
    // SSE long-lived
    if first.contains("GET /api/v1/events") {
        serve_sse(sock, events).await;
        return;
    }

    // Ops shell
    if first.contains("GET / ")
        || first.contains("GET /ops")
        || first.contains("GET /index.html")
        || first.contains("GET /ops.html")
    {
        write_response(sock, "200 OK", "text/html; charset=utf-8", OPS_HTML).await;
        return;
    }

    if first.starts_with("POST /api/v1/tasks") {
        let body = extract_http_body(raw);
        let (status, ct, resp) = post_task(body, write, metrics, events, alerts).await;
        write_response(sock, status, ct, &resp).await;
        return;
    }

    if first.contains("GET /api/v1/alerts") {
        let body = alerts_json(alerts).to_string();
        write_response(sock, "200 OK", "application/json", &body).await;
        return;
    }

    // Reuse surface routing for GETs
    let (status, ct, body) =
        route_get(first, health, gov, cells, runs, Some(metrics), plugins, alerts);
    write_response(sock, status, ct, &body).await;
}

fn route_get(
    path: &str,
    h: &RuntimeHealth,
    g: &GovernorSnapshot,
    c: &[String],
    r: &SharedRunLog,
    metrics: Option<&SharedMetrics>,
    plugins: &[String],
    alerts: &SharedAlerts,
) -> (&'static str, &'static str, String) {
    let cell_refs: Vec<&str> = c.iter().map(|s| s.as_str()).collect();

    if path.contains("GET /api/v1/surface") || path.contains("GET /dashboard") {
        let mut body = build_surface_body_full(h, g, &cell_refs, r, metrics, plugins);
        if let Some(obj) = body.as_object_mut() {
            obj.insert("alerts".into(), alerts_json(alerts));
            obj.insert(
                "write_api".into(),
                json!("POST /api/v1/tasks → Signal SubmitTask → TaskCell"),
            );
            obj.insert("events_api".into(), json!("GET /api/v1/events (SSE)"));
            obj.insert("ops_shell".into(), json!("/ops"));
        }
        return ("200 OK", "application/json", body.to_string());
    }

    if path.contains("GET /metrics") && !path.contains("/api/v1/metrics") {
        let body = metrics
            .map(|m| m.prometheus_text())
            .unwrap_or_else(|| "# no metrics\n".into());
        return ("200 OK", "text/plain; version=0.0.4", body);
    }

    if path.contains("GET /api/v1/metrics") {
        let body = match metrics {
            Some(m) => serde_json::to_string(&m.snapshot()).unwrap_or_else(|_| "{}".into()),
            None => json!({"error":"metrics not attached"}).to_string(),
        };
        return ("200 OK", "application/json", body);
    }

    if path.contains("GET /api/v1/lens/") {
        let id = extract_path_id(path, "/api/v1/lens/").unwrap_or("");
        if let Some(m) = metrics {
            m.inc_lens();
        }
        match project_lens(id, h, g, r, metrics, plugins) {
            Some(v) => ("200 OK", "application/json", v.to_string()),
            None => (
                "404 Not Found",
                "application/json",
                json!({"error":"unknown lens","id": id, "available": list_lens_ids()}).to_string(),
            ),
        }
    } else if path.contains("GET /api/v1/lens") {
        (
            "200 OK",
            "application/json",
            json!({"lenses": list_lens_ids()}).to_string(),
        )
    } else if path.contains("GET /api/v1/plugins") {
        (
            "200 OK",
            "application/json",
            json!({"plugins": plugins, "hot_reload": true}).to_string(),
        )
    } else if path.contains("GET /health") || path.contains("GET /api/v1/health") {
        (
            "200 OK",
            "application/json",
            json!({
                "status": if h.degraded { "degraded" } else if h.started { "ok" } else { "starting" },
                "started": h.started,
                "preflight_passed": h.preflight_passed,
                "cells_running": h.cells_running,
                "entropy_score": h.entropy_score,
                "degraded": h.degraded,
                "last_heartbeat_ms": h.last_heartbeat_ms,
                "metrics_active": h.metrics_active,
                "product": "ule-taskflow",
                "history": "witness-only",
                "admit": "governor"
            })
            .to_string(),
        )
    } else {
        (
            "404 Not Found",
            "application/json",
            json!({"error":"not found"}).to_string(),
        )
    }
}

async fn post_task(
    body: String,
    write: Option<Arc<tokio::sync::Mutex<WriteRuntime>>>,
    metrics: &SharedMetrics,
    events: &SharedEventBus,
    alerts: &SharedAlerts,
) -> (&'static str, &'static str, String) {
    let Some(write) = write else {
        return (
            "503 Service Unavailable",
            "application/json",
            json!({"error":"write host not attached","hint":"boot ProductGateway with WriteRuntime"}).to_string(),
        );
    };

    let payload: Value = match serde_json::from_str(body.trim_matches('\0').trim()) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                "application/json",
                json!({"error": format!("invalid json: {e}")}).to_string(),
            );
        }
    };

    // Normalize to task shape expected by pipeline adapter
    let task_payload = if payload.get("title").is_some() {
        payload
    } else {
        json!({
            "title": payload.get("title").and_then(|v| v.as_str()).unwrap_or("api-task"),
            "priority": payload.get("priority").and_then(|v| v.as_u64()).unwrap_or(1),
            "payload": payload.get("payload").cloned().unwrap_or(json!("from-api")),
        })
    };

    let guard = write.lock().await;
    guard.host.clear_outcome().await;
    if let Err(e) = guard
        .host
        .runtime
        .publish_command(
            SIGNAL_SUBMIT,
            task_payload,
            Some(crate::task_cell::TASK_CELL_ID),
            RuntimeTier::Exec,
        )
        .await
    {
        return (
            "500 Internal Server Error",
            "application/json",
            json!({"error": format!("publish: {e}")}).to_string(),
        );
    }

    let outcome = match guard.host.wait_outcome(Duration::from_secs(5)).await {
        Ok(o) => o,
        Err(e) => {
            return (
                "504 Gateway Timeout",
                "application/json",
                json!({"error": e, "admit":"governor"}).to_string(),
            );
        }
    };

    if outcome.ok {
        metrics.inc_task_ok(outcome.witnesses.len() as u64);
    } else {
        metrics.inc_task_fail(outcome.witnesses.len() as u64);
        if let Some(ref err) = outcome.error {
            record_run_failure_alert(
                events,
                alerts,
                &outcome.governor_level,
                outcome.governor_score,
                err,
            );
        }
    }

    let label = outcome
        .result
        .as_ref()
        .map(|r| r.id.clone())
        .unwrap_or_else(|| "task".into());
    events.publish(crate::events::EventBus::task_completed(
        outcome.ok,
        &label,
        &outcome.governor_level,
        outcome.witnesses.len(),
    ));

    let status = if outcome.ok {
        "201 Created"
    } else if outcome
        .error
        .as_deref()
        .map(|e| e.contains("governor") || e.contains("rejected"))
        .unwrap_or(false)
    {
        "403 Forbidden"
    } else {
        "422 Unprocessable Entity"
    };

    let resp = json!({
        "ok": outcome.ok,
        "error": outcome.error,
        "governor_level": outcome.governor_level,
        "governor_score": outcome.governor_score,
        "witness_count": outcome.witnesses.len(),
        "circuit": outcome.circuit,
        "result": outcome.result.as_ref().map(|r| json!({
            "id": r.id,
            "plan": r.plan,
            "stored": r.stored,
        })),
        "admit_authority": "governor",
        "path": "POST /api/v1/tasks → Signal SubmitTask → TaskCell → Composer",
    })
    .to_string();

    (status, "application/json", resp)
}

async fn serve_sse(sock: &mut tokio::net::TcpStream, events: &SharedEventBus) {
    let headers = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Type: text/event-stream\r\n",
        "Cache-Control: no-cache\r\n",
        "Connection: keep-alive\r\n",
        "Access-Control-Allow-Origin: *\r\n",
        "\r\n"
    );
    let _ = sock.write_all(headers.as_bytes()).await;
    let hello = DomainEvent::new("stream.open", json!({"product":"ule-taskflow"}));
    let _ = sock.write_all(hello.to_sse_data().as_bytes()).await;

    let mut rx = events.subscribe();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        if tokio::time::Instant::now() > deadline {
            break;
        }
        tokio::select! {
            ev = rx.recv() => {
                match ev {
                    Ok(e) => {
                        if sock.write_all(e.to_sse_data().as_bytes()).await.is_err() {
                            break;
                        }
                        let _ = sock.flush().await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(15)) => {
                // heartbeat comment
                if sock.write_all(b": ping\n\n").await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn write_response(
    sock: &mut tokio::net::TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = sock.write_all(resp.as_bytes()).await;
}

fn extract_http_body(raw: &str) -> String {
    raw.split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .trim_matches('\0')
        .to_string()
}

fn extract_path_id<'a>(path: &'a str, marker: &str) -> Option<&'a str> {
    let start = path.find(marker)? + marker.len();
    let rest = &path[start..];
    let id = rest
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c| c == '/' || c == '?' || c == '\0');
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

/// Boot write-capable host for gateway (path tests / CLI).
pub async fn boot_write_runtime(
    metrics: SharedMetrics,
    events: SharedEventBus,
    alerts: SharedAlerts,
    runs: SharedRunLog,
    governor: GovernorConfig,
) -> Result<Arc<tokio::sync::Mutex<WriteRuntime>>, String> {
    use crate::runtime_host::{RunRequest, RuntimeHost};
    use crate::pipeline::FailMode;

    let mut req = RunRequest::default();
    req.fail = FailMode::None;
    req.governor = governor;
    req.submissions = 0;
    let host = RuntimeHost::boot(&req).await?;
    Ok(Arc::new(tokio::sync::Mutex::new(WriteRuntime {
        host,
        metrics,
        events,
        alerts,
        runs,
    })))
}

/// Helper used by tests: evaluate governor and link alert (real product_decide path).
pub fn demo_governor_alert_path(
    force_reject: bool,
    events: &SharedEventBus,
    alerts: &SharedAlerts,
) -> Option<crate::alert_bridge::AlertRecord> {
    let g = if force_reject {
        let mut gov = Governor::with_config(GovernorConfig::default());
        gov.trip();
        gov
    } else {
        Governor::for_demo()
    };
    link_governor_decision(&g, events, alerts, "demo_path")
}
