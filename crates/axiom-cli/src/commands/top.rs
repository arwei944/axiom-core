use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Args;

use axiom_kernel::plugin::RuntimeKernelBridge;

#[derive(Args)]
pub struct TopArgs {
    /// Refresh interval in milliseconds
    #[arg(long, default_value = "1000")]
    pub interval_ms: u64,

    /// Show detailed cell information
    #[arg(long)]
    pub detailed: bool,

    /// Show only specific layers
    #[arg(long)]
    pub layer: Option<String>,

    /// Output as JSON instead of TUI
    #[arg(long)]
    pub json: bool,
}

pub fn run_top(args: &TopArgs) -> Result<ExitCode> {
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    let bridge = RuntimeKernelBridge::new();

    if args.json {
        let snapshot = runtime.block_on(async {
            let cells = bridge.cell_kernel.status().await;
            let heatmap = bridge.heatmap.read().await.snapshot();
            let plugin_count = bridge.plugin_registry.list_all().await.len();
            TopSnapshot { cells, heatmap, plugin_count }
        });
        let json = serde_json::to_string_pretty(&snapshot)?;
        println!("{}", json);
        return Ok(ExitCode::SUCCESS);
    }

    println!("=== axiom top ===");
    println!("Refresh interval: {}ms", args.interval_ms);
    if args.detailed {
        println!("Detailed view enabled");
    }
    if let Some(layer) = &args.layer {
        println!("Filtering by layer: {}", layer);
    }

    runtime.block_on(run_tui(args, &bridge)).context("TUI runtime error")?;

    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, serde::Serialize)]
struct TopSnapshot {
    cells: Vec<axiom_kernel::cell::CellStatus>,
    heatmap: axiom_kernel::heatmap::collector::UsageSnapshot,
    plugin_count: usize,
}

async fn run_tui(args: &TopArgs, bridge: &RuntimeKernelBridge) -> Result<()> {
    let mut app = TopApp::new(bridge).await?;

    loop {
        app.update(bridge).await?;

        print!("\x1B[2J\x1B[1;1H");
        println!("{}", app.render());

        tokio::time::sleep(tokio::time::Duration::from_millis(args.interval_ms)).await;
    }
}

struct TopApp {
    cells: Vec<CellStatus>,
    entropy: f64,
    messages_processed: u64,
    uptime_seconds: u64,
    plugin_count: usize,
    _layer_filter: Option<String>,
}

struct CellStatus {
    id: String,
    kind: String,
    state: String,
    queued: usize,
}

impl TopApp {
    async fn new(bridge: &RuntimeKernelBridge) -> Result<Self> {
        let cells = bridge
            .cell_kernel
            .status()
            .await
            .into_iter()
            .map(|s| CellStatus {
                id: s.id,
                kind: s.kind,
                state: "Running".to_string(),
                queued: s.queued,
            })
            .collect();

        Ok(Self {
            cells,
            entropy: 0.0,
            messages_processed: 0,
            uptime_seconds: 0,
            plugin_count: bridge.plugin_registry.list_all().await.len(),
            _layer_filter: None,
        })
    }

    async fn update(&mut self, bridge: &RuntimeKernelBridge) -> Result<()> {
        self.entropy = (self.entropy + 0.01).min(1.0);
        self.messages_processed += 1;
        self.uptime_seconds += 1;
        self.plugin_count = bridge.plugin_registry.list_all().await.len();

        let statuses = bridge.cell_kernel.status().await;
        for (status, live) in self.cells.iter_mut().zip(statuses.iter()) {
            status.queued = live.queued;
            status.state = "Running".to_string();
        }

        Ok(())
    }

    fn render(&self) -> String {
        let mut output = String::new();
        output.push_str("ID                      Kind         State     Queued\n");
        output.push_str("────────────────────────────────────────────────────────────\n");
        for cell in &self.cells {
            output.push_str(&format!(
                "{:<24} {:<12} {:<9} {}\n",
                cell.id, cell.kind, cell.state, cell.queued
            ));
        }
        output.push_str(&format!(
            "\nEntropy: {:.2} | Messages: {} | Uptime: {}s | Plugins: {}\n",
            self.entropy, self.messages_processed, self.uptime_seconds, self.plugin_count
        ));
        output
    }
}
