//! Constraint runtime tests: interceptors, guard, capability version, witness records.

use axiom_kernel::id::{CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::signal::{SignalKind, VectorClock};
use axiom_kernel::KernelError;
use axiom_runtime::bus::{BusInterceptor, InterceptDecision, MessageBus};
use axiom_runtime::constraint_validator::{ConstraintValidator, ValidationContext};
use axiom_runtime::interceptors::{
    CapabilityVersionInterceptor, GuardInterceptor, HopLimitInterceptor, IdempotencyInterceptor,
    SchemaVersionInterceptor,
};
use std::sync::Arc;

fn make_env(hops: u32, id: &str, signal_type: &str) -> axiom_kernel::signal::SignalEnvelope {
    axiom_kernel::signal::SignalEnvelope {
        msg_id: MsgId::new(id),
        correlation_id: CorrelationId::new("c"),
        trace_id: None,
        signal_type: signal_type.to_string(),
        vector_clock: VectorClock::new(),
        timestamp_ns: 0,
        kind: SignalKind::Command,
        source_layer: Layer::Exec,
        target_layer: Layer::Exec,
        source_cell: None,
        target_cell: None,
        payload: serde_json::Value::Null,
        schema_version: axiom_kernel::SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: hops,
    }
}

#[test]
fn capability_version_interceptor_allows_when_no_registered() {
    let interceptor =
        CapabilityVersionInterceptor::new(ConstraintValidator::new(ValidationContext::default()));
    let env = make_env(0, "cap-ok", "Test");
    assert!(matches!(interceptor.intercept(&env), InterceptDecision::Allow));
}

#[test]
fn guard_interceptor_blocks_forbidden_signal() {
    let interceptor = GuardInterceptor;
    let mut env = make_env(0, "guard-bad", "ForbiddenSignal");
    env.source_layer = Layer::Exec;
    env.target_layer = Layer::Exec;
    assert!(matches!(interceptor.intercept(&env), InterceptDecision::Reject { .. }));
}

#[test]
fn guard_interceptor_allows_normal_signal() {
    let interceptor = GuardInterceptor;
    let env = make_env(0, "guard-ok", "NormalSignal");
    assert!(matches!(interceptor.intercept(&env), InterceptDecision::Allow));
}

#[tokio::test]
async fn bus_rejects_illegal_message_with_reason() {
    let bus = MessageBus::new();
    bus.register_interceptor(Arc::new(GuardInterceptor)).await;
    let mut env = make_env(0, "bus-bad", "ForbiddenSignal");
    env.source_layer = Layer::Exec;
    env.target_layer = Layer::Exec;
    let result = bus.publish(env).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    if let KernelError::SignalValidationFailed(reason) = err {
        assert!(reason.contains("guard blocked signal"));
    } else {
        panic!("expected SignalValidationFailed, got: {err:?}");
    }
}

#[tokio::test]
async fn constraint_validator_shared_across_interceptors() {
    let validator =
        ConstraintValidator::new(ValidationContext::from_envelope(&make_env(0, "v", "T")));
    let cap = CapabilityVersionInterceptor::new(validator.clone());
    let guard = GuardInterceptor;
    let env = make_env(0, "shared", "NormalSignal");
    assert!(matches!(cap.intercept(&env), InterceptDecision::Allow));
    assert!(matches!(guard.intercept(&env), InterceptDecision::Allow));
}

#[test]
fn hop_limit_interceptor_blocks_excessive_hops() {
    let interceptor = HopLimitInterceptor::new(3);
    let env = make_env(3, "hop", "Test");
    assert!(matches!(interceptor.intercept(&env), InterceptDecision::Reject { .. }));
}

#[test]
fn schema_version_interceptor_blocks_zero() {
    let interceptor = SchemaVersionInterceptor;
    let mut env = make_env(0, "schema", "Test");
    env.schema_version = axiom_kernel::SchemaVersion::new(0);
    assert!(matches!(interceptor.intercept(&env), InterceptDecision::Reject { .. }));
}

#[test]
fn idempotency_interceptor_blocks_duplicate() {
    let interceptor = IdempotencyInterceptor::default();
    let env = make_env(0, "dup", "Test");
    assert!(matches!(interceptor.intercept(&env), InterceptDecision::Allow));
    assert!(matches!(interceptor.intercept(&env), InterceptDecision::Reject { .. }));
}
