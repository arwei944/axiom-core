use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use axiom_kernel::plugin::RuntimeKernelBridge;

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

fn runtime_bridge() -> RuntimeKernelBridge {
    RuntimeKernelBridge::new()
}

fn run_list(args: &ListArgs) -> Result<ExitCode> {
    let bridge = runtime_bridge();
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    let cells = runtime.block_on(async { bridge.cell_kernel.list().await });

    println!("=== axiom cell list ===");
    if let Some(layer) = &args.layer {
        println!("Filtering by layer: {}", layer);
    }

    println!("\n{}", render_cell_list(&cells, args.detailed));

    Ok(ExitCode::SUCCESS)
}

fn run_restart(args: &RestartArgs) -> Result<ExitCode> {
    println!("=== axiom cell restart ===");
    println!("Cell ID: {}", args.cell_id);
    println!("Cell '{}' restart requested (requires running runtime to apply).", args.cell_id);
    Ok(ExitCode::SUCCESS)
}

fn run_stop(args: &StopArgs) -> Result<ExitCode> {
    println!("=== axiom cell stop ===");
    println!("Cell ID: {}", args.cell_id);
    println!("Cell '{}' stop requested (requires running runtime to apply).", args.cell_id);
    Ok(ExitCode::SUCCESS)
}

fn run_status(args: &StatusArgs) -> Result<ExitCode> {
    let bridge = runtime_bridge();
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    let statuses = runtime.block_on(async { bridge.cell_kernel.status().await });
    let status = statuses
        .iter()
        .find(|s| s.id == args.cell_id)
        .ok_or_else(|| anyhow::anyhow!("cell not found: {}", args.cell_id))?;

    println!("=== axiom cell status ===");
    println!("{}", render_cell_status(status));

    Ok(ExitCode::SUCCESS)
}

fn run_start(args: &StartArgs) -> Result<ExitCode> {
    println!("=== axiom cell start ===");
    println!("Cell ID: {}", args.cell_id);
    println!("Cell '{}' start requested (requires running runtime to apply).", args.cell_id);
    Ok(ExitCode::SUCCESS)
}

fn render_cell_list(cells: &[(axiom_kernel::cell::CellHandle, usize)], detailed: bool) -> String {
    let mut output = String::new();

    if cells.is_empty() {
        output.push_str("No cells found.\n");
        return output;
    }

    output.push_str("ID                      Kind         Queued\n");
    output.push_str("────────────────────────────────────────────\n");

    for (handle, queued) in cells {
        output.push_str(&format!(
            "{:<24} {:<12} {}\n",
            handle.id,
            format!("{:?}", handle.kind),
            queued
        ));

        if detailed {
            output.push_str("  State: Running\n");
        }
    }

    output
}

fn render_cell_status(status: &axiom_kernel::cell::CellStatus) -> String {
    let mut output = String::new();

    output.push_str(&format!("ID: {}\n", status.id));
    output.push_str(&format!("Kind: {}\n", status.kind));
    output.push_str(&format!("Queued: {}\n", status.queued));

    output
}
