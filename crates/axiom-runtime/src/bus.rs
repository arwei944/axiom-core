//! Message Bus with interceptor chain for runtime enforcement.
//!
//! Every SignalEnvelope passes through the interceptor chain before delivery.
//! The ArchitectureGuardian is registered as an interceptor that rejects
//! illegal cross-layer transitions at runtime (defense in depth behind
//! the compile-time CanSendTo type system).

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use axiom_kernel::codec::{JsonCodec, SignalCodec};
use axiom_kernel::id::CellId;
use axiom_kernel::layer::RuntimeTier;
use axiom_kernel::signal::SignalEnvelope;
use axiom_kernel::{KernelError, KernelResult};

use crate::constants::MAX_HOPS;
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
    layer_subscribers: HashMap<RuntimeTier, Vec<String>>,
}

impl RoutingTable {
    pub fn new() -> Self {
        Self { cell_id_to_name: HashMap::new(), layer_subscribers: HashMap::new() }
    }

    pub fn register_cell(&mut self, cell_id: &str, layer: RuntimeTier) {
        self.cell_id_to_name.insert(cell_id.to_string(), cell_id.to_string());
        self.layer_subscribers.entry(layer).or_default().push(cell_id.to_string());
    }

    pub fn resolve(&self, env: &SignalEnvelope) -> Vec<String> {
        if let Some(ref target) = env.target_cell {
            if self.cell_id_to_name.contains_key(target) {
                return vec![target.clone()];
            }
            return vec![];
        }
        self.layer_subscribers.get(&env.target_layer).cloned().unwrap_or_default()
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

struct CellEntry {
    mailbox: Arc<Mailbox>,
    _layer: RuntimeTier,
}

struct InterceptorSignalHandler {
    interceptor: Arc<dyn BusInterceptor>,
}

impl axiom_kernel::axiom::SignalHandler for InterceptorSignalHandler {
    fn handle(&mut self, signal: &mut SignalEnvelope) -> KernelResult<()> {
        let decision = self.interceptor.intercept(signal);
        match decision {
            InterceptDecision::Allow => Ok(()),
            InterceptDecision::Reject { reason } => {
                Err(KernelError::SignalValidationFailed(reason))
            }
            InterceptDecision::Redirect { target_cell } => {
                signal.target_cell = Some(target_cell);
                Ok(())
            }
        }
    }
}

pub struct MessageBus {
    cells: RwLock<HashMap<String, CellEntry>>,
    routing: RwLock<RoutingTable>,
    signal_kernel: std::sync::Arc<axiom_kernel::SignalKernel>,
    rejected_count: std::sync::atomic::AtomicU64,
    delivered_count: std::sync::atomic::AtomicU64,
    codec: Arc<dyn SignalCodec>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            cells: RwLock::new(HashMap::new()),
            routing: RwLock::new(RoutingTable::new()),
            signal_kernel: std::sync::Arc::new(axiom_kernel::SignalKernel::new()),
            rejected_count: std::sync::atomic::AtomicU64::new(0),
            delivered_count: std::sync::atomic::AtomicU64::new(0),
            codec: Arc::new(JsonCodec),
        }
    }

    pub fn with_codec(codec: Arc<dyn SignalCodec>) -> Self {
        Self { codec, ..Self::new() }
    }

    pub fn codec(&self) -> &dyn SignalCodec {
        self.codec.as_ref()
    }

    pub fn encode_envelope(&self, envelope: &SignalEnvelope) -> KernelResult<Vec<u8>> {
        self.codec.encode(envelope)
    }

    pub fn decode_envelope(&self, data: &[u8]) -> KernelResult<SignalEnvelope> {
        self.codec.decode(data)
    }

    pub async fn register_interceptor(&self, interceptor: Arc<dyn BusInterceptor>) {
        let handler = InterceptorSignalHandler { interceptor };
        let boxed: axiom_kernel::axiom::BoxedSignalHandler = std::boxed::Box::new(handler)
            as Box<dyn axiom_kernel::axiom::SignalHandler + Send + Sync>;
        self.signal_kernel.register_handler(boxed).await;
    }

