use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct CellArgs {
    #[command(subcommand)]
    pub command: CellCommands,
}

#[derive(Subcommand)]
pub enum CellCommands {
    /// List all cells
    List(ListArgs),
    /// Restart a cell
    Restart(RestartArgs),
    /// Stop a cell
    Stop(StopArgs),
    /// Show cell status
    Status(StatusArgs),
    /// Start a cell
    Start(StartArgs),
}

#[derive(Args)]
pub struct ListArgs {
    /// Filter by layer
    #[arg(long)]
    pub layer: Option<String>,

    /// Show detailed information
    #[arg(long)]
    pub detailed: bool,
}

#[derive(Args)]
pub struct RestartArgs {
    /// Cell ID to restart
    pub cell_id: String,

    /// Force restart even if cell is not running
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct StopArgs {
    /// Cell ID to stop
    pub cell_id: String,

    /// Force stop even if cell is already stopped
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Cell ID to check
    pub cell_id: String,
}

#[derive(Args)]
pub struct StartArgs {
    /// Cell ID to start
    pub cell_id: String,
}

pub fn run_cell(args: &CellArgs) -> Result<ExitCode> {
    match &args.command {
        CellCommands::List(list_args) => run_list(list_args),
        CellCommands::Restart(restart_args) => run_restart(restart_args),
        CellCommands::Stop(stop_args) => run_stop(stop_args),
        CellCommands::Status(status_args) => run_status(status_args),
        CellCommands::Start(start_args) => run_start(start_args),
    }
}

fn run_list(args: &ListArgs) -> Result<ExitCode> {
    println!("=== axiom cell list ===");
    if let Some(layer) = &args.layer {
        println!("Filtering by layer: {}", layer);
    }

    let cells = fetch_cell_list(args.layer.as_deref()).context("Failed to fetch cell list")?;

    println!("\n{}", render_cell_list(&cells, args.detailed));

    Ok(ExitCode::SUCCESS)
}

fn run_restart(args: &RestartArgs) -> Result<ExitCode> {
    println!("=== axiom cell restart ===");
    println!("Cell ID: {}", args.cell_id);

    let result = restart_cell(&args.cell_id, args.force).context("Failed to restart cell")?;

    println!("\n{}", result);

    Ok(ExitCode::SUCCESS)
}

fn run_stop(args: &StopArgs) -> Result<ExitCode> {
    println!("=== axiom cell stop ===");
    println!("Cell ID: {}", args.cell_id);

    let result = stop_cell(&args.cell_id, args.force).context("Failed to stop cell")?;

    println!("\n{}", result);

    Ok(ExitCode::SUCCESS)
}

fn run_status(args: &StatusArgs) -> Result<ExitCode> {
    println!("=== axiom cell status ===");
    println!("Cell ID: {}", args.cell_id);

    let status = fetch_cell_status(&args.cell_id).context("Failed to fetch cell status")?;

    println!("\n{}", render_cell_status(&status));

    Ok(ExitCode::SUCCESS)
}

fn run_start(args: &StartArgs) -> Result<ExitCode> {
    println!("=== axiom cell start ===");
    println!("Cell ID: {}", args.cell_id);

    let result = start_cell(&args.cell_id).context("Failed to start cell")?;

    println!("\n{}", result);

    Ok(ExitCode::SUCCESS)
}

struct CellInfo {
    id: String,
    layer: String,
    state: String,
    version: String,
    messages_processed: u64,
    errors: u64,
    restart_count: u64,
    uptime_seconds: u64,
}

struct CellStatusData {
    id: String,
    layer: String,
    state: String,
    version: String,
    messages_processed: u64,
    errors: u64,
    restart_count: u64,
    uptime_seconds: u64,
    mailbox_depth: u64,
    last_message_time: Option<String>,
    supervision_strategy: String,
}

fn fetch_cell_list(layer: Option<&str>) -> Result<Vec<CellInfo>> {
    let mut cells = vec![
        CellInfo {
            id: "entropy-governor".to_string(),
            layer: "Oversight".to_string(),
            state: "Running".to_string(),
            version: "0.1.0".to_string(),
            messages_processed: 1200,
            errors: 0,
            restart_count: 0,
            uptime_seconds: 3600,
        },
        CellInfo {
            id: "architecture-guardian".to_string(),
            layer: "Oversight".to_string(),
            state: "Running".to_string(),
            version: "0.1.0".to_string(),
            messages_processed: 850,
            errors: 0,
            restart_count: 0,
            uptime_seconds: 3600,
        },
        CellInfo {
            id: "agent-planner".to_string(),
            layer: "Agent".to_string(),
            state: "Running".to_string(),
            version: "0.1.0".to_string(),
            messages_processed: 2400,
            errors: 2,
            restart_count: 0,
            uptime_seconds: 1800,
        },
        CellInfo {
            id: "validator".to_string(),
            layer: "Validate".to_string(),
            state: "Running".to_string(),
            version: "0.1.0".to_string(),
            messages_processed: 3100,
            errors: 5,
            restart_count: 1,
            uptime_seconds: 1200,
        },
        CellInfo {
            id: "exec-worker".to_string(),
            layer: "Exec".to_string(),
            state: "Running".to_string(),
            version: "0.1.0".to_string(),
            messages_processed: 5200,
            errors: 10,
            restart_count: 2,
            uptime_seconds: 600,
        },
        CellInfo {
            id: "legacy-worker".to_string(),
            layer: "Exec".to_string(),
            state: "Stopped".to_string(),
            version: "0.0.9".to_string(),
            messages_processed: 100,
            errors: 50,
            restart_count: 5,
            uptime_seconds: 0,
        },
    ];

    if let Some(l) = layer {
        cells.retain(|c| c.layer.eq_ignore_ascii_case(l));
    }

    Ok(cells)
}

fn fetch_cell_status(cell_id: &str) -> Result<CellStatusData> {
    Ok(CellStatusData {
        id: cell_id.to_string(),
        layer: "Exec".to_string(),
        state: "Running".to_string(),
        version: "0.1.0".to_string(),
        messages_processed: 5200,
        errors: 10,
        restart_count: 2,
        uptime_seconds: 600,
        mailbox_depth: 5,
        last_message_time: Some("2024-01-15T10:30:00.000Z".to_string()),
        supervision_strategy: "Restart (max_retries: 3)".to_string(),
    })
}

fn restart_cell(cell_id: &str, _force: bool) -> Result<String> {
    Ok(format!(
        "\x1B[32m✓ Cell '{}' restarted successfully\x1B[0m",
        cell_id
    ))
}

fn stop_cell(cell_id: &str, _force: bool) -> Result<String> {
    Ok(format!("\x1B[33m✓ Cell '{}' stopped\x1B[0m", cell_id))
}

fn start_cell(cell_id: &str) -> Result<String> {
    Ok(format!(
        "\x1B[32m✓ Cell '{}' started successfully\x1B[0m",
        cell_id
    ))
}

fn render_cell_list(cells: &[CellInfo], detailed: bool) -> String {
    let mut output = String::new();

    if cells.is_empty() {
        output.push_str("No cells found.\n");
        return output;
    }

    output.push_str("ID                      Layer    State    Msgs    Errors  Restarts\n");
    output.push_str("─────────────────────────────────────────────────────────────────\n");

    for cell in cells {
        let state_color = match cell.state.as_str() {
            "Running" => "\x1B[32m",
            "Restarting" => "\x1B[33m",
            "CircuitOpen" => "\x1B[31m",
            "Stopped" => "\x1B[90m",
            _ => "",
        };

        output.push_str(&format!(
            "{:<22} {:<8} {} {:<8}\x1B[0m {:>6}  {:>6}  {:>8}\n",
            cell.id,
            cell.layer,
            state_color,
            cell.state,
            cell.messages_processed,
            cell.errors,
            cell.restart_count
        ));

        if detailed {
            output.push_str(&format!(
                "  Version: {} | Uptime: {}s\n",
                cell.version, cell.uptime_seconds
            ));
        }
    }

    output
}

fn render_cell_status(status: &CellStatusData) -> String {
    let mut output = String::new();

    let state_color = match status.state.as_str() {
        "Running" => "\x1B[32m",
        "Restarting" => "\x1B[33m",
        "CircuitOpen" => "\x1B[31m",
        "Stopped" => "\x1B[90m",
        _ => "",
    };

    output.push_str(&format!("ID: {}\n", status.id));
    output.push_str(&format!("Layer: {}\n", status.layer));
    output.push_str(&format!("State: {} {}\x1B[0m\n", state_color, status.state));
    output.push_str(&format!("Version: {}\n", status.version));
    output.push_str(&format!(
        "Messages Processed: {}\n",
        status.messages_processed
    ));
    output.push_str(&format!("Errors: {}\n", status.errors));
    output.push_str(&format!("Restarts: {}\n", status.restart_count));
    output.push_str(&format!("Uptime: {} seconds\n", status.uptime_seconds));
    output.push_str(&format!("Mailbox Depth: {}\n", status.mailbox_depth));
    if let Some(time) = &status.last_message_time {
        output.push_str(&format!("Last Message: {}\n", time));
    }
    output.push_str(&format!(
        "Supervision Strategy: {}\n",
        status.supervision_strategy
    ));

    output
}
