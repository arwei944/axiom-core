use crate::api::{DataSourceError, EntropySnapshotData, RuntimeDataSource, SignalEventData};
use crate::bus::MessageBus;
use crate::dlq::DeadLetterQueue;
use crate::entropy_gov::{EntropyGovernorCell, GovernanceAction};
use crate::runtime::RuntimeKernelBridge;
use crate::supervisor::Supervisor;
use crate::AxiomRuntime;
use crate::RuntimeConfig;
use crate::RuntimeHealth;
use axiom_kernel::entropy::EntropyLevel;
use axiom_kernel::heatmap::collector::UsageSnapshot;
use std::sync::Arc;
use tokio::sync::RwLock;

impl AxiomRuntime {
    pub fn new(config: RuntimeConfig) -> Self {
        let bus = Arc::new(MessageBus::new());
        // P1-2: plumb RuntimeConfig backoff into Supervisor
        let supervisor = Arc::new(Supervisor::with_backoff(crate::supervisor::BackoffConfig {
            base_ms: config.backoff_base_ms,
            cap_ms: config.backoff_cap_ms,
            multiplier: config.backoff_multiplier,
        }));
        let governor = Arc::new(EntropyGovernorCell::default());
        let throttle_state = Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new()));
        let emergency_mode = Arc::new(parking_lot::RwLock::new(false));
        let events_since_snapshot =
            Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new()));
        let dlq_capacity = config.dlq_capacity;
        Self {
            bus,
            supervisor,
            governor,
            config,
            cells: RwLock::new(Vec::new()),
            stop_tx: tokio::sync::Mutex::new(None),
            dispatch_handle: tokio::sync::Mutex::new(None),
            health: Arc::new(RwLock::new(RuntimeHealth::default())),
            dlq: Arc::new(DeadLetterQueue::new(dlq_capacity)),
            auto_interceptors: std::sync::atomic::AtomicBool::new(true),
            witness_store: Arc::new(RwLock::new(None)),
            snapshot_store: Arc::new(RwLock::new(None)),
            throttle_state,
            emergency_mode,
            events_since_snapshot,
            kernel_bridge: RuntimeKernelBridge::new(),
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

impl RuntimeDataSource for AxiomRuntime {
    fn get_health(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<RuntimeHealth, DataSourceError>> + Send + '_>,
    > {
        Box::pin(async { Ok(self.health.read().await.clone()) })
    }

    fn get_cells(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Vec<crate::runtime::RegisteredCell>, DataSourceError>,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async { Ok(self.cells.read().await.clone()) })
    }

    fn get_entropy_snapshot(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<EntropySnapshotData, DataSourceError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async {
            let snapshot = self.governor.snapshot();
            let level_str = match snapshot.level {
                EntropyLevel::Green => "green",
                EntropyLevel::Yellow => "yellow",
                EntropyLevel::Red => "red",
                EntropyLevel::Critical => "critical",
            };
            let last_action_str = snapshot.last_action.as_ref().map(|a| match a {
                GovernanceAction::None => "none".to_string(),
                GovernanceAction::Warn { message } => format!("warn: {}", message),
                GovernanceAction::Throttle { target_cell, factor } => {
                    format!(
                        "throttle: {} at factor {}",
                        target_cell.as_deref().unwrap_or("all"),
                        factor
                    )
                }
                GovernanceAction::Emergency { reason } => format!("emergency: {}", reason),
            });
            let per_cell: Vec<(String, f64)> = snapshot.per_cell.into_iter().collect();
            Ok(EntropySnapshotData {
                global_value: snapshot.global.value,
                level: level_str.to_string(),
                per_cell,
                last_action: last_action_str,
            })
        })
    }

    fn get_heatmap(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<UsageSnapshot, DataSourceError>> + Send + '_>,
    > {
        Box::pin(async { Ok(self.kernel_bridge.heatmap.read().await.snapshot()) })
    }

    fn subscribe_signals(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<SignalEventData>, DataSourceError> {
        Err(DataSourceError::SignalSubscribeFailed)
    }
}
