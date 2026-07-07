use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Args;

use axiom_kernel::plugin::RuntimeKernelBridge;

#[derive(Args)]
pub struct EntropyArgs {
    /// Output as JSON instead of text
    #[arg(long)]
    pub json: bool,
}

pub fn run_entropy(args: &EntropyArgs) -> Result<ExitCode> {
    let bridge = RuntimeKernelBridge::new();
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    let snapshot = runtime.block_on(async {
        let cells = bridge.cell_kernel.list().await;
        let heatmap = bridge.heatmap.read().await.snapshot();
        let plugin_count = bridge.plugin_registry.list_all().await.len();
        EntropySnapshot {
            cells: cells.len(),
            queued: cells.iter().map(|(_, q)| *q).sum(),
            heatmap,
            plugin_count,
        }
    });

    if args.json {
        let json = serde_json::to_string_pretty(&snapshot)?;
        println!("{}", json);
        return Ok(ExitCode::SUCCESS);
    }

    println!("=== axiom entropy ===");
    println!("Cells: {}", snapshot.cells);
    println!("Queued: {}", snapshot.queued);
    println!("Plugins: {}", snapshot.plugin_count);
    println!(
        "Heatmap signals: {}",
        snapshot.heatmap.hot_signals.iter().map(|(_, v)| v).sum::<u64>()
    );

    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, serde::Serialize)]
struct EntropySnapshot {
    cells: usize,
    queued: usize,
    heatmap: axiom_kernel::heatmap::collector::UsageSnapshot,
    plugin_count: usize,
}
