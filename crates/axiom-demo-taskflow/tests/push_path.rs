//! SSE push path: subscribe then POST task → receive task.completed.

use axiom_demo_taskflow::alert_bridge::new_alert_log;
use axiom_demo_taskflow::events::{new_event_bus, EVENT_TASK_COMPLETED};
use axiom_demo_taskflow::health::http_exchange;
use axiom_demo_taskflow::metrics::new_metrics;
use axiom_demo_taskflow::product_gateway::{boot_write_runtime, GatewayConfig, ProductGateway};
use axiom_demo_taskflow::run_log::new_run_log;
use axiom_demo_taskflow::surface::GovernorSnapshot;
use axiom_isa::GovernorConfig;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn sse_receives_task_completed_after_write() {
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
    .expect("boot");

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
        events: events.clone(),
        alerts,
        write: Some(write),
    })
    .await
    .expect("gw");
    let addr = server.addr();
    tokio::time::sleep(Duration::from_millis(30)).await;

    // Connect SSE
    let mut sock = tokio::net::TcpStream::connect(addr).await.expect("sse connect");
    let req = format!(
        "GET /api/v1/events HTTP/1.1\r\nHost: {addr}\r\nAccept: text/event-stream\r\n\r\n"
    );
    sock.write_all(req.as_bytes()).await.unwrap();

    // Fire write on another task
    let post_addr = addr;
    let post = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(80)).await;
        http_exchange(
            "POST",
            &format!("{post_addr}/api/v1/tasks"),
            Some(r#"{"title":"sse-task","priority":1,"payload":"push"}"#),
        )
        .await
    });

    let mut reader = BufReader::new(sock);
    let mut line = String::new();
    let mut got_task_event = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    while tokio::time::Instant::now() < deadline {
        line.clear();
        let n = tokio::time::timeout(Duration::from_millis(500), reader.read_line(&mut line))
            .await;
        match n {
            Ok(Ok(0)) => break,
            Ok(Ok(_)) => {
                if line.contains(EVENT_TASK_COMPLETED) || line.contains("task.completed") {
                    got_task_event = true;
                    break;
                }
                if line.contains("stream.open") {
                    continue;
                }
            }
            _ => continue,
        }
    }

    let post_res = post.await.expect("join").expect("post");
    assert!(
        post_res.0 == 201 || post_res.0 == 200,
        "post failed: {:?}",
        post_res
    );
    assert!(
        got_task_event,
        "expected SSE task.completed after write path"
    );

    server.stop().await;
}

#[test]
fn encode_helpers_are_real_module() {
    let s = axiom_demo_taskflow::events::encode_task_completed_sse(true, "x", "Green", 3);
    assert!(s.contains("task.completed"));
    assert!(s.contains("data:"));
}
