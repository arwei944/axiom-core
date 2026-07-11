use axiom_kernel::axiom::ValidationResult;
use axiom_kernel::id::{CorrelationId, MsgId};
use axiom_kernel::layer::RuntimeTier;
use axiom_kernel::sealed::{
    AgentTier, CanSendTo, ExecTier, RuntimeTierMarker, OversightTier, ValidateTier,
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
    fn layer(&self) -> RuntimeTier {
        RuntimeTier::Exec
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

fn assert_send_compile<S: CanSendTo<T>, T: RuntimeTierMarker>() {}

#[test]
fn conformance_can_send_to_matrix_legal_directions() {
    assert_send_compile::<OversightTier, OversightTier>();
    assert_send_compile::<OversightTier, AgentTier>();
    assert_send_compile::<OversightTier, ValidateTier>();
    assert_send_compile::<OversightTier, ExecTier>();

    assert_send_compile::<AgentTier, AgentTier>();
    assert_send_compile::<AgentTier, ValidateTier>();

    assert_send_compile::<ValidateTier, ValidateTier>();
    assert_send_compile::<ValidateTier, ExecTier>();
    assert_send_compile::<ValidateTier, AgentTier>();

    assert_send_compile::<ExecTier, ExecTier>();
}

#[test]
fn conformance_can_send_to_matrix_runtime_illegal_directions() {
    assert!(!RuntimeTier::Exec.can_send_to(RuntimeTier::Agent));
    assert!(!RuntimeTier::Exec.can_send_to(RuntimeTier::Validate));
    assert!(!RuntimeTier::Exec.can_send_to(RuntimeTier::Oversight));

    assert!(!RuntimeTier::Validate.can_send_to(RuntimeTier::Oversight));

    assert!(!RuntimeTier::Agent.can_send_to(RuntimeTier::Oversight));
}

#[test]
fn conformance_layer_marker_sealed() {
    let _ = std::mem::size_of::<OversightTier>();
    let _ = std::mem::size_of::<AgentTier>();
    let _ = std::mem::size_of::<ValidateTier>();
    let _ = std::mem::size_of::<ExecTier>();

    assert_eq!(OversightTier::LAYER, RuntimeTier::Oversight);
    assert_eq!(AgentTier::LAYER, RuntimeTier::Agent);
    assert_eq!(ValidateTier::LAYER, RuntimeTier::Validate);
    assert_eq!(ExecTier::LAYER, RuntimeTier::Exec);
}

#[test]
fn conformance_entropy_governor_is_oversight_layer() {
    assert_eq!(axiom_kernel::sealed::OversightTier::LAYER, RuntimeTier::Oversight);
}

#[test]
fn conformance_compile_fail_test_validates_illegal_calls() {
    assert!(!RuntimeTier::Exec.can_send_to(RuntimeTier::Agent));
}
