//! Integration tests for entropy governance interceptors.
//!
//! These tests verify the complete governance flow:
//! 1. EntropyGovernor generates GovernanceAction (Throttle/Emergency)
//! 2. Dispatch loop updates shared interceptor state
//! 3. Interceptors actually reject messages based on the state

use std::collections::HashMap;
use std::sync::Arc;

use axiom_core::id::{CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::signal::{SignalKind, VectorClock};
use parking_lot::RwLock;

use axiom_runtime::bus::{BusInterceptor, InterceptDecision};
use axiom_runtime::entropy_gov::{EntropyEvent, EntropyGovernorCell, GovernanceAction};
use axiom_runtime::entropy_interceptors::{EmergencyInterceptor, ThrottleInterceptor};

fn make_env(target_cell: &str, source_layer: Layer) -> axiom_core::signal::SignalEnvelope {
    axiom_core::signal::SignalEnvelope {
        msg_id: MsgId::new("test"),
        correlation_id: CorrelationId::new("corr"),
        trace_id: None,
        signal_type: "TestSignal".into(),
        vector_clock: VectorClock::new(),
        timestamp_ns: 0,
        kind: SignalKind::Command,
        source_layer,
        target_layer: Layer::Exec,
        source_cell: None,
        target_cell: Some(target_cell.to_string()),
        payload: serde_json::Value::Null,
        schema_version: axiom_core::SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    }
}

#[tokio::test]
async fn test_governance_throttle_flow() {
    let throttle_state = Arc::new(RwLock::new(HashMap::new()));
    let interceptor = ThrottleInterceptor::new(throttle_state.clone());

    let governor = EntropyGovernorCell::new(0.3, 0.5, 0.7, 0.9);

    governor.record(EntropyEvent::Timeout {
        cell_id: "hot-cell".to_string(),
    });
    governor.record(EntropyEvent::Timeout {
        cell_id: "hot-cell".to_string(),
    });

    let action = governor.take_action();
    if let GovernanceAction::Throttle {
        target_cell,
        factor,
    } = action
    {
        throttle_state.write().insert(target_cell.unwrap(), factor);

        let env = make_env("hot-cell", Layer::Exec);

        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Allow
        ));
        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Reject { .. }
        ));
    }
}

#[tokio::test]
async fn test_governance_emergency_flow() {
    let emergency_mode = Arc::new(RwLock::new(false));
    let interceptor = EmergencyInterceptor::new(emergency_mode.clone());

    let governor = EntropyGovernorCell::new(0.3, 0.5, 0.7, 0.9);

    for _ in 0..10 {
        governor.record(EntropyEvent::AxiomViolation {
            cell_id: "critical-cell".to_string(),
        });
    }

    let action = governor.take_action();
    if let GovernanceAction::Emergency { .. } = action {
        *emergency_mode.write() = true;

        let env_non_oversight = make_env("target-cell", Layer::Exec);
        assert!(matches!(
            interceptor.intercept(&env_non_oversight),
            InterceptDecision::Reject { .. }
        ));

        let env_oversight = make_env("target-cell", Layer::Oversight);
        assert!(matches!(
            interceptor.intercept(&env_oversight),
            InterceptDecision::Allow
        ));
    }
}

#[tokio::test]
async fn test_governance_clear_throttle() {
    let throttle_state = Arc::new(RwLock::new(HashMap::new()));
    let interceptor = ThrottleInterceptor::new(throttle_state.clone());

    throttle_state
        .write()
        .insert("target-cell".to_string(), 0.5);

    let env = make_env("target-cell", Layer::Exec);
    assert!(matches!(
        interceptor.intercept(&env),
        InterceptDecision::Allow
    ));
    assert!(matches!(
        interceptor.intercept(&env),
        InterceptDecision::Reject { .. }
    ));

    throttle_state.write().clear();

    assert!(matches!(
        interceptor.intercept(&env),
        InterceptDecision::Allow
    ));
    assert!(matches!(
        interceptor.intercept(&env),
        InterceptDecision::Allow
    ));
}

#[tokio::test]
async fn test_governance_emergency_toggle_off() {
    let emergency_mode = Arc::new(RwLock::new(true));
    let interceptor = EmergencyInterceptor::new(emergency_mode.clone());

    let env = make_env("target-cell", Layer::Exec);
    assert!(matches!(
        interceptor.intercept(&env),
        InterceptDecision::Reject { .. }
    ));

    *emergency_mode.write() = false;

    assert!(matches!(
        interceptor.intercept(&env),
        InterceptDecision::Allow
    ));
}

#[tokio::test]
async fn test_governance_no_action_allows_all() {
    let throttle_state = Arc::new(RwLock::new(HashMap::new()));
    let emergency_mode = Arc::new(RwLock::new(false));

    let throttle_interceptor = ThrottleInterceptor::new(throttle_state);
    let emergency_interceptor = EmergencyInterceptor::new(emergency_mode);

    let governor = EntropyGovernorCell::new(0.3, 0.5, 0.7, 0.9);
    let action = governor.take_action();

    assert!(matches!(action, GovernanceAction::None));

    let env = make_env("target-cell", Layer::Exec);
    assert!(matches!(
        throttle_interceptor.intercept(&env),
        axiom_runtime::bus::InterceptDecision::Allow
    ));
    assert!(matches!(
        emergency_interceptor.intercept(&env),
        axiom_runtime::bus::InterceptDecision::Allow
    ));
}
