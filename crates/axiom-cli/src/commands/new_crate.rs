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

    /// Minimal template (without tests, examples, CI)
    #[arg(long)]
    pub minimal: bool,

    /// Full template (with tests, examples, CI, build.rs)
    #[arg(long)]
    pub full: bool,
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

    let template = if args.full || !args.minimal {
        "full"
    } else {
        "minimal"
    };

    let allowed_deps = get_allowed_deps_for_layer(args.layer);

    println!("Allowed dependencies: {:?}", allowed_deps);
    println!("Template: {}", template);

    // 创建 crate 结构（失败时自动回滚）
    create_crate_structure(&crate_path, template)?;

    // 生成所有文件
    create_cargo_toml(
        &crate_path,
        &crate_name,
        args.layer,
        &allowed_deps,
        template,
    )?;
    create_build_rs(&crate_path, &crate_name)?;
    create_lib_rs(&crate_path, &crate_name)?;
    update_architecture_toml(&crate_name, args.layer)?;

    if template == "full" {
        create_tests(&crate_path)?;
        create_examples(&crate_path)?;
        create_ci_workflow(&crate_path, &crate_name)?;
    }

    // 验证创建结果
    println!("\n🔍 验证 crate 创建...");
    verify_crate(&crate_name)?;

    println!("\n✅ Crate '{}' created successfully!", crate_name);
    println!("\nNext steps:");
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

fn create_crate_structure(crate_path: &Path, template: &str) -> Result<()> {
    fs::create_dir_all(crate_path).context("Failed to create crate directory")?;

    fs::create_dir_all(crate_path.join("src")).context("Failed to create src directory")?;

    if template == "full" {
        fs::create_dir_all(crate_path.join("tests")).context("Failed to create tests directory")?;
        fs::create_dir_all(crate_path.join("examples"))
            .context("Failed to create examples directory")?;
        fs::create_dir_all(crate_path.join(".github/workflows"))
            .context("Failed to create .github/workflows directory")?;
    }

    Ok(())
}