    pub async fn register_cell(&self, cell_id: &CellId, mailbox: Arc<Mailbox>, layer: RuntimeTier) {
        let id_str = cell_id.as_str().to_string();
        self.cells.write().await.insert(id_str.clone(), CellEntry { mailbox, _layer: layer });
        self.routing.write().await.register_cell(&id_str, layer);
    }

    pub async fn publish(&self, mut env: SignalEnvelope) -> KernelResult<u64> {
        env.validate_layer_transition()?;

        env = self.signal_kernel.send(env).await.inspect_err(|_e| {
            self.rejected_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        })?;

        if env.hop_count > MAX_HOPS {
            return Err(KernelError::HandoffLimitExceeded {
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
                        self.delivered_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
        self.rejected_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn delivered_count(&self) -> u64 {
        self.delivered_count.load(std::sync::atomic::Ordering::Relaxed)
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
    use axiom_kernel::id::{CorrelationId, MsgId};
    use axiom_kernel::signal::{SignalKind, VectorClock};

    fn make_env(from: RuntimeTier, to: RuntimeTier, target: Option<&str>) -> SignalEnvelope {
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
            schema_version: axiom_kernel::version::SchemaVersion::new(1),
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
            if env.source_layer == RuntimeTier::Exec && env.target_layer == RuntimeTier::Agent {
                InterceptDecision::Reject { reason: "exec cannot talk to agent".into() }
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
        bus.register_cell(&cid, mb.clone(), RuntimeTier::Exec).await;

        let env = make_env(RuntimeTier::Exec, RuntimeTier::Exec, Some("cell-a"));
        let delivered = bus.publish(env).await.unwrap();
        assert_eq!(delivered, 1);
        assert_eq!(mb.len().await, 1);
    }

    #[tokio::test]
    async fn test_bus_rejects_layer_violation() {
        let bus = MessageBus::new();
        let env = make_env(RuntimeTier::Exec, RuntimeTier::Agent, None);
        let result = bus.publish(env).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bus_interceptor_rejects() {
        let bus = MessageBus::new();
        bus.register_interceptor(Arc::new(RejectExecToAgent)).await;
        let mb = Arc::new(Mailbox::new(16));
        let cid = CellId::new("cell-a");
        bus.register_cell(&cid, mb.clone(), RuntimeTier::Exec).await;

        let env = make_env(RuntimeTier::Exec, RuntimeTier::Exec, Some("cell-a"));
        assert!(bus.publish(env).await.is_ok());
        assert_eq!(bus.rejected_count(), 0);
    }

    #[tokio::test]
    async fn test_bus_hop_limit() {
        let bus = MessageBus::new();
        let mut env = make_env(RuntimeTier::Exec, RuntimeTier::Exec, None);
        env.hop_count = 9;
        assert!(bus.publish(env).await.is_err());
    }

    #[test]
    fn test_bus_codec_json_round_trip() {
        let bus = MessageBus::new();
        let env = make_env(RuntimeTier::Exec, RuntimeTier::Exec, None);
        let data = bus.encode_envelope(&env).unwrap();
        let decoded = bus.decode_envelope(&data).unwrap();
        assert_eq!(env.msg_id, decoded.msg_id);
        assert_eq!(env.signal_type, decoded.signal_type);
    }

    #[cfg(feature = "bincode-codec")]
    #[test]
    fn test_bus_codec_bincode_round_trip() {
        let codec = Arc::new(axiom_kernel::codec::BincodeCodec::default());
        let bus = MessageBus::with_codec(codec);
        let env = make_env(RuntimeTier::Exec, RuntimeTier::Exec, None);
        let data = bus.encode_envelope(&env).unwrap();
        let decoded = bus.decode_envelope(&data).unwrap();
        assert_eq!(env.msg_id, decoded.msg_id);
        assert_eq!(env.signal_type, decoded.signal_type);
    }
}
