//! ULE commercial product CLI — task + handoff + surface + write gateway + plugins.
//!
//! ```text
//! cargo run -p axiom-demo-taskflow -- success
//! cargo run -p axiom-demo-taskflow -- handoff
//! cargo run -p axiom-demo-taskflow -- surface
//! cargo run -p axiom-demo-taskflow -- gateway
//! cargo run -p axiom-demo-taskflow -- plugin
//! ```

use axiom_demo_taskflow::agent_host::{run_handoff, HandoffRequestSpec};
use axiom_demo_taskflow::alert_bridge::new_alert_log;
use axiom_demo_taskflow::events::new_event_bus;
use axiom_demo_taskflow::health::{fetch_health, http_exchange};
use axiom_demo_taskflow::metrics::new_metrics;
use axiom_demo_taskflow::pipeline::FailMode;
use axiom_demo_taskflow::plugin_host::ProductPluginHost;
use axiom_demo_taskflow::product_gateway::{boot_write_runtime, GatewayConfig, ProductGateway};
use axiom_demo_taskflow::run_log::new_run_log;
use axiom_demo_taskflow::runtime_host::{run_commercial, RunRequest, RuntimeHost};
use axiom_demo_taskflow::surface::{GovernorSnapshot, SurfaceServer};
use axiom_demo_taskflow::task_cell::TaskRunOutcome;
use axiom_isa::{GovernorConfig, HandoffRequest, WitnessJournal, WorkbenchLimits};
use axiom_kernel::entropy::EntropyLevel;
use axiom_kernel::witness::Witness;
use clap::{Parser, ValueEnum};
use serde_json::json;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Debug, Clone, ValueEnum)]
enum Scenario {
    Success,
    Fail,
    Melt,
    Health,
    /// U3: Handoff → Workbench closed loop on Agent Cell.
    Handoff,
    /// U3: Governor refuses handoff admit.
    HandoffReject,
    /// U4: unified surface (health + governor + runs + metrics + lenses).
    Surface,
    /// Plugin product path: registry + sandbox + hot-reload.
    Plugin,
    /// Full product gateway: write path + SSE + ops shell.
    Gateway,
}

#[derive(Parser, Debug)]
#[command(
    name = "taskflow",
    about = "ULE commercial: AxiomRuntime · Witness · Governor · Handoff/Workbench · Surface"
)]
struct Cli {
    #[arg(value_enum)]
    scenario: Scenario,
    #[arg(long)]
    verbose: bool,
    #[arg(long, default_value = "127.0.0.1:19092")]
    health_addr: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.scenario {
        Scenario::Success => run_success(cli.verbose).await,
        Scenario::Fail => run_fail(cli.verbose).await,
        Scenario::Melt => run_melt(cli.verbose).await,
        Scenario::Health => run_health(&cli.health_addr).await,
        Scenario::Handoff => run_handoff_ok(cli.verbose).await,
        Scenario::HandoffReject => run_handoff_reject(cli.verbose).await,
        Scenario::Surface => run_surface(&cli.health_addr).await,
        Scenario::Plugin => run_plugin(cli.verbose).await,
        Scenario::Gateway => run_gateway(&cli.health_addr).await,
    }
}

fn print_witnesses(ws: &[Witness], verbose: bool) {
    println!("--- Witness chain ({} records) ---", ws.len());
    for (i, w) in ws.iter().enumerate() {
        let prev = w
            .prev_hash
            .map(|h| hex_short(&h.0))
            .unwrap_or_else(|| "GENESIS".into());
        let hash = hex_short(&w.hash.0);
        let outcome = match &w.outcome {
            axiom_kernel::witness::TransitionOutcome::Success => "ok".into(),
            axiom_kernel::witness::TransitionOutcome::Failed { reason } => {
                format!("FAIL:{reason}")
            }
            axiom_kernel::witness::TransitionOutcome::AxiomViolated {
                axiom_name,
                message,
            } => format!("AXIOM:{axiom_name}:{message}"),
        };
        println!("  [{i:02}] {prev} -> {hash}  {}  ({outcome})", w.summary);
        if verbose {
            println!("         {}", serde_json::to_string(w).unwrap_or_default());
        }
    }
    match WitnessJournal::verify_chain(ws) {
        Ok(()) => println!("--- chain integrity: OK ---"),
        Err(e) => println!("--- chain integrity: BAD ({e}) ---"),
    }
}

