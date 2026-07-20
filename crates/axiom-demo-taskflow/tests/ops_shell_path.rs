//! Ops shell is embedded and served without Node.

use axiom_demo_taskflow::alert_bridge::new_alert_log;
use axiom_demo_taskflow::events::new_event_bus;
use axiom_demo_taskflow::health::http_exchange;
use axiom_demo_taskflow::metrics::new_metrics;
use axiom_demo_taskflow::product_gateway::{boot_write_runtime, GatewayConfig, ProductGateway};
use axiom_demo_taskflow::run_log::new_run_log;
use axiom_demo_taskflow::surface::GovernorSnapshot;
use axiom_isa::GovernorConfig;
use std::time::Duration;

#[tokio::test]
async fn ops_html_served_and_references_surface_api() {
    let metrics = new_metrics();
    let events = new_event_bus();
    let alerts = new_alert_log();
    let runs = new_run_log();
    let write = boot_write_runtime(
        metrics.clone(),
        events.clone(),
        alerts.clone(),
        runs.clone(),
        GovernorConfig::default(),
    )
    .await
    .unwrap();
    let health = write.lock().await.host.health().await;
    let server = ProductGateway::start(GatewayConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        health,
        gov: GovernorSnapshot {
            level: "Green".into(),
            score: 0.0,
            decision: "allow".into(),
        },
        cells: vec!["task-cell".into()],
        runs,
        metrics,
        plugins: vec![],
        events,
        alerts,
        write: Some(write),
    })
    .await
    .unwrap();
    let addr = server.addr();
    tokio::time::sleep(Duration::from_millis(30)).await;

    let (st, html) = http_exchange("GET", &format!("{addr}/ops"), None)
        .await
        .unwrap();
    assert_eq!(st, 200, "{html}");
    assert!(html.contains("ULE Ops Shell"), "{html}");
    assert!(html.contains("/api/v1/surface"), "{html}");
    assert!(html.contains("/api/v1/tasks"), "{html}");
    assert!(html.contains("EventSource"), "{html}");
    assert!(
        !html.contains("require("),
        "browser shell must not use node require"
    );

    let (st, body) = http_exchange("GET", &format!("{addr}/api/v1/alerts"), None)
        .await
        .unwrap();
    assert_eq!(st, 200, "{body}");
    assert!(body.contains("alerts"), "{body}");

    server.stop().await;
}

#[test]
fn ops_html_embedded_in_binary() {
    // Structural: include_str path used by product_gateway
    let html = include_str!("../static/ops.html");
    assert!(html.contains("fetch('/api/v1/surface')") || html.contains("/api/v1/surface"));
}
