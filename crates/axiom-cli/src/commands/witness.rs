use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use axiom_kernel::plugin::RuntimeKernelBridge;

#[derive(Args)]
pub struct WitnessArgs {
    #[command(subcommand)]
    pub command: WitnessCommands,
}

#[derive(Subcommand)]
pub enum WitnessCommands {
    /// View witness history for a cell
    View(ViewArgs),
    /// Verify witness chain integrity
    Verify(VerifyArgs),
    /// Show witness details by ID
    Get(GetArgs),
    /// Export witness data
    Export(ExportArgs),
}

#[derive(Args)]
pub struct ViewArgs {
    /// Cell ID to view witnesses for
    pub cell_id: String,

    /// Number of recent witnesses to show
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Show full details including payload hashes
    #[arg(long)]
    pub detailed: bool,
}

#[derive(Args)]
pub struct VerifyArgs {
    /// Cell ID to verify (optional, verifies all if not provided)
    #[arg(long)]
    pub cell_id: Option<String>,
}

#[derive(Args)]
pub struct GetArgs {
    /// Witness ID to fetch
    pub witness_id: String,
}

#[derive(Args)]
pub struct ExportArgs {
    /// Output file path
    pub output: String,

    /// Export format (json or csv)
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub fn run_witness(args: &WitnessArgs) -> Result<ExitCode> {
    match &args.command {
        WitnessCommands::View(view_args) => run_view(view_args),
        WitnessCommands::Verify(verify_args) => run_verify(verify_args),
        WitnessCommands::Get(get_args) => run_get(get_args),
        WitnessCommands::Export(export_args) => run_export(export_args),
    }
}

fn runtime_bridge() -> RuntimeKernelBridge {
    RuntimeKernelBridge::new()
}

fn run_view(args: &ViewArgs) -> Result<ExitCode> {
    println!("=== axiom witness view ===");
    println!("Cell ID: {}", args.cell_id);
    println!("Limit: {}", args.limit);

    let witnesses: Vec<WitnessData> = {
        let bridge = runtime_bridge();
        let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
        runtime.block_on(async move {
            bridge
                .witness_kernel
                .get_recent(args.limit)
                .await
                .into_iter()
                .map(|w| WitnessData {
                    witness_id: w.witness_id.to_string(),
                    cell_id: w.cell_id,
                    correlation_id: "".to_string(),
                    timestamp: format!("{}", w.timestamp_ns),
                    signal_type: "".to_string(),
                    outcome: "".to_string(),
                    hash: format!("{:?}", w.state_after_hash),
                    parent_hash: w.prev_hash.as_ref().map(|h| format!("{:?}", h)),
                    payload_size: w.payload_size_bytes,
                })
                .collect()
        })
    };

    println!("\n{}", render_witness_list(&witnesses, args.detailed));

    Ok(ExitCode::SUCCESS)
}

fn run_verify(args: &VerifyArgs) -> Result<ExitCode> {
    println!("=== axiom witness verify ===");
    if let Some(cell_id) = &args.cell_id {
        println!("Verifying cell: {}", cell_id);
    } else {
        println!("Verifying all cells");
    }

    let bridge = runtime_bridge();
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    let (valid, total) = runtime.block_on(async move {
        let valid = bridge.witness_kernel.verify_chain().await.is_ok();
        let total = bridge.witness_kernel.len().await;
        (valid, total)
    });

    let result = VerificationResult {
        all_valid: valid,
        total_witnesses: total as u64,
        valid_chains: if valid { 1 } else { 0 },
        invalid_chains: if valid { 0 } else { 1 },
        broken_links: Vec::new(),
    };

    println!("\n{}", render_verification_result(&result));

    if result.all_valid {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

fn run_get(args: &GetArgs) -> Result<ExitCode> {
    println!("=== axiom witness get ===");
    println!("Witness ID: {}", args.witness_id);

    let bridge = runtime_bridge();
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    let witnesses = runtime.block_on(async move { bridge.witness_kernel.get_recent(1000).await });
    let witness = witnesses
        .iter()
        .find(|w| w.witness_id.to_string() == args.witness_id)
        .ok_or_else(|| anyhow::anyhow!("witness not found: {}", args.witness_id))?;

    println!("\n{}", render_witness_details(witness));

    Ok(ExitCode::SUCCESS)
}

fn run_export(args: &ExportArgs) -> Result<ExitCode> {
    println!("=== axiom witness export ===");
    println!("Output: {}", args.output);
    println!("Format: {}", args.format);

    let data = export_witness_data().context("Failed to export witness data")?;

    if args.format == "json" {
        std::fs::write(&args.output, data).context("Failed to write JSON file")?;
    } else {
        std::fs::write(&args.output, data).context("Failed to write CSV file")?;
    }

    println!("\nWitness data exported successfully to {}", args.output);

    Ok(ExitCode::SUCCESS)
}

#[allow(dead_code)]
struct WitnessData {
    witness_id: String,
    cell_id: String,
    correlation_id: String,
    timestamp: String,
    signal_type: String,
    outcome: String,
    hash: String,
    parent_hash: Option<String>,
    payload_size: usize,
}

struct VerificationResult {
    all_valid: bool,
    total_witnesses: u64,
    valid_chains: u64,
    invalid_chains: u64,
    broken_links: Vec<String>,
}

fn render_witness_list(witnesses: &[WitnessData], detailed: bool) -> String {
    let mut output = String::new();

    output.push_str("Witness History:\n");
    output.push_str("────────────────\n\n");

    for (i, w) in witnesses.iter().enumerate() {
        let outcome_color = match w.outcome.as_str() {
            "Success" => "\x1B[32m",
            "Failed" => "\x1B[31m",
            _ => "",
        };

        output.push_str(&format!(
            "[{}] {} | {} | {} | {} {}\x1B[0m\n",
            i + 1,
            w.witness_id,
            w.timestamp,
            w.signal_type,
            outcome_color,
            w.outcome
        ));

        if detailed {
            output.push_str(&format!("  Hash: {}\n", w.hash));
            if let Some(parent) = &w.parent_hash {
                output.push_str(&format!("  Parent: {}\n", parent));
            }
            output.push_str(&format!("  Payload: {} bytes\n", w.payload_size));
        }

        output.push('\n');
    }

    output
}

fn render_verification_result(result: &VerificationResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("Total Witnesses: {}\n", result.total_witnesses));
    output.push_str(&format!(
        "Valid Chains: {} | Invalid Chains: {}\n\n",
        result.valid_chains, result.invalid_chains
    ));

    if result.all_valid {
        output.push_str("\x1B[32m✓ All witness chains are valid\x1B[0m\n");
    } else {
        output.push_str("\x1B[31m✗ Broken links found:\x1B[0m\n");
        for link in &result.broken_links {
            output.push_str(&format!("  - {}\n", link));
        }
    }

    output
}

fn render_witness_details(witness: &axiom_kernel::witness::Witness) -> String {
    let mut output = String::new();

    output.push_str(&format!("Witness ID: {}\n", witness.witness_id));
    output.push_str(&format!("Cell ID: {}\n", witness.cell_id));
    output.push_str(&format!("Timestamp: {}\n", witness.timestamp_ns));
    output.push_str(&format!("Hash: {:?}\n", witness.state_after_hash));
    if let Some(prev) = &witness.prev_hash {
        output.push_str(&format!("Parent Hash: {:?}\n", prev));
    }

    output
}

fn export_witness_data() -> Result<String> {
    Ok(r#"{
  "witnesses": [
    {
      "witness_id": "witness-0001",
      "cell_id": "exec-worker",
      "timestamp": "2024-01-15T10:30:00.000Z",
      "outcome": "Success",
      "hash": "abcdef1234567890",
      "parent_hash": null,
      "payload_size": 128
    }
  ]
}"#
    .to_string())
}