fn hex_short(bytes: &[u8; 32]) -> String {
    bytes[..4]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn print_outcome(label: &str, o: &TaskRunOutcome, verbose: bool) {
    println!("\n== {label} ==");
    print_witnesses(&o.witnesses, verbose);
    println!(
        "ok={} circuit={} governor={}({:.2})",
        o.ok, o.circuit, o.governor_level, o.governor_score
    );
    if let Some(ref r) = o.result {
        println!("result: {r:?}");
    }
    if let Some(ref e) = o.error {
        println!("error: {e}");
    }
}

async fn run_success(verbose: bool) {
    println!("=== commercial SUCCESS (AxiomRuntime) ===");
    println!("host=Axiom history=Witness admit=Governor isa=Atom/Port/Adapter/Composer");
    let outcomes = match run_commercial(RunRequest {
        fail: FailMode::None,
        payload: json!({
            "title": "ship-mvp",
            "priority": 2,
            "payload": "wire four primitives"
        }),
        submissions: 1,
        ..Default::default()
    })
    .await
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("runtime error: {e}");
            std::process::exit(1);
        }
    };
    let o = &outcomes[0];
    print_outcome("submit#1", o, verbose);
    if o.ok && o.witnesses.len() >= 5 {
        match WitnessJournal::verify_chain(&o.witnesses) {
            Ok(()) => {
                println!("DEMO OK — runtime Signal→Composer-in-Cell→Witness completed.");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("chain broken: {e}");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("expected successful multi-step Witness chain");
        std::process::exit(1);
    }
}

async fn run_fail(verbose: bool) {
    println!("=== commercial FAIL (retry + circuit on runtime) ===");
    let outcomes = match run_commercial(RunRequest {
        fail: FailMode::ExecuteAlways,
        governor: GovernorConfig {
            reject_from: EntropyLevel::Critical,
            force_open: false,
        },
        payload: json!({
            "title": "flaky-job",
            "priority": 1,
            "payload": "should hit circuit"
        }),
        submissions: 4,
        ..Default::default()
    })
    .await
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("runtime error: {e}");
            std::process::exit(1);
        }
    };

    for (i, o) in outcomes.iter().enumerate() {
        print_outcome(&format!("submit#{}", i + 1), o, verbose);
    }

    let saw = outcomes.iter().any(|o| {
        o.circuit.contains("Open")
            || o.error
                .as_deref()
                .map(|e| e.contains("circuit open") || e.contains("governor rejected"))
                .unwrap_or(false)
    });
    if saw {
        println!("DEMO OK — failure path: circuit and/or Governor (Witness-audited).");
        std::process::exit(0);
    }
    eprintln!("expected circuit-open or governor reject");
    std::process::exit(1);
}

async fn run_melt(verbose: bool) {
    println!("=== commercial MELT (Governor refuse admit) ===");
    let outcomes = match run_commercial(RunRequest {
        fail: FailMode::None,
        preload_entropy: true,
        payload: json!({
            "title": "should-reject",
            "priority": 1,
            "payload": "governor closed the gate"
        }),
        submissions: 1,
        ..Default::default()
    })
    .await
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("runtime error: {e}");
            std::process::exit(1);
        }
    };
    let o = &outcomes[0];
    print_outcome("submit#1", o, verbose);
    let rejected = !o.ok
        && o.error
            .as_deref()
            .map(|e| e.contains("governor") || e.contains("rejected") || e.contains("entropy"))
            .unwrap_or(false);
    if rejected {
        println!("DEMO OK — Governor is the sole admit authority on runtime path.");
        std::process::exit(0);
    }
    eprintln!("expected governor reject, got ok={}", o.ok);
    std::process::exit(1);
}

