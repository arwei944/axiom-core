//! Commercial Lens projections (T12 / UNIFIED_MODEL Lens).
//!
//! Lens is a **read-only view** over product state — not a second history,
//! not cross-Cell mutation. Surface exposes `/api/v1/lens/{id}`.

use crate::metrics::{MetricsSnapshot, SharedMetrics};
use crate::run_log::{snapshot_runs, SharedRunLog};
use crate::surface::GovernorSnapshot;
use axiom_kernel::axiom::{KernelResult, Lens, Projection, State};
use axiom_runtime::RuntimeHealth;
use serde_json::{json, Value};

pub const LENS_RUNS: &str = "ule.runs";
pub const LENS_GOVERNOR: &str = "ule.governor";
pub const LENS_HEALTH: &str = "ule.health";
pub const LENS_METRICS: &str = "ule.metrics";
pub const LENS_PLUGINS: &str = "ule.plugins";

/// Project recent runs from SharedRunLog JSON state.
pub struct RunsLens;

impl Lens for RunsLens {
    fn id(&self) -> &'static str {
        LENS_RUNS
    }

    fn project(&self, state: &State) -> KernelResult<Projection> {
        // State payload is JSON: { "runs": [...] } or raw array.
        let v: Value = serde_json::from_slice(&state.data).unwrap_or(json!([]));
        let data = serde_json::to_vec(&json!({
            "lens": LENS_RUNS,
            "kind": "runs",
            "data": v,
        }))
        .unwrap_or_default();
        Ok(Projection::new(data).with_metadata("lens", LENS_RUNS))
    }
}

pub struct GovernorLens;

impl Lens for GovernorLens {
    fn id(&self) -> &'static str {
        LENS_GOVERNOR
    }

    fn project(&self, state: &State) -> KernelResult<Projection> {
        let v: Value = serde_json::from_slice(&state.data).unwrap_or(json!({}));
        let data = serde_json::to_vec(&json!({
            "lens": LENS_GOVERNOR,
            "kind": "governor",
            "data": v,
            "admit_authority": "governor",
        }))
        .unwrap_or_default();
        Ok(Projection::new(data).with_metadata("lens", LENS_GOVERNOR))
    }
}

pub struct HealthLens;

impl Lens for HealthLens {
    fn id(&self) -> &'static str {
        LENS_HEALTH
    }

    fn project(&self, state: &State) -> KernelResult<Projection> {
        let v: Value = serde_json::from_slice(&state.data).unwrap_or(json!({}));
        let data = serde_json::to_vec(&json!({
            "lens": LENS_HEALTH,
            "kind": "health",
            "data": v,
            "history": "witness-only",
        }))
        .unwrap_or_default();
        Ok(Projection::new(data).with_metadata("lens", LENS_HEALTH))
    }
}

pub struct MetricsLens;

impl Lens for MetricsLens {
    fn id(&self) -> &'static str {
        LENS_METRICS
    }

    fn project(&self, state: &State) -> KernelResult<Projection> {
        let v: Value = serde_json::from_slice(&state.data).unwrap_or(json!({}));
        let data = serde_json::to_vec(&json!({
            "lens": LENS_METRICS,
            "kind": "metrics",
            "data": v,
        }))
        .unwrap_or_default();
        Ok(Projection::new(data).with_metadata("lens", LENS_METRICS))
    }
}

/// Build State for a named commercial lens from live host handles.
pub fn state_for_lens(
    id: &str,
    health: &RuntimeHealth,
    gov: &GovernorSnapshot,
    runs: &SharedRunLog,
    metrics: Option<&SharedMetrics>,
    plugins: &[String],
) -> Option<State> {
    let json_val = match id {
        LENS_RUNS => json!(snapshot_runs(runs)),
        LENS_GOVERNOR => json!({
            "level": gov.level,
            "score": gov.score,
            "decision": gov.decision,
            "admit_authority": "governor",
        }),
        LENS_HEALTH => json!({
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
        }),
        LENS_METRICS => {
            let snap = metrics
                .map(|m| m.snapshot())
                .unwrap_or(MetricsSnapshot {
                    tasks_submitted: 0,
                    tasks_ok: 0,
                    tasks_fail: 0,
                    handoffs_submitted: 0,
                    handoffs_ok: 0,
                    handoffs_rejected: 0,
                    governor_allows: 0,
                    governor_rejects: 0,
                    witnesses_emitted: 0,
                    workbench_executions: 0,
                    lens_queries: 0,
                    plugin_invocations: 0,
                });
            serde_json::to_value(snap).unwrap_or(json!({}))
        }
        LENS_PLUGINS => json!({ "plugins": plugins, "hot_reload": true }),
        _ => return None,
    };
    Some(State::new(serde_json::to_vec(&json_val).unwrap_or_default()))
}

/// Project a commercial lens id against live state.
pub fn project_lens(
    id: &str,
    health: &RuntimeHealth,
    gov: &GovernorSnapshot,
    runs: &SharedRunLog,
    metrics: Option<&SharedMetrics>,
    plugins: &[String],
) -> Option<Value> {
    let state = state_for_lens(id, health, gov, runs, metrics, plugins)?;
    let projection = match id {
        LENS_RUNS => RunsLens.project(&state).ok()?,
        LENS_GOVERNOR => GovernorLens.project(&state).ok()?,
        LENS_HEALTH => HealthLens.project(&state).ok()?,
        LENS_METRICS => MetricsLens.project(&state).ok()?,
        LENS_PLUGINS => {
            let data = serde_json::to_vec(&json!({
                "lens": LENS_PLUGINS,
                "kind": "plugins",
                "data": serde_json::from_slice::<Value>(&state.data).unwrap_or(json!({})),
                "hot_reload": true,
            }))
            .unwrap_or_default();
            Projection::new(data).with_metadata("lens", LENS_PLUGINS)
        }
        _ => return None,
    };
    serde_json::from_slice(&projection.data).ok()
}

pub fn list_lens_ids() -> Vec<&'static str> {
    vec![LENS_RUNS, LENS_GOVERNOR, LENS_HEALTH, LENS_METRICS, LENS_PLUGINS]
}
