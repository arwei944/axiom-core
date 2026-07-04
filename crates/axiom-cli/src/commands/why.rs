use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args)]
pub struct WhyArgs {
    /// Entity ID to analyze (cell ID, signal ID, or witness ID)
    pub entity_id: String,

    /// Show full causal chain with all details
    #[arg(long)]
    pub full: bool,
}

pub fn run_why(args: &WhyArgs) -> Result<ExitCode> {
    println!("=== axiom why ===");
    println!("Analyzing entity: {}", args.entity_id);
    if args.full {
        println!("Full analysis mode: enabled");
    }

    let causal_data = fetch_causal_data(&args.entity_id).context("Failed to fetch causal data")?;

    println!("\n{}", render_causal_chain(&causal_data, args.full));

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

fn fetch_causal_data(entity_id: &str) -> Result<CausalData> {
    Ok(CausalData {
        entity_id: entity_id.to_string(),
        entity_type: "Cell".to_string(),
        causes: vec![
            CausalLink {
                entity_id: "user-request-123".to_string(),
                entity_type: "Signal".to_string(),
                relationship: "Triggered by".to_string(),
                timestamp: "2024-01-15T10:30:00.000Z".to_string(),
                details: Some("User requested code review".to_string()),
            },
            CausalLink {
                entity_id: "plan-generated-456".to_string(),
                entity_type: "Witness".to_string(),
                relationship: "Depends on".to_string(),
                timestamp: "2024-01-15T10:30:00.120Z".to_string(),
                details: Some("Agent planner generated execution plan".to_string()),
            },
        ],
        effects: vec![
            CausalLink {
                entity_id: "validation-789".to_string(),
                entity_type: "Signal".to_string(),
                relationship: "Triggered".to_string(),
                timestamp: "2024-01-15T10:30:00.340Z".to_string(),
                details: Some("Sent validation request to Validate layer".to_string()),
            },
            CausalLink {
                entity_id: "review-completed-abc".to_string(),
                entity_type: "Witness".to_string(),
                relationship: "Produced".to_string(),
                timestamp: "2024-01-15T10:30:01.150Z".to_string(),
                details: Some("Generated code review with 3 issues".to_string()),
            },
        ],
    })
}

fn render_causal_chain(data: &CausalData, full: bool) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Entity: {} (Type: {})\n\n",
        data.entity_id, data.entity_type
    ));

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
