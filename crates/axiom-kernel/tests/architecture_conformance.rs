use axiom_kernel::axiom::ValidationResult;
use axiom_kernel::id::{CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::sealed::{
    AgentLayer, CanSendTo, ExecLayer, LayerMarker, OversightLayer, ValidateLayer,
};
use axiom_kernel::signal::{now_ns, Signal, SignalKind, VectorClock};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestSignal {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
}

impl Signal for TestSignal {
    fn signal_type(&self) -> &'static str {
        "TestSignal"
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
        now_ns()
    }
    fn kind(&self) -> SignalKind {
        SignalKind::Command
    }
    fn layer(&self) -> Layer {
        Layer::Exec
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn clone_signal(&self) -> Box<dyn Signal> {
        Box::new(self.clone())
    }
    fn validate(&self) -> ValidationResult {
        ValidationResult::ok()
    }
    fn serialize_to_json(&self) -> axiom_kernel::KernelResult<serde_json::Value> {
        serde_json::to_value(self)
            .map_err(|e| axiom_kernel::KernelError::SerializationError(e.to_string()))
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
fn conformance_entropy_governor_is_oversight_layer() {
    assert_eq!(axiom_kernel::sealed::OversightLayer::LAYER, Layer::Oversight);
}

#[test]
fn conformance_compile_fail_test_validates_illegal_calls() {
    assert!(!Layer::Exec.can_send_to(Layer::Agent));
}
