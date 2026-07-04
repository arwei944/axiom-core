//! Runtime-level error path integration tests.
//!
//! These tests verify the runtime's behavior under error conditions:
//! 1. LayerViolation - Exec -> Agent rejected by ArchitectureGuardian
//! 2. Witness hash chain break detection
//! 3. DLQ collects failed messages
//! 4. Cell crash recovery via Supervisor
//! 5. Mailbox bounded capacity

use axiom_core::cell::SupervisionStrategy;
use axiom_core::id::{CorrelationId, MsgId, WitnessId};
use axiom_core::layer::Layer;
use axiom_core::signal::{SignalEnvelope, SignalKind, VectorClock};
use axiom_core::version::VersionInfo;
use axiom_core::witness::{TransitionOutcome, Witness, WitnessHash, WitnessKind, WitnessMetrics};
use axiom_runtime::bus::BusInterceptor;
use axiom_runtime::guardian::ArchitectureGuardian;
use std::sync::Arc;

// ========== Test 1: LayerViolation (Exec -> Agent rejected) ==========

#[tokio::test]
async fn test_layer_violation_exec_to_agent_rejected() {
    let guardian = ArchitectureGuardian::new();

    let env = make_env("exec-cell", "agent-cell", Layer::Exec, Layer::Agent);
    let decision = guardian.intercept(&env);

    assert!(
        matches!(
            decision,
            axiom_runtime::bus::InterceptDecision::Reject { .. }
        ),
        "Exec -> Agent should be rejected by ArchitectureGuardian"
    );
}

#[tokio::test]
async fn test_layer_violation_oversight_can_send_to_all() {
    let guardian = ArchitectureGuardian::new();

    for target_layer in [Layer::Oversight, Layer::Agent, Layer::Validate, Layer::Exec] {
        let env = make_env("oversight-cell", "target", Layer::Oversight, target_layer);
        let decision = guardian.intercept(&env);
        assert!(
            matches!(decision, axiom_runtime::bus::InterceptDecision::Allow),
            "Oversight -> {:?} should be allowed",
            target_layer
        );
    }
}

#[tokio::test]
async fn test_layer_violation_exec_to_exec_allowed() {
    let guardian = ArchitectureGuardian::new();

    let env = make_env("exec-1", "exec-2", Layer::Exec, Layer::Exec);
    let decision = guardian.intercept(&env);

    assert!(
        matches!(decision, axiom_runtime::bus::InterceptDecision::Allow),
        "Exec -> Exec should be allowed"
    );
}

#[tokio::test]
async fn test_layer_violation_validate_to_exec_allowed() {
    let guardian = ArchitectureGuardian::new();

    let env = make_env("val-cell", "exec-cell", Layer::Validate, Layer::Exec);
    let decision = guardian.intercept(&env);

    assert!(
        matches!(decision, axiom_runtime::bus::InterceptDecision::Allow),
        "Validate -> Exec should be allowed"
    );
}

#[tokio::test]
async fn test_layer_violation_exec_to_validate_rejected() {
    let guardian = ArchitectureGuardian::new();

    let env = make_env("exec-cell", "val-cell", Layer::Exec, Layer::Validate);
    let decision = guardian.intercept(&env);

    assert!(
        matches!(
            decision,
            axiom_runtime::bus::InterceptDecision::Reject { .. }
        ),
        "Exec -> Validate should be rejected"
    );
}

// ========== Test 2: Witness hash chain break detection ==========

#[test]
fn test_witness_chain_valid_chain_passes() {
    let w1 = make_witness("cell-a", 1, None, "first");
    let w2 = make_witness("cell-a", 2, Some(w1.hash), "second");
    let w3 = make_witness("cell-a", 3, Some(w2.hash), "third");

    let chain = vec![w1, w2, w3];
    assert!(
        Witness::verify_chain_integrity(&chain),
        "valid chain should verify successfully"
    );
}

