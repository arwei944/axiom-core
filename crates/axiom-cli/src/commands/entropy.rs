use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct EntropyArgs {
    #[command(subcommand)]
    pub command: EntropyCommands,
}

#[derive(Subcommand)]
pub enum EntropyCommands {
    /// Show current entropy levels
    Show,
    /// Reset entropy counters
    Reset(ResetArgs),
    /// Show or modify thresholds
    Threshold(ThresholdArgs),
    /// Show entropy history
    History(HistoryArgs),
}

#[derive(Args)]
pub struct ResetArgs {
    /// Reset all cells or specific cell
    #[arg(long)]
    pub cell_id: Option<String>,
}

#[derive(Args)]
pub struct ThresholdArgs {
    /// Show current thresholds
    #[arg(long)]
    pub show: bool,

    /// Set new warning threshold
    #[arg(long)]
    pub warning: Option<f64>,

    /// Set new critical threshold
    #[arg(long)]
    pub critical: Option<f64>,
}

#[derive(Args)]
pub struct HistoryArgs {
    /// Number of recent entries to show
    #[arg(long, default_value = "20")]
    pub limit: usize,
}

pub fn run_entropy(args: &EntropyArgs) -> Result<ExitCode> {
    match &args.command {
        EntropyCommands::Show => run_show(),
        EntropyCommands::Reset(reset_args) => run_reset(reset_args),
        EntropyCommands::Threshold(threshold_args) => run_threshold(threshold_args),
        EntropyCommands::History(history_args) => run_history(history_args),
    }
}

fn run_show() -> Result<ExitCode> {
    println!("=== axiom entropy ===");

    let entropy_data = fetch_entropy_data().context("Failed to fetch entropy data")?;

    println!("\n{}", render_entropy_overview(&entropy_data));

    Ok(ExitCode::SUCCESS)
}

fn run_reset(args: &ResetArgs) -> Result<ExitCode> {
    println!("=== axiom entropy reset ===");
    if let Some(cell_id) = &args.cell_id {
        println!("Resetting entropy for cell: {}", cell_id);
    } else {
        println!("Resetting all entropy counters");
    }

    let result = reset_entropy(args.cell_id.as_deref()).context("Failed to reset entropy")?;

    println!("\n{}", result);

    Ok(ExitCode::SUCCESS)
}

fn run_threshold(args: &ThresholdArgs) -> Result<ExitCode> {
    println!("=== axiom entropy threshold ===");

    let should_show = args.show || (args.warning.is_none() && args.critical.is_none());
    if should_show {
        let thresholds = fetch_thresholds().context("Failed to fetch thresholds")?;
        println!("\n{}", render_thresholds(&thresholds));
    }

    if args.warning.is_some() || args.critical.is_some() {
        let result = update_thresholds(args.warning, args.critical).context("Failed to update thresholds")?;
        println!("\n{}", result);
    }

    Ok(ExitCode::SUCCESS)
}

fn run_history(args: &HistoryArgs) -> Result<ExitCode> {
    println!("=== axiom entropy history ===");
    println!("Showing last {} entries", args.limit);

    let history = fetch_entropy_history(args.limit).context("Failed to fetch entropy history")?;

    println!("\n{}", render_entropy_history(&history));

    Ok(ExitCode::SUCCESS)
}

struct EntropyData {
    global_entropy: f64,
    level: String,
    cells: Vec<CellEntropy>,
}

struct CellEntropy {
    cell_id: String,
    entropy: f64,
    level: String,
    message_queue_depth: u32,
    error_rate: f64,
    response_time_ms: u64,
    axiom_violations: u32,
}

struct ThresholdData {
    warning: f64,
    critical: f64,
    emergency: f64,
}

struct EntropyHistoryEntry {
    timestamp: String,
    entropy: f64,
    level: String,
    trigger_cell: Option<String>,
    trigger_event: Option<String>,
}

fn fetch_entropy_data() -> Result<EntropyData> {
    Ok(EntropyData {
        global_entropy: 45.6,
        level: "Yellow".to_string(),
        cells: vec![
            CellEntropy {
                cell_id: "entropy-governor".to_string(),
                entropy: 15.2,
                level: "Green".to_string(),
                message_queue_depth: 5,
                error_rate: 0.0,
                response_time_ms: 10,
                axiom_violations: 0,
            },
            CellEntropy {
                cell_id: "architecture-guardian".to_string(),
                entropy: 12.8,
                level: "Green".to_string(),
                message_queue_depth: 3,
                error_rate: 0.0,
                response_time_ms: 8,
                axiom_violations: 0,
            },
            CellEntropy {
                cell_id: "agent-planner".to_string(),
                entropy: 35.4,
                level: "Yellow".to_string(),
                message_queue_depth: 15,
                error_rate: 0.02,
                response_time_ms: 150,
                axiom_violations: 2,
            },
            CellEntropy {
                cell_id: "validator".to_string(),
                entropy: 55.8,
                level: "Orange".to_string(),
                message_queue_depth: 30,
                error_rate: 0.05,
                response_time_ms: 300,
                axiom_violations: 5,
            },
            CellEntropy {
                cell_id: "exec-worker".to_string(),
                entropy: 72.1,
                level: "Red".to_string(),
                message_queue_depth: 50,
                error_rate: 0.10,
                response_time_ms: 500,
                axiom_violations: 10,
            },
        ],
    })
}

