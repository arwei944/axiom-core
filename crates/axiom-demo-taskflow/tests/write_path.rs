//! Production-shaped write path: POST /api/v1/tasks → Signal → TaskCell.

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
async fn post_tasks_drives_real_cell_outcome() {
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
    .expect("boot write");

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
        metrics: metrics.clone(),
        plugins: vec![],
        events,
        alerts,
        write: Some(write),
    })
    .await
    .expect("gateway");

    let addr = server.addr();
    tokio::time::sleep(Duration::from_millis(40)).await;

    let body = r#"{"title":"write-path","priority":2,"payload":"path-driving"}"#;
    let (status, resp) = http_exchange(
        "POST",
        &format!("{addr}/api/v1/tasks"),
        Some(body),
    )
    .await
    .expect("post");

    assert!(
        status == 201 || status == 200,
        "status={status} body={resp}"
    );
    assert!(resp.contains("\"ok\":true") || resp.contains("\"ok\": true"), "{resp}");
    assert!(resp.contains("admit_authority"), "{resp}");
    assert!(resp.contains("witness_count"), "{resp}");
    assert!(
        resp.contains("SubmitTask") || resp.contains("TaskCell") || resp.contains("path"),
        "{resp}"
    );

    // Metrics must move on real path
    let snap = metrics.snapshot();
    assert!(
        snap.tasks_submitted >= 1 || snap.tasks_ok >= 1,
        "metrics not updated: {snap:?}"
    );

    server.stop().await;
}
