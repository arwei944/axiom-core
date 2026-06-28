//! Axiom Runtime - the main entry point that wires all components.
//!
//! Boots up Bus (with ArchitectureGuardian interceptor), Mailbox per
//! Cell, Supervisor per Cell, EntropyGovernor, and runs the dispatcher
//! loop. Validates migration chain, version compatibility, and
//! startup preflight before processing any messages.
//!
//! L2 Oversight interceptors are provided by the axiom-oversight crate
//! and can be registered via bus().register_interceptor().

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{oneshot, RwLock};
use tokio::task::JoinHandle;

use axiom_core::error::AxiomError;
use axiom_core::id::{CellId, CorrelationId};
use axiom_core::layer::Layer;
use axiom_core::signal::{SignalEnvelope, SignalKind, VectorClock};

use crate::bus::MessageBus;
use crate::entropy_gov::EntropyGovernor;
use crate::guardian::ArchitectureGuardian;
use crate::mailbox::Mailbox;
use crate::supervisor::Supervisor;

pub struct CellRegistration {
    pub id: CellId,
    pub layer: Layer,
    pub version: axiom_core::version::Version,
    pub supervision_strategy: axiom_core::cell::SupervisionStrategy,
}

pub struct RuntimeConfig {
    pub mailbox_capacity: usize,
    pub entropy_threshold: f64,
    pub entropy_cooldown_ms: u64,
    pub dispatch_poll_interval_ms: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            mailbox_capacity: 1024,
            entropy_threshold: 100.0,
            entropy_cooldown_ms: 60_000,
            dispatch_poll_interval_ms: 10,
        }
    }
}

struct RegisteredCell {
    id: CellId,
    mailbox: Arc<Mailbox>,
    #[allow(dead_code)]
    layer: Layer,
    #[allow(dead_code)]
    version: axiom_core::version::Version,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeHealth {
    pub started: bool,
    pub uptime_ms: u64,
    pub cells_running: u64,
    pub cells_stopped: u64,
    pub total_restarts: u64,
    pub messages_delivered: u64,
    pub messages_rejected: u64,
    pub entropy_score: f64,
    pub preflight_passed: bool,
}

static CMD_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_msg_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let n = CMD_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("cmd-{ts}-{n}")
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

pub struct RuntimeBuilder {
    config: RuntimeConfig,
    auto_register_builtin_interceptors: bool,
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
            auto_register_builtin_interceptors: true,
        }
    }

    pub fn with_config(mut self, config: RuntimeConfig) -> Self {
        self.config = config;
        self
    }

    pub fn mailbox_capacity(mut self, cap: usize) -> Self {
        self.config.mailbox_capacity = cap;
        self
    }

    pub fn auto_register_builtins(mut self, b: bool) -> Self {
        self.auto_register_builtin_interceptors = b;
        self
    }

    pub fn build(self) -> AxiomRuntime {
        let rt = AxiomRuntime::new(self.config);
        rt.auto_interceptors
            .store(self.auto_register_builtin_interceptors, Ordering::Relaxed);
        rt
    }
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AxiomRuntime {
    bus: Arc<MessageBus>,
    supervisor: Arc<Supervisor>,
    governor: Arc<EntropyGovernor>,
    config: RuntimeConfig,
    cells: RwLock<Vec<RegisteredCell>>,
    stop_tx: tokio::sync::Mutex<Option<oneshot::Sender<()>>>,
    dispatch_handle: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    health: Arc<RwLock<RuntimeHealth>>,
    dlq: Arc<crate::dlq::DeadLetterQueue>,
    auto_interceptors: std::sync::atomic::AtomicBool,
}

impl AxiomRuntime {
    pub fn new(config: RuntimeConfig) -> Self {
        let bus = Arc::new(MessageBus::new());
        let supervisor = Arc::new(Supervisor::new());
        let governor = Arc::new(EntropyGovernor::new(config.entropy_threshold));
        Self {
            bus,
            supervisor,
            governor,
            config,
            cells: RwLock::new(Vec::new()),
            stop_tx: tokio::sync::Mutex::new(None),
            dispatch_handle: tokio::sync::Mutex::new(None),
            health: Arc::new(RwLock::new(RuntimeHealth::default())),
            dlq: Arc::new(crate::dlq::DeadLetterQueue::default()),
            auto_interceptors: std::sync::atomic::AtomicBool::new(true),
        }
    }

    pub fn dlq(&self) -> Arc<crate::dlq::DeadLetterQueue> {
        self.dlq.clone()
    }

    pub fn bus(&self) -> Arc<MessageBus> {
        self.bus.clone()
    }

