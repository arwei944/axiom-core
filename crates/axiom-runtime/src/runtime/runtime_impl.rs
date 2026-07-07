use crate::bus::MessageBus;
use crate::dlq::DeadLetterQueue;
use crate::entropy_gov::EntropyGovernorCell;
use crate::supervisor::Supervisor;
use crate::AxiomRuntime;
use crate::RuntimeConfig;
use crate::RuntimeHealth;
use crate::runtime::RuntimeKernelBridge;
use std::sync::Arc;
use tokio::sync::RwLock;

impl AxiomRuntime {
    pub fn new(config: RuntimeConfig) -> Self {
        let bus = Arc::new(MessageBus::new());
        let supervisor = Arc::new(Supervisor::new());
        let governor = Arc::new(EntropyGovernorCell::default());
        let throttle_state = Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new()));
        let emergency_mode = Arc::new(parking_lot::RwLock::new(false));
        let events_since_snapshot =
            Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new()));
        Self {
            bus,
            supervisor,
            governor,
            config,
            cells: RwLock::new(Vec::new()),
            stop_tx: tokio::sync::Mutex::new(None),
            dispatch_handle: tokio::sync::Mutex::new(None),
            health: Arc::new(RwLock::new(RuntimeHealth::default())),
            dlq: Arc::new(DeadLetterQueue::default()),
            auto_interceptors: std::sync::atomic::AtomicBool::new(true),
            witness_store: Arc::new(RwLock::new(None)),
            snapshot_store: Arc::new(RwLock::new(None)),
            throttle_state,
            emergency_mode,
            events_since_snapshot,
            kernel_bridge: RuntimeKernelBridge::new(),
            #[cfg(feature = "metrics")]
            metrics_server: crate::MetricsServer::default(),
        }
    }

    pub fn dlq(&self) -> Arc<DeadLetterQueue> {
        self.dlq.clone()
    }

    pub fn bus(&self) -> Arc<MessageBus> {
        self.bus.clone()
    }

    pub fn supervisor(&self) -> Arc<Supervisor> {
        self.supervisor.clone()
    }

    pub fn governor(&self) -> Arc<EntropyGovernorCell> {
        self.governor.clone()
    }

    pub async fn set_witness_store(&self, store: Arc<dyn axiom_store::EventStore>) {
        *self.witness_store.write().await = Some(store);
    }

    pub async fn set_snapshot_store(&self, store: Arc<dyn axiom_store::SnapshotStore>) {
        *self.snapshot_store.write().await = Some(store);
    }
}

impl Default for AxiomRuntime {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}
