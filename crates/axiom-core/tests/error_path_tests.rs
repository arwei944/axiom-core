use axiom_core::entropy::EntropyScore;
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::schema::{ValidationError, ValidationResult};
use axiom_core::signal::{Signal, SignalEnvelope, SignalKind, VectorClock};
use axiom_core::{AxiomError, SchemaVersion};

#[test]
fn test_layer_violation_exec_to_agent_runtime() {
    let cell_id = CellId::new("exec-layer-cell");
    let mut ctx = axiom_core::context::CellContext::new(&cell_id, Layer::Exec);

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

    let signal = TestSignal::new("hello");

    let result = ctx.reply(&envelope, signal);
    assert!(result.is_ok());
}

#[test]
fn test_witness_chain_break() {
    let cell_id = CellId::new("witness-chain-test");
    let mut ctx = axiom_core::context::CellContext::new(&cell_id, Layer::Exec);

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

    let (_outgoing, witnesses) = ctx.end_processing();
    assert!(!witnesses.is_empty());
}

#[test]
fn test_signal_validation_failure() {
    let signal = BadSignal::new("bad");

    let result = signal.validate();
    assert!(result.has_errors());
}

#[test]
fn test_cell_crash_recovery() {
    let cell_id = CellId::new("crash-recovery-test");
    let mut ctx = axiom_core::context::CellContext::new(&cell_id, Layer::Exec);

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

    let (_outgoing, witnesses) = ctx.end_processing();
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TestSignal {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    value: String,
}

impl TestSignal {
    fn new(value: &str) -> Self {
        Self {
            msg_id: MsgId::generate(),
            correlation_id: CorrelationId::generate(),
            vector_clock: VectorClock::new(),
            value: value.to_string(),
        }
    }
}

impl Signal for TestSignal {
    fn signal_type(&self) -> &'static str {
        "test"
    }
    fn msg_id(&self) -> &MsgId {
        &self.msg_id
    }
    fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }
    fn vector_clock(&self) -> &VectorClock {
        &self.vector_clock
    }
    fn timestamp_ns(&self) -> u64 {
        0
    }
    fn kind(&self) -> SignalKind {
        SignalKind::Command
    }
    fn layer(&self) -> Layer {
        Layer::Exec
    }
    fn validate(&self) -> ValidationResult {
        ValidationResult::ok()
    }
    fn serialize_to_json(&self) -> axiom_core::Result<serde_json::Value> {
        serde_json::to_value(self).map_err(|e| AxiomError::SignalSerialization {
            signal_type: "LayerViolationSignal".into(),
            message: e.to_string(),
        })
    }
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }
    fn clone_signal(&self) -> Box<dyn Signal> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BadSignal {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    value: String,
}

impl BadSignal {
    fn new(value: &str) -> Self {
        Self {
            msg_id: MsgId::generate(),
            correlation_id: CorrelationId::generate(),
            vector_clock: VectorClock::new(),
            value: value.to_string(),
        }
    }
}

impl Signal for BadSignal {
    fn signal_type(&self) -> &'static str {
        "bad"
    }
    fn msg_id(&self) -> &MsgId {
        &self.msg_id
    }
    fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }
    fn vector_clock(&self) -> &VectorClock {
        &self.vector_clock
    }
    fn timestamp_ns(&self) -> u64 {
        0
    }
    fn kind(&self) -> SignalKind {
        SignalKind::Command
    }
    fn layer(&self) -> Layer {
        Layer::Exec
    }
    fn validate(&self) -> ValidationResult {
        ValidationResult::from_errors(vec![ValidationError::error(
            "value",
            "intentional validation failure",
        )])
    }
    fn serialize_to_json(&self) -> axiom_core::Result<serde_json::Value> {
        Err(AxiomError::SignalSerialization {
            signal_type: "bad".into(),
            message: "intentional failure".to_string(),
        })
    }
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }
    fn clone_signal(&self) -> Box<dyn Signal> {
        Box::new(self.clone())
    }
}
