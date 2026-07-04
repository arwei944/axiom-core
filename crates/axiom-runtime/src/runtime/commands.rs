use super::AxiomRuntime;
use axiom_core::clock::global_clock;
use axiom_core::id::{CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::signal::{SignalKind, VectorClock};
use axiom_core::SchemaVersion;
use serde_json::Value;

impl AxiomRuntime {
    pub async fn publish_command(
        &self,
        signal_type: &str,
        payload: Value,
        target_cell: Option<&str>,
        target_layer: Layer,
    ) -> Result<u64, axiom_core::error::AxiomError> {
        let id = next_msg_id();
        let corr_id = format!("corr-{id}");
        let env = axiom_core::signal::SignalEnvelope {
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

    pub async fn submit_signal<S: axiom_core::Signal>(
        &self,
        signal: S,
        target_cell: Option<&str>,
        target_layer: Layer,
    ) -> Result<u64, axiom_core::error::AxiomError> {
        let validation = signal.validate();
        if validation.has_errors() {
            return Err(axiom_core::error::AxiomError::SignalValidation {
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
            return Err(axiom_core::error::AxiomError::LayerViolation {
                from: source_layer,
                to: target_layer,
                source_cell: "external".to_string(),
                signal_type: signal.signal_type().to_string(),
            });
        }

        let env = match target_cell {
            Some(tc) => axiom_core::signal::SignalEnvelope::to_cell(&signal, tc, target_layer)?,
            None => axiom_core::signal::SignalEnvelope::new(&signal, target_layer)?,
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
