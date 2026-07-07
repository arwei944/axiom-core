use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Args;

use axiom_kernel::plugin::RuntimeKernelBridge;

#[derive(Args)]
pub struct TraceArgs {
    /// Output as JSON instead of text
    #[arg(long)]
    pub json: bool,

    /// Cell ID to trace
    #[arg(long)]
    pub cell_id: Option<String>,
}

pub fn run_trace(args: &TraceArgs) -> Result<ExitCode> {
    let bridge = RuntimeKernelBridge::new();
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    let snapshot = runtime.block_on(async {
        let cells = bridge.cell_kernel.list().await;
        let heatmap = bridge.heatmap.read().await.snapshot();
        TraceSnapshot {
            cells: cells.len(),
            queued: cells.iter().map(|(_, q)| *q).sum(),
            heatmap,
        }
    });

    if args.json {
        let json = serde_json::to_string_pretty(&snapshot)?;
        println!("{}", json);
        return Ok(ExitCode::SUCCESS);
    }

    println!("=== axiom trace ===");
    if let Some(cell_id) = &args.cell_id {
        println!("Tracing cell: {}", cell_id);
    }
    println!("Cells: {}", snapshot.cells);
    println!("Queued: {}", snapshot.queued);
    println!(
        "Heatmap signals: {}",
        snapshot.heatmap.hot_signals.iter().map(|(_, v)| v).sum::<u64>()
    );

    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, serde::Serialize)]
struct TraceSnapshot {
    cells: usize,
    queued: usize,
    heatmap: axiom_kernel::heatmap::collector::UsageSnapshot,
}
