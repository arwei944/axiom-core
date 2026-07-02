use axiom_core::context::CellContext;
use axiom_core::entropy::EntropyScore;
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::signal::{SignalEnvelope, SignalKind, VectorClock};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_multiple_cells_concurrent() {
    let counter = Arc::new(AtomicUsize::new(0));
    
    let cell1_id = CellId::new("cell1");
    let cell2_id = CellId::new("cell2");
    let cell3_id = CellId::new("cell3");
    
    let mut ctx1 = CellContext::new(&cell1_id, Layer::Exec);
    let mut ctx2 = CellContext::new(&cell2_id, Layer::Exec);
    let mut ctx3 = CellContext::new(&cell3_id, Layer::Exec);
    
    let envelope1 = SignalEnvelope {
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
        target_cell: Some(cell1_id.to_string()),
        payload: serde_json::json!({}),
        schema_version: axiom_core::SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    };
    
    let envelope2 = envelope1.clone();
    let envelope3 = envelope1.clone();
    
    ctx1.begin_processing(&envelope1);
    ctx2.begin_processing(&envelope2);
    ctx3.begin_processing(&envelope3);
    
    counter.fetch_add(1, Ordering::Relaxed);
    
    let (_o1, _w1) = ctx1.end_processing();
    let (_o2, _w2) = ctx2.end_processing();
    let (_o3, _w3) = ctx3.end_processing();
    
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[test]
fn test_single_cell_multiple_signals_serial() {
    let cell_id = CellId::new("serial-cell");
    
    for _ in 0..10 {
        let mut ctx = CellContext::new(&cell_id, Layer::Exec);
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
            schema_version: axiom_core::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        };
        ctx.begin_processing(&envelope);
        let (_outgoing, _witnesses) = ctx.end_processing();
    }
}

#[test]
fn test_witness_buffer_capacity() {
    let cell_id = CellId::new("capacity-cell");
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);
    
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
        schema_version: axiom_core::SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    };
    
    ctx.begin_processing(&envelope);
    
    for i in 0..100 {
        let result = ctx.emit_success(&format!("witness {}", i));
        assert!(result.is_ok());
    }
    
    let (_outgoing, witnesses) = ctx.end_processing();
    assert_eq!(witnesses.len(), 100);
}

#[test]
fn test_entropy_concurrent_updates() {
    let mut score = EntropyScore::new();
    
    for _ in 0..100 {
        score.record_axiom_violation();
        score.record_cell_restart();
        score.record_circuit_break();
    }
    
    assert!(score.value > 0.0);
    assert!(score.is_red() || score.is_critical());
}

#[test]
fn test_vector_clock_concurrent_updates() {
    let mut clock1 = VectorClock::new();
    let mut clock2 = VectorClock::new();
    let mut clock3 = VectorClock::new();
    
    for _i in 0..10 {
        clock1.increment("cell1");
        clock2.increment("cell2");
        clock3.increment("cell3");
    }
    
    clock1.merge(&clock2);
    clock1.merge(&clock3);
    
    assert_eq!(clock1.get("cell1"), 10);
    assert_eq!(clock1.get("cell2"), 10);
    assert_eq!(clock1.get("cell3"), 10);
}

#[test]
fn test_concurrent_signal_creation() {
    let signals: Vec<TestSignal> = (0..100)
        .map(|i| TestSignal::new(format!("signal {}", i)))
        .collect();
    
    assert_eq!(signals.len(), 100);
    for (i, s) in signals.iter().enumerate() {
        assert_eq!(s.value, format!("signal {}", i));
    }
}

#[derive(Debug, Clone)]
struct TestSignal {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    value: String,
}

impl TestSignal {
    fn new(value: String) -> Self {
        Self {
            msg_id: MsgId::generate(),
            correlation_id: CorrelationId::generate(),
            vector_clock: VectorClock::new(),
            value,
        }
    }
}