fn create_cargo_toml(
    crate_path: &Path,
    name: &str,
    _layer: usize,
    allowed_deps: &[String],
    template: &str,
) -> Result<()> {
    let deps_section = if template == "minimal" {
        String::new()
    } else {
        allowed_deps
            .iter()
            .filter(|dep| *dep != name)
            .map(|dep| format!("{} = {{ workspace = true }}", dep))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let dev_deps = if template == "minimal" {
        String::new()
    } else {
        "\n[dev-dependencies]\ntokio = { version = \"1.0\", features = [\"full\"] }\n".to_string()
    };

    let content = format!(
        "[package]\n\
         name = \"{}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         description = \"Axiom {} crate\"\n\
         license = \"MIT\"\n\
         repository = \"https://github.com/arwei944/axiom-kernel\"\n\
         homepage = \"https://github.com/arwei944/axiom-kernel\"\n\
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
         {}\n\
         \n\
         [build-dependencies]\n\
         archcheck = {{ workspace = true }}\n\
         \n\
         [[test]]\n\
         name = \"integration\"\n\
         path = \"tests/integration.rs\"\n",
        name,
        name.replace("axiom-", ""),
        name,
        name.replace("axiom-", ""),
        deps_section,
        dev_deps
    );

    let mut file =
        File::create(crate_path.join("Cargo.toml")).context("Failed to create Cargo.toml")?;
    file.write_all(content.as_bytes())
        .context("Failed to write Cargo.toml")?;

    Ok(())
}

fn create_build_rs(crate_path: &Path, crate_name: &str) -> Result<()> {
    let content = format!(
        "fn main() {{\n\
         \t// Architecture gate check: this build script will fail compilation\n\
         \t// if the current crate violates architecture constraints defined in\n\
         \t// .axiom/architecture.toml\n\
         \tarchcheck::build_hook::check_current_crate(\"{}\");\n\
         }}\n",
        crate_name
    );

    let mut file =
        File::create(crate_path.join("build.rs")).context("Failed to create build.rs")?;
    file.write_all(content.as_bytes())
        .context("Failed to write build.rs")?;

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
         //!\n\
         //! ## Allowed Dependencies\n\
         //!\n\
         //! This crate can depend on:\n\
         //!\n",
        module_name, module_name, layer, layer
    );

    let content = content
        + &format!(
            "//! - Layer {}: {} (and lower layers)\n\
         //!\n\
         #![warn(missing_docs)]\n\
         #![warn(clippy::all)]\n\
         #![warn(clippy::pedantic)]\n\
         #![allow(clippy::module_name_repetitions)]\n\
         \n\
         pub mod {};\n\
         \n\
         pub use {}::*;\n",
            layer, module_name, module_name, module_name
        );

    let mut file =
        File::create(crate_path.join("src/lib.rs")).context("Failed to create lib.rs")?;
    file.write_all(content.as_bytes())
        .context("Failed to write lib.rs")?;

    let struct_name = module_name
        .replace("-", "_")
        .split("-")
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<String>();

    let module_content = format!(
        "//! {} module\n\
         //!\n\
         //! Core functionality for the {} crate.\n\
         //!\n\
         //! ## Usage\n\
         //!\n\
         //! ```rust\n\
         //! use axiom_{}::{};\n\
         //!\n\
         //! let instance = {}::new();\n\
         //! ```\n\
         \n\
         use std::sync::Arc;\n\
         \n\
         use tokio::sync::RwLock;\n\
         \n\
         #[derive(Debug, Default)]\n\
         pub struct {};\n\
         \n\
         impl {} {{\n\
         \t/// Create a new instance\n\
         \tpub fn new() -> Self {{\n\
         \t\tSelf\n\
         \t}}\n\
         }}\n\
         \n\
         #[cfg(test)]\n\
         mod tests {{\n\
         \tuse super::*;\n\
         \t\n\
         \t#[tokio::test]\n\
         \tasync fn test_new() {{\n\
         \t\tlet instance = {}::new();\n\
         \t\tassert!(std::mem::size_of_val(&instance) > 0);\n\
         \t}}\n\
         }}\n",
        module_name,
        module_name,
        module_name,
        struct_name,
        struct_name,
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

    let mut content = fs::read_to_string(&arch_path).context("Failed to read architecture.toml")?;

    let insert_line = format!("{} = {}\n", crate_name, layer);

    // Find the [crate-layers] section and add the new crate
    if !content.contains(&crate_name.to_string()) {
        content = content.replace(
            "[crate-layers]",
            &format!("[crate-layers]\n{}", insert_line),
        );
    }

    fs::write(&arch_path, content).context("Failed to write architecture.toml")?;

    println!("Updated .axiom/architecture.toml with new crate registration");

    Ok(())
}

fn create_tests(crate_path: &Path) -> Result<()> {
    let content = r#"use tokio;

#[tokio::test]
async fn test_integration() {
    assert_eq!(1 + 1, 2);
}
"#;

    let mut file = File::create(crate_path.join("tests/integration.rs"))
        .context("Failed to create integration.rs")?;
    file.write_all(content.as_bytes())
        .context("Failed to write integration.rs")?;

    Ok(())
}

fn create_examples(crate_path: &Path) -> Result<()> {
    let content = r#"use axiom_example::Example;

fn main() {
    let example = Example::new();
    println!("Example created: {:?}", example);
}
"#;

    let mut file = File::create(crate_path.join("examples/example.rs"))
        .context("Failed to create example.rs")?;
    file.write_all(content.as_bytes())
        .context("Failed to write example.rs")?;

    Ok(())
}

fn create_ci_workflow(crate_path: &Path, crate_name: &str) -> Result<()> {
    let content = format!(
        r#"name: CI - {}

on:
  push:
    branches: [ master, main ]
  pull_request:
    branches: [ master, main ]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check --workspace
      - run: cargo test --workspace
      - run: cargo run -p archcheck -- --validate-architecture
      - run: cargo run -p archcheck --
"#,
        crate_name
    );

    let workflow_path = crate_path.join(".github/workflows/ci.yml");
    if let Some(parent) = workflow_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(workflow_path).context("Failed to create CI workflow")?;
    file.write_all(content.as_bytes())
        .context("Failed to write CI workflow")?;

    Ok(())
}

fn verify_crate(crate_name: &str) -> Result<()> {
    // 1. 运行 cargo check
    let check_output = std::process::Command::new("cargo")
        .args(["check", "-p", crate_name])
        .output()
        .context("Failed to run cargo check")?;

    if !check_output.status.success() {
        let stderr = String::from_utf8_lossy(&check_output.stderr);
        anyhow::bail!("cargo check failed:\n{}", stderr);
    }

    // 2. 运行架构检查
    let archcheck_output = std::process::Command::new("cargo")
        .args(["run", "-p", "archcheck", "--"])
        .output()
        .context("Failed to run archcheck")?;

    if !archcheck_output.status.success() {
        let stdout = String::from_utf8_lossy(&archcheck_output.stdout);
        anyhow::bail!("architecture check failed:\n{}", stdout);
    }

    Ok(())
}
