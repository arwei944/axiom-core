use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args)]
pub struct NewToolArgs {
    pub name: String,

    #[arg(long)]
    pub permission: Option<String>,

    #[arg(long)]
    pub output_dir: Option<String>,
}

pub fn run_new_tool(args: &NewToolArgs) -> Result<std::process::ExitCode> {
    let output_dir = args.output_dir.as_deref().unwrap_or("src/tools");
    let file_name = format!("{}.rs", snake_case(&args.name));
    let file_path = PathBuf::from(output_dir).join(&file_name);

    if file_path.exists() {
        return Err(anyhow::anyhow!(
            "Tool file '{}' already exists",
            file_path.display()
        ));
    }

    fs::create_dir_all(output_dir)
        .context("Failed to create output directory")?;

    let permission = args.permission.as_deref().unwrap_or("none");

    println!("Creating Tool '{}' (permission: {})...", args.name, permission);

    create_tool_file(&file_path, &args.name, permission)?;
    update_tools_mod(output_dir, &args.name)?;

    println!("\nTool created successfully!");
    println!("File: {}", file_path.display());
    println!("\nSecurity constraints:");
    println!("  - Required permission: {}", permission);
    println!("  - Permission check is automatically enforced by ToolRegistry");

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

fn create_tool_file(file_path: &Path, name: &str, permission: &str) -> Result<()> {
    let snake_name = snake_case(name);
    let description = format!("{} tool for the Axiom runtime", name);
    let has_permission = permission != "none";

    let required_permission_str = if has_permission {
        format!("Some(\"{}\".to_string())", permission)
    } else {
        "None".to_string()
    };

    let mut content = String::new();
    content.push_str(&format!("//! {} - Axiom Tool\n", name));
    content.push_str("//! \n");
    content.push_str("//! # Security Constraints\n");
    content.push_str("//! \n");
    content.push_str(&format!("//! - Required permission: {}\n", permission));
    content.push_str("//! - This tool will be checked for permission before execution\n");
    content.push_str("//! - Every invocation generates a Witness record\n");
    content.push_str("//! \n");
    content.push_str("use serde_json::Value;\n");
    content.push('\n');
    content.push_str("use axiom_tool::Tool;\n");
    content.push_str("use axiom_tool::ToolError;\n");
    content.push_str("use axiom_tool::ToolInfo;\n");
    content.push_str("use axiom_tool::ToolParameter;\n");
    content.push('\n');
    content.push_str(&format!("pub struct {};\n", name));
    content.push('\n');
    content.push_str(&format!("impl {} {{\n", name));
    content.push_str("    /// Create a new instance\n");
    content.push_str("    pub fn new() -> Self {\n");
    content.push_str("        Self\n");
    content.push_str("    }\n");
    content.push_str("}\n");
    content.push('\n');
    content.push_str("#[async_trait::async_trait]\n");
    content.push_str(&format!("impl Tool for {} {{\n", name));
    content.push_str("    fn info(&self) -> ToolInfo {\n");
    content.push_str("        ToolInfo {\n");
    content.push_str(&format!("            name: \"{}\".to_string(),\n", snake_name.replace("_", "-")));
    content.push_str(&format!("            description: \"{}\".to_string(),\n", description));
    content.push_str("            parameters: vec![\n");
    content.push_str("                ToolParameter {\n");
    content.push_str("                    name: \"input\".to_string(),\n");
    content.push_str("                    description: \"Input data for the tool\".to_string(),\n");
    content.push_str("                    required: true,\n");
    content.push_str("                    schema: serde_json::json!({{\"type\": \"object\"}}),\n");
    content.push_str("                },\n");
    content.push_str("            ],\n");
    content.push_str(&format!("            required_permission: {},\n", required_permission_str));
    content.push_str("            version: \"1.0.0\".to_string(),\n");
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push('\n');
    content.push_str("    async fn execute(&self, parameters: &Value) -> Result<Value, ToolError> {\n");
    content.push_str("        let input = parameters\n");
    content.push_str("            .get(\"input\")\n");
    content.push_str("            .ok_or(ToolError::InvalidParameters(\"input is required\".to_string()))?;\n");
    content.push('\n');
    content.push_str("        tracing::info!(\"{} executing with input: {{:?}}\", self.info().name);\n");
    content.push('\n');
    content.push_str("        Ok(serde_json::json!({{\n");
    content.push_str("            \"result\": \"success\",\n");
    content.push_str("            \"input\": input,\n");
    content.push_str("        }))\n");
    content.push_str("    }\n");
    content.push_str("}\n");
    content.push('\n');
    content.push_str("#[cfg(test)]\n");
    content.push_str("mod tests {\n");
    content.push_str("    use super::*;\n");
    content.push_str("    use serde_json::json;\n");
    content.push('\n');
    content.push_str("    #[tokio::test]\n");
    content.push_str("    async fn test_execute() {\n");
    content.push_str(&format!("        let tool = {}::new();\n", name));
    content.push_str("        let result = tool.execute(&json!({{\"input\": \"test\"}})).await;\n");
    content.push_str("        assert!(result.is_ok());\n");
    content.push_str("    }\n");
    content.push('\n');
    content.push_str("    #[tokio::test]\n");
    content.push_str("    async fn test_missing_input() {\n");
    content.push_str(&format!("        let tool = {}::new();\n", name));
    content.push_str("        let result = tool.execute(&json!({{}})).await;\n");
    content.push_str("        assert!(result.is_err());\n");
    content.push_str("    }\n");
    content.push_str("}\n");

    let mut file = File::create(file_path)
        .context("Failed to create tool file")?;
    file.write_all(content.as_bytes())
        .context("Failed to write tool file")?;

    Ok(())
}

fn update_tools_mod(output_dir: &str, name: &str) -> Result<()> {
    let mod_path = PathBuf::from(output_dir).join("mod.rs");
    let snake_name = snake_case(name);

    let content = if mod_path.exists() {
        let mut existing = fs::read_to_string(&mod_path)
            .context("Failed to read mod.rs")?;
        
        if !existing.contains(&format!("pub mod {};", snake_name)) {
            existing.push_str(&format!("pub mod {};\n", snake_name));
        }
        existing
    } else {
        format!("pub mod {};\n", snake_name)
    };

    fs::write(&mod_path, content)
        .context("Failed to write mod.rs")?;

    Ok(())
}
