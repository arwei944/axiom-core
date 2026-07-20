//! Strict 初心: commercial write paths use only `product_admit`.
//!
//! - Structural: TaskCell + AgentCell source must call `product_admit` by name.
//! - Path: HTTP allow yields ok + witnesses; forced Governor reject yields 403-class non-ok
//!   without claiming stored success.

use axiom_demo_taskflow::alert_bridge::new_alert_log;
use axiom_demo_taskflow::events::new_event_bus;
use axiom_demo_taskflow::health::http_exchange;
use axiom_demo_taskflow::metrics::new_metrics;
use axiom_demo_taskflow::product_gateway::{
    boot_write_runtime, boot_write_runtime_ex, GatewayConfig, ProductGateway,
};
use axiom_demo_taskflow::run_log::new_run_log;
use axiom_demo_taskflow::runtime_host::{run_commercial, RunRequest};
use axiom_demo_taskflow::surface::GovernorSnapshot;
use axiom_isa::{GovernorConfig, WitnessJournal};
use axiom_kernel::entropy::EntropyLevel;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

fn commercial_cell_sources() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    vec![root.join("task_cell.rs"), root.join("agent_cell.rs")]
}

/// Structural: sole product admit API must appear; bare `.admit(&mut journal)` must not.
#[test]
fn commercial_cells_call_product_admit_by_name() {
    for path in commercial_cell_sources() {
        let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        assert!(
            src.contains("product_admit"),
            "{} must call product_admit by name",
            path.display()
        );
        // Forbid direct admit on product path (product_admit is the only greppable surface).
        let bare = src
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .any(|l| l.contains(".admit(") && !l.contains("product_admit"));
        assert!(
            !bare,
            "{} must not call Governor::admit directly; use product_admit",
            path.display()
        );
        // Must not reference runtime entropy cell as admit.
        assert!(
            !src.contains("EntropyGovernorCell"),
            "{} must not use EntropyGovernorCell for product admit",
            path.display()
        );
    }
}

#[tokio::test]
async fn gateway_allow_requires_product_admit_witness_trail() {
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
        events,
        alerts,
        write: Some(write),
    })
    .await
    .expect("gw");
    let addr = server.addr();
    tokio::time::sleep(Duration::from_millis(40)).await;

    let (st, body) = http_exchange(
        "POST",
        &format!("{addr}/api/v1/tasks"),
        Some(r#"{"title":"admit-ok","priority":1,"payload":"ok"}"#),
    )
    .await
    .expect("post");

    assert!(st == 201 || st == 200, "st={st} body={body}");
    assert!(body.contains("\"ok\":true") || body.contains("\"ok\": true"), "{body}");
    assert!(body.contains("admit_authority"), "{body}");
    assert!(body.contains("witness_count"), "{body}");
    // Response must not claim success without going through governor-tagged path narrative.
    let v: serde_json::Value = serde_json::from_str(body.split('\0').next().unwrap_or(&body))
        .unwrap_or(json!({}));
    let wc = v["witness_count"].as_u64().unwrap_or(0);
    assert!(wc >= 1, "product_admit emits governor witness; count={wc} body={body}");

    server.stop().await;
}

#[tokio::test]
async fn gateway_reject_when_governor_tripped_no_store_success() {
    let metrics = new_metrics();
    let events = new_event_bus();
    let alerts = new_alert_log();
    let runs = new_run_log();
    let write = boot_write_runtime_ex(
        metrics.clone(),
        events.clone(),
        alerts.clone(),
        runs.clone(),
        GovernorConfig::default(),
        true,  // trip_governor → product_admit fails
        false,
    )
    .await
    .expect("boot tripped");

    let health = write.lock().await.host.health().await;
    let server = ProductGateway::start(GatewayConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        health,
        gov: GovernorSnapshot {
            level: "Critical".into(),
            score: 9.0,
            decision: "reject".into(),
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
    .expect("gw");
    let addr = server.addr();
    tokio::time::sleep(Duration::from_millis(40)).await;

    let (st, body) = http_exchange(
        "POST",
        &format!("{addr}/api/v1/tasks"),
        Some(r#"{"title":"admit-deny","priority":1,"payload":"no"}"#),
    )
    .await
    .expect("post");

    assert!(
        st == 403 || st == 422 || (st >= 400 && st < 500),
        "expected client error on governor reject, st={st} body={body}"
    );
    assert!(
        body.contains("\"ok\":false") || body.contains("\"ok\": false"),
        "{body}"
    );
    assert!(
        body.contains("governor") || body.contains("reject") || body.contains("open"),
        "error should be governance-related: {body}"
    );
    // Must not claim stored success
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body.split('\0').next().unwrap_or(&body))
    {
        if let Some(res) = v.get("result") {
            if !res.is_null() {
                assert_ne!(
                    res.get("stored").and_then(|s| s.as_bool()),
                    Some(true),
                    "must not store after admit reject: {body}"
                );
            }
        }
    }

    server.stop().await;
}

#[tokio::test]
async fn runtime_reject_path_uses_product_admit_governor_witness() {
    let outcomes = run_commercial(RunRequest {
        fail: axiom_demo_taskflow::pipeline::FailMode::None,
        trip_governor: true,
        payload: json!({"title": "t", "priority": 1, "payload": "x"}),
        submissions: 1,
        ..Default::default()
    })
    .await
    .expect("run");
    let o = &outcomes[0];
    assert!(!o.ok);
    WitnessJournal::verify_chain(&o.witnesses).expect("chain");
    assert!(
        o.witnesses
            .iter()
            .any(|w| w.summary.contains("governor") && w.summary.contains("reject")),
        "product_admit reject must leave governor reject witness: {:?}",
        o.witnesses.iter().map(|w| &w.summary).collect::<Vec<_>>()
    );
    assert!(o.result.is_none() || !o.result.as_ref().map(|r| r.stored).unwrap_or(false));
}

#[tokio::test]
async fn runtime_allow_records_governor_admit_before_ports() {
    let outcomes = run_commercial(RunRequest {
        fail: axiom_demo_taskflow::pipeline::FailMode::None,
        payload: json!({"title": "admit-order", "priority": 1, "payload": "y"}),
        submissions: 1,
        governor: GovernorConfig {
            reject_from: EntropyLevel::Critical,
            force_open: false,
        },
        ..Default::default()
    })
    .await
    .expect("run");
    let o = &outcomes[0];
    assert!(o.ok, "{:?}", o.error);
    let summaries: Vec<&str> = o.witnesses.iter().map(|w| w.summary.as_str()).collect();
    let gov_i = summaries
        .iter()
        .position(|s| s.contains("governor") && s.contains("admit"))
        .expect("governor admit witness from product_admit");
    let port_i = summaries.iter().position(|s| s.contains("port:"));
    if let Some(pi) = port_i {
        assert!(
            gov_i < pi,
            "product_admit must precede ports: {summaries:?}"
        );
    }
}
