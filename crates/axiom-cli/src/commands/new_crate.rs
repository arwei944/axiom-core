use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args)]
pub struct NewCrateArgs {
    /// Name of the crate to create (without axiom- prefix)
    pub name: String,

    /// Layer level for the crate (0-7, 0=cli, 7=core)
    #[arg(long, required = true)]
    pub layer: usize,

    /// Minimal template (without tests and examples)
    #[arg(long)]
    pub minimal: bool,
}

pub fn run_new_crate(args: &NewCrateArgs) -> Result<std::process::ExitCode> {
    let crate_name = format!("axiom-{}", args.name);

    if args.layer > 7 {
        return Err(anyhow::anyhow!("Layer must be between 0 and 7"));
    }

    let crate_path = PathBuf::from("crates").join(&crate_name);

    if crate_path.exists() {
        return Err(anyhow::anyhow!(
            "Crate '{}' already exists at {}",
            crate_name,
            crate_path.display()
        ));
    }

    println!("Creating crate '{}' at layer {}...", crate_name, args.layer);

    let allowed_deps = get_allowed_deps_for_layer(args.layer);

    println!("Allowed dependencies: {:?}", allowed_deps);

    create_crate_structure(&crate_path, args.minimal)?;
    create_cargo_toml(&crate_path, &crate_name, args.layer, &allowed_deps)?;
    create_lib_rs(&crate_path, &crate_name)?;
    update_architecture_toml(&crate_name, args.layer)?;

    if !args.minimal {
        create_tests(&crate_path)?;
    }

    println!("\nCrate created successfully!");
    println!("Next steps:");
    println!("  1. Add dependencies to Cargo.toml (only from allowed list)");
    println!("  2. Implement your crate logic");
    println!("  3. Run `cargo check -p {}` to verify", crate_name);

    Ok(std::process::ExitCode::SUCCESS)
}