async fn run_handoff_ok(verbose: bool) {
    println!("=== U3 HANDOFF (AgentHandoff → Workbench → Witness) ===");
    println!("protocol=HandoffRequest signal=AgentHandoff history=Witness admit=Governor");
    let o = match run_handoff(HandoffRequestSpec::default()).await {
        Ok(o) => o,
        Err(e) => {
            eprintln!("handoff error: {e}");
            std::process::exit(1);
        }
    };
    print_witnesses(&o.witnesses, verbose);
    println!(
        "ok={} governor={}({:.2}) result={:?}",
        o.ok, o.governor_level, o.governor_score, o.result
    );
    if let Some(ref e) = o.error {
        println!("error: {e}");
    }
    if o.ok && o.witnesses.len() >= 4 {
        if WitnessJournal::verify_chain(&o.witnesses).is_ok() {
            println!("DEMO OK — Handoff→Workbench closed loop under AxiomRuntime.");
            std::process::exit(0);
        }
    }
    eprintln!("handoff path failed");
    std::process::exit(1);
}

async fn run_handoff_reject(verbose: bool) {
    println!("=== U3 HANDOFF-REJECT (Governor refuse) ===");
    let o = match run_handoff(HandoffRequestSpec {
        preload_entropy: true,
        handoff: HandoffRequest::new("tok-x", "a", "b", "echo", "blocked"),
        ..Default::default()
    })
    .await
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    print_witnesses(&o.witnesses, verbose);
    let rejected = !o.ok
        && o.error
            .as_deref()
            .map(|e| e.contains("governor") || e.contains("rejected"))
            .unwrap_or(false);
    if rejected {
        println!("DEMO OK — Handoff admit refused by sole Governor.");
        std::process::exit(0);
    }
    eprintln!("expected governor reject on handoff");
    std::process::exit(1);
}