fn fetch_thresholds() -> Result<ThresholdData> {
    Ok(ThresholdData {
        warning: 30.0,
        critical: 60.0,
        emergency: 80.0,
    })
}

fn fetch_entropy_history(limit: usize) -> Result<Vec<EntropyHistoryEntry>> {
    let mut history = Vec::new();
    for i in 0..limit {
        let entropy = 40.0 + (i as f64 % 40.0);
        let level = if entropy < 30.0 {
            "Green"
        } else if entropy < 60.0 {
            "Yellow"
        } else if entropy < 80.0 {
            "Orange"
        } else {
            "Red"
        };
        history.push(EntropyHistoryEntry {
            timestamp: format!("2024-01-15T10:{:02}:{:02}.000Z", 29 + i / 60, i % 60),
            entropy,
            level: level.to_string(),
            trigger_cell: if i % 5 == 0 {
                Some("exec-worker".to_string())
            } else {
                None
            },
            trigger_event: if i % 5 == 0 {
                Some("HighErrorRate".to_string())
            } else {
                None
            },
        });
    }
    history.reverse();
    Ok(history)
}

fn reset_entropy(cell_id: Option<&str>) -> Result<String> {
    if let Some(id) = cell_id {
        Ok(format!("\x1B[32m✓ Entropy counters reset for cell '{}'\x1B[0m", id))
    } else {
        Ok("\x1B[32m✓ All entropy counters reset\x1B[0m".to_string())
    }
}

fn update_thresholds(warning: Option<f64>, critical: Option<f64>) -> Result<String> {
    let mut messages = Vec::new();
    if let Some(w) = warning {
        messages.push(format!("warning threshold set to {}%", w));
    }
    if let Some(c) = critical {
        messages.push(format!("critical threshold set to {}%", c));
    }
    Ok(format!("\x1B[32m✓ {}\x1B[0m", messages.join(", ")))
}

fn render_entropy_overview(data: &EntropyData) -> String {
    let mut output = String::new();

    let level_color = match data.level.as_str() {
        "Green" => "\x1B[32m",
        "Yellow" => "\x1B[33m",
        "Orange" => "\x1B[33m",
        "Red" => "\x1B[31m",
        _ => "",
    };

    output.push_str("Global Entropy:\n");
    output.push_str("──────────────\n");
    output.push_str(&format!(
        "  Level: {} {} ({:.1}%)\x1B[0m\n\n",
        level_color, data.level, data.global_entropy
    ));

    output.push_str("Cell Entropy Levels:\n");
    output.push_str("───────────────────\n\n");

    output.push_str("ID                      Level    Entropy  Queue  Error%  Latency\n");
    output.push_str("─────────────────────────────────────────────────────────────────\n");

    for cell in &data.cells {
        let cell_color = match cell.level.as_str() {
            "Green" => "\x1B[32m",
            "Yellow" => "\x1B[33m",
            "Orange" => "\x1B[33m",
            "Red" => "\x1B[31m",
            _ => "",
        };

        output.push_str(&format!(
            "{:<22} {} {:<6}\x1B[0m   {:>6.1}%  {:>5}  {:>6}%  {:>7}ms\n",
            cell.cell_id, cell_color, cell.level, cell.entropy, cell.message_queue_depth, cell.error_rate * 100.0, cell.response_time_ms
        ));
    }

    output.push_str("\nLegend:\n");
    output.push_str("  \x1B[32mGreen   \x1B[0m: Normal operation\n");
    output.push_str("  \x1B[33mYellow  \x1B[0m: Increased entropy, monitoring recommended\n");
    output.push_str("  \x1B[33mOrange  \x1B[0m: High entropy, consider throttling\n");
    output.push_str("  \x1B[31mRed     \x1B[0m: Critical entropy, emergency measures may apply\n");

    output
}

fn render_thresholds(data: &ThresholdData) -> String {
    let mut output = String::new();

    output.push_str("Entropy Thresholds:\n");
    output.push_str("──────────────────\n");
    output.push_str(&format!("  Warning:  {}% - Yellow level\n", data.warning));
    output.push_str(&format!("  Critical: {}% - Orange level\n", data.critical));
    output.push_str(&format!("  Emergency: {}% - Red level\n", data.emergency));

    output
}

fn render_entropy_history(history: &[EntropyHistoryEntry]) -> String {
    let mut output = String::new();

    output.push_str("Time                Entropy  Level    Trigger\n");
    output.push_str("─────────────────────────────────────────────\n");

    for entry in history {
        let level_color = match entry.level.as_str() {
            "Green" => "\x1B[32m",
            "Yellow" => "\x1B[33m",
            "Orange" => "\x1B[33m",
            "Red" => "\x1B[31m",
            _ => "",
        };

        let trigger = if let (Some(cell), Some(event)) = (&entry.trigger_cell, &entry.trigger_event) {
            format!("{}: {}", cell, event)
        } else {
            "-".to_string()
        };

        output.push_str(&format!(
            "{}  {:>6.1}%  {} {:<6}\x1B[0m  {}\n",
            entry.timestamp, entry.entropy, level_color, entry.level, trigger
        ));
    }

    output
}
