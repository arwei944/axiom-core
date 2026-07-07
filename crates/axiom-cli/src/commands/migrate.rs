use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use serde::Deserialize;

#[derive(Args)]
pub struct MigrateArgs {
    /// Path to the project to migrate (default: current directory)
    #[arg(long)]
    pub path: Option<String>,

    /// Run in dry-run mode (show changes without applying)
    #[arg(long)]
    pub dry_run: bool,

    /// Apply automated fixes
    #[arg(long)]
    pub apply: bool,

    /// Show detailed migration report
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Debug, Deserialize)]
struct CargoToml {
    #[serde(default)]
    dependencies: BTreeMap<String, Dependency>,
    #[serde(default)]
    #[serde(rename = "dev-dependencies")]
    dev_dependencies: BTreeMap<String, Dependency>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum Dependency {
    String(String),
    Map(BTreeMap<String, String>),
}

#[derive(Debug, Default)]
struct MigrationReport {
    cargo_toml_changes: Vec<String>,
    source_file_changes: Vec<FileChange>,
    manual_actions: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct FileChange {
    file: PathBuf,
    changes: Vec<LineChange>,
}

#[derive(Debug)]
struct LineChange {
    line: usize,
    old: String,
    new: String,
}

const OLD_CRATE: &str = "axiom-core";
const NEW_CRATE: &str = "axiom-kernel";

const IMPORT_MAPPINGS: &[(&str, &str)] = &[
    ("axiom_core::cell::", "axiom_kernel::cell::"),
    ("axiom_core::signal::", "axiom_kernel::signal::"),
    ("axiom_core::axiom::", "axiom_kernel::axiom::"),
    ("axiom_core::witness::", "axiom_kernel::witness::"),
    ("axiom_core::layer::", "axiom_kernel::layer::"),
    ("axiom_core::context::", "axiom_kernel::context::"),
    ("axiom_core::guard::", "axiom_kernel::guard::"),
    ("axiom_core::lens::", "axiom_kernel::lens::"),
    ("axiom_core::registry::", "axiom_kernel::registry::"),
    ("axiom_core::entropy::", "axiom_kernel::entropy::"),
    ("axiom_core::id::", "axiom_kernel::id::"),
    ("axiom_core::tool::", "axiom_kernel::tool::"),
    ("axiom_core::plugin::", "axiom_kernel::plugin::"),
    ("axiom_core::version::", "axiom_kernel::version::"),
    ("axiom_core::codec::", "axiom_kernel::codec::"),
    ("axiom_core::clock::", "axiom_kernel::clock::"),
    ("axiom_core::gate", "axiom_kernel::gate"),
];

const MACRO_MAPPINGS: &[(&str, &str)] = &[
    ("axiom_core::signal", "axiom_kernel::signal"),
    ("axiom_core::cell", "axiom_kernel::cell"),
    ("axiom_core::axiom", "axiom_kernel::axiom"),
    ("axiom_core::guard", "axiom_kernel::guard"),
    ("axiom_core::lens", "axiom_kernel::lens"),
];

pub fn run_migrate(args: &MigrateArgs) -> Result<std::process::ExitCode> {
    let project_path = match &args.path {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().context("Failed to get current directory")?,
    };

    if !project_path.exists() {
        return Err(anyhow::anyhow!("Path '{}' does not exist", project_path.display()));
    }

    println!("=== Axiom Migration Tool ===");
    println!("Project: {}", project_path.display());
    println!(
        "Mode: {}",
        if args.dry_run {
            "dry-run"
        } else {
            if args.apply {
                "apply"
            } else {
                "report"
            }
        }
    );
    println!();

    let mut report = MigrationReport::default();

    analyze_cargo_toml(&project_path, &mut report)?;
    analyze_source_files(&project_path, &mut report)?;

    print_report(&report, args.verbose);

    if args.apply && !report.source_file_changes.is_empty() {
        apply_changes(&report)?;
        println!("\n✅ Applied {} file changes", report.source_file_changes.len());
    }

    if !report.manual_actions.is_empty() {
        println!("\n⚠️  Manual actions required:");
        for (i, action) in report.manual_actions.iter().enumerate() {
            println!("  {}. {}", i + 1, action);
        }
    }

    if !report.warnings.is_empty() {
        println!("\n⚠️  Warnings:");
        for warning in &report.warnings {
            println!("  - {}", warning);
        }
    }

    let total_changes = report.cargo_toml_changes.len() + report.source_file_changes.len();
    println!("\nTotal changes needed: {}", total_changes);

    if total_changes == 0 && report.manual_actions.is_empty() {
        println!("✅ Your project is already compatible with axiom-kernel!");
        Ok(std::process::ExitCode::SUCCESS)
    } else {
        Ok(std::process::ExitCode::SUCCESS)
    }
}

fn analyze_cargo_toml(project_path: &Path, report: &mut MigrationReport) -> Result<()> {
    let cargo_toml_path = project_path.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        report.warnings.push("No Cargo.toml found, skipping dependency analysis".to_string());
        return Ok(());
    }

    let content = fs::read_to_string(&cargo_toml_path)
        .context(format!("Failed to read {}", cargo_toml_path.display()))?;

    let mut new_content = content.clone();

