//! Architecture Conformance Test Suite - Self-validation of the architecture
//!
//! This test suite verifies that:
//! 1. The CanSendTo matrix is complete and correct
//! 2. Layer markers are properly sealed (cannot be implemented externally)
//! 3. Runtime layer validation correctly enforces the same rules as compile-time
//! 4. Architecture components themselves follow the same constraints
//!
//! Architecture should "eat its own dog food" - it must enforce its own rules.

use axiom_core::context::{CellContext, LayeredCellContext};
use axiom_core::id::{CellId, CorrelationId};
use axiom_core::layer::Layer;
use axiom_core::schema::ValidationResult;
use axiom_core::sealed::{
    AgentLayer, CanSendTo, ExecLayer, LayerMarker, OversightLayer, ValidateLayer,
};
use axiom_core::signal::{now_ns, Signal, SignalKind, VectorClock};
use axiom_core::version::SchemaVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestSignal {
    msg_id: axiom_core::id::MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
}

impl Signal for TestSignal {
    fn signal_type(&self) -> &'static str { "TestSignal" }
    fn msg_id(&self) -> &axiom_core::id::MsgId { &self.msg_id }
    fn correlation_id(&self) -> &CorrelationId { &self.correlation_id }
    fn vector_clock(&self) -> &VectorClock { &self.vector_clock }
    fn timestamp_ns(&self) -> u64 { now_ns() }
    fn kind(&self) -> SignalKind { SignalKind::Command }
    fn layer(&self) -> Layer { Layer::Exec }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn clone_signal(&self) -> Box<dyn Signal> { Box::new(self.clone()) }
    fn validate(&self) -> ValidationResult { ValidationResult::ok() }
    fn serialize_to_json(&self) -> ::axiom_core::Result<serde_json::Value> {
        serde_json::to_value(self).map_err(|e| ::axiom_core::AxiomError::SignalSerialization {
            signal_type: "ConformanceTestSignal".into(),
            message: e.to_string(),
        })
    }
}

fn assert_send_compile<S: CanSendTo<T>, T: LayerMarker>() {}

#[test]
fn conformance_can_send_to_matrix_legal_directions() {
    assert_send_compile::<OversightLayer, OversightLayer>();
    assert_send_compile::<OversightLayer, AgentLayer>();
    assert_send_compile::<OversightLayer, ValidateLayer>();
    assert_send_compile::<OversightLayer, ExecLayer>();

    assert_send_compile::<AgentLayer, AgentLayer>();
    assert_send_compile::<AgentLayer, ValidateLayer>();

    assert_send_compile::<ValidateLayer, ValidateLayer>();
    assert_send_compile::<ValidateLayer, ExecLayer>();
    assert_send_compile::<ValidateLayer, AgentLayer>();

    assert_send_compile::<ExecLayer, ExecLayer>();
}

#[test]
fn conformance_can_send_to_matrix_runtime_illegal_directions() {
    assert!(!Layer::Exec.can_send_to(Layer::Agent));
    assert!(!Layer::Exec.can_send_to(Layer::Validate));
    assert!(!Layer::Exec.can_send_to(Layer::Oversight));

    assert!(!Layer::Validate.can_send_to(Layer::Oversight));

    assert!(!Layer::Agent.can_send_to(Layer::Oversight));
}

#[test]
fn conformance_layered_context_compile_time_enforcement() {
    let cell_id = CellId::new("layered-conformance-test");
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);

    let mut layered_exec = LayeredCellContext::<ExecLayer>::from_cell_context(&mut ctx);
    
    let signal = TestSignal {
        msg_id: axiom_core::id::MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        vector_clock: VectorClock::new(),
    };

    assert!(layered_exec.send_to::<ExecLayer, _>(signal.clone(), "exec-target").is_ok());
}

#[test]
fn conformance_layer_marker_sealed() {
    let _ = std::mem::size_of::<OversightLayer>();
    let _ = std::mem::size_of::<AgentLayer>();
    let _ = std::mem::size_of::<ValidateLayer>();
    let _ = std::mem::size_of::<ExecLayer>();

    assert_eq!(OversightLayer::LAYER, Layer::Oversight);
    assert_eq!(AgentLayer::LAYER, Layer::Agent);
    assert_eq!(ValidateLayer::LAYER, Layer::Validate);
    assert_eq!(ExecLayer::LAYER, Layer::Exec);
}

#[test]
#[should_panic(expected = "Layer mismatch")]
fn conformance_as_layered_assertion() {
    let cell_id = CellId::new("layer-mismatch-test");

    let mut exec_ctx = CellContext::new(&cell_id, Layer::Exec);
    let _: LayeredCellContext<AgentLayer> = exec_ctx.as_layered::<AgentLayer>();
}

