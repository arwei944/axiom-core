use crate::checker::Severity;
use crate::loader::Architecture;
use crate::reporter::{report_json, report_text};
use anyhow::{Context, Result};
use clap::{Arg, ArgAction, Command};
use std::path::PathBuf;

mod checker;
mod loader;
mod reporter;

fn main() -> Result<()> {
    let matches = Command::new("archcheck")
        .about("Architecture governance checker for axiom workspace")
        .arg(
            Arg::new("architecture")
                .long("architecture")
                .short('a')
                .value_name("FILE")
                .default_value(".axiom/architecture.toml")
                .help("Path to architecture.toml"),
        )
        .arg(
            Arg::new("workspace")
                .long("workspace")
                .short('w')
                .value_name("DIR")
                .default_value(".")
                .help("Workspace root directory"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .short('f')
                .value_name("FORMAT")
                .default_value("text")
                .help("Output format: text or json"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("FILE")
                .help("Write report to file instead of stdout"),
        )
        .arg(
            Arg::new("validate-architecture")
                .long("validate-architecture")
                .action(ArgAction::SetTrue)
                .help("Validate architecture.toml syntax only"),
        )
        .arg(
            Arg::new("list-crates")
                .long("list-crates")
                .action(ArgAction::SetTrue)
                .help("List all registered crates"),
        )
        .get_matches();

    let arch_path = PathBuf::from(
        // foxguard: ignore[rs/no-path-traversal] — CLI args are controlled
        matches.get_one::<String>("architecture").expect("architecture path is required"),
    );
    let workspace_path = PathBuf::from(
        // foxguard: ignore[rs/no-path-traversal] — CLI args are controlled
        matches.get_one::<String>("workspace").expect("workspace path is required"),
    );
    let format = matches.get_one::<String>("format").expect("format is required");

    if matches.get_flag("validate-architecture") {
        let _ = Architecture::load(&arch_path).context("validate architecture.toml")?;
        println!("architecture.toml is valid.");
        return Ok(());
    }

    let architecture = Architecture::load(&arch_path).with_context(|| {
        format!("failed to load architecture.toml from {}", arch_path.display())
    })?;

    if matches.get_flag("list-crates") {
        println!("Registered crates: {}", architecture.crate_layers.len());
        for (name, layer) in &architecture.crate_layers {
            println!("  layer {}: {}", layer, name);
        }
        return Ok(());
    }

    let violations = checker::check_all(&architecture, &workspace_path);

    let output = match format.as_str() {
        "json" => report_json(&violations),
        _ => report_text(&violations),
    };

    if let Some(output_path) = matches.get_one::<String>("output") {
        std::fs::write(output_path, output).context("writing report file")?;
    } else {
        println!("{}", output);
    }

    if violations.iter().any(|v| v.severity == Severity::Blocker) {
        std::process::exit(1);
    }

    Ok(())
}