    if content.contains(OLD_CRATE) {
        new_content = new_content.replace(OLD_CRATE, NEW_CRATE);

        let cargo: CargoToml = toml::from_str(&content).context("Failed to parse Cargo.toml")?;

        for name in cargo.dependencies.keys() {
            if name == OLD_CRATE {
                report
                    .cargo_toml_changes
                    .push(format!("Replace '{}' with '{}'", OLD_CRATE, NEW_CRATE));
            }
        }
        for name in cargo.dev_dependencies.keys() {
            if name == OLD_CRATE {
                report.cargo_toml_changes.push(format!(
                    "Replace '{}' with '{}' in dev-dependencies",
                    OLD_CRATE, NEW_CRATE
                ));
            }
        }

        if !report.cargo_toml_changes.is_empty() {
            if let Ok(mut file) = File::create(&cargo_toml_path) {
                file.write_all(new_content.as_bytes())
                    .context(format!("Failed to write {}", cargo_toml_path.display()))?;
            }
        }
    }

    Ok(())
}

fn analyze_source_files(project_path: &Path, report: &mut MigrationReport) -> Result<()> {
    let src_dir = project_path.join("src");
    if !src_dir.exists() {
        report.warnings.push("No src/ directory found, skipping source analysis".to_string());
        return Ok(());
    }

    let rust_files: Vec<PathBuf> = find_rust_files(&src_dir);

    for file in rust_files {
        analyze_file(&file, report)?;
    }

    let tests_dir = project_path.join("tests");
    if tests_dir.exists() {
        let test_files: Vec<PathBuf> = find_rust_files(&tests_dir);
        for file in test_files {
            analyze_file(&file, report)?;
        }
    }

    let examples_dir = project_path.join("examples");
    if examples_dir.exists() {
        let example_files: Vec<PathBuf> = find_rust_files(&examples_dir);
        for file in example_files {
            analyze_file(&file, report)?;
        }
    }

    Ok(())
}

fn find_rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(find_rust_files(&path));
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    files
}

fn analyze_file(file: &Path, report: &mut MigrationReport) -> Result<()> {
    let content = fs::read_to_string(file).context(format!("Failed to read {}", file.display()))?;

    let mut changes = Vec::new();
    let mut lines = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let mut new_line = line.to_string();
        let mut changed = false;

        for &(old, new) in IMPORT_MAPPINGS {
            if new_line.contains(old) {
                new_line = new_line.replace(old, new);
                changed = true;
            }
        }

        for &(old, new) in MACRO_MAPPINGS {
            if new_line.contains(&format!("#[{}", old)) {
                new_line = new_line.replace(&format!("#[{}", old), &format!("#[{}", new));
                changed = true;
            }
        }

        if changed && line != new_line {
            changes.push(LineChange {
                line: line_num + 1,
                old: line.to_string(),
                new: new_line.clone(),
            });
        }

        lines.push(new_line);
    }

    check_cell_handle_signature(file, &content, report);
    check_context_usage(file, &content, report);

    if !changes.is_empty() {
        report.source_file_changes.push(FileChange { file: file.to_path_buf(), changes });
    }

    Ok(())
}

fn check_cell_handle_signature(file: &Path, content: &str, report: &mut MigrationReport) {
    if (content.contains("impl Cell for") || content.contains("async fn handle(&mut self"))
        && content.contains("async fn handle(&mut self")
        && content.contains("CellContext")
    {
        report.manual_actions.push(format!(
            "{}: Update `Cell::handle` method signature. See MIGRATION.md for details.",
            file.display()
        ));
    }
}

fn check_context_usage(file: &Path, content: &str, report: &mut MigrationReport) {
    if content.contains("ctx.emit_witness(") && !content.contains("ctx.witness().") {
        report.manual_actions.push(format!(
            "{}: Update witness emission to use builder pattern: `ctx.witness().summary(...)`",
            file.display()
        ));
    }

    if content.contains("AxiomChain::from_registry") && !content.contains("DynAxiomChain") {
        report.manual_actions.push(format!(
            "{}: Update AxiomChain to DynAxiomChain::from_registry_for_layer",
            file.display()
        ));
    }
}

fn print_report(report: &MigrationReport, verbose: bool) {
    if !report.cargo_toml_changes.is_empty() {
        println!("📦 Cargo.toml changes:");
        for change in &report.cargo_toml_changes {
            println!("  ✓ {}", change);
        }
    }

    if !report.source_file_changes.is_empty() {
        println!("\n📝 Source file changes:");
        for file_change in &report.source_file_changes {
            println!("  {}", file_change.file.display());
            if verbose {
                for line_change in &file_change.changes {
                    println!("    Line {}:", line_change.line);
                    println!("      - {}", line_change.old);
                    println!("      + {}", line_change.new);
                }
            } else {
                println!("    {} changes", file_change.changes.len());
            }
        }
    }
}

fn apply_changes(report: &MigrationReport) -> Result<()> {
    for file_change in &report.source_file_changes {
        let content = fs::read_to_string(&file_change.file)
            .context(format!("Failed to read {}", file_change.file.display()))?;

        let mut lines: Vec<&str> = content.lines().collect();

        for line_change in &file_change.changes {
            let idx = line_change.line - 1;
            if idx < lines.len() {
                lines[idx] = &line_change.new;
            }
        }

        let new_content = lines.join("\n");

        let mut file = File::create(&file_change.file)
            .context(format!("Failed to create {}", file_change.file.display()))?;
        file.write_all(new_content.as_bytes())
            .context(format!("Failed to write {}", file_change.file.display()))?;
    }

    Ok(())
}
