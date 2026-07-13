use crate::entropy_gov::{EntropyEvent, GovernanceAction};
use crate::supervisor::SupervisionDecision;
use axiom_kernel::cell::RuntimeCellHandle;
use axiom_kernel::context::{CellContext, OutgoingEnvelope, OutgoingWitness};
use axiom_kernel::id::{CellId, CorrelationId, MsgId};
use axiom_kernel::layer::RuntimeTier;
use axiom_kernel::signal::{SignalEnvelope, SignalKind, VectorClock};
use axiom_kernel::version::SchemaVersion;
use axiom_kernel::KernelError;
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;

type RegisteredCellData = (
    Arc<crate::mailbox::Mailbox>,
    CellId,
    RuntimeTier,
    Option<Arc<TokioMutex<RuntimeCellHandle>>>,
    Option<Arc<dyn Fn() -> RuntimeCellHandle + Send + Sync>>,
);

const SNAPSHOT_INTERVAL: u64 = 100;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub async fn run_dispatch_loop(
    mut rx: tokio::sync::oneshot::Receiver<()>,
    poll_interval: u64,
    cells_data: Vec<RegisteredCellData>,
    ctx: crate::dispatch::DispatchContext,
) {
    let crate::dispatch::DispatchContext {
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
    } = ctx;

    let mut interval = tokio::time::interval(Duration::from_millis(poll_interval));

    loop {
        tokio::select! {
            _ = &mut rx => {
                tracing::info!("runtime dispatcher shutting down");
                break;
            }
            _ = interval.tick() => {
                let oversight_cell_id = CellId::new("oversight:runtime");
                let mut oversight_ctx = CellContext::new(&oversight_cell_id, RuntimeTier::Oversight);

                if let Some(kernel) = &cell_kernel {
                    if process_kernel_messages(kernel, &bus, &supervisor, &governor, &mut oversight_ctx).await.is_err() {
                        continue;
                    }
                }

                for (mb, cid, layer, cell, factory) in &cells_data {
                    if !supervisor.before_handle(cid.as_str()).await {
                        handle_circuit_break(cid.as_str(), &governor, &mut oversight_ctx);
                        continue;
                    }

                    while let Some(env) = mb.pop().await {
                        if let Some(cell_lock) = cell {
                            #[allow(clippy::needless_borrow)]
                            process_cell_message(
                                env,
                                cid.clone(),
                                *layer,
                                cell_lock,
                                &factory,
                                &bus,
                                &supervisor,
                                &governor,
                                &witness_store,
                                &snapshot_store,
                                &dlq,
                                &events_since_snapshot,
                                &mut oversight_ctx,
                            ).await;
                        } else {
                            supervisor.record_success(cid.as_str()).await;
                        }
                    }
                }

                process_entropy_governance(&governor, &throttle_state, &emergency_mode).await;
            }
        }
    }
}

async fn process_kernel_messages(
    kernel: &Arc<axiom_kernel::CellKernel>,
    bus: &Arc<crate::bus::MessageBus>,
    supervisor: &Arc<crate::supervisor::Supervisor>,
    governor: &Arc<crate::entropy_gov::EntropyGovernorCell>,
    oversight_ctx: &mut CellContext<'_>,
) -> Result<(), ()> {
    let kernel_cells = kernel.list().await;
    if kernel_cells.is_empty() {
        return Ok(());
    }

    if !supervisor.before_handle("kernel").await {
        handle_circuit_break("kernel", governor, oversight_ctx);
        return Err(());
    }

    for (handle, queued) in kernel_cells {
        for _ in 0..queued {
            if let Ok(Some(_msg)) = kernel.receive(&handle).await {
                let env = SignalEnvelope {
                    msg_id: MsgId::new("kernel-msg"),
                    correlation_id: CorrelationId::new("kernel"),
                    trace_id: None,
                    signal_type: "KernelMessage".into(),
                    vector_clock: VectorClock::new(),
                    timestamp_ns: 0,
                    kind: SignalKind::Event,
                    source_layer: RuntimeTier::Exec,
                    target_layer: RuntimeTier::Exec,
                    source_cell: None,
                    target_cell: None,
                    payload: serde_json::Value::Null,
                    schema_version: SchemaVersion::new(1),
                    parent_msg_id: None,
                    hop_count: 0,
                };
                if let Err(e) = bus.publish(env).await {
                    tracing::error!(
                        error = %e,
                        cell_id = "kernel",
                        "failed to publish kernel message"
                    );
                    governor.record(EntropyEvent::DroppedMessage { cell_id: "kernel".to_string() });
                }
            }
        }
        supervisor.record_success(handle.id.to_string().as_str()).await;
    }

    Ok(())
}

