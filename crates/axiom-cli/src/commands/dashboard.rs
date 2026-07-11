use std::process::ExitCode;

use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct DashboardArgs {
    /// Listen address for dashboard server
    #[arg(long, default_value = "0.0.0.0:9091")]
    pub addr: String,

    /// Open browser automatically
    #[arg(long)]
    pub open: bool,
}

pub fn run_dashboard(args: &DashboardArgs) -> Result<ExitCode> {
    println!("=== axiom dashboard ===");
    println!("Dashboard endpoints are served by the runtime when started with metrics enabled.");
    println!("Expected endpoints:");
    println!("  http://{}/dashboard/health", args.addr);
    println!("  http://{}/dashboard/cells", args.addr);
    println!("  http://{}/dashboard/heatmap", args.addr);
    println!("  ws://{}/dashboard/ws", args.addr);
    if args.open {
        println!("Open {} in a browser to view the dashboard.", args.addr);
    }
    Ok(ExitCode::SUCCESS)
}
