//! Domain event bus for SSE push (commercial floor).

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;

pub const EVENT_TASK_COMPLETED: &str = "task.completed";
pub const EVENT_GOVERNOR_ALERT: &str = "governor.alert";
pub const EVENT_RUN_RECORDED: &str = "run.recorded";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub r#type: String,
    pub ts_ms: u64,
    pub payload: serde_json::Value,
}

impl DomainEvent {
    pub fn new(type_name: impl Into<String>, payload: serde_json::Value) -> Self {
        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            r#type: type_name.into(),
            ts_ms,
            payload,
        }
    }

    pub fn to_sse_data(&self) -> String {
        let body = serde_json::to_string(self).unwrap_or_else(|_| "{}".into());
        format!("event: {}\ndata: {}\n\n", self.r#type, body)
    }

    pub fn json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<DomainEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: DomainEvent) {
        let _ = self.tx.send(event);
    }

    pub fn task_completed(ok: bool, label: &str, governor_level: &str, witness_count: usize) -> DomainEvent {
        DomainEvent::new(
            EVENT_TASK_COMPLETED,
            json!({
                "ok": ok,
                "label": label,
                "governor_level": governor_level,
                "witness_count": witness_count,
            }),
        )
    }

    pub fn governor_alert(level: &str, score: f64, reason: &str) -> DomainEvent {
        DomainEvent::new(
            EVENT_GOVERNOR_ALERT,
            json!({
                "level": level,
                "score": score,
                "reason": reason,
                "admit_authority": "governor",
            }),
        )
    }
}

pub type SharedEventBus = Arc<EventBus>;

pub fn new_event_bus() -> SharedEventBus {
    Arc::new(EventBus::new(64))
}

/// Encode helpers used by path tests (real modules, no mock reimplementation).
pub fn encode_task_completed_sse(
    ok: bool,
    label: &str,
    governor_level: &str,
    witness_count: usize,
) -> String {
    EventBus::task_completed(ok, label, governor_level, witness_count).to_sse_data()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_payload_contains_type() {
        let s = encode_task_completed_sse(true, "ship", "Green", 5);
        assert!(s.contains("event: task.completed"), "{s}");
        assert!(s.contains("\"ok\":true"), "{s}");
        assert!(s.contains("ship"), "{s}");
    }

    #[tokio::test]
    async fn bus_delivers_to_subscriber() {
        let bus = EventBus::new(8);
        let mut rx = bus.subscribe();
        bus.publish(EventBus::governor_alert("Red", 0.9, "melt"));
        let ev = rx.recv().await.expect("event");
        assert_eq!(ev.r#type, EVENT_GOVERNOR_ALERT);
        assert!(ev.payload["reason"].as_str().unwrap().contains("melt"));
    }
}
