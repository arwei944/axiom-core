//! Message Bus with interceptor chain for runtime enforcement.
//!
//! Every SignalEnvelope passes through the interceptor chain before delivery.
//! The ArchitectureGuardian is registered as an interceptor that rejects
//! illegal cross-layer transitions at runtime (defense in depth behind
//! the compile-time CanSendTo type system).

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use axiom_core::error::AxiomError;
use axiom_core::id::CellId;
use axiom_core::layer::Layer;
use axiom_core::signal::SignalEnvelope;

use crate::mailbox::Mailbox;

#[derive(Debug, Clone)]
pub enum InterceptDecision {
    Allow,
    Reject { reason: String },
    Redirect { target_cell: String },
}

pub trait BusInterceptor: Send + Sync {
    fn name(&self) -> &'static str;
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision;
}

pub struct RoutingTable {
    cell_id_to_name: HashMap<String, String>,
    layer_subscribers: HashMap<Layer, Vec<String>>,
}

impl RoutingTable {
    pub fn new() -> Self {
        Self {
            cell_id_to_name: HashMap::new(),
            layer_subscribers: HashMap::new(),
        }
    }

    pub fn register_cell(&mut self, cell_id: &str, layer: Layer) {
        self.cell_id_to_name
            .insert(cell_id.to_string(), cell_id.to_string());
        self.layer_subscribers
            .entry(layer)
            .or_default()
            .push(cell_id.to_string());
    }

    pub fn resolve(&self, env: &SignalEnvelope) -> Vec<String> {
        if let Some(ref target) = env.target_cell {
            if self.cell_id_to_name.contains_key(target) {
                return vec![target.clone()];
            }
            return vec![];
        }
        self.layer_subscribers
            .get(&env.target_layer)
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

struct CellEntry {
    mailbox: Arc<Mailbox>,
    #[allow(dead_code)]
    layer: Layer,
}

pub struct MessageBus {
    cells: RwLock<HashMap<String, CellEntry>>,
    routing: RwLock<RoutingTable>,
    interceptors: RwLock<Vec<Arc<dyn BusInterceptor>>>,
    rejected_count: std::sync::atomic::AtomicU64,
    delivered_count: std::sync::atomic::AtomicU64,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            cells: RwLock::new(HashMap::new()),
            routing: RwLock::new(RoutingTable::new()),
            interceptors: RwLock::new(Vec::new()),
            rejected_count: std::sync::atomic::AtomicU64::new(0),
            delivered_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub async fn register_interceptor(&self, interceptor: Arc<dyn BusInterceptor>) {
        self.interceptors.write().await.push(interceptor);
    }

    pub async fn register_cell(&self, cell_id: &CellId, mailbox: Arc<Mailbox>, layer: Layer) {
        let id_str = cell_id.as_str().to_string();
        self.cells
            .write()
            .await
            .insert(id_str.clone(), CellEntry { mailbox, layer });
        self.routing.write().await.register_cell(&id_str, layer);
    }

    pub async fn publish(&self, mut env: SignalEnvelope) -> Result<u64, AxiomError> {
        env.validate_layer_transition()?;

        for interceptor in self.interceptors.read().await.iter() {
            match interceptor.intercept(&env) {
                InterceptDecision::Allow => {}
                InterceptDecision::Reject { reason } => {
                    self.rejected_count
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return Err(AxiomError::LayerViolation {
                        from: env.source_layer,
                        to: env.target_layer,
                        signal_type: format!(
                            "{} (rejected by {}: {})",
                            env.signal_type,
                            interceptor.name(),
                            reason
                        ),
                        source_cell: env.source_cell.clone().unwrap_or_default(),
                    });
                }
                InterceptDecision::Redirect { target_cell } => {
                    env.target_cell = Some(target_cell);
                }
            }
        }

        if env.hop_count > 8 {
            return Err(AxiomError::HandoffLimitExceeded {
                msg_id: env.msg_id.to_string(),
                hops: env.hop_count,
                correlation_id: env.correlation_id.to_string(),
            });
        }

        let targets = self.routing.read().await.resolve(&env);
        let mut delivered = 0u64;
        let cells = self.cells.read().await;

        for target in &targets {
            if let Some(entry) = cells.get(target) {
                match entry.mailbox.push(env.clone()).await {
                    Ok(()) => {
                        delivered += 1;
                        self.delivered_count
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    Err(_) => {
                        tracing::warn!(
                            target_cell = target,
                            signal_type = %env.signal_type,
                            "mailbox full, message dropped"
                        );
                    }
                }
            }
        }

        Ok(delivered)
    }

    pub fn rejected_count(&self) -> u64 {
        self.rejected_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn delivered_count(&self) -> u64 {
        self.delivered_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn cell_count(&self) -> usize {
        self.cells.read().await.len()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_core::id::{CorrelationId, MsgId};
    use axiom_core::signal::{SignalKind, VectorClock};

    fn make_env(from: Layer, to: Layer, target: Option<&str>) -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new("test"),
            correlation_id: CorrelationId::new("c1"),
            trace_id: None,
            signal_type: "TestSignal".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            kind: SignalKind::Command,
            source_layer: from,
            target_layer: to,
            source_cell: None,
            target_cell: target.map(|s| s.to_string()),
            payload: serde_json::Value::Null,
            schema_version: axiom_core::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        }
    }

    struct RejectExecToAgent;
    impl BusInterceptor for RejectExecToAgent {
        fn name(&self) -> &'static str {
            "reject-exec-agent"
        }
        fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
            if env.source_layer == Layer::Exec && env.target_layer == Layer::Agent {
                InterceptDecision::Reject {
                    reason: "exec cannot talk to agent".into(),
                }
            } else {
                InterceptDecision::Allow
            }
        }
    }

    #[tokio::test]
    async fn test_bus_delivers_to_registered_cell() {
        let bus = MessageBus::new();
        let mb = Arc::new(Mailbox::new(16));
        let cid = CellId::new("cell-a");
        bus.register_cell(&cid, mb.clone(), Layer::Exec).await;

        let env = make_env(Layer::Exec, Layer::Exec, Some("cell-a"));
        let delivered = bus.publish(env).await.unwrap();
        assert_eq!(delivered, 1);
        assert_eq!(mb.len().await, 1);
    }

    #[tokio::test]
    async fn test_bus_rejects_layer_violation() {
        let bus = MessageBus::new();
        let env = make_env(Layer::Exec, Layer::Agent, None);
        let result = bus.publish(env).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bus_interceptor_rejects() {
        let bus = MessageBus::new();
        bus.register_interceptor(Arc::new(RejectExecToAgent)).await;
        let mb = Arc::new(Mailbox::new(16));
        let cid = CellId::new("cell-a");
        bus.register_cell(&cid, mb.clone(), Layer::Exec).await;

        let env = make_env(Layer::Exec, Layer::Exec, Some("cell-a"));
        assert!(bus.publish(env).await.is_ok());
        assert_eq!(bus.rejected_count(), 0);
    }

    #[tokio::test]
    async fn test_bus_hop_limit() {
        let bus = MessageBus::new();
        let mut env = make_env(Layer::Exec, Layer::Exec, None);
        env.hop_count = 9;
        assert!(bus.publish(env).await.is_err());
    }
}