fn handle_circuit_break(
    cell_id: &str,
    governor: &Arc<crate::entropy_gov::EntropyGovernorCell>,
    oversight_ctx: &mut CellContext<'_>,
) {
    governor.record(EntropyEvent::CircuitBreak { cell_id: cell_id.to_string() });

    if let Err(e) = oversight_ctx
        .emit_failure(&format!("circuit_break: {}", cell_id), "supervisor circuit breaker open")
    {
        tracing::error!(
            error = %e,
            cell_id = cell_id,
            "failed to emit oversight failure for circuit break"
        );
        governor.record(EntropyEvent::DroppedMessage { cell_id: cell_id.to_string() });
    }
}

#[allow(clippy::too_many_arguments, clippy::needless_borrow)]
async fn process_cell_message(
    env: SignalEnvelope,
    cid: CellId,
    layer: RuntimeTier,
    cell_lock: &Arc<TokioMutex<RuntimeCellHandle>>,
    factory: &Option<Arc<dyn Fn() -> RuntimeCellHandle + Send + Sync>>,
    bus: &Arc<crate::bus::MessageBus>,
    supervisor: &Arc<crate::supervisor::Supervisor>,
    governor: &Arc<crate::entropy_gov::EntropyGovernorCell>,
    witness_store: &Arc<tokio::sync::RwLock<Option<Arc<dyn axiom_store::EventStore>>>>,
    snapshot_store: &Arc<tokio::sync::RwLock<Option<Arc<dyn axiom_store::SnapshotStore>>>>,
    dlq: &Arc<crate::dlq::DeadLetterQueue>,
    events_since_snapshot: &Arc<parking_lot::RwLock<std::collections::HashMap<String, u64>>>,
    oversight_ctx: &mut CellContext<'_>,
) {
    let mut cell_guard = cell_lock.lock().await;
    let mut ctx = CellContext::new(&cid, layer);
    ctx.begin_processing(&env);

    let unwind_result = AssertUnwindSafe(cell_guard.handle_dyn(env, &mut ctx)).catch_unwind().await;

    let (handle_result, outgoing, witnesses) = match unwind_result {
        Ok(v) => v,
        Err(panic_payload) => {
            let msg = panic_payload
                .downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| panic_payload.downcast_ref::<&str>().copied())
                .unwrap_or("unknown panic");
            tracing::error!(cell_id = cid.as_str(), panic = msg, "cell handle panicked");
            (
                Err(KernelError::CellCrashed {
                    cell_id: cid.as_str().to_string(),
                    message: msg.to_string(),
                }),
                Vec::new(),
                Vec::new(),
            )
        }
    };

    if !witnesses.is_empty() {
        persist_witnesses_and_snapshots(
            &witnesses,
            layer,
            witness_store,
            snapshot_store,
            dlq,
            governor,
            events_since_snapshot,
        )
        .await;
    }

    match handle_result {
        Ok(()) => {
            handle_cell_success(cid.as_str(), outgoing, bus, supervisor, governor, oversight_ctx)
                .await;
        }
        Err(e) => {
            tracing::warn!(
                cell_id = cid.as_str(),
                error = %e,
                "cell handle failed"
            );
            let decision = supervisor.record_panic(cid.as_str()).await;
            governor.record(EntropyEvent::AxiomViolation { cell_id: cid.as_str().to_string() });

            if let Err(emit_err) = oversight_ctx
                .emit_axiom_violation("cell_handle_failed", &format!("{}: {}", cid.as_str(), e))
            {
                tracing::error!(
                    error = %emit_err,
                    cell_id = cid.as_str(),
                    "failed to emit oversight axiom violation"
                );
                governor.record(EntropyEvent::DroppedMessage { cell_id: cid.as_str().to_string() });
            }

            handle_supervision_decision(
                cid.as_str(),
                decision,
                governor,
                oversight_ctx,
                factory.clone(),
                Some(cell_lock.clone()),
            )
            .await;
        }
    }
}