    pub fn supervisor(&self) -> Arc<Supervisor> {
        self.supervisor.clone()
    }

    pub fn governor(&self) -> Arc<EntropyGovernor> {
        self.governor.clone()
    }

    pub async fn mailbox_for(&self, cell_id: &str) -> Option<Arc<Mailbox>> {
        let cells = self.cells.read().await;
        cells
            .iter()
            .find(|c| c.id.as_str() == cell_id)
            .map(|c| c.mailbox.clone())
    }

    pub async fn register_cell(&self, reg: CellRegistration) -> Result<Arc<Mailbox>, AxiomError> {
        let mailbox = Arc::new(Mailbox::new(self.config.mailbox_capacity));
        self.bus
            .register_cell(&reg.id, mailbox.clone(), reg.layer)
            .await;
        self.supervisor
            .register_cell(reg.id.as_str(), reg.supervision_strategy)
            .await;

        self.cells.write().await.push(RegisteredCell {
            id: reg.id.clone(),
            mailbox: mailbox.clone(),
            layer: reg.layer,
            version: reg.version,
        });
        Ok(mailbox)
    }

    async fn preflight(&self) -> Result<(), Vec<String>> {
        let mut issues = Vec::new();

        if let Err(gaps) = axiom_core::verify_migration_chain_completeness(1) {
            issues.extend(gaps);
        }

        let cells = self.cells.read().await;
        for reg in cells.iter() {
            if reg.version.major != 0 {
                issues.push(format!(
                    "cell {} has non-zero major version {}: not 0.x pre-release",
                    reg.id.as_str(),
                    reg.version.major
                ));
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    pub async fn start(&self) -> Result<(), AxiomError> {
        match self.preflight().await {
            Ok(()) => {
                tracing::info!("preflight passed");
            }
            Err(issues) => {
                for i in &issues {
                    tracing::error!("preflight: {i}");
                }
                return Err(AxiomError::Internal(format!(
                    "preflight failed: {}",
                    issues.join("; ")
                )));
            }
        }

        let guardian = Arc::new(ArchitectureGuardian::new());
        self.bus.register_interceptor(guardian).await;

        if self.auto_interceptors.load(Ordering::Relaxed) {
            self.bus
                .register_interceptor(Arc::new(crate::interceptors::HopLimitInterceptor::default()))
                .await;
            self.bus
                .register_interceptor(Arc::new(
                    crate::interceptors::IdempotencyInterceptor::default(),
                ))
                .await;
            self.bus
                .register_interceptor(Arc::new(crate::interceptors::SchemaVersionInterceptor))
                .await;
            self.bus
                .register_interceptor(Arc::new(
                    crate::interceptors::LoopDetectInterceptor::default(),
                ))
                .await;
        }

        let (tx, rx) = oneshot::channel::<()>();
        *self.stop_tx.lock().await = Some(tx);

        let cells_count = self.cells.read().await.len();
        {
            let mut h = self.health.write().await;
            h.started = true;
            h.preflight_passed = true;
            h.cells_running = cells_count as u64;
        }

        let _bus = self.bus.clone();
        let supervisor = self.supervisor.clone();
        let governor = self.governor.clone();
        let poll_interval = self.config.dispatch_poll_interval_ms;
        let entropy_cooldown = self.config.entropy_cooldown_ms;

        let cells_data: Vec<_> = self
            .cells
            .read()
            .await
            .iter()
            .map(|r| (r.mailbox.clone(), r.id.clone()))
            .collect();
        let cells_len = cells_data.len();

        let handle = tokio::spawn(async move {
            let mut rx = rx;
            let mut interval = tokio::time::interval(Duration::from_millis(poll_interval));
            loop {
                tokio::select! {
                    _ = &mut rx => {
                        tracing::info!("runtime dispatcher shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        for (mb, cid) in &cells_data {
                            if !supervisor.before_handle(cid.as_str()).await {
                                continue;
                            }
                            while mb.pop().await.is_some() {
                                supervisor.record_success(cid.as_str()).await;
                            }
                        }

                        if governor.should_reduce(entropy_cooldown) {
                            tracing::warn!(
                                score = governor.snapshot().score,
                                "entropy threshold exceeded; auto-reduction triggered"
                            );
                            governor.reset();
                        }
                    }
                }
            }
        });

        *self.dispatch_handle.lock().await = Some(handle);

        let bus_h = self.bus.clone();
        let gov_h = self.governor.clone();
        let health = self.health.clone();
        tokio::spawn(async move {
            let start = Instant::now();
            let mut tick = tokio::time::interval(Duration::from_secs(1));
            loop {
                tick.tick().await;
                let mut h = health.write().await;
                h.uptime_ms = start.elapsed().as_millis() as u64;
                h.messages_delivered = bus_h.delivered_count();
                h.messages_rejected = bus_h.rejected_count();
                h.entropy_score = gov_h.snapshot().score;
                h.total_restarts = 0;
                h.cells_running = cells_len as u64;
            }
        });

        tracing::info!("runtime started with {cells_count} cells");
        Ok(())
    }

    pub async fn stop(&self) {
        if let Some(tx) = self.stop_tx.lock().await.take() {
            let _ = tx.send(());
        }
        if let Some(h) = self.dispatch_handle.lock().await.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), h).await;
        }
        self.health.write().await.started = false;
        tracing::info!("runtime stopped");
    }

