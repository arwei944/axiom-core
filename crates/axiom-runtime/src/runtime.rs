//! Axiom Runtime - the main entry point that wires all components.
//!
//! Boots up Bus (with ArchitectureGuardian interceptor), Mailbox per
//! Cell, Supervisor per Cell, EntropyGovernorCell, and runs the dispatcher
//! loop. Validates migration chain, version compatibility, and
//! startup preflight before processing any messages.
//!
//! L2 Oversight interceptors are provided by the axiom-oversight crate
//! and can be registered via bus().register_interceptor().

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{oneshot, RwLock};
use tokio::task::JoinHandle;

use axiom_core::context::CellContext;
use axiom_core::error::AxiomError;
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::signal::{now_ns, SignalEnvelope, SignalKind, VectorClock};

use crate::bus::MessageBus;
use crate::constraint_validator::{ConstraintValidator, ValidationContext};
use crate::entropy_gov::{EntropyEvent, EntropyGovernorCell, GovernanceAction};
use crate::entropy_interceptors::{EmergencyInterceptor, ThrottleInterceptor};
use crate::guardian::ArchitectureGuardian;
use crate::mailbox::Mailbox;
use crate::supervisor::{SupervisionDecision, Supervisor};
use futures::FutureExt;
use std::panic::AssertUnwindSafe;

fn witness_to_event(
    witness: &axiom_core::witness::Witness,
    layer: Layer,
) -> Result<axiom_store::Event, AxiomError> {
    let payload =
        serde_json::to_value(witness).map_err(|e| AxiomError::WitnessSerialization {
            cell_id: witness.cell_id.clone(),
            message: e.to_string(),
        })?;

    let outcome = match &witness.outcome {
        axiom_core::witness::TransitionOutcome::Success => axiom_store::EventOutcome::Success,
        axiom_core::witness::TransitionOutcome::Failed { reason } => {
            axiom_store::EventOutcome::Failed { reason: reason.clone() }
        }
        axiom_core::witness::TransitionOutcome::AxiomViolated {
            axiom_name,
            message,
        } => axiom_store::EventOutcome::AxiomViolated {
            axiom_name: axiom_name.clone(),
            message: message.clone(),
        },
    };

    let witness_hash_data = axiom_store::WitnessHashData {
        prev_hash: witness.prev_hash.as_ref().map(|h| h.0),
        state_before_hash: witness.state_before_hash.as_ref().map(|h| h.0),
        state_after_hash: witness.state_after_hash.as_ref().map(|h| h.0),
        hash: witness.hash.0,
        signal_fingerprint: witness.signal_fingerprint,
    };

    let event = axiom_store::EventBuilder::new(&witness.cell_id, "witness", payload)
        .event_id(witness.witness_id.as_str())
        .cell_id(&witness.cell_id)
        .correlation_id(witness.correlation_id.clone())
        .triggering_msg_id(witness.triggering_msg_id.clone().unwrap_or_else(|| {
            axiom_core::id::MsgId::new("unknown")
        }))
        .vector_clock(witness.vector_clock.clone())
        .layer(layer)
        .timestamp_ns(witness.timestamp_ns)
        .outcome(outcome)
        .summary(&witness.summary)
        .witness_hash(witness_hash_data)
        .payload_size_bytes(witness.payload_size_bytes)
        .build();
    Ok(event)
}

pub struct CellRegistration {
    pub id: CellId,
    pub layer: Layer,
    pub version: axiom_core::version::Version,
    pub supervision_strategy: axiom_core::cell::SupervisionStrategy,
    /// Optional cell handle for actual message dispatch.
    /// When `None`, messages are drained from the mailbox but not processed.
    pub cell: Option<axiom_core::cell::CellHandle>,
    /// Optional factory function for cell restart.
    /// When present, the runtime will recreate the cell on failure.
    pub factory: Option<Arc<dyn Fn() -> axiom_core::cell::CellHandle + Send + Sync>>,
}

