use std::process::ExitCode;

use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct EntropyArgs {
    /// Output as JSON instead of text
    #[arg(long)]
    pub json: bool,
}

pub fn run_entropy(_args: &EntropyArgs) -> Result<ExitCode> {
    println!(
        "{}",
        serde_json::json!({"mode":"entropy","system_entropy":0.0,"cell_entropies":[]})
    );
    Ok(ExitCode::SUCCESS)
}