#[test]
fn test_witness_chain_broken_chain_fails() {
    let w1 = make_witness("cell-a", 1, None, "first");
    let broken = make_witness("cell-a", 2, Some(WitnessHash([9u8; 32])), "broken");
    let w3 = make_witness("cell-a", 3, None, "third");

    let chain = vec![w1, broken, w3];
    assert!(
        !Witness::verify_chain_integrity(&chain),
        "broken chain should fail verification"
    );
}

#[test]
fn test_witness_chain_empty_passes() {
    let empty: Vec<Witness> = vec![];
    assert!(Witness::verify_chain_integrity(&empty));
}

#[test]
fn test_witness_chain_single_element_passes() {
    let w = make_witness("single", 1, None, "only");
    let chain = vec![w];
    assert!(Witness::verify_chain_integrity(&chain));
}

// ========== Test 3: DLQ collects failed messages ==========

#[test]
fn test_dlq_basic_enqueue_and_drain() {
    use axiom_runtime::dlq::DeadLetterQueue;

    let dlq = DeadLetterQueue::new(100);

    let env = make_env("src", "dst", Layer::Exec, Layer::Exec);
    dlq.enqueue(env, "test failure reason");

    assert_eq!(dlq.len(), 1);

    let drained = dlq.drain();
    assert_eq!(drained.len(), 1);
    assert_eq!(dlq.len(), 0);
}

#[test]
fn test_dlq_capacity_limit_drops_oldest() {
    use axiom_runtime::dlq::DeadLetterQueue;

    let dlq = DeadLetterQueue::new(3);

    for i in 0..5 {
        let env = make_env(&format!("src-{}", i), "dst", Layer::Exec, Layer::Exec);
        dlq.enqueue(env, &format!("failure-{}", i));
    }

    assert_eq!(dlq.len(), 3);
    let drained = dlq.drain();
    assert_eq!(drained[0].reason, "failure-2");
    assert_eq!(drained[1].reason, "failure-3");
    assert_eq!(drained[2].reason, "failure-4");
}

// ========== Test 4: Cell crash recovery via Supervisor ==========

#[tokio::test]
async fn test_supervisor_restart_with_exponential_backoff() {
    use axiom_runtime::supervisor::{SupervisionDecision, Supervisor};

    let supervisor = Supervisor::new();
    supervisor
        .register_cell(
            "crashy-cell",
            SupervisionStrategy::Restart { max_retries: 3 },
        )
        .await;

    let d1 = supervisor.record_panic("crashy-cell").await;
    assert!(matches!(
        d1,
        SupervisionDecision::Restart { backoff_ms: 100 }
    ));

    let d2 = supervisor.record_panic("crashy-cell").await;
    assert!(matches!(
        d2,
        SupervisionDecision::Restart { backoff_ms: 200 }
    ));

    let d3 = supervisor.record_panic("crashy-cell").await;
    assert!(matches!(
        d3,
        SupervisionDecision::Restart { backoff_ms: 400 }
    ));

    let d4 = supervisor.record_panic("crashy-cell").await;
    assert!(matches!(d4, SupervisionDecision::Stop));
}

#[tokio::test]
async fn test_supervisor_stop_strategy() {
    use axiom_runtime::supervisor::{SupervisionDecision, Supervisor};

    let supervisor = Supervisor::new();
    supervisor
        .register_cell("stop-cell", SupervisionStrategy::Stop)
        .await;

    let d = supervisor.record_panic("stop-cell").await;
    assert!(matches!(d, SupervisionDecision::Stop));
}

#[tokio::test]
async fn test_supervisor_circuit_breaker_opens_and_resets() {
    use axiom_runtime::supervisor::Supervisor;

    let supervisor = Supervisor::new();
    supervisor
        .register_cell(
            "cb-cell",
            SupervisionStrategy::CircuitBreak {
                failure_threshold: 2,
                reset_after_ms: 50,
            },
        )
        .await;

    assert!(supervisor.before_handle("cb-cell").await);
    let _ = supervisor.record_panic("cb-cell").await;
    assert!(supervisor.before_handle("cb-cell").await);
    let _ = supervisor.record_panic("cb-cell").await;

    assert!(!supervisor.before_handle("cb-cell").await);

    tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;
    assert!(supervisor.before_handle("cb-cell").await);
}

