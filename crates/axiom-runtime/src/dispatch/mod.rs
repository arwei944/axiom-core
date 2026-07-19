pub mod r#loop;
pub mod witness;

pub use witness::witness_to_event;

type RegisteredCellData = (
    std::sync::Arc<crate::mailbox::Mailbox>,
    axiom_kernel::id::CellId,
    axiom_kernel::layer::RuntimeTier,
    Option<std::sync::Arc<tokio::sync::Mutex<axiom_kernel::cell::RuntimeCellHandle>>>,
    Option<std::sync::Arc<dyn Fn() -> axiom_kernel::cell::RuntimeCellHandle + Send + Sync>>,
);

#[derive(Clone)]
pub struct DispatchContext {
    pub bus: std::sync::Arc<crate::bus::MessageBus>,
    pub supervisor: std::sync::Arc<crate::supervisor::Supervisor>,
    pub governor: std::sync::Arc<crate::entropy_gov::EntropyGovernorCell>,
    pub witness_store:
        std::sync::Arc<tokio::sync::RwLock<Option<std::sync::Arc<dyn axiom_store::EventStore>>>>,
    pub snapshot_store:
        std::sync::Arc<tokio::sync::RwLock<Option<std::sync::Arc<dyn axiom_store::SnapshotStore>>>>,
    pub throttle_state: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, f64>>>,
    pub emergency_mode: std::sync::Arc<parking_lot::RwLock<bool>>,
    pub dlq: std::sync::Arc<crate::dlq::DeadLetterQueue>,
    pub events_since_snapshot:
        std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, u64>>>,
    pub cell_kernel: Option<std::sync::Arc<axiom_kernel::CellKernel>>,
    /// Shared with health poller — production liveness probe (P2-4).
    pub health: std::sync::Arc<tokio::sync::RwLock<crate::runtime::RuntimeHealth>>,
}

impl DispatchContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bus: std::sync::Arc<crate::bus::MessageBus>,
        supervisor: std::sync::Arc<crate::supervisor::Supervisor>,
        governor: std::sync::Arc<crate::entropy_gov::EntropyGovernorCell>,
        witness_store: std::sync::Arc<
            tokio::sync::RwLock<Option<std::sync::Arc<dyn axiom_store::EventStore>>>,
        >,
        snapshot_store: std::sync::Arc<
            tokio::sync::RwLock<Option<std::sync::Arc<dyn axiom_store::SnapshotStore>>>,
        >,
        throttle_state: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, f64>>>,
        emergency_mode: std::sync::Arc<parking_lot::RwLock<bool>>,
        dlq: std::sync::Arc<crate::dlq::DeadLetterQueue>,
        events_since_snapshot: std::sync::Arc<
            parking_lot::RwLock<std::collections::HashMap<String, u64>>,
        >,
        cell_kernel: Option<std::sync::Arc<axiom_kernel::CellKernel>>,
        health: std::sync::Arc<tokio::sync::RwLock<crate::runtime::RuntimeHealth>>,
    ) -> Self {
        Self {
            bus,
            supervisor,
            governor,
            witness_store,
            snapshot_store,
            throttle_state,
            emergency_mode,
            dlq,
            events_since_snapshot,
            cell_kernel,
            health,
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub async fn dispatch_loop(
    rx: tokio::sync::oneshot::Receiver<()>,
    poll_interval: u64,
    cells_data: Vec<RegisteredCellData>,
    ctx: DispatchContext,
) {
    r#loop::run_dispatch_loop(rx, poll_interval, cells_data, ctx).await;
}
