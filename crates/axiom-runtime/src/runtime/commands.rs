use super::AxiomRuntime;
use axiom_kernel::clock::global_clock;
use axiom_kernel::id::{CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::signal::{SignalEnvelope, SignalKind, VectorClock};
use axiom_kernel::version::SchemaVersion;
use axiom_kernel::{KernelError, KernelResult};
use serde_json::Value;

impl AxiomRuntime {
    pub async fn publish_command(
        &self,
        signal_type: &str,
        payload: Value,
        target_cell: Option<&str>,
        target_layer: Layer,
    ) -> KernelResult<u64> {
        let id = next_msg_id();
        let corr_id = format!("corr-{id}");
        let env = SignalEnvelope {
            msg_id: MsgId::new(id),
            correlation_id: CorrelationId::new(corr_id),
            trace_id: None,
            signal_type: signal_type.to_string(),
            vector_clock: VectorClock::new(),
            timestamp_ns: global_clock().now_ns(),
            kind: SignalKind::Command,
            source_layer: Layer::Oversight,
            target_layer,
            source_cell: None,
            target_cell: target_cell.map(|s| s.to_string()),
            payload,
            schema_version: SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        };
        self.bus.publish(env).await
    }

    pub async fn submit_signal<S: axiom_kernel::signal::Signal>(
        &self,
        signal: S,
        target_cell: Option<&str>,
        target_layer: Layer,
    ) -> KernelResult<u64> {
        let validation = signal.validate();
        if validation.has_errors() {
            return Err(KernelError::SignalValidation {
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
            return Err(KernelError::LayerViolation {
                from: source_layer,
                to: target_layer,
                source_cell: "external".to_string(),
                signal_type: signal.signal_type().to_string(),
            });
        }

        let payload = signal.serialize_to_json()?;
        let env = SignalEnvelope {
            msg_id: signal.msg_id().clone(),
            correlation_id: signal.correlation_id().clone(),
            trace_id: signal.trace_id().cloned(),
            signal_type: signal.signal_type().to_string(),
            vector_clock: signal.vector_clock().clone(),
            timestamp_ns: signal.timestamp_ns(),
            kind: signal.kind(),
            source_layer: signal.layer(),
            target_layer,
            source_cell: signal.sender().map(|s| s.to_string()),
            target_cell: target_cell.map(|s| s.to_string()),
            payload,
            schema_version: signal.schema_version(),
            parent_msg_id: None,
            hop_count: 0,
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
}

static CMD_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_msg_id() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let n = CMD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("cmd-{ts}-{n}")
}