#[tokio::test]
async fn test_supervisor_escalate_strategy() {
    use axiom_runtime::supervisor::{SupervisionDecision, Supervisor};

    let supervisor = Supervisor::new();
    supervisor
        .register_cell("esc-cell", SupervisionStrategy::Escalate)
        .await;

    let d = supervisor.record_panic("esc-cell").await;
    assert!(matches!(d, SupervisionDecision::Escalate));
}

// ========== Test 5: Mailbox bounded capacity ==========

#[tokio::test]
async fn test_mailbox_bounded_capacity_overflow() {
    use axiom_runtime::mailbox::Mailbox;

    let mailbox = Arc::new(Mailbox::new(4));

    for i in 0..4 {
        let env = make_env(&format!("src-{}", i), "dst", Layer::Exec, Layer::Exec);
        let result = mailbox.push(env).await;
        assert!(result.is_ok(), "msg {} should fit in capacity 4", i);
    }

    let overflow = make_env("overflow", "dst", Layer::Exec, Layer::Exec);
    let result = mailbox.push(overflow).await;
    assert!(result.is_err(), "5th message should overflow capacity 4");
}

#[tokio::test]
async fn test_mailbox_fifo_ordering() {
    use axiom_runtime::mailbox::Mailbox;

    let mailbox = Arc::new(Mailbox::new(16));

    for i in 0..5 {
        let mut env = make_env("src", "dst", Layer::Exec, Layer::Exec);
        env.signal_type = format!("msg-{}", i);
        mailbox.push(env).await.unwrap();
    }

    for i in 0..5 {
        let received = mailbox.pop().await.unwrap();
        assert_eq!(received.signal_type, format!("msg-{}", i));
    }
}

#[tokio::test]
async fn test_mailbox_drain_clears_all() {
    use axiom_runtime::mailbox::Mailbox;

    let mailbox = Arc::new(Mailbox::new(16));

    for i in 0..5 {
        let mut env = make_env("src", "dst", Layer::Exec, Layer::Exec);
        env.signal_type = format!("msg-{}", i);
        mailbox.push(env).await.unwrap();
    }

    assert_eq!(mailbox.len().await, 5);

    let drained = mailbox.drain().await;
    assert_eq!(drained.len(), 5);
    assert_eq!(mailbox.len().await, 0);

    for (i, env) in drained.iter().enumerate().take(5) {
        assert_eq!(env.signal_type, format!("msg-{}", i));
    }
}

// ========== Helper functions ==========

fn make_env(
    source: &str,
    target: &str,
    source_layer: Layer,
    target_layer: Layer,
) -> SignalEnvelope {
    SignalEnvelope {
        msg_id: MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        trace_id: None,
        signal_type: "TestSignal".to_string(),
        vector_clock: VectorClock::new(),
        timestamp_ns: 0,
        kind: SignalKind::Command,
        source_layer,
        target_layer,
        source_cell: Some(source.to_string()),
        target_cell: Some(target.to_string()),
        payload: serde_json::json!({}),
        schema_version: axiom_core::version::SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    }
}

fn make_witness(cell_id: &str, seq: u64, prev_hash: Option<WitnessHash>, summary: &str) -> Witness {
    let w = Witness {
        witness_id: WitnessId::new(format!("wit-{}", seq)),
        schema_version: axiom_core::version::SchemaVersion::new(1),
        cell_id: cell_id.to_string(),
        correlation_id: CorrelationId::new("test-corr"),
        trace_id: None,
        triggering_msg_id: Some(MsgId::new(format!("msg-{}", seq))),
        vector_clock: VectorClock::new(),
        timestamp_ns: seq * 1000,
        prev_hash,
        state_before_hash: None,
        state_after_hash: None,
        hash: WitnessHash([seq as u8; 32]),
        summary: summary.to_string(),
        outcome: TransitionOutcome::Success,
        metrics: WitnessMetrics::default(),
        version_info: VersionInfo::current(),
        signal_fingerprint: [0u8; 32],
        payload_size_bytes: 0,
        kind: WitnessKind::StateTransition,
    };
    w
}
