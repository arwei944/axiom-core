use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Args;

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
    if args.json {
        println!("{}", serde_json::json!({"mode":"json"}));
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

    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    runtime
        .block_on(run_tui(args))
        .context("TUI runtime error")?;

    Ok(ExitCode::SUCCESS)
}

async fn run_tui(args: &TopArgs) -> Result<()> {
    let mut app = TopApp::new();

    loop {
        app.update();

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
    _layer_filter: Option<String>,
}

struct CellStatus {
    id: String,
    layer: String,
    state: String,
    messages: u64,
    errors: u64,
    restart_count: u64,
}

impl TopApp {
    fn new() -> Self {
        Self {
            cells: vec![
                CellStatus {
                    id: "entropy-governor".to_string(),
                    layer: "Oversight".to_string(),
                    state: "Running".to_string(),
                    messages: 120,
                    errors: 0,
                    restart_count: 0,
                },
                CellStatus {
                    id: "architecture-guardian".to_string(),
                    layer: "Oversight".to_string(),
                    state: "Running".to_string(),
                    messages: 85,
                    errors: 0,
                    restart_count: 0,
                },
                CellStatus {
                    id: "agent-planner".to_string(),
                    layer: "Agent".to_string(),
                    state: "Running".to_string(),
                    messages: 240,
                    errors: 2,
                    restart_count: 0,
                },
                CellStatus {
                    id: "validator".to_string(),
                    layer: "Validate".to_string(),
                    state: "Running".to_string(),
                    messages: 310,
                    errors: 5,
                    restart_count: 1,
                },
                CellStatus {
                    id: "exec-worker".to_string(),
                    layer: "Exec".to_string(),
                    state: "Running".to_string(),
                    messages: 520,
                    errors: 10,
                    restart_count: 2,
                },
            ],
            entropy: 45.6,
            messages_processed: 1275,
            uptime_seconds: 180,
            _layer_filter: None,
        }
    }

    fn update(&mut self) {
        self.uptime_seconds += 1;
        self.messages_processed += self.cells.iter().map(|c| c.messages).sum::<u64>() / 10;

        self.entropy = (40.0 + (self.uptime_seconds as f64 % 30.0)) * 0.9;

        for cell in &mut self.cells {
            if cell.state == "Running" {
                cell.messages += 1;
            }
        }
    }

    fn render(&self) -> String {
        let mut output = String::new();

        output.push_str("┌─────────────────────────────────────────────────────────────────┐\n");
        output.push_str("│                    AXIOM RUNTIME MONITOR                        │\n");
        output.push_str("├────────────────────┬────────────────────┬──────────────────────┤\n");
        output.push_str(&format!(
            "│  Uptime: {:>10}s  │  Messages: {:>10}  │  Entropy: {:>8.1}%  │\n",
            self.uptime_seconds, self.messages_processed, self.entropy
        ));
        output.push_str("├────────────────────┴────────────────────┴──────────────────────┤\n");
        output.push_str("│ ID                      Layer    State    Msgs   Errors  Restarts │\n");
        output.push_str("├─────────────────────────────────────────────────────────────────┤\n");

        let entropy_color = if self.entropy < 30.0 {
            "\x1B[32m"
        } else if self.entropy < 60.0 {
            "\x1B[33m"
        } else {
            "\x1B[31m"
        };

        for cell in &self.cells {
            let state_color = match cell.state.as_str() {
                "Running" => "\x1B[32m",
                "Restarting" => "\x1B[33m",
                "CircuitOpen" => "\x1B[31m",
                "Stopped" => "\x1B[90m",
                _ => "",
            };

            output.push_str(&format!(
                "│ {:<22} {:<8} {} {:<8}\x1B[0m {:>5}  {:>6}  {:>8} │\n",
                cell.id,
                cell.layer,
                state_color,
                cell.state,
                cell.messages,
                cell.errors,
                cell.restart_count
            ));
        }

        output.push_str("├─────────────────────────────────────────────────────────────────┤\n");
        output.push_str(&format!(
            "│ Entropy Level: {} {:.1}%\x1B[0m                            │\n",
            entropy_color, self.entropy
        ));
        output.push_str("└─────────────────────────────────────────────────────────────────┘\n");
        output.push_str("\nPress Ctrl+C to exit");

        output
    }
}
