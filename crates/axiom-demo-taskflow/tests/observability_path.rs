//! T12 — surface exposes metrics, lenses, plugins, full health.

use axiom_demo_taskflow::agent_host::{AgentHost, HandoffRequestSpec};
use axiom_demo_taskflow::health::fetch_health;
use axiom_demo_taskflow::metrics::new_metrics;
use axiom_demo_taskflow::plugin_host::ProductPluginHost;
use axiom_demo_taskflow::surface::SurfaceServer;
use std::time::Duration;

#[tokio::test]
async fn surface_metrics_lens_plugins() {
    let metrics = new_metrics();
    let plugins = ProductPluginHost::new();
    plugins.boot_defaults().await.expect("plugin boot");
    let plugin_ids = plugins.plugin_ids().await;

    let spec = HandoffRequestSpec::default();
    let host = AgentHost::boot(&spec).await.expect("boot");
    host.submit_handoff(&spec.handoff).await.expect("submit");
    let o = host.wait_outcome(Duration::from_secs(5)).await.expect("wait");
    if o.ok {
        metrics.inc_handoff_ok(o.witnesses.len() as u64);
    } else {
        metrics.inc_handoff_reject(o.witnesses.len() as u64);
    }

    let h = host.health().await;
    let mut gov = host.governor_snap.clone();
    if let Ok(g) = host.last.lock() {
        if let Some(ref out) = *g {
            gov.level = out.governor_level.clone();
            gov.score = out.governor_score;
            gov.decision = if out.ok {
                "allow".into()
            } else {
                "reject".into()
            };
        }
    }

    let server = SurfaceServer::start_full(
        "127.0.0.1:0".parse().unwrap(),
        h,
        gov,
        vec!["agent-cell".into()],
        host.runs.clone(),
        Some(metrics.clone()),
        plugin_ids,
    )
    .await
    .expect("server");
    let addr = server.addr();
    tokio::time::sleep(Duration::from_millis(40)).await;

    let (st, body) = fetch_health(&format!("{addr}/api/v1/surface"))
        .await
        .expect("surface");
    assert_eq!(st, 200, "{body}");
    assert!(body.contains("observability"), "{body}");
    assert!(body.contains("lenses"), "{body}");
    assert!(body.contains("degraded"), "{body}");
    assert!(body.contains("last_heartbeat_ms"), "{body}");
    assert!(body.contains("isa_policy"), "{body}");

    let (st, body) = fetch_health(&format!("{addr}/metrics"))
        .await
        .expect("prom");
    assert_eq!(st, 200, "{body}");
    assert!(body.contains("ule_handoffs_submitted"), "{body}");
    assert!(body.contains("ule_witnesses_emitted"), "{body}");

    let (st, body) = fetch_health(&format!("{addr}/api/v1/lens/ule.runs"))
        .await
        .expect("lens");
    assert_eq!(st, 200, "{body}");
    assert!(body.contains("ule.runs") || body.contains("runs"), "{body}");

    let (st, body) = fetch_health(&format!("{addr}/api/v1/plugins"))
        .await
        .expect("plugins");
    assert_eq!(st, 200, "{body}");
    assert!(body.contains("builtin.echo") || body.contains("hot_reload"), "{body}");

    server.stop().await;
    host.stop().await;
}
