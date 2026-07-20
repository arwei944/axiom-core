//! Alert ↔ Governor linkage (demo floor, not SaaS alert product).
//!
//! When Governor rejects or entropy is elevated, emit a domain alert event
//! for Surface / SSE consumers.

use crate::events::{EventBus, SharedEventBus};
use axiom_isa::{product_decide, Decision, Governor};
use serde::Serialize;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
pub struct AlertRecord {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub governor_level: String,
    pub governor_score: f64,
}

pub type SharedAlerts = Arc<Mutex<Vec<AlertRecord>>>;

pub fn new_alert_log() -> SharedAlerts {
    Arc::new(Mutex::new(Vec::new()))
}

/// Map Governor decision into an alert + SSE event when not Allow.
pub fn link_governor_decision(
    governor: &Governor,
    events: &SharedEventBus,
    alerts: &SharedAlerts,
    context: &str,
) -> Option<AlertRecord> {
    let decision = product_decide(governor);
    let level = format!("{:?}", governor.level());
    let score = governor.score();
    match decision {
        Decision::Allow => None,
        Decision::Reject { reason } => {
            let rec = AlertRecord {
                id: format!("gov-{}", alerts.lock().map(|a| a.len()).unwrap_or(0)),
                severity: "critical".into(),
                title: "governor_reject".into(),
                detail: format!("{context}: {reason}"),
                governor_level: level.clone(),
                governor_score: score,
            };
            if let Ok(mut g) = alerts.lock() {
                g.push(rec.clone());
                if g.len() > 64 {
                    let n = g.len() - 64;
                    g.drain(0..n);
                }
            }
            events.publish(EventBus::governor_alert(&level, score, &rec.detail));
            Some(rec)
        }
    }
}

/// Record an alert when a task/handoff fails for governance reasons.
pub fn record_run_failure_alert(
    events: &SharedEventBus,
    alerts: &SharedAlerts,
    governor_level: &str,
    governor_score: f64,
    error: &str,
) {
    let is_gov = error.to_lowercase().contains("governor")
        || error.to_lowercase().contains("reject")
        || error.to_lowercase().contains("entropy");
    if !is_gov {
        return;
    }
    let rec = AlertRecord {
        id: format!("run-{}", alerts.lock().map(|a| a.len()).unwrap_or(0)),
        severity: "warning".into(),
        title: "run_rejected".into(),
        detail: error.to_string(),
        governor_level: governor_level.to_string(),
        governor_score,
    };
    if let Ok(mut g) = alerts.lock() {
        g.push(rec.clone());
    }
    events.publish(EventBus::governor_alert(
        governor_level,
        governor_score,
        error,
    ));
}

pub fn alerts_json(alerts: &SharedAlerts) -> serde_json::Value {
    let list = alerts.lock().map(|g| g.clone()).unwrap_or_default();
    json!({ "alerts": list, "source": "governor_linkage" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_isa::GovernorConfig;
    use axiom_kernel::entropy::EntropyLevel;

    #[test]
    fn reject_emits_alert() {
        let events = crate::events::new_event_bus();
        let alerts = new_alert_log();
        let mut cfg = GovernorConfig::default();
        cfg.reject_from = EntropyLevel::Green;
        let mut g = axiom_isa::Governor::with_config(cfg);
        g.trip();
        let rec = link_governor_decision(&g, &events, &alerts, "test").expect("alert");
        assert_eq!(rec.title, "governor_reject");
        assert!(!alerts.lock().unwrap().is_empty());
    }
}
