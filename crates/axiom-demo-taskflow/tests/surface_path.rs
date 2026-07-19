//! U4: unified surface body fields from real host state.

use axiom_demo_taskflow::agent_host::{AgentHost, HandoffRequestSpec};
use axiom_demo_taskflow::health::fetch_health;
use axiom_demo_taskflow::surface::SurfaceServer;
use std::time::Duration;

#[tokio::test]
async fn surface_exposes_governor_and_runs() {
    let spec = HandoffRequestSpec::default();
    let host = AgentHost::boot(&spec).await.expect("boot");
    host.submit_handoff(&spec.handoff).await.expect("submit");
    let _ = host.wait_outcome(Duration::from_secs(5)).await.expect("wait");

    let h = host.health().await;
    let mut gov = host.governor_snap.clone();
    if let Ok(g) = host.last.lock() {
        if let Some(ref o) = *g {
            gov.level = o.governor_level.clone();
            gov.score = o.governor_score;
            gov.decision = if o.ok {
                "allow".into()
            } else {
                "reject".into()
            };
        }
    }
    let runs = host.runs.clone();
    let server = SurfaceServer::start(
        "127.0.0.1:0".parse().unwrap(),
        h,
        gov,
        vec!["agent-cell".into()],
        runs,
    )
    .await
    .expect("server");
    let addr = server.addr();
    tokio::time::sleep(Duration::from_millis(30)).await;
    let (status, body) = fetch_health(&format!("{addr}/api/v1/surface"))
        .await
        .expect("fetch");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("admit_authority"), "{body}");
    assert!(body.contains("governor"), "{body}");
    assert!(body.contains("witness-only"), "{body}");
    assert!(body.contains("recent_runs"), "{body}");
    assert!(body.contains("decide_api"), "{body}");
    // T12 fields present even without start_full (defaults).
    assert!(body.contains("health"), "{body}");
    server.stop().await;
    host.stop().await;
}
