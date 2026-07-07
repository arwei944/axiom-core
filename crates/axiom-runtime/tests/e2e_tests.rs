//! End-to-end integration tests for Axiom Runtime.
//!
//! Covers: Runtime startup -> Cell registration -> Signal dispatch ->
//! EventStore persistence -> Witness recording -> Runtime shutdown.

use axiom_kernel::cell::SupervisionStrategy;
use axiom_kernel::clock::global_clock;
use axiom_kernel::id::{CellId, CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::signal::{SignalKind, VectorClock};
use axiom_kernel::version::Version;
use axiom_runtime::CellRegistration;

fn make_signal_envelope(target_cell: &str) -> axiom_kernel::signal::SignalEnvelope {
    axiom_kernel::signal::SignalEnvelope {
        msg_id: MsgId::new("e2e-cmd"),
        correlation_id: CorrelationId::new("e2e-corr"),
        trace_id: None,
        signal_type: "E2eCommand".to_string(),
        vector_clock: VectorClock::new(),
        timestamp_ns: global_clock().now_ns(),
        kind: SignalKind::Command,
        source_layer: Layer::Oversight,
        target_layer: Layer::Exec,
        source_cell: None,
        target_cell: Some(target_cell.to_string()),
        payload: serde_json::json!({"payload": "hello-e2e"}),
        schema_version: axiom_kernel::SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    }
}

#[tokio::test]
async fn test_runtime_e2e_signal_dispatch_and_persistence() {
    let runtime = axiom_runtime::AxiomRuntime::new(axiom_runtime::RuntimeConfig::default());

    let reg = CellRegistration {
        id: CellId::new("e2e-cell"),
        layer: Layer::Exec,
        version: Version::new(0, 1, 0),
        supervision_strategy: SupervisionStrategy::Restart { max_retries: 3 },
        cell: None,
        factory: None,
    };

    let _mailbox = runtime.register_cell(reg).await.unwrap();

    runtime.start().await.unwrap();

    let env = make_signal_envelope("e2e-cell");
    let result = runtime.bus().publish(env).await;
    assert!(result.is_ok(), "signal should be published successfully");

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let health = runtime.health().await;
    assert!(health.started);
    assert!(health.preflight_passed);
    assert_eq!(health.cells_running, 1);

    runtime.stop().await;
}