async fn run_health(addr: &str) {
    println!("=== commercial HEALTH ===");
    let bind: SocketAddr = addr.parse().unwrap_or_else(|_| {
        eprintln!("bad --health-addr");
        std::process::exit(1);
    });
    let host = match RuntimeHost::boot(&RunRequest {
        fail: FailMode::None,
        submissions: 0,
        ..Default::default()
    })
    .await
    {
        Ok(h) => h,
        Err(e) => {
            eprintln!("boot: {e}");
            std::process::exit(1);
        }
    };
    let h = host.health().await;
    let runs = axiom_demo_taskflow::run_log::new_run_log();
    let gov = axiom_demo_taskflow::surface::GovernorSnapshot {
        level: "Green".into(),
        score: 0.0,
        decision: "allow".into(),
    };
    let server = match SurfaceServer::start(
        bind,
        h,
        gov,
        vec!["task-cell".into()],
        runs,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let local = server.addr();
    tokio::time::sleep(Duration::from_millis(50)).await;
    match fetch_health(&format!("{local}/health")).await {
        Ok((status, body)) => {
            println!("HTTP {status}");
            println!("{body}");
            server.stop().await;
            host.stop().await;
            if status == 200 && body.contains("\"status\":\"ok\"") {
                println!("HEALTH OK");
                std::process::exit(0);
            }
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{e}");
            server.stop().await;
            host.stop().await;
            std::process::exit(1);
        }
    }
}

async fn run_surface(addr: &str) {
    // Surface CLI now uses ProductGateway (same read routes + ops).
    run_gateway(addr).await
}

async fn run_gateway(addr: &str) {
    println!("=== PRODUCT GATEWAY (write + SSE + ops + surface) ===");
    let bind: SocketAddr = addr.parse().unwrap_or_else(|_| {
        eprintln!("bad addr");
        std::process::exit(1);
    });

    let metrics = new_metrics();
    let events = new_event_bus();
    let alerts = new_alert_log();
    let runs = new_run_log();
    let plugins = ProductPluginHost::new();
    let _ = plugins.boot_defaults().await;
    let plugin_ids = plugins.plugin_ids().await;

    let write = match boot_write_runtime(
        metrics.clone(),
        events.clone(),
        alerts.clone(),
        runs.clone(),
        GovernorConfig::default(),
    )
    .await
    {
        Ok(w) => w,
        Err(e) => {
            eprintln!("write boot: {e}");
            std::process::exit(1);
        }
    };
    let health = write.lock().await.host.health().await;
    let server = match ProductGateway::start(GatewayConfig {
        bind,
        health,
        gov: GovernorSnapshot {
            level: "Green".into(),
            score: 0.0,
            decision: "allow".into(),
        },
        cells: vec!["task-cell".into()],
        runs,
        metrics: metrics.clone(),
        plugins: plugin_ids,
        events,
        alerts,
        write: Some(write),
    })
    .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let local = server.addr();
    println!("ops      http://{local}/ops");
    println!("surface  http://{local}/api/v1/surface");
    println!("write    POST http://{local}/api/v1/tasks");
    println!("events   GET  http://{local}/api/v1/events (SSE)");
    println!("metrics  http://{local}/metrics");

    tokio::time::sleep(Duration::from_millis(50)).await;
    let surface_ok = match fetch_health(&format!("{local}/api/v1/surface")).await {
        Ok((st, body)) => {
            println!("SURFACE {st}");
            st == 200 && body.contains("write_api") && body.contains("admit_authority")
        }
        Err(e) => {
            eprintln!("{e}");
            false
        }
    };
    let write_ok = match http_exchange(
        "POST",
        &format!("{local}/api/v1/tasks"),
        Some(r#"{"title":"cli-gateway","priority":1,"payload":"demo"}"#),
    )
    .await
    {
        Ok((st, body)) => {
            println!("WRITE {st} {body}");
            (st == 201 || st == 200) && body.contains("witness_count")
        }
        Err(e) => {
            eprintln!("write: {e}");
            false
        }
    };
    let ops_ok = match fetch_health(&format!("{local}/ops")).await {
        Ok((st, body)) => st == 200 && body.contains("ULE Ops Shell"),
        Err(_) => false,
    };
    let metrics_ok = match fetch_health(&format!("{local}/metrics")).await {
        Ok((st, body)) => st == 200 && body.contains("ule_tasks"),
        Err(_) => false,
    };

    server.stop().await;
    if surface_ok && write_ok && ops_ok && metrics_ok {
        println!("GATEWAY OK — surface + write path + ops shell + metrics.");
        std::process::exit(0);
    }
    eprintln!(
        "gateway incomplete surface={surface_ok} write={write_ok} ops={ops_ok} metrics={metrics_ok}"
    );
    std::process::exit(1);
}

async fn run_plugin(verbose: bool) {
    println!("=== PLUGIN product path (registry + sandbox + hot-reload) ===");
    let host = ProductPluginHost::new();
    if let Err(e) = host.boot_defaults().await {
        eprintln!("boot: {e}");
        std::process::exit(1);
    }
    let out = match host.invoke_echo("ule-commercial").await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("invoke: {e}");
            std::process::exit(1);
        }
    };
    if verbose {
        println!("echo reply: {out}");
    }
    if let Err(e) = host.hot_reload_echo().await {
        eprintln!("hot-reload: {e}");
        std::process::exit(1);
    }
    let out2 = match host.invoke_echo("after-reload").await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("invoke after reload: {e}");
            std::process::exit(1);
        }
    };
    let ids = host.plugin_ids().await;
    println!("plugins={ids:?}");
    println!("sandbox_denies_shell={}", axiom_demo_taskflow::plugin_host::sandbox_denies_shell());

    // Workbench plugin_echo intent uses sandbox Port path with plugin_id limit.
    let mut lim = WorkbenchLimits::commercial_default();
    lim.plugin_id = Some("builtin.echo".into());
    println!(
        "workbench limits: max_steps={} mem_mb={} plugin={:?}",
        lim.max_steps, lim.memory_limit_mb, lim.plugin_id
    );

    if out.contains("ule-commercial")
        && out2.contains("after-reload")
        && ids.iter().any(|id| id == "builtin.echo")
    {
        println!("PLUGIN OK — register + sandbox invoke + hot-reload.");
        std::process::exit(0);
    }
    eprintln!("plugin path failed out={out:?} out2={out2:?}");
    std::process::exit(1);
}
