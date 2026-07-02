//! Integration tests for Cell restart mechanism with exponential backoff.

use axiom_core::cell::SupervisionStrategy;
use axiom_runtime::supervisor::{Supervisor, SupervisionDecision};

#[tokio::test]
async fn test_restart_with_exponential_backoff() {
    let supervisor = Supervisor::new();
    supervisor
        .register_cell("test-cell", SupervisionStrategy::Restart { max_retries: 5 })
        .await;

    let d1 = supervisor.record_panic("test-cell").await;
    assert!(matches!(d1, SupervisionDecision::Restart { backoff_ms: 100 }));

    let d2 = supervisor.record_panic("test-cell").await;
    assert!(matches!(d2, SupervisionDecision::Restart { backoff_ms: 200 }));

    let d3 = supervisor.record_panic("test-cell").await;
    assert!(matches!(d3, SupervisionDecision::Restart { backoff_ms: 400 }));

    let d4 = supervisor.record_panic("test-cell").await;
    assert!(matches!(d4, SupervisionDecision::Restart { backoff_ms: 800 }));

    let d5 = supervisor.record_panic("test-cell").await;
    assert!(matches!(d5, SupervisionDecision::Restart { backoff_ms: 1600 }));

    let d6 = supervisor.record_panic("test-cell").await;
    assert!(matches!(d6, SupervisionDecision::Stop));
}

#[tokio::test]
async fn test_backoff_caps_at_30_seconds() {
    let supervisor = Supervisor::new();
    supervisor
        .register_cell("test-cell", SupervisionStrategy::Restart { max_retries: 10 })
        .await;

    for attempt in 1..=10 {
        let decision = supervisor.record_panic("test-cell").await;
        if let SupervisionDecision::Restart { backoff_ms } = decision {
            assert!(backoff_ms <= 30_000, "backoff should cap at 30s");
            if attempt >= 10 {
                assert_eq!(backoff_ms, 30_000, "backoff should reach cap at attempt 10");
            }
        } else {
            break;
        }
    }
}

#[tokio::test]
async fn test_circuit_breaker_stops_calls() {
    let supervisor = Supervisor::new();
    supervisor
        .register_cell(
            "test-cell",
            SupervisionStrategy::CircuitBreak {
                failure_threshold: 2,
                reset_after_ms: 100,
            },
        )
        .await;

    assert!(supervisor.before_handle("test-cell").await);
    let _ = supervisor.record_panic("test-cell").await;
    assert!(supervisor.before_handle("test-cell").await);
    let _ = supervisor.record_panic("test-cell").await;
    assert!(!supervisor.before_handle("test-cell").await);

    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    assert!(supervisor.before_handle("test-cell").await);
}

#[tokio::test]
async fn test_restart_count_tracking() {
    let supervisor = Supervisor::new();
    supervisor
        .register_cell("test-cell", SupervisionStrategy::Restart { max_retries: 3 })
        .await;

    assert_eq!(supervisor.restart_count("test-cell").await, 0);
    let _ = supervisor.record_panic("test-cell").await;
    assert_eq!(supervisor.restart_count("test-cell").await, 1);
    let _ = supervisor.record_panic("test-cell").await;
    assert_eq!(supervisor.restart_count("test-cell").await, 2);
    let _ = supervisor.record_panic("test-cell").await;
    assert_eq!(supervisor.restart_count("test-cell").await, 3);
    let _ = supervisor.record_panic("test-cell").await;
    assert_eq!(supervisor.restart_count("test-cell").await, 4);
}

#[tokio::test]
async fn test_success_resets_restart_count() {
    let supervisor = Supervisor::new();
    supervisor
        .register_cell("test-cell", SupervisionStrategy::Restart { max_retries: 5 })
        .await;

    let _ = supervisor.record_panic("test-cell").await;
    assert_eq!(supervisor.restart_count("test-cell").await, 1);

    supervisor.record_success("test-cell").await;
    assert_eq!(supervisor.restart_count("test-cell").await, 1);
}

#[tokio::test]
async fn test_circuit_break_decision_includes_until_time() {
    let supervisor = Supervisor::new();
    supervisor
        .register_cell(
            "test-cell",
            SupervisionStrategy::CircuitBreak {
                failure_threshold: 1,
                reset_after_ms: 5000,
            },
        )
        .await;

    let decision = supervisor.record_panic("test-cell").await;
    if let SupervisionDecision::CircuitBreak { until } = decision {
        let now = std::time::Instant::now();
        assert!(until > now);
        assert!(until < now + std::time::Duration::from_secs(6));
    } else {
        panic!("expected CircuitBreak decision");
    }
}

#[tokio::test]
async fn test_stop_strategy_stops_after_first_panic() {
    let supervisor = Supervisor::new();
    supervisor
        .register_cell("test-cell", SupervisionStrategy::Stop)
        .await;

    let decision = supervisor.record_panic("test-cell").await;
    assert!(matches!(decision, SupervisionDecision::Stop));
}

#[tokio::test]
async fn test_escalate_strategy() {
    let supervisor = Supervisor::new();
    supervisor
        .register_cell("test-cell", SupervisionStrategy::Escalate)
        .await;

    let decision = supervisor.record_panic("test-cell").await;
    assert!(matches!(decision, SupervisionDecision::Escalate));
}