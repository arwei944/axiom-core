use crate::runtime::kernel_bridge::RuntimeKernelBridge;
use crate::entropy_gov::EntropyGovernorCell;
use crate::mailbox::Mailbox;
use crate::supervisor::Supervisor;
use axiom_kernel::cell::RuntimeCellHandle;
use axiom_kernel::cell::SupervisionStrategy;
use axiom_kernel::id::CellId;
use axiom_kernel::layer::Layer;
use axiom_kernel::version::Version;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

pub struct CellRegistration {
    /// Cell 唯一标识。
    pub id: CellId,
    /// Cell 所属层，用于层间访问控制。
    pub layer: Layer,
    /// Cell 实现的 schema 版本。
    pub version: Version,
    /// 监管策略：重启、停止、熔断或升级。
    pub supervision_strategy: SupervisionStrategy,
    /// 已实例化的 Cell 句柄；与 `factory` 二选一。
    pub cell: Option<RuntimeCellHandle>,
    /// Cell 工厂函数，用于按需创建实例。
    pub factory: Option<Arc<dyn Fn() -> RuntimeCellHandle + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Mailbox 容量上限。
    pub mailbox_capacity: usize,
    /// 熵分阈值，超过后触发治理动作。
    pub entropy_threshold: f64,
    /// 同 Cell 熵事件冷却时间。
    pub entropy_cooldown_ms: u64,
    /// Dispatch 轮询间隔。
    pub dispatch_poll_interval_ms: u64,
    /// Metrics 服务监听地址。
    pub metrics_endpoint: Option<String>,
    /// 是否启用遥测。
    pub telemetry_enabled: bool,
}

pub struct RegisteredCell {
    id: CellId,
    mailbox: Arc<Mailbox>,
    layer: Layer,
    version: Version,
    cell: Option<Arc<TokioMutex<RuntimeCellHandle>>>,
    factory: Option<Arc<dyn Fn() -> RuntimeCellHandle + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub struct RuntimeHealth {
    /// Runtime 是否已启动。
    pub started: bool,
    /// 运行时长（毫秒）。
    pub uptime_ms: u64,
    /// 当前运行中的 Cell 数量。
    pub cells_running: u64,
    /// 已停止的 Cell 数量。
    pub cells_stopped: u64,
    /// 累计重启次数。
    pub total_restarts: u64,
    /// 已投递消息总数。
    pub messages_delivered: u64,
    /// 被拒绝消息总数。
    pub messages_rejected: u64,
    /// 当前熵分。
    pub entropy_score: f64,
    /// 启动自检是否通过。
    pub preflight_passed: bool,
    /// Metrics 服务地址。
    pub metrics_endpoint: Option<String>,
    /// 遥测是否启用。
    pub telemetry_enabled: bool,
    /// 事件存储是否连接。
    pub store_connected: bool,
    /// 快照存储是否连接。
    pub snapshot_store_connected: bool,
}

pub struct RuntimeBuilder {
    config: RuntimeConfig,
    auto_register_builtin_interceptors: bool,
}

pub struct AxiomRuntime {
    bus: std::sync::Arc<crate::bus::MessageBus>,
    supervisor: std::sync::Arc<Supervisor>,
    governor: std::sync::Arc<EntropyGovernorCell>,
    config: RuntimeConfig,
    cells: tokio::sync::RwLock<Vec<RegisteredCell>>,
    stop_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    dispatch_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    health: std::sync::Arc<tokio::sync::RwLock<RuntimeHealth>>,
    dlq: std::sync::Arc<crate::dlq::DeadLetterQueue>,
    auto_interceptors: std::sync::atomic::AtomicBool,
    witness_store:
        std::sync::Arc<tokio::sync::RwLock<Option<std::sync::Arc<dyn axiom_store::EventStore>>>>,
    snapshot_store:
        std::sync::Arc<tokio::sync::RwLock<Option<std::sync::Arc<dyn axiom_store::SnapshotStore>>>>,
    throttle_state: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, f64>>>,
    emergency_mode: std::sync::Arc<parking_lot::RwLock<bool>>,
    events_since_snapshot:
        std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, u64>>>,
    #[cfg(feature = "metrics")]
    metrics_server: crate::MetricsServer,
    pub kernel_bridge: RuntimeKernelBridge,
}

pub mod builder;
pub mod commands;
pub mod config;
pub mod health;
pub mod kernel_bridge;
pub mod monitoring;
pub mod registration;
pub mod runtime_impl;
pub mod start;
#[cfg(test)]
pub mod tests;
