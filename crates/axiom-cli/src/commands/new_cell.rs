use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args)]
pub struct NewCellArgs {
    pub name: String,

    #[arg(long, required = true)]
    pub layer: String,

    #[arg(long)]
    pub output_dir: Option<String>,
}

pub fn run_new_cell(args: &NewCellArgs) -> Result<std::process::ExitCode> {
    let layer = args.layer.to_lowercase();

    if !["oversight", "agent", "validate", "exec"].contains(&layer.as_str()) {
        return Err(anyhow::anyhow!("Layer must be one of: oversight, agent, validate, exec"));
    }

    let output_dir = args.output_dir.as_deref().unwrap_or("src/cells");
    let file_name = format!("{}.rs", snake_case(&args.name));
    let file_path = PathBuf::from(output_dir).join(&file_name);

    if file_path.exists() {
        return Err(anyhow::anyhow!("Cell file '{}' already exists", file_path.display()));
    }

    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    println!("Creating Cell '{}' at layer '{}'...", args.name, layer);

    create_cell_file(&file_path, &args.name, &layer)?;
    update_cells_mod(output_dir, &args.name)?;

    println!("\nCell created successfully!");
    println!("File: {}", file_path.display());
    println!("\nConstraints automatically applied:");
    println!("  - Layer: {}", layer);
    println!("  - Can only send to: {}", get_allowed_targets(&layer));
    println!("  - Cannot send to: {}", get_forbidden_targets(&layer));

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

fn get_layer_marker(layer: &str) -> &'static str {
    match layer {
        "oversight" => "OversightTier",
        "agent" => "AgentTier",
        "validate" => "ValidateTier",
        "exec" => "ExecTier",
        _ => "ExecTier",
    }
}

fn get_allowed_targets(layer: &str) -> String {
    match layer {
        "oversight" => "oversight, agent, validate, exec".to_string(),
        "agent" => "agent, validate".to_string(),
        "validate" => "agent, validate, exec".to_string(),
        "exec" => "exec".to_string(),
        _ => "exec".to_string(),
    }
}

fn get_forbidden_targets(layer: &str) -> String {
    match layer {
        "oversight" => "none".to_string(),
        "agent" => "oversight, exec".to_string(),
        "validate" => "oversight".to_string(),
        "exec" => "oversight, agent, validate".to_string(),
        _ => "oversight, agent, validate".to_string(),
    }
}

fn create_cell_file(file_path: &Path, name: &str, layer: &str) -> Result<()> {
    let snake_name = snake_case(name);
    let layer_marker = get_layer_marker(layer);
    let allowed = get_allowed_targets(layer);
    let forbidden = get_forbidden_targets(layer);

    let mut content = String::new();
    content.push_str(&format!("//! {} - {} layer Cell\n", name, layer));
    content.push_str("//! \n");
    content.push_str("//! # Architecture Constraints\n");
    content.push_str("//! \n");
    content.push_str(&format!("//! - This Cell can only send signals to: {}\n", allowed));
    content.push_str(&format!("//! - This Cell cannot send signals to: {}\n", forbidden));
    content.push_str("//! \n");
    content.push_str("use axiom_kernel::cell::Cell;\n");
    content.push_str("use axiom_kernel::context::CellContext;\n");
    content.push_str("use axiom_kernel::error::AxiomError;\n");
    content.push_str("use axiom_kernel::id::CellId;\n");
    content.push_str(&format!("use axiom_kernel::layer::{};\n", layer_marker));
    content.push_str("use serde::{{Deserialize, Serialize}};\n");
    content.push_str("use serde_json::Value;\n");
    content.push('\n');
    content.push_str("#[derive(Debug, Default)]\n");
    content.push_str(&format!("pub struct {};\n", name));
    content.push('\n');
    content.push_str(&format!("impl Cell for {} {{\n", name));
    content.push_str(&format!("    type Message = {}Message;\n", name));
    content.push_str(&format!("    type Layer = {};\n", layer_marker));
    content.push('\n');
    content.push_str("    fn id(&self) -> &CellId {\n");
    content.push_str(&format!(
        "        static ID: CellId = CellId::new_static(\"{}\");\n",
        snake_name
    ));
    content.push_str("        &ID\n");
    content.push_str("    }\n");
    content.push('\n');
    content.push_str("    async fn handle(\n");
    content.push_str("        &mut self,\n");
    content.push_str("        msg: Self::Message,\n");
    content.push_str("        ctx: &mut CellContext<'_>,\n");
    content.push_str("    ) -> Result<(), AxiomError> {\n");
    content.push_str(&format!("        tracing::info!(\"{} received: {{:?}}\", msg);\n", name));
    content.push_str("        Ok(())\n");
    content.push_str("    }\n");
    content.push_str("}\n");
    content.push('\n');
    content.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    content.push_str(&format!("pub struct {}Message {{\n", name));
    content.push_str("    /// Message payload\n");
    content.push_str("    pub payload: Value,\n");
    content.push_str("}\n");
    content.push('\n');
    content.push_str(&format!("impl axiom_kernel::signal::Signal for {}Message {{\n", name));
    content.push_str("    fn signal_type(&self) -> &str {\n");
    content.push_str(&format!("        \"{}Message\"\n", name));
    content.push_str("    }\n");
    content.push_str("}\n");

    let mut file = File::create(file_path).context("Failed to create cell file")?;
    file.write_all(content.as_bytes()).context("Failed to write cell file")?;

    Ok(())
}

fn update_cells_mod(output_dir: &str, name: &str) -> Result<()> {
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
