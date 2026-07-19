//! U4 unified observation surface — single host truth source (T12).
//!
//! Routes:
//! - `GET /health` | `/api/v1/health`
//! - `GET /api/v1/surface` | `/dashboard`
//! - `GET /metrics` | `/api/v1/metrics`  (Prometheus text / JSON)
//! - `GET /api/v1/lens` | `/api/v1/lens/{id}`
//! - `GET /api/v1/plugins`
//!
//! Admit authority field is always `"governor"` (sole decide API).

use crate::lenses::{list_lens_ids, project_lens};
use crate::metrics::SharedMetrics;
use crate::run_log::{snapshot_runs, SharedRunLog};
use axiom_isa::{product_decide, Decision, Governor};
use axiom_runtime::RuntimeHealth;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// Snapshot of Governor for the surface (cloneable metrics).
#[derive(Clone)]
pub struct GovernorSnapshot {
    pub level: String,
    pub score: f64,
    pub decision: String,
}

impl GovernorSnapshot {
    pub fn from_governor(g: &Governor) -> Self {
        let d = product_decide(g);
        let decision = match d {
            Decision::Allow => "allow".into(),
            Decision::Reject { reason } => format!("reject:{reason}"),
        };
        Self {
            level: format!("{:?}", g.level()),
            score: g.score(),
            decision,
        }
    }
}

pub type SharedGovernorSnap = Arc<Mutex<GovernorSnapshot>>;

pub fn new_gov_snap(g: &Governor) -> SharedGovernorSnap {
    Arc::new(Mutex::new(GovernorSnapshot::from_governor(g)))
}

/// Build unified JSON for `/api/v1/surface`.
pub fn build_surface_body(
    health: &RuntimeHealth,
    gov: &GovernorSnapshot,
    cells: &[&str],
    runs: &SharedRunLog,
) -> Value {
    build_surface_body_full(health, gov, cells, runs, None, &[])
}

/// Full surface with metrics + plugins (T12 product floor).
pub fn build_surface_body_full(
    health: &RuntimeHealth,
    gov: &GovernorSnapshot,
    cells: &[&str],
    runs: &SharedRunLog,
    metrics: Option<&SharedMetrics>,
    plugins: &[String],
) -> Value {
    let recent = snapshot_runs(runs);
    let metrics_json = metrics.map(|m| m.snapshot());
    json!({
        "product": "ule-taskflow",
        "host": "axiom-runtime",
        "history": "witness-only",
        "admit_authority": "governor",
        "decide_api": "axiom_isa::product_decide / Governor::decide / product_admit",
        "isa_policy": axiom_isa::policy_text(),
        "status": if health.degraded {
            "degraded"
        } else if health.started {
            "ok"
        } else {
            "starting"
        },
        "health": {
            "started": health.started,
            "preflight_passed": health.preflight_passed,
            "cells_running": health.cells_running,
            "cells_stopped": health.cells_stopped,
            "total_restarts": health.total_restarts,
            "messages_delivered": health.messages_delivered,
            "messages_rejected": health.messages_rejected,
            "entropy_score": health.entropy_score,
            "degraded": health.degraded,
            "last_heartbeat_ms": health.last_heartbeat_ms,
            "metrics_active": health.metrics_active,
            "metrics_endpoint": health.metrics_endpoint,
            "telemetry_enabled": health.telemetry_enabled,
            "store_connected": health.store_connected,
            "snapshot_store_connected": health.snapshot_store_connected,
        },
        "governor": {
            "level": gov.level,
            "score": gov.score,
            "decision": gov.decision,
        },
        "cells": cells,
        "recent_runs": recent,
        "metrics": metrics_json,
        "lenses": list_lens_ids(),
        "plugins": {
            "ids": plugins,
            "hot_reload": true,
        },
        "observability": {
            "surface": "/api/v1/surface",
            "health": "/api/v1/health",
            "metrics_prom": "/metrics",
            "metrics_json": "/api/v1/metrics",
            "lens": "/api/v1/lens/{id}",
            "plugins": "/api/v1/plugins",
        },
    })
}

pub struct SurfaceServer {
    pub addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    join: Option<tokio::task::JoinHandle<()>>,
}

impl SurfaceServer {
    pub async fn start(
        bind: SocketAddr,
        health: RuntimeHealth,
        gov: GovernorSnapshot,
        cells: Vec<String>,
        runs: SharedRunLog,
    ) -> Result<Self, String> {
        Self::start_full(bind, health, gov, cells, runs, None, vec![]).await
    }

    pub async fn start_full(
        bind: SocketAddr,
        health: RuntimeHealth,
        gov: GovernorSnapshot,
        cells: Vec<String>,
        runs: SharedRunLog,
        metrics: Option<SharedMetrics>,
        plugins: Vec<String>,
    ) -> Result<Self, String> {
        let listener = TcpListener::bind(bind)
            .await
            .map_err(|e| format!("bind surface: {e}"))?;
        let addr = listener.local_addr().map_err(|e| e.to_string())?;
        let (tx, mut rx) = oneshot::channel::<()>();

        let join = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut rx => break,
                    acc = listener.accept() => {
                        match acc {
                            Ok((mut sock, _)) => {
                                let h = health.clone();
                                let g = gov.clone();
                                let c = cells.clone();
                                let r = runs.clone();
                                let m = metrics.clone();
                                let p = plugins.clone();
                                tokio::spawn(async move {
                                    let mut buf = [0u8; 8192];
                                    let _ = sock.read(&mut buf).await;
                                    let req = String::from_utf8_lossy(&buf);
                                    let path = req.lines().next().unwrap_or("");
                                    let (status, content_type, body) =
                                        route_request(path, &h, &g, &c, &r, m.as_ref(), &p);
                                    let resp = format!(
                                        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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

fn route_request(
    path: &str,
    h: &RuntimeHealth,
    g: &GovernorSnapshot,
    c: &[String],
    r: &SharedRunLog,
    metrics: Option<&SharedMetrics>,
    plugins: &[String],
) -> (&'static str, &'static str, String) {
    let cell_refs: Vec<&str> = c.iter().map(|s| s.as_str()).collect();

    if path.contains("GET /api/v1/surface") || path.contains("GET /dashboard") {
        let body = build_surface_body_full(h, g, &cell_refs, r, metrics, plugins).to_string();
        return ("200 OK", "application/json", body);
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
        // Extract lens id after /api/v1/lens/
        let id = extract_lens_id(path).unwrap_or("");
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
        let body = json!({
            "lenses": list_lens_ids(),
            "usage": "/api/v1/lens/{id}",
        })
        .to_string();
        ("200 OK", "application/json", body)
    } else if path.contains("GET /api/v1/plugins") {
        let body = json!({
            "plugins": plugins,
            "hot_reload": true,
            "sandbox": "NativePluginSandbox allow-list",
        })
        .to_string();
        ("200 OK", "application/json", body)
    } else if path.contains("GET /health") || path.contains("GET /api/v1/health") {
        let body = json!({
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
        .to_string();
        ("200 OK", "application/json", body)
    } else {
        (
            "404 Not Found",
            "application/json",
            json!({"error":"not found"}).to_string(),
        )
    }
}

fn extract_lens_id(path: &str) -> Option<&str> {
    // Request line: GET /api/v1/lens/ule.runs HTTP/1.1
    let marker = "/api/v1/lens/";
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