fn get_allowed_deps_for_layer(layer: usize) -> Vec<String> {
    let mut deps = Vec::new();
    let arch_path = PathBuf::from(".axiom/architecture.toml");
    if let Ok(content) = fs::read_to_string(&arch_path) {
        let mut in_crate_layers = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "[crate-layers]" {
                in_crate_layers = true;
                continue;
            }
            if in_crate_layers && trimmed.starts_with('[') {
                break;
            }
            if in_crate_layers && trimmed.contains('=') {
                let crate_name = trimmed.split('=').next().map(|s| s.trim()).unwrap_or("");
                if !crate_name.is_empty() {
                    if let Some(layer_str) = trimmed.split('=').nth(1) {
                        if let Ok(crate_layer) = layer_str.trim().parse::<usize>() {
                            if crate_layer >= layer && !deps.contains(&crate_name.to_string()) {
                                deps.push(crate_name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    deps
}

fn create_crate_structure(crate_path: &Path, minimal: bool) -> Result<()> {
    fs::create_dir_all(crate_path)
        .context("Failed to create crate directory")?;

    fs::create_dir_all(crate_path.join("src"))
        .context("Failed to create src directory")?;

    if !minimal {
        fs::create_dir_all(crate_path.join("tests"))
            .context("Failed to create tests directory")?;
        fs::create_dir_all(crate_path.join("examples"))
            .context("Failed to create examples directory")?;
    }

    Ok(())
}

fn create_cargo_toml(crate_path: &Path, name: &str, _layer: usize, allowed_deps: &[String]) -> Result<()> {
    let deps_section = allowed_deps
        .iter()
        .filter(|dep| *dep != name)
        .map(|dep| format!("{} = {{ workspace = true }}", dep))
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(
        "[package]\n\
         name = \"{}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         description = \"Axiom {} crate\"\n\
         license = \"MIT\"\n\
         repository = \"https://github.com/arwei944/axiom-core\"\n\
         homepage = \"https://github.com/arwei944/axiom-core\"\n\
         documentation = \"https://docs.rs/{}\"\n\
         readme = \"README.md\"\n\
         keywords = [\"axiom\", \"agent\", \"runtime\"]\n\
         categories = [\"asynchronous\", \"concurrency\", \"network-programming\"]\n\
         \n\
         [lib]\n\
         name = \"{}\"\n\
         path = \"src/lib.rs\"\n\
         \n\
         [dependencies]\n\
         {}\n\
         \n\
         [dev-dependencies]\n\
         tokio = {{ version = \"1.0\", features = [\"full\"] }}\n\
         \n\
         [[test]]\n\
         name = \"integration\"\n\
         path = \"tests/integration.rs\"\n",
        name,
        name.replace("axiom-", ""),
        name,
        name.replace("axiom-", ""),
        deps_section
    );

    let mut file = File::create(crate_path.join("Cargo.toml"))
        .context("Failed to create Cargo.toml")?;
    file.write_all(content.as_bytes())
        .context("Failed to write Cargo.toml")?;

    Ok(())
}

fn create_lib_rs(crate_path: &Path, name: &str) -> Result<()> {
    let module_name = name.replace("axiom-", "");
    let layer = get_layer_for_crate(name);

    let content = format!(
        "//! Axiom {} crate\n\
         //!\n\
         //! This crate provides {} functionality for the Axiom runtime.\n\
         //!\n\
         //! # Architecture Constraints\n\
         //!\n\
         //! This crate is at layer {} and can only depend on crates at layer >= {}.\n\
         \n\
         #![warn(missing_docs)]\n\
         #![warn(clippy::all)]\n\
         #![warn(clippy::pedantic)]\n\
         #![allow(clippy::module_name_repetitions)]\n\
         \n\
         pub mod {};\n\
         \n\
         pub use {}::*;\n",
        module_name,
        module_name,
        layer,
        layer,
        module_name,
        module_name
    );

    let mut file = File::create(crate_path.join("src/lib.rs"))
        .context("Failed to create lib.rs")?;
    file.write_all(content.as_bytes())
        .context("Failed to write lib.rs")?;

    let struct_name = module_name.replace("-", "_").split("-").map(|s| {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        }
    }).collect::<String>();

    let module_content = format!(
        "//! {} module\n\
         //!\n\
         //! Core functionality for the {} crate.\n\
         \n\
         use std::sync::Arc;\n\
         \n\
         use tokio::sync::RwLock;\n\
         \n\
         #[derive(Debug, Default)]\n\
         pub struct {};\n\
         \n\
         impl {} {{\n             /// Create a new instance\n             pub fn new() -> Self {{\n                 Self\n             }}\n         }}\n\
         \n\
         #[cfg(test)]\n         mod tests {{\n             use super::*;\n             \n             #[tokio::test]\n             async fn test_new() {{\n                 let instance = {}::new();\n                 assert!(std::mem::size_of_val(&instance) > 0);\n             }}\n         }}\n",
        module_name,
        module_name,
        struct_name,
        struct_name,
        struct_name
    );

    let mut file = File::create(crate_path.join(format!("src/{}.rs", module_name)))
        .context("Failed to create module.rs")?;
    file.write_all(module_content.as_bytes())
        .context("Failed to write module.rs")?;

    Ok(())
}

fn get_layer_for_crate(name: &str) -> usize {
    let arch_path = PathBuf::from(".axiom/architecture.toml");
    if let Ok(content) = fs::read_to_string(&arch_path) {
        let mut in_crate_layers = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "[crate-layers]" {
                in_crate_layers = true;
                continue;
            }
            if in_crate_layers && trimmed.starts_with('[') {
                break;
            }
            if in_crate_layers && trimmed.starts_with(name) && trimmed.contains('=') {
                if let Some(layer_str) = trimmed.split('=').nth(1) {
                    if let Ok(layer) = layer_str.trim().parse::<usize>() {
                        return layer;
                    }
                }
            }
        }
    }
    4 // default layer
}

fn update_architecture_toml(crate_name: &str, layer: usize) -> Result<()> {
    let arch_path = PathBuf::from(".axiom/architecture.toml");

    let mut content = fs::read_to_string(&arch_path)
        .context("Failed to read architecture.toml")?;

    let insert_line = format!("{} = {}\n", crate_name, layer);

    // Find the [crate-layers] section and add the new crate
    if !content.contains(&format!("{}", crate_name)) {
        content = content.replace(
            "[crate-layers]",
            &format!("[crate-layers]\n{}", insert_line)
        );
    }

    fs::write(&arch_path, content)
        .context("Failed to write architecture.toml")?;

    println!("Updated .axiom/architecture.toml with new crate registration");

    Ok(())
}

fn create_tests(crate_path: &Path) -> Result<()> {
    let content = "use tokio;\n\
                   \n\
                   #[tokio::test]\n\
                   async fn test_integration() {\n\
                       assert_eq!(1 + 1, 2);\n\
                   }\n";

    let mut file = File::create(crate_path.join("tests/integration.rs"))
        .context("Failed to create integration.rs")?;
    file.write_all(content.as_bytes())
        .context("Failed to write integration.rs")?;

    Ok(())
}