async fn persist_witnesses_and_snapshots(
    witnesses: &[OutgoingWitness],
    layer: RuntimeTier,
    witness_store: &Arc<tokio::sync::RwLock<Option<Arc<dyn axiom_store::EventStore>>>>,
    snapshot_store: &Arc<tokio::sync::RwLock<Option<Arc<dyn axiom_store::SnapshotStore>>>>,
    dlq: &Arc<crate::dlq::DeadLetterQueue>,
    governor: &Arc<crate::entropy_gov::EntropyGovernorCell>,
    events_since_snapshot: &Arc<parking_lot::RwLock<std::collections::HashMap<String, u64>>>,
) {
    if let Some(store) = witness_store.read().await.clone() {
        let mut events = Vec::new();
        let mut failed_witnesses = Vec::new();

        for ow in witnesses {
            match crate::dispatch::witness::witness_to_event(&ow.0, layer) {
                Ok(event) => events.push(event),
                Err(e) => {
                    failed_witnesses.push((ow.clone(), e.to_string()));
                }
            }
        }

        if !failed_witnesses.is_empty() {
            tracing::warn!(
                count = failed_witnesses.len(),
                "failed to serialize witnesses, queuing to DLQ"
            );
            for (witness, reason) in failed_witnesses {
                enqueue_failed_witness(&witness, &reason, dlq, governor);
            }
        }

        if !events.is_empty() {
            let event_count = events.len();
            if let Err(e) = store.append_batch(events.clone()).await {
                tracing::error!(
                    error = %e,
                    count = event_count,
                    "failed to persist witnesses to event store"
                );
            } else if let Err(chain_err) = axiom_store::verify_witness_chain(&events) {
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

        for ow in witnesses {
            let cell_id_str = ow.0.cell_id.to_string();
            let mut counts = events_since_snapshot.write();
            let count = counts.entry(cell_id_str.clone()).or_insert(0);
            *count += 1;

            if *count >= SNAPSHOT_INTERVAL {
                pending_snapshots.push(axiom_store::Snapshot {
                    aggregate_id: cell_id_str.clone(),
                    sequence_number: ow
                        .0
                        .hash
                        .0
                        .iter()
                        .fold(0u64, |acc, &b| acc.wrapping_shl(8) | b as u64),
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
                tracing::info!(cell_id = snapshot.cell_id, "snapshot saved");
            }
        }
    }
}

fn enqueue_failed_witness(
    witness: &OutgoingWitness,
    reason: &str,
    dlq: &Arc<crate::dlq::DeadLetterQueue>,
    governor: &Arc<crate::entropy_gov::EntropyGovernorCell>,
) {
    let env = SignalEnvelope {
        msg_id: MsgId::new("witness-dlq"),
        correlation_id: witness.0.correlation_id.clone(),
        trace_id: witness.0.trace_id.clone(),
        signal_type: "WitnessSerializationFailed".into(),
        vector_clock: witness.0.vector_clock.clone(),
        timestamp_ns: witness.0.timestamp_ns,
        kind: SignalKind::Event,
        source_layer: RuntimeTier::Exec,
        target_layer: RuntimeTier::Oversight,
        source_cell: Some(witness.0.cell_id.clone()),
        target_cell: Some("oversight:witness-dlq".to_string()),
        payload: serde_json::json!({
            "witness_id": witness.0.witness_id.as_str(),
            "cell_id": witness.0.cell_id,
            "reason": reason,
        }),
        schema_version: SchemaVersion::new(1),
        parent_msg_id: witness.0.triggering_msg_id.clone(),
        hop_count: 0,
    };

    if let Err(e) = dlq.enqueue(env, reason) {
        tracing::error!(
            error = %e,
            witness_id = witness.0.witness_id.as_str(),
            cell_id = witness.0.cell_id,
            "failed to enqueue failed witness to dlq"
        );
        governor.record(EntropyEvent::DroppedMessage { cell_id: witness.0.cell_id.clone() });
    }
}

async fn handle_cell_success(
    cell_id: &str,
    outgoing: Vec<OutgoingEnvelope>,
    bus: &Arc<crate::bus::MessageBus>,
    supervisor: &Arc<crate::supervisor::Supervisor>,
    governor: &Arc<crate::entropy_gov::EntropyGovernorCell>,
    oversight_ctx: &mut CellContext<'_>,
) {
    for out in outgoing {
        if let Err(e) = bus.publish(out.0).await {
            tracing::warn!(
                error = %e,
                "failed to publish outgoing signal"
            );
            governor.record(EntropyEvent::DroppedMessage { cell_id: cell_id.to_string() });
            emit_oversight_failure(
                oversight_ctx,
                &format!("dropped_message: {}", cell_id),
                &format!("failed to publish: {}", e),
                governor,
                cell_id,
            );
        }
    }
    supervisor.record_success(cell_id).await;
}

async fn handle_supervision_decision(
    cell_id: &str,
    decision: SupervisionDecision,
    governor: &Arc<crate::entropy_gov::EntropyGovernorCell>,
    oversight_ctx: &mut CellContext<'_>,
    factory: Option<Arc<dyn Fn() -> RuntimeCellHandle + Send + Sync>>,
    cell_lock: Option<Arc<TokioMutex<RuntimeCellHandle>>>,
) {
    match decision {
        SupervisionDecision::Restart { backoff_ms } => {
            governor.record(EntropyEvent::CellRestart { cell_id: cell_id.to_string() });
            emit_oversight_success(
                oversight_ctx,
                &format!("cell_restart: {} (backoff_ms={})", cell_id, backoff_ms),
                governor,
                cell_id,
            );

            if let (Some(f), Some(cell_lock)) = (factory, cell_lock) {
                if backoff_ms > 0 {
                    tracing::info!(
                        cell_id = cell_id,
                        backoff_ms = backoff_ms,
                        "cell restart: waiting backoff"
                    );
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
                let new_handle = f();
                let mut guard = cell_lock.lock().await;
                *guard = new_handle;
                tracing::info!(cell_id = cell_id, backoff_ms = backoff_ms, "cell restarted");
            }
        }
        SupervisionDecision::CircuitBreak { .. } => {
            governor.record(EntropyEvent::CircuitBreak { cell_id: cell_id.to_string() });
            emit_oversight_failure(
                oversight_ctx,
                &format!("circuit_break: {}", cell_id),
                "supervisor triggered circuit break",
                governor,
                cell_id,
            );
        }
        SupervisionDecision::Stop | SupervisionDecision::Escalate => {
            emit_oversight_failure(
                oversight_ctx,
                &format!("cell_stopped: {}", cell_id),
                "supervisor stopped cell",
                governor,
                cell_id,
            );
        }
    }
}

fn emit_oversight_success(
    ctx: &mut CellContext<'_>,
    message: &str,
    governor: &Arc<crate::entropy_gov::EntropyGovernorCell>,
    cell_id: &str,
) {
    if let Err(emit_err) = ctx.emit_success(message) {
        tracing::error!(
            error = %emit_err,
            cell_id = cell_id,
            "failed to emit oversight success"
        );
        governor.record(EntropyEvent::DroppedMessage { cell_id: cell_id.to_string() });
    }
}

fn emit_oversight_failure(
    ctx: &mut CellContext<'_>,
    message: &str,
    details: &str,
    governor: &Arc<crate::entropy_gov::EntropyGovernorCell>,
    cell_id: &str,
) {
    if let Err(emit_err) = ctx.emit_failure(message, details) {
        tracing::error!(
            error = %emit_err,
            cell_id = cell_id,
            "failed to emit oversight failure"
        );
        governor.record(EntropyEvent::DroppedMessage { cell_id: cell_id.to_string() });
    }
}

async fn process_entropy_governance(
    governor: &Arc<crate::entropy_gov::EntropyGovernorCell>,
    throttle_state: &Arc<parking_lot::RwLock<std::collections::HashMap<String, f64>>>,
    emergency_mode: &Arc<parking_lot::RwLock<bool>>,
) {
    let action = governor.take_action();
    match action {
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
                throttle_state.write().insert(tc.clone(), factor);
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
