use crate::bus::MessageBus;
use crate::dlq::DeadLetterQueue;
use crate::entropy_gov::{EntropyEvent, EntropyGovernorCell, GovernanceAction};
use crate::supervisor::{SupervisionDecision, Supervisor};
use axiom_kernel::cell::RuntimeCellHandle;
use axiom_kernel::context::CellContext;
use axiom_kernel::KernelError;
use axiom_kernel::cell::CellKernel;
use axiom_kernel::id::{CellId, CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::signal::{SignalEnvelope, SignalKind, VectorClock};
use axiom_kernel::version::SchemaVersion;
use axiom_store::{EventStore, SnapshotStore};
use futures::FutureExt;
use parking_lot::RwLock as ParkingRwLock;
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex as TokioMutex, RwLock};

type RegisteredCellData = (
    Arc<crate::mailbox::Mailbox>,
    CellId,
    Layer,
    Option<Arc<TokioMutex<RuntimeCellHandle>>>,
    Option<Arc<dyn Fn() -> RuntimeCellHandle + Send + Sync>>,
);

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub async fn run_dispatch_loop(
    mut rx: tokio::sync::oneshot::Receiver<()>,
    poll_interval: u64,
    cells_data: Vec<RegisteredCellData>,
    bus: Arc<MessageBus>,
    supervisor: Arc<Supervisor>,
    governor: Arc<EntropyGovernorCell>,
    witness_store: Arc<RwLock<Option<Arc<dyn EventStore>>>>,
    snapshot_store: Arc<RwLock<Option<Arc<dyn SnapshotStore>>>>,
    throttle_state: Arc<ParkingRwLock<HashMap<String, f64>>>,
    emergency_mode: Arc<ParkingRwLock<bool>>,
    dlq: Arc<DeadLetterQueue>,
    events_since_snapshot: Arc<ParkingRwLock<HashMap<String, u64>>>,
    cell_kernel: Option<Arc<CellKernel>>,
) {
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

                if let Some(kernel) = &cell_kernel {
                    let kernel_cells = kernel.list().await;
                    if !kernel_cells.is_empty() {
                        if !supervisor.before_handle("kernel").await {
                            governor.record(EntropyEvent::CircuitBreak {
                                cell_id: "kernel".to_string(),
                            });
                            let _ = oversight_ctx.emit_failure(
                                "circuit_break: kernel",
                                "supervisor circuit breaker open",
                            );
                            continue;
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
                                        source_layer: Layer::Exec,
                                        target_layer: Layer::Exec,
                                        source_cell: None,
                                        target_cell: None,
                                        payload: serde_json::Value::Null,
                                        schema_version: SchemaVersion::new(1),
                                        parent_msg_id: None,
                                        hop_count: 0,
                                    };
                                    let _ = bus.publish(env).await;
                                }
                            }
                            supervisor.record_success(handle.id.to_string().as_str()).await;
                        }
                        continue;
                    }
                }

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
                                if let Some(store) = witness_store.read().await.clone() {
                                    let mut events = Vec::new();
                                    let mut failed_witnesses = Vec::new();
                                    for ow in &witnesses {
                                        match crate::dispatch::witness::witness_to_event(&ow.0, *layer) {
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
                                                schema_version: SchemaVersion::new(1),
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
                            supervisor.record_success(cid.as_str()).await;
                        }
                    }
                }

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
}