impl CellRegistration {
    /// Create a registration without a cell handle (mailbox-only mode).
    pub fn new(id: CellId, layer: Layer) -> Self {
        Self {
            id,
            layer,
            version: axiom_core::version::Version::new(0, 1, 0),
            supervision_strategy: axiom_core::cell::SupervisionStrategy::default(),
            cell: None,
            factory: None,
        }
    }

    /// Attach a cell handle so the dispatch loop will invoke `Cell::handle`.
    pub fn with_cell(mut self, cell: axiom_core::cell::CellHandle) -> Self {
        self.cell = Some(cell);
        self
    }

    /// Attach a factory function so the runtime can restart the cell on failure.
    pub fn with_factory<F>(mut self, factory: F) -> Self
    where
        F: Fn() -> axiom_core::cell::CellHandle + Send + Sync + 'static,
    {
        self.factory = Some(Arc::new(factory));
        self
    }
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
    layer: Layer,
    #[allow(dead_code)]
    version: axiom_core::version::Version,
    cell: Option<Arc<tokio::sync::Mutex<axiom_core::cell::CellHandle>>>,
    #[allow(dead_code)]
    factory: Option<Arc<dyn Fn() -> axiom_core::cell::CellHandle + Send + Sync>>,
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
    governor: Arc<EntropyGovernorCell>,
    config: RuntimeConfig,
    cells: RwLock<Vec<RegisteredCell>>,
    stop_tx: tokio::sync::Mutex<Option<oneshot::Sender<()>>>,
    dispatch_handle: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    health: Arc<RwLock<RuntimeHealth>>,
    dlq: Arc<crate::dlq::DeadLetterQueue>,
    auto_interceptors: std::sync::atomic::AtomicBool,
    witness_store: Arc<RwLock<Option<Arc<dyn axiom_store::EventStore>>>>,
    snapshot_store: Arc<RwLock<Option<Arc<dyn axiom_store::SnapshotStore>>>>,
    throttle_state: Arc<parking_lot::RwLock<HashMap<String, f64>>>,
    emergency_mode: Arc<parking_lot::RwLock<bool>>,
    events_since_snapshot: Arc<parking_lot::RwLock<HashMap<String, u64>>>,
}

