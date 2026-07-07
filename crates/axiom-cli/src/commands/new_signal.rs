use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args)]
pub struct NewSignalArgs {
    /// Name of the Signal to create (e.g., UserRequest)
    pub name: String,

    /// Kind of the Signal (command, event, query, reply)
    #[arg(long, required = true)]
    pub kind: String,

    /// Layer for the Signal (oversight, agent, validate, exec)
    #[arg(long, required = true)]
    pub layer: String,

    /// Output directory (default: src/signals)
    #[arg(long)]
    pub output_dir: Option<String>,
}

pub fn run_new_signal(args: &NewSignalArgs) -> Result<std::process::ExitCode> {
    let kind = args.kind.to_lowercase();
    let layer = args.layer.to_lowercase();

    if !["command", "event", "query", "reply"].contains(&kind.as_str()) {
        return Err(anyhow::anyhow!("Kind must be one of: command, event, query, reply"));
    }

    if !["oversight", "agent", "validate", "exec"].contains(&layer.as_str()) {
        return Err(anyhow::anyhow!("Layer must be one of: oversight, agent, validate, exec"));
    }

    let output_dir = args.output_dir.as_deref().unwrap_or("src/signals");
    let file_name = format!("{}.rs", snake_case(&args.name));
    let file_path = PathBuf::from(output_dir).join(&file_name);

    if file_path.exists() {
        return Err(anyhow::anyhow!("Signal file '{}' already exists", file_path.display()));
    }

    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    println!("Creating Signal '{}' (kind: {}, layer: {})...", args.name, kind, layer);

    create_signal_file(&file_path, &args.name, &kind, &layer)?;
    update_signals_mod(output_dir, &args.name)?;

    println!("\nSignal created successfully!");
    println!("File: {}", file_path.display());

    Ok(std::process::ExitCode::SUCCESS)
}

fn snake_case(name: &str) -> String {
    name.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if c.is_uppercase() && i > 0 {
                vec!['_', c.to_ascii_lowercase()]
            } else {
                vec![c.to_ascii_lowercase()]
            }
        })
        .collect()
}

fn get_valid_targets(layer: &str) -> String {
    match layer {
        "oversight" => "all layers".to_string(),
        "agent" => "agent, validate".to_string(),
        "validate" => "agent, validate, exec".to_string(),
        "exec" => "exec".to_string(),
        _ => "exec".to_string(),
    }
}

fn create_signal_file(file_path: &Path, name: &str, kind: &str, layer: &str) -> Result<()> {
    let content = format!(
        "//! {} - {} signal (layer: {})\n\
         //!\n\
         //! # Architecture Constraints\n\
         //!\n\
         //! - Signal kind: {}\n\
         //! - Source layer: {}\n\
         //! - Valid targets: {}\n\
         //!\n\
         use axiom_kernel::id::CorrelationId;\n\
         use axiom_kernel::id::MsgId;\n\
         use axiom_kernel::signal::Signal;\n\
         use axiom_kernel::signal::SignalKind;\n\
         use axiom_kernel::vector_clock::VectorClock;\n\
         \n\
         #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\n\
         pub struct {} {{\n             /// Message ID\n             pub msg_id: MsgId,\n             /// Correlation ID for tracing\n             pub correlation_id: CorrelationId,\n             /// Vector clock for causality tracking\n             pub vector_clock: VectorClock,\n             /// Signal payload\n             pub payload: serde_json::Value,\n         }}\n\
         \n\
         impl Signal for {} {{\n             fn signal_type(&self) -> &str {{\n                 \"{}\"\n             }}\n             \n\
             fn kind(&self) -> SignalKind {{\n                 SignalKind::{}\n             }}\n         }}\n\
         \n\
         impl axiom_kernel::schema::Schema for {} {{\n             fn validate(&self) -> Result<(), axiom_kernel::error::ValidationError> {{\n                 if self.payload.is_null() {{\n                     return Err(axiom_kernel::error::ValidationError::Invalid(\n                         \"payload cannot be null\".to_string(),\n                     ));\n                 }}\n                 Ok(())\n             }}\n         }}\n",
        name,
        kind,
        layer,
        kind,
        layer,
        get_valid_targets(layer),
        name,
        name,
        name,
        kind.to_uppercase(),
        name
    );

    let mut file = File::create(file_path).context("Failed to create signal file")?;
    file.write_all(content.as_bytes()).context("Failed to write signal file")?;

    Ok(())
}

fn update_signals_mod(output_dir: &str, name: &str) -> Result<()> {
    let mod_path = PathBuf::from(output_dir).join("mod.rs");
    let snake_name = snake_case(name);

    let content = if mod_path.exists() {
        let mut existing = fs::read_to_string(&mod_path).context("Failed to read mod.rs")?;

        if !existing.contains(&format!("pub mod {};", snake_name)) {
            existing.push_str(&format!("pub mod {};\n", snake_name));
        }
        existing
    } else {
        format!("pub mod {};\n", snake_name)
    };

    fs::write(&mod_path, content).context("Failed to write mod.rs")?;

    Ok(())
}
