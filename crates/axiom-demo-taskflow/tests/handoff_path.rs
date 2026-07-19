//! U3 integration: real Handoff → AgentCell → Workbench → Witness on AxiomRuntime.

use axiom_demo_taskflow::agent_host::{run_handoff, HandoffRequestSpec};
use axiom_isa::{HandoffRequest, WitnessJournal};

#[tokio::test]
async fn handoff_workbench_witness_chain() {
    let o = run_handoff(HandoffRequestSpec::default())
        .await
        .expect("handoff run");
    assert!(o.ok, "expected ok, err={:?}", o.error);
    assert!(
        o.witnesses.len() >= 4,
        "expected multi-step Witness, got {}",
        o.witnesses.len()
    );
    WitnessJournal::verify_chain(&o.witnesses).expect("chain");
    let r = o.result.expect("result");
    assert!(r.success);
    assert!(!r.proposal.is_empty());
    assert!(o.witnesses.iter().any(|w| w.summary.contains("governor")));
    assert!(o
        .witnesses
        .iter()
        .any(|w| w.summary.contains("workbench") || w.summary.contains("composer")));
}

#[tokio::test]
async fn handoff_governor_refuse() {
    let o = run_handoff(HandoffRequestSpec {
        preload_entropy: true,
        handoff: HandoffRequest::new("t", "s", "d", "echo", "x"),
        ..Default::default()
    })
    .await
    .expect("run");
    assert!(!o.ok);
    let err = o.error.unwrap_or_default();
    assert!(
        err.contains("governor") || err.contains("rejected") || err.contains("entropy"),
        "{err}"
    );
    WitnessJournal::verify_chain(&o.witnesses).expect("chain on reject");
}

#[tokio::test]
async fn handoff_rejects_disallowed_intent() {
    let o = run_handoff(HandoffRequestSpec {
        handoff: HandoffRequest::new("t", "s", "d", "shell_raw", "rm -rf /"),
        ..Default::default()
    })
    .await
    .expect("run");
    assert!(!o.ok, "shell_raw must fail allow-list");
    WitnessJournal::verify_chain(&o.witnesses).expect("chain");
}