impl AxiomRuntime {
    pub fn new(config: RuntimeConfig) -> Self {
        let bus = Arc::new(MessageBus::new());
        let supervisor = Arc::new(Supervisor::new());
        let governor = Arc::new(EntropyGovernorCell::default());
        let throttle_state = Arc::new(parking_lot::RwLock::new(HashMap::new()));
        let emergency_mode = Arc::new(parking_lot::RwLock::new(false));
        let events_since_snapshot = Arc::new(parking_lot::RwLock::new(HashMap::new()));
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
            witness_store: Arc::new(RwLock::new(None)),
            snapshot_store: Arc::new(RwLock::new(None)),
            throttle_state,
            emergency_mode,
            events_since_snapshot,
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

    pub fn governor(&self) -> Arc<EntropyGovernorCell> {
        self.governor.clone()
    }

    pub async fn set_witness_store(&self, store: Arc<dyn axiom_store::EventStore>) {
        *self.witness_store.write().await = Some(store);
    }

    pub async fn set_snapshot_store(&self, store: Arc<dyn axiom_store::SnapshotStore>) {
        *self.snapshot_store.write().await = Some(store);
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

        let cell = reg.cell.map(|c| Arc::new(tokio::sync::Mutex::new(c)));
        self.cells.write().await.push(RegisteredCell {
            id: reg.id.clone(),
            mailbox: mailbox.clone(),
            layer: reg.layer,
            version: reg.version,
            cell,
            factory: reg.factory,
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
                return Err(AxiomError::Internal {
                    message: format!("preflight failed: {}", issues.join("; ")),
                });
            }
        }

        let guardian = Arc::new(ArchitectureGuardian::new());
        self.bus.register_interceptor(guardian).await;

        let throttle = Arc::new(ThrottleInterceptor::new(self.throttle_state.clone()));
        self.bus.register_interceptor(throttle).await;

        let emergency = Arc::new(EmergencyInterceptor::new(self.emergency_mode.clone()));
        self.bus.register_interceptor(emergency).await;

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
            self.bus
                .register_interceptor(Arc::new(
                    crate::interceptors::CapabilityVersionInterceptor::new(
                        ConstraintValidator::new(ValidationContext::default()),
                    ),
                ))
                .await;
            self.bus
                .register_interceptor(Arc::new(crate::interceptors::GuardInterceptor))
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

        let bus = self.bus.clone();
        let supervisor = self.supervisor.clone();
        let governor = self.governor.clone();
        let witness_store = self.witness_store.clone();
        let snapshot_store = self.snapshot_store.clone();
        let throttle_state = self.throttle_state.clone();
        let emergency_mode = self.emergency_mode.clone();
        let dlq = self.dlq.clone();
        let events_since_snapshot = self.events_since_snapshot.clone();
        let poll_interval = self.config.dispatch_poll_interval_ms;

        let cells_data: Vec<_> = self
            .cells
            .read()
            .await
            .iter()
            .map(|r| {
                (
                    r.mailbox.clone(),
                    r.id.clone(),
                    r.layer,
                    r.cell.clone(),
                    r.factory.clone(),
                )
            })
            .collect();
        let cells_len = cells_data.len();
        // Collect cell IDs separately for the health-check task (cells_data is moved into dispatch task)
        let cell_ids: Vec<CellId> = cells_data
            .iter()
            .map(|(_, cid, _, _, _)| cid.clone())
            .collect();

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
                        let oversight_cell_id = CellId::new("oversight:runtime");
                        let mut oversight_ctx = CellContext::new(&oversight_cell_id, Layer::Oversight);

                        for (mb, cid, layer, cell, factory) in &cells_data {
                            if !supervisor.before_handle(cid.as_str()).await {
                                governor.record(EntropyEvent::CircuitBreak {
                                    cell_id: cid.as_str().to_string(),
                                });
                                let _ = oversight_ctx.emit_failure(
                                    &format!("circuit_break: {}", cid.as_str()),
                                    "supervisor circuit breaker open",
                                );
                                continue;
                            }
                            while let Some(env) = mb.pop().await {
                                if let Some(cell_lock) = cell {
                                    // Cell registered → actually invoke Cell::handle.
                                    // handle_dyn calls end_processing() internally
                                    // and returns (Result, outgoing envelopes) to
                                    // avoid borrow-checker conflicts with boxed
                                    // futures capturing &mut ctx.
                                    let mut cell_guard = cell_lock.lock().await;
                                    let mut ctx = CellContext::new(cid, *layer);
                                    ctx.begin_processing(&env);
                                    let unwind_result = AssertUnwindSafe(
                                        cell_guard.handle_dyn(env, &mut ctx),
                                    )
                                    .catch_unwind()
                                    .await;
                                    let (handle_result, outgoing, witnesses) =
                                        match unwind_result {
                                            Ok(v) => v,
                                            Err(panic_payload) => {
                                                let msg = panic_payload
                                                    .downcast_ref::<String>()
                                                    .map(|s| s.as_str())
                                                    .or_else(|| {
                                                        panic_payload
                                                            .downcast_ref::<&str>()
                                                            .copied()
                                                    })
                                                    .unwrap_or("unknown panic");
                                                tracing::error!(
                                                    cell_id = cid.as_str(),
                                                    panic = msg,
                                                    "cell handle panicked"
                                                );
                                                (
                                                    Err(axiom_core::AxiomError::CellCrashed {
                                                        cell_id: cid.as_str().to_string(),
                                                        message: msg.to_string(),
                                                    }),
                                                    Vec::new(),
                                                    Vec::new(),
                                                )
                                            }
                                        };

                                    // Persist witnesses to event store if configured
                                    if !witnesses.is_empty() {
                                        if let Some(store) = witness_store.read().await.clone() {
                                            let mut events = Vec::new();
                                            let mut failed_witnesses = Vec::new();
                                            for ow in &witnesses {
                                                match witness_to_event(&ow.0, *layer) {
                                                    Ok(event) => events.push(event),
                                                    Err(e) => {
                                                        failed_witnesses.push((ow.0.clone(), e.to_string()));
                                                    }
                                                }
                                            }
                                            if !failed_witnesses.is_empty() {
                                                tracing::warn!(
                                                    count = failed_witnesses.len(),
                                                    "failed to serialize witnesses, queuing to DLQ"
                                                );
                                                for (witness, reason) in failed_witnesses {
                                                    let env = SignalEnvelope {
                                                        msg_id: MsgId::new("witness-dlq"),
                                                        correlation_id: witness.correlation_id.clone(),
                                                        trace_id: witness.trace_id.clone(),
                                                        signal_type: "WitnessSerializationFailed".into(),
                                                        vector_clock: witness.vector_clock.clone(),
                                                        timestamp_ns: witness.timestamp_ns,
                                                        kind: SignalKind::Event,
                                                        source_layer: *layer,
                                                        target_layer: Layer::Oversight,
                                                        source_cell: Some(witness.cell_id.clone()),
                                                        target_cell: Some("oversight:witness-dlq".to_string()),
                                                        payload: serde_json::json!({
                                                            "witness_id": witness.witness_id.as_str(),
                                                            "cell_id": witness.cell_id,
                                                            "reason": reason,
                                                        }),
                                                        schema_version: axiom_core::SchemaVersion::new(1),
                                                        parent_msg_id: witness.triggering_msg_id.clone(),
                                                        hop_count: 0,
                                                    };
                                                    dlq.enqueue(env, &reason);
                                                }
                                            }
                                            if !events.is_empty() {
                                                let event_count = events.len();
                                                if let Err(e) =
                                                    store.append_batch(events.clone()).await
                                                {
                                                    tracing::error!(
                                                        error = %e,
                                                        count = event_count,
                                                        "failed to persist witnesses to event store"
                                                    );
                                                } else if let Err(chain_err) =
                                                    axiom_store::verify_witness_chain(&events)
                                                {
                                                    tracing::warn!(
                                                        error = %chain_err,
                                                        count = event_count,
                                                        "witness chain integrity check failed after persistence"
                                                    );
                                                }
                                            }
                                        }

                                        if let Some(ss) = snapshot_store.read().await.clone() {
                                            let mut pending_snapshots = Vec::new();
                                            for ow in &witnesses {
                                                let cell_id_str = ow.0.cell_id.to_string();
                                                let mut counts = events_since_snapshot.write();
                                                let count = counts.entry(cell_id_str.clone()).or_insert(0);
                                                *count += 1;

                                                if *count >= 100 {
                                                    pending_snapshots.push(axiom_store::Snapshot {
                                                        aggregate_id: cell_id_str.clone(),
                                                        sequence_number: ow.0.hash.0.iter().fold(0u64, |acc, &b| acc.wrapping_shl(8) | b as u64),
                                                        state: serde_json::json!({
                                                            "cell_id": ow.0.cell_id,
                                                            "witness_id": ow.0.witness_id.to_string(),
                                                            "summary": ow.0.summary,
                                                            "outcome": format!("{:?}", ow.0.outcome),
                                                        }),
                                                        schema_version: 1,
                                                        created_at_ns: ow.0.timestamp_ns,
                                                        cell_id: cell_id_str,
                                                        vector_clock: ow.0.vector_clock.clone(),
                                                    });
                                                    *count = 0;
                                                }
                                            }
                                            for snapshot in pending_snapshots {
                                                if let Err(e) = ss.save_snapshot(snapshot.clone()).await {
                                                    tracing::error!(
                                                        cell_id = snapshot.cell_id,
                                                        error = %e,
                                                        "failed to save snapshot"
                                                    );
                                                } else {
                                                    tracing::info!(
                                                        cell_id = snapshot.cell_id,
                                                        "snapshot saved"
                                                    );
                                                }
                                            }
                                        }
                                    }

                                    match handle_result {
                                                Ok(()) => {
                                                    for out in outgoing {
                                                        if let Err(e) = bus.publish(out.0).await {
                                                            tracing::warn!(
                                                                error = %e,
                                                                "failed to publish outgoing signal"
                                                            );
                                                            governor.record(EntropyEvent::DroppedMessage {
                                                                cell_id: cid.as_str().to_string(),
                                                            });
                                                            let _ = oversight_ctx.emit_failure(
                                                                &format!("dropped_message: {}", cid.as_str()),
                                                                &format!("failed to publish: {}", e),
                                                            );
                                                        }
                                                    }
                                                    supervisor.record_success(cid.as_str()).await;
                                                }
                                                Err(e) => {
                                                    tracing::warn!(
                                                        cell_id = cid.as_str(),
                                                        error = %e,
                                                        "cell handle failed"
                                                    );
                                                    let decision = supervisor
                                                        .record_panic(cid.as_str())
                                                        .await;
                                                    governor.record(EntropyEvent::AxiomViolation {
                                                        cell_id: cid.as_str().to_string(),
                                                    });
                                                    let _ = oversight_ctx.emit_axiom_violation(
                                                        "cell_handle_failed",
                                                        &format!("{}: {}", cid.as_str(), e),
                                                    );
                                                    match decision {
                                                        SupervisionDecision::Restart { backoff_ms } => {
                                                            governor.record(EntropyEvent::CellRestart {
                                                                cell_id: cid.as_str().to_string(),
                                                            });
                                                            let _ = oversight_ctx.emit_success(
                                                                &format!("cell_restart: {} (backoff_ms={})", cid.as_str(), backoff_ms),
                                                            );
                                                            if let (Some(f), Some(cell_lock)) =
                                                                (factory, cell)
                                                            {
                                                                if backoff_ms > 0 {
                                                                    tracing::info!(
                                                                        cell_id = cid.as_str(),
                                                                        backoff_ms = backoff_ms,
                                                                        "cell restart: waiting backoff"
                                                                    );
                                                                    tokio::time::sleep(
                                                                        Duration::from_millis(backoff_ms),
                                                                    )
                                                                    .await;
                                                                }
                                                                let new_handle = f();
                                                                let mut guard = cell_lock.lock().await;
                                                                *guard = new_handle;
                                                                tracing::info!(
                                                                    cell_id = cid.as_str(),
                                                                    backoff_ms = backoff_ms,
                                                                    "cell restarted"
                                                                );
                                                            }
                                                        }
                                                        SupervisionDecision::CircuitBreak { .. } => {
                                                            governor.record(EntropyEvent::CircuitBreak {
                                                                cell_id: cid.as_str().to_string(),
                                                            });
                                                            let _ = oversight_ctx.emit_failure(
                                                                &format!("circuit_break: {}", cid.as_str()),
                                                                "supervisor triggered circuit break",
                                                            );
                                                        }
                                                        SupervisionDecision::Stop
                                                        | SupervisionDecision::Escalate => {
                                                            let _ = oversight_ctx.emit_failure(
                                                                &format!("cell_stopped: {}", cid.as_str()),
                                                                "supervisor stopped cell",
                                                            );
                                                        }
                                                    }
                                        }
                                    }
                                } else {
                                    // No cell registered → drain mailbox (backward compat)
                                    supervisor.record_success(cid.as_str()).await;
                                }
                            }
                        }

                        // Entropy governance: check if action is needed
                        let action = governor.take_action();
                        match &action {
                            GovernanceAction::None => {
                                let mut throttle = throttle_state.write();
                                if !throttle.is_empty() {
                                    throttle.clear();
                                    tracing::info!("entropy governance: all throttles lifted");
                                }
                                if *emergency_mode.read() {
                                    *emergency_mode.write() = false;
                                    tracing::info!("entropy governance: emergency mode lifted");
                                }
                            }
                            GovernanceAction::Warn { message } => {
                                tracing::warn!(message, "entropy governance: warn");
                            }
                            GovernanceAction::Throttle { target_cell, factor } => {
                                tracing::warn!(
                                    target_cell = ?target_cell,
                                    factor = factor,
                                    "entropy governance: throttling hottest cell"
                                );
                                if let Some(ref tc) = target_cell {
                                    throttle_state
                                        .write()
                                        .insert(tc.clone(), *factor);
                                }
                            }
                            GovernanceAction::Emergency { reason } => {
                                tracing::error!(
                                    reason = reason,
                                    "entropy governance: emergency — stopping new message acceptance"
                                );
                                *emergency_mode.write() = true;
                            }
                        }
                    }
                }
            }
        });

        *self.dispatch_handle.lock().await = Some(handle);

        let bus_h = self.bus.clone();
        let gov_h = self.governor.clone();
        let sup_h = self.supervisor.clone();
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
                h.entropy_score = gov_h.snapshot().global.value;
                // Aggregate real restart counts from the supervisor
                let mut total = 0u64;
                for cid in &cell_ids {
                    total += sup_h.restart_count(cid.as_str()).await;
                }
                h.total_restarts = total;
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

    pub async fn submit_signal<S: axiom_core::Signal>(
        &self,
        signal: S,
        target_cell: Option<&str>,
        target_layer: Layer,
    ) -> Result<u64, AxiomError> {
        let validation = signal.validate();
        if validation.has_errors() {
            return Err(AxiomError::SignalValidation {
                signal_type: signal.signal_type().to_string(),
                message: format!("{}", validation),
            });
        }
        if validation.has_warnings() {
            tracing::warn!(
                signal_type = signal.signal_type(),
                "signal validation produced warnings"
            );
        }

        let source_layer = signal.layer();
        if !source_layer.can_send_to(target_layer) {
            return Err(AxiomError::LayerViolation {
                from: source_layer,
                to: target_layer,
                source_cell: "external".to_string(),
                signal_type: signal.signal_type().to_string(),
            });
        }

        let env = match target_cell {
            Some(tc) => SignalEnvelope::to_cell(&signal, tc, target_layer)?,
            None => SignalEnvelope::new(&signal, target_layer)?,
        };

        let correlation_id = env.correlation_id.clone();
        tracing::debug!(
            signal_type = env.signal_type,
            correlation_id = correlation_id.as_str(),
            target_cell = target_cell.unwrap_or("broadcast"),
            target_layer = target_layer.as_str(),
            "external signal submitted"
        );

        self.bus.publish(env).await
    }

    pub async fn snapshot_viz(&self) -> Result<serde_json::Value, AxiomError> {
        let cells = self.cells.read().await;
        let cell_nodes = cells
            .iter()
            .map(|r| serde_json::json!({
                "id": r.id.as_str(),
                "name": r.id.as_str(),
                "layer": r.layer.as_str(),
                "status": if r.cell.is_some() { "running" } else { "mailbox" }
            }))
            .collect::<Vec<_>>();

        let mut edges = Vec::new();
        for from in cells.iter() {
            for to in cells.iter() {
                if from.id != to.id && from.layer.can_send_to(to.layer) {
                    edges.push(serde_json::json!({
                        "from": from.id.as_str(),
                        "to": to.id.as_str()
                    }));
                }
            }
        }

        let topology = serde_json::json!({
            "cells": cell_nodes,
            "edges": edges
        });

        let timeline = serde_json::json!({"entries": []});

        let entropy_snapshot = self.governor.snapshot();
        let entropy = serde_json::json!({
            "system_entropy": entropy_snapshot.global.value,
            "cell_entropies": entropy_snapshot.per_cell.iter().map(|(k, v)| (k.clone(), *v)).collect::<Vec<_>>(),
            "status": format!("{:?}", entropy_snapshot.global.level())
        });

        let flow = serde_json::json!({"records": []});

        Ok(serde_json::json!({
            "topology": topology,
            "timeline": timeline,
            "entropy": entropy,
            "flow": flow
        }))
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
                cell: None,
                factory: None,
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
                cell: None,
                factory: None,
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
        let gov = Arc::new(EntropyGovernorCell::default());
        for _ in 0..5 {
            gov.record(EntropyEvent::CellRestart {
                cell_id: "c1".into(),
            });
        }
        let snap = gov.snapshot();
        assert!(snap.global.value > 1.0, "score should exceed threshold");
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
                cell: None,
                factory: None,
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
