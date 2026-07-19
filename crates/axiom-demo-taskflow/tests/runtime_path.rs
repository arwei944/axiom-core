//! Integration tests: real AxiomRuntime + TaskCell commercial path.

use axiom_demo_taskflow::pipeline::FailMode;
use axiom_demo_taskflow::runtime_host::{run_commercial, RunRequest};
use axiom_isa::{GovernorConfig, WitnessJournal};
use axiom_kernel::entropy::EntropyLevel;
use serde_json::json;

#[tokio::test]
async fn runtime_success_witness_chain() {
    let outcomes = run_commercial(RunRequest {
        fail: FailMode::None,
        payload: json!({"title": "t1", "priority": 2, "payload": "hello"}),
        submissions: 1,
        ..Default::default()
    })
    .await
    .expect("runtime run");

    let o = &outcomes[0];
    assert!(o.ok, "expected ok, err={:?}", o.error);
    assert!(
        o.witnesses.len() >= 5,
        "expected multi-step chain, got {}",
        o.witnesses.len()
    );
    WitnessJournal::verify_chain(&o.witnesses).expect("chain integrity");
    assert!(o.result.as_ref().map(|r| r.stored).unwrap_or(false));
    assert!(o.witnesses.iter().any(|w| w.summary.contains("governor")));
    assert!(o.witnesses.iter().any(|w| w.summary.contains("composer")));
    assert!(o.witnesses.iter().any(|w| w.summary.contains("atom")));
    assert!(o.witnesses.iter().any(|w| w.summary.contains("port")));
}

#[tokio::test]
async fn runtime_governor_refuses_when_entropy_high() {
    let outcomes = run_commercial(RunRequest {
        fail: FailMode::None,
        preload_entropy: true,
        payload: json!({"title": "x", "priority": 1, "payload": "y"}),
        submissions: 1,
        ..Default::default()
    })
    .await
    .expect("runtime run");

    let o = &outcomes[0];
    assert!(!o.ok, "governor should refuse");
    let err = o.error.as_deref().unwrap_or("");
    assert!(
        err.contains("governor") || err.contains("rejected") || err.contains("entropy"),
        "unexpected error: {err}"
    );
    WitnessJournal::verify_chain(&o.witnesses).expect("chain still valid on reject");
    assert!(o
        .witnesses
        .iter()
        .any(|w| w.summary.contains("governor") && w.summary.contains("reject")));
}

#[tokio::test]
async fn runtime_fail_opens_circuit_or_rejects() {
    let outcomes = run_commercial(RunRequest {
        fail: FailMode::ExecuteAlways,
        governor: GovernorConfig {
            reject_from: EntropyLevel::Critical,
            force_open: false,
        },
        payload: json!({"title": "flaky", "priority": 1, "payload": "z"}),
        submissions: 4,
        ..Default::default()
    })
    .await
    .expect("runtime run");

    assert_eq!(outcomes.len(), 4);
    let saw = outcomes.iter().any(|o| {
        o.circuit.contains("Open")
            || o.error
                .as_deref()
                .map(|e| e.contains("circuit open") || e.contains("exhausted"))
                .unwrap_or(false)
    });
    assert!(saw, "expected circuit/retry failure path: {outcomes:?}");
    for o in &outcomes {
        WitnessJournal::verify_chain(&o.witnesses).expect("each chain valid");
    }
}
