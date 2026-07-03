use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args)]
pub struct TraceArgs {
    /// Output as JSON instead of text
    #[arg(long)]
    pub json: bool,
}

pub fn run_trace(_args: &TraceArgs) -> Result<ExitCode> {
    println!("{}", serde_json::json!({"mode":"trace","entries":[]}));
    Ok(ExitCode::SUCCESS)
}
