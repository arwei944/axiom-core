//! Common utilities for benchmarks and stress tests.

use axiom_kernel::id::{CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::signal::{SignalEnvelope, SignalKind, VectorClock};
use axiom_kernel::version::SchemaVersion;

/// Create a test signal envelope with minimal allocation overhead.
pub fn make_signal(signal_type: &str, src: &str, dst: &str) -> SignalEnvelope {
    SignalEnvelope {
        msg_id: MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        trace_id: None,
        signal_type: signal_type.to_string(),
        vector_clock: VectorClock::new(),
        timestamp_ns: 0,
        kind: SignalKind::Command,
        source_layer: Layer::Exec,
        target_layer: Layer::Exec,
        source_cell: Some(src.to_string()),
        target_cell: Some(dst.to_string()),
        payload: serde_json::json!({}),
        schema_version: SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    }
}

/// Create a batch of test signals.
pub fn make_signal_batch(count: usize) -> Vec<SignalEnvelope> {
    (0..count).map(|i| make_signal("BenchSignal", &format!("src-{i}"), "dst")).collect()
}
