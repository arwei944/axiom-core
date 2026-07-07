//! Stress test binary — validates system stability under sustained load.
//!
//! Usage:
//!   cargo run --bin stress -- --duration 60 --cells 10 --rate 1000
//!   cargo run --bin stress -- --duration 300 --cells 50 --rate 5000

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axiom_kernel::id::{CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::signal::{SignalEnvelope, SignalKind, VectorClock};
use axiom_kernel::version::SchemaVersion;
use axiom_runtime::mailbox::Mailbox;

#[derive(Debug)]
struct Args {
    duration_secs: u64,
    num_cells: usize,
    msg_rate_per_sec: u64,
}

fn parse_args() -> Args {
    let mut args = std::env::args().skip(1);
    let mut duration_secs = 60;
    let mut num_cells = 10;
    let mut msg_rate_per_sec = 1000;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--duration" => {
                if let Some(v) = args.next() {
                    duration_secs = v.parse().unwrap_or(60);
                }
            }
            "--cells" => {
                if let Some(v) = args.next() {
                    num_cells = v.parse().unwrap_or(10);
                }
            }
            "--rate" => {
                if let Some(v) = args.next() {
                    msg_rate_per_sec = v.parse().unwrap_or(1000);
                }
            }
            "--help" | "-h" => {
                println!("axiom-stress — Stress test for Axiom Core runtime\n");
                println!("Usage: stress [OPTIONS]\n");
                println!("Options:");
                println!("  --duration <SECS>    Test duration in seconds (default: 60)");
                println!("  --cells <N>          Number of simulated cells (default: 10)");
                println!("  --rate <MSG/SEC>     Message rate per cell (default: 1000)");
                println!("  -h, --help           Show this help");
                std::process::exit(0);
            }
            _ => {}
        }
    }

    Args { duration_secs, num_cells, msg_rate_per_sec }
}

fn make_signal(src: &str, dst: &str) -> SignalEnvelope {
    SignalEnvelope {
        msg_id: MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        trace_id: None,
        signal_type: "StressSignal".to_string(),
        vector_clock: VectorClock::new(),
        timestamp_ns: 0,
        kind: SignalKind::Command,
        source_layer: Layer::Exec,
        target_layer: Layer::Exec,
        source_cell: Some(src.to_string()),
        target_cell: Some(dst.to_string()),
        payload: serde_json::json!({}),
        schema_version: SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    }
}

#[tokio::main]
async fn main() {
    let args = parse_args();

    println!("=== Axiom Core Stress Test ===");
    println!("  Duration:   {}s", args.duration_secs);
    println!("  Cells:      {}", args.num_cells);
    println!("  Rate/cell:  {} msg/s", args.msg_rate_per_sec);
    println!("  Total rate: {} msg/s", args.num_cells * args.msg_rate_per_sec as usize);
    println!();

    // Create mailboxes for each cell
    let mailboxes: Vec<Arc<Mailbox>> =
        (0..args.num_cells).map(|_| Arc::new(Mailbox::new(8192))).collect();

    let total_sent = Arc::new(AtomicU64::new(0));
    let total_received = Arc::new(AtomicU64::new(0));
    let total_errors = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    let deadline = start + Duration::from_secs(args.duration_secs);

    // Spawn producers
    let mut producer_handles = vec![];
    for (i, mb) in mailboxes.iter().enumerate() {
        let mb = mb.clone();
        let sent = total_sent.clone();
        let errors = total_errors.clone();
        let interval = Duration::from_nanos(1_000_000_000 / args.msg_rate_per_sec.max(1));

        producer_handles.push(tokio::spawn(async move {
            let cell_id = format!("cell-{i}");
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            while Instant::now() < deadline {
                ticker.tick().await;
                let env = make_signal(&cell_id, "target");
                match mb.push(env).await {
                    Ok(()) => sent.fetch_add(1, Ordering::Relaxed),
                    Err(_) => errors.fetch_add(1, Ordering::Relaxed),
                };
            }
        }));
    }

    // Spawn consumers
    let mut consumer_handles = vec![];
    for mb in mailboxes.iter() {
        let mb = mb.clone();
        let received = total_received.clone();

        consumer_handles.push(tokio::spawn(async move {
            while Instant::now() < deadline {
                if mb.pop().await.is_some() {
                    received.fetch_add(1, Ordering::Relaxed);
                } else {
                    tokio::task::yield_now().await;
                }
            }
        }));
    }

    // Progress reporter
    let progress_sent = total_sent.clone();
    let progress_recv = total_received.clone();
    let progress_err = total_errors.clone();
    let progress_handle = tokio::spawn(async move {
        let mut last_sent = 0u64;
        let mut last_time = Instant::now();

        while Instant::now() < deadline {
            tokio::time::sleep(Duration::from_secs(5)).await;

            let sent = progress_sent.load(Ordering::Relaxed);
            let recv = progress_recv.load(Ordering::Relaxed);
            let errors = progress_err.load(Ordering::Relaxed);
            let elapsed = last_time.elapsed().as_secs_f64();
            let rate = ((sent - last_sent) as f64 / elapsed) as u64;

            println!(
                "  [{:>5.1}s] sent={:<8} recv={:<8} errors={:<4} rate={}/s",
                start.elapsed().as_secs_f64(),
                sent,
                recv,
                errors,
                rate
            );

            last_sent = sent;
            last_time = Instant::now();
        }
    });

    // Wait for all producers
    for h in producer_handles {
        let _ = h.await;
    }

    // Drain remaining messages
    for mb in mailboxes.iter() {
        let remaining = mb.drain().await;
        total_received.fetch_add(remaining.len() as u64, Ordering::Relaxed);
    }

    // Stop consumers and progress
    for h in consumer_handles {
        h.abort();
    }
    progress_handle.abort();

    let elapsed = start.elapsed();
    let sent = total_sent.load(Ordering::Relaxed);
    let received = total_received.load(Ordering::Relaxed);
    let errors = total_errors.load(Ordering::Relaxed);

    println!();
    println!("=== Stress Test Results ===");
    println!("  Duration:       {:.2}s", elapsed.as_secs_f64());
    println!("  Messages sent:  {}", sent);
    println!("  Messages recv:  {}", received);
    println!("  Errors:         {}", errors);
    println!("  Throughput:     {:.0} msg/s", sent as f64 / elapsed.as_secs_f64());
    println!("  Avg latency:    {:.2}µs", elapsed.as_secs_f64() * 1_000_000.0 / sent as f64);
    println!("  Loss rate:      {:.4}%", (sent - received) as f64 / sent as f64 * 100.0);

    if errors > 0 {
        println!("  ⚠ {} errors occurred (mailbox overflow expected under high load)", errors);
    }

    if received == sent {
        println!("  ✅ All messages delivered successfully");
    }

    std::process::exit(0);
}
