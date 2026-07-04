use crate::runtime::AxiomRuntime;
use crate::runtime::CellRegistration;
use crate::runtime::RuntimeConfig;
use axiom_core::cell::SupervisionStrategy;
use axiom_core::clock::global_clock;
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::signal::{SignalKind, VectorClock};
use axiom_core::version::Version;
use std::time::Duration;

fn env_from_to(from: Layer, to: Layer, target: Option<&str>) -> axiom_core::signal::SignalEnvelope {
    axiom_core::signal::SignalEnvelope {
        msg_id: MsgId::new("test-msg"),
        correlation_id: CorrelationId::new("test-corr"),
        trace_id: None,
        signal_type: "Test".into(),
        vector_clock: VectorClock::new(),
        timestamp_ns: global_clock().now_ns(),
        kind: SignalKind::Command,
        source_layer: from,
        target_layer: to,
        source_cell: None,
        target_cell: target.map(|s| s.to_string()),
        payload: serde_json::Value::Null,
        schema_version: axiom_core::SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    }
}

#[tokio::test]
async fn test_l2_guardian_blocks_exec_to_agent() {
    let rt = AxiomRuntime::new(RuntimeConfig {
        mailbox_capacity: 16,
        ..Default::default()
    });
    let _mb = rt
        .register_cell(CellRegistration {
            id: CellId::new("exec-cell"),
            layer: Layer::Exec,
            version: Version::new(0, 1, 0),
            supervision_strategy: SupervisionStrategy::Restart { max_retries: 2 },
            cell: None,
            factory: None,
        })
        .await
        .unwrap();
    rt.start().await.unwrap();

    let bad = env_from_to(Layer::Exec, Layer::Agent, None);
    let result = rt.bus().publish(bad).await;
    assert!(
        result.is_err(),
        "Exec->Agent must be blocked by L2 guardian (compile-time CanSendTo already prevents it, but runtime doubles down)"
    );

    let good = env_from_to(Layer::Oversight, Layer::Exec, Some("exec-cell"));
    assert!(rt.bus().publish(good).await.is_ok());

    rt.stop().await;
}

#[tokio::test]
async fn test_runtime_health_updates() {
    let rt = AxiomRuntime::default();
    let _mb = rt
        .register_cell(CellRegistration {
            id: CellId::new("cell-1"),
            layer: Layer::Exec,
            version: Version::new(0, 1, 0),
            supervision_strategy: SupervisionStrategy::default(),
            cell: None,
            factory: None,
        })
        .await
        .unwrap();
    rt.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let h = rt.health().await;
    assert!(h.started);
    assert!(h.preflight_passed);
    assert_eq!(h.cells_running, 1);
    rt.stop().await;
}

#[tokio::test]
async fn test_entropy_governor_triggers() {
    let gov = std::sync::Arc::new(crate::entropy_gov::EntropyGovernorCell::default());
    for _ in 0..5 {
        gov.record(crate::entropy_gov::EntropyEvent::CellRestart {
            cell_id: "c1".into(),
        });
    }
    let snap = gov.snapshot();
    assert!(snap.global.value > 1.0, "score should exceed threshold");
}

#[tokio::test]
async fn test_preflight_rejects_bad_version() {
    let rt = AxiomRuntime::default();
    let _ = rt
        .register_cell(CellRegistration {
            id: CellId::new("v1-cell"),
            layer: Layer::Exec,
            version: Version::new(1, 0, 0),
            supervision_strategy: SupervisionStrategy::default(),
            cell: None,
            factory: None,
        })
        .await
        .unwrap();
    let result = rt.start().await;
    assert!(
        result.is_err(),
        "preflight must reject cells with non-zero major version"
    );
}