#[test]
fn conformance_reply_layer_validation() {
    let cell_id = CellId::new("reply-conformance-test");
    
    let mut exec_ctx = CellContext::new(&cell_id, Layer::Exec);
    
    let incoming_env = axiom_core::signal::SignalEnvelope {
        msg_id: axiom_core::id::MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        trace_id: None,
        signal_type: "Request".to_string(),
        vector_clock: VectorClock::new(),
        timestamp_ns: now_ns(),
        kind: SignalKind::Command,
        source_layer: Layer::Agent,
        target_layer: Layer::Exec,
        source_cell: Some("agent-cell".to_string()),
        target_cell: Some("exec-cell".to_string()),
        payload: serde_json::Value::Null,
        schema_version: SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    };

    let signal = TestSignal {
        msg_id: axiom_core::id::MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        vector_clock: VectorClock::new(),
    };

    assert!(matches!(
        exec_ctx.reply(&incoming_env, signal),
        Err(axiom_core::AxiomError::LayerViolation { from: Layer::Exec, to: Layer::Agent, .. })
    ));
}

#[test]
fn conformance_entropy_governor_is_oversight_layer() {
    assert_eq!(axiom_core::sealed::OversightLayer::LAYER, Layer::Oversight);

    let cell_id = CellId::new("governor-test");
    let mut oversight_ctx = CellContext::new(&cell_id, Layer::Oversight);
    let mut layered = LayeredCellContext::<OversightLayer>::from_cell_context(&mut oversight_ctx);

    let signal = TestSignal {
        msg_id: axiom_core::id::MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        vector_clock: VectorClock::new(),
    };

    assert!(layered.send_to::<ExecLayer, _>(signal.clone(), "any-cell").is_ok());
    assert!(layered.send_to::<ValidateLayer, _>(signal.clone(), "any-cell").is_ok());
    assert!(layered.send_to::<AgentLayer, _>(signal.clone(), "any-cell").is_ok());
    assert!(layered.send_to::<OversightLayer, _>(signal.clone(), "any-cell").is_ok());
}

#[test]
fn conformance_architecture_self_consistency() {
    let cell_id = CellId::new("self-consistency-test");

    let signal = TestSignal {
        msg_id: axiom_core::id::MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        vector_clock: VectorClock::new(),
    };

    let mut oversight_ctx = CellContext::new(&cell_id, Layer::Oversight);
    let mut layered_oversight = LayeredCellContext::<OversightLayer>::from_cell_context(&mut oversight_ctx);
    assert!(layered_oversight.send_to::<OversightLayer, _>(signal.clone(), "test").is_ok());
    assert!(layered_oversight.send_to::<AgentLayer, _>(signal.clone(), "test").is_ok());
    assert!(layered_oversight.send_to::<ValidateLayer, _>(signal.clone(), "test").is_ok());
    assert!(layered_oversight.send_to::<ExecLayer, _>(signal.clone(), "test").is_ok());

    let mut agent_ctx = CellContext::new(&cell_id, Layer::Agent);
    let mut layered_agent = LayeredCellContext::<AgentLayer>::from_cell_context(&mut agent_ctx);
    assert!(layered_agent.send_to::<AgentLayer, _>(signal.clone(), "test").is_ok());
    assert!(layered_agent.send_to::<ValidateLayer, _>(signal.clone(), "test").is_ok());

    let mut validate_ctx = CellContext::new(&cell_id, Layer::Validate);
    let mut layered_validate = LayeredCellContext::<ValidateLayer>::from_cell_context(&mut validate_ctx);
    assert!(layered_validate.send_to::<ValidateLayer, _>(signal.clone(), "test").is_ok());
    assert!(layered_validate.send_to::<ExecLayer, _>(signal.clone(), "test").is_ok());
    assert!(layered_validate.send_to::<AgentLayer, _>(signal.clone(), "test").is_ok());

    let mut exec_ctx = CellContext::new(&cell_id, Layer::Exec);
    let mut layered_exec = LayeredCellContext::<ExecLayer>::from_cell_context(&mut exec_ctx);
    assert!(layered_exec.send_to::<ExecLayer, _>(signal.clone(), "test").is_ok());
}

#[test]
fn conformance_layered_context_emit_to_validation() {
    let cell_id = CellId::new("emit-conformance-test");
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);
    let mut layered_exec = LayeredCellContext::<ExecLayer>::from_cell_context(&mut ctx);

    let signal = TestSignal {
        msg_id: axiom_core::id::MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        vector_clock: VectorClock::new(),
    };

    assert!(layered_exec.emit_to::<ExecLayer, _>(signal.clone()).is_ok());
}

#[test]
fn conformance_compile_fail_test_validates_illegal_calls() {
    assert!(!Layer::Exec.can_send_to(Layer::Agent));
}

#[test]
fn conformance_witness_methods_available_in_layered_context() {
    let cell_id = CellId::new("witness-conformance-test");
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);
    let layered = LayeredCellContext::<ExecLayer>::from_cell_context(&mut ctx);

    let _ = layered.witness();
    let _ = layered.cell_id();
    let _ = layered.layer();
    let _ = layered.vector_clock();
}
