use axiom_kernel::entropy::EntropyScore;
use axiom_kernel::id::{CellId, CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::signal::{SignalEnvelope, SignalKind, VectorClock};
use axiom_kernel::version::SchemaVersion;

#[test]
fn test_witness_chain_break() {
    let cell_id = CellId::new("witness-chain-test");
    let mut ctx = axiom_kernel::context::CellContext::new(&cell_id, Layer::Exec);

    let envelope = SignalEnvelope {
        msg_id: MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        trace_id: None,
        signal_type: "test".to_string(),
        vector_clock: VectorClock::new(),
        timestamp_ns: 0,
        kind: SignalKind::Command,
        source_layer: Layer::Exec,
        target_layer: Layer::Exec,
        source_cell: None,
        target_cell: Some(cell_id.to_string()),
        payload: serde_json::json!({}),
        schema_version: SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    };

    ctx.begin_processing(&envelope);

    let result = ctx.emit_success("broken chain test");
    assert!(result.is_ok());

    let witnesses = ctx.take_witnesses();
    assert!(!witnesses.is_empty());
}

#[test]
fn test_cell_crash_recovery() {
    let cell_id = CellId::new("crash-recovery-test");
    let mut ctx = axiom_kernel::context::CellContext::new(&cell_id, Layer::Exec);

    let envelope = SignalEnvelope {
        msg_id: MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        trace_id: None,
        signal_type: "test".to_string(),
        vector_clock: VectorClock::new(),
        timestamp_ns: 0,
        kind: SignalKind::Command,
        source_layer: Layer::Exec,
        target_layer: Layer::Exec,
        source_cell: None,
        target_cell: Some(cell_id.to_string()),
        payload: serde_json::json!({}),
        schema_version: SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    };

    ctx.begin_processing(&envelope);

    let result = ctx.emit_failure("cell crashed", "test crash");
    assert!(result.is_ok());

    let witnesses = ctx.take_witnesses();
    assert!(!witnesses.is_empty());
}

#[test]
fn test_entropy_increases_on_errors() {
    let mut score = EntropyScore::new();
    assert!(score.is_green());

    score.record_axiom_violation();
    score.record_cell_restart();

    assert!(score.value > 0.0);
}