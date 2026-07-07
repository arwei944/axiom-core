use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Args;

use axiom_kernel::plugin::RuntimeKernelBridge;

#[derive(Args)]
pub struct WhyArgs {
    /// Entity ID to analyze (cell ID, signal ID, or witness ID)
    pub entity_id: String,

    /// Show full causal chain with all details
    #[arg(long)]
    pub full: bool,
}

pub fn run_why(args: &WhyArgs) -> Result<ExitCode> {
    let bridge = RuntimeKernelBridge::new();
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    let data = runtime.block_on(async move {
        let cells = bridge.cell_kernel.list().await;
        let witness_count = bridge.witness_kernel.len().await;
        let _heatmap = bridge.heatmap.read().await.snapshot();
        CausalData {
            entity_id: args.entity_id.clone(),
            entity_type: "Cell".to_string(),
            causes: vec![CausalLink {
                entity_id: "kernel-bus".to_string(),
                entity_type: "Signal".to_string(),
                relationship: "Dispatched by".to_string(),
                timestamp: format!(
                    "{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                ),
                details: Some(format!("{} cells registered", cells.len())),
            }],
            effects: vec![CausalLink {
                entity_id: format!("witness-chain-{}", witness_count),
                entity_type: "Witness".to_string(),
                relationship: "Produced".to_string(),
                timestamp: format!(
                    "{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                ),
                details: Some(format!("{} witnesses recorded", witness_count)),
            }],
        }
    });

    println!("=== axiom why ===");
    println!("Analyzing entity: {}", args.entity_id);
    if args.full {
        println!("Full analysis mode: enabled");
    }

    println!("\n{}", render_causal_chain(&data, args.full));

    Ok(ExitCode::SUCCESS)
}

struct CausalData {
    entity_id: String,
    entity_type: String,
    causes: Vec<CausalLink>,
    effects: Vec<CausalLink>,
}

struct CausalLink {
    entity_id: String,
    entity_type: String,
    relationship: String,
    timestamp: String,
    details: Option<String>,
}

fn render_causal_chain(data: &CausalData, full: bool) -> String {
    let mut output = String::new();

    output.push_str(&format!("Entity: {} (Type: {})\n\n", data.entity_id, data.entity_type));

    output.push_str("Causes (What led to this):\n");
    output.push_str("────────────────────────\n");

    for (i, link) in data.causes.iter().enumerate() {
        output.push_str(&format!(
            "[{}] {} {} {}\n",
            i + 1,
            link.entity_id,
            link.relationship,
            link.entity_type
        ));
        output.push_str(&format!("  Time: {}\n", link.timestamp));
        if full {
            if let Some(details) = &link.details {
                output.push_str(&format!("  Details: {}\n", details));
            }
        }
        output.push('\n');
    }

    output.push_str("Effects (What this caused):\n");
    output.push_str("──────────────────────────\n");

    for (i, link) in data.effects.iter().enumerate() {
        output.push_str(&format!(
            "[{}] {} {} {}\n",
            i + 1,
            link.entity_id,
            link.relationship,
            link.entity_type
        ));
        output.push_str(&format!("  Time: {}\n", link.timestamp));
        if full {
            if let Some(details) = &link.details {
                output.push_str(&format!("  Details: {}\n", details));
            }
        }
        output.push('\n');
    }

    output
}