    pub async fn health(&self) -> RuntimeHealth {
        self.health.read().await.clone()
    }

    pub async fn publish_command(
        &self,
        signal_type: &str,
        payload: serde_json::Value,
        target_cell: Option<&str>,
        target_layer: Layer,
    ) -> Result<u64, AxiomError> {
        let id = next_msg_id();
        let corr_id = format!("corr-{id}");
        let env = SignalEnvelope {
            msg_id: axiom_core::id::MsgId::new(id),
            correlation_id: CorrelationId::new(corr_id),
            trace_id: None,
            signal_type: signal_type.to_string(),
            vector_clock: VectorClock::new(),
            timestamp_ns: now_ns(),
            kind: SignalKind::Command,
            source_layer: Layer::Oversight,
            target_layer,
            source_cell: None,
            target_cell: target_cell.map(|s| s.to_string()),
            payload,
            schema_version: axiom_core::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        };
        self.bus.publish(env).await
    }
}

impl Default for AxiomRuntime {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_core::id::{CorrelationId, MsgId};
    use axiom_core::signal::{SignalKind, VectorClock};

    fn env_from_to(from: Layer, to: Layer, target: Option<&str>) -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new("test-msg"),
            correlation_id: CorrelationId::new("test-corr"),
            trace_id: None,
            signal_type: "Test".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: now_ns(),
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

    #[tokio::test]
    async fn test_l2_guardian_blocks_exec_to_agent() {
        let rt = AxiomRuntime::new(RuntimeConfig {
            mailbox_capacity: 16,
            ..Default::default()
        });
        let _mb = rt
            .register_cell(CellRegistration {
                id: CellId::new("exec-cell"),
                layer: Layer::Exec,
                version: axiom_core::version::Version::new(0, 1, 0),
                supervision_strategy: axiom_core::cell::SupervisionStrategy::Restart {
                    max_retries: 2,
                },
            })
            .await
            .unwrap();
        rt.start().await.unwrap();

        let bad = env_from_to(Layer::Exec, Layer::Agent, None);
        let result = rt.bus().publish(bad).await;
        assert!(
            result.is_err(),
            "Exec->Agent must be blocked by L2 guardian (compile-time CanSendTo already prevents it, but runtime doubles down)"
        );

        let good = env_from_to(Layer::Oversight, Layer::Exec, Some("exec-cell"));
        assert!(rt.bus().publish(good).await.is_ok());

        rt.stop().await;
    }

    #[tokio::test]
    async fn test_runtime_health_updates() {
        let rt = AxiomRuntime::default();
        let _mb = rt
            .register_cell(CellRegistration {
                id: CellId::new("cell-1"),
                layer: Layer::Exec,
                version: axiom_core::version::Version::new(0, 1, 0),
                supervision_strategy: axiom_core::cell::SupervisionStrategy::default(),
            })
            .await
            .unwrap();
        rt.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        let h = rt.health().await;
        assert!(h.started);
        assert!(h.preflight_passed);
        assert_eq!(h.cells_running, 1);
        rt.stop().await;
    }

    #[tokio::test]
    async fn test_entropy_governor_triggers() {
        let gov = Arc::new(EntropyGovernor::new(1.0));
        for _ in 0..5 {
            gov.record_restart();
        }
        let snap = gov.snapshot();
        assert!(snap.score > 1.0, "score should exceed threshold");
    }

    #[tokio::test]
    async fn test_preflight_rejects_bad_version() {
        let rt = AxiomRuntime::default();
        let _ = rt
            .register_cell(CellRegistration {
                id: CellId::new("v1-cell"),
                layer: Layer::Exec,
                version: axiom_core::version::Version::new(1, 0, 0),
                supervision_strategy: axiom_core::cell::SupervisionStrategy::default(),
            })
            .await
            .unwrap();
        let result = rt.start().await;
        assert!(
            result.is_err(),
            "preflight must reject cells with non-zero major version"
        );
    }
}
