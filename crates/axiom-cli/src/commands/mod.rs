use std::process::ExitCode;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};

use crate::checks;

mod init;
use init::install_hooks;

mod new;
use new::run_new;

mod new_crate;
use new_crate::run_new_crate;

mod new_cell;
use new_cell::run_new_cell;

mod new_signal;
use new_signal::run_new_signal;

mod new_tool;
use new_tool::run_new_tool;

mod env_check;
use env_check::run_env_check;

mod run;
use run::{run_dev, run_run};

mod top;
use top::run_top;

mod trace;
use trace::run_trace;

mod why;
use why::run_why;

mod witness;
use witness::run_witness;

mod cell;
use cell::run_cell;

mod entropy;
use entropy::run_entropy;

mod heatmap;
use heatmap::run_heatmap;

mod plugin;
use plugin::run_plugin;

#[derive(Parser)]
#[command(
    name = "axm",
    about = "Axiom CLI - development automation gates",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new axiom project
    New(new::NewArgs),
    /// Create a new axiom crate with layer constraints
    NewCrate(new_crate::NewCrateArgs),
    /// Create a new Cell with layer annotation
    NewCell(new_cell::NewCellArgs),
    /// Create a new Signal with layer annotation
    NewSignal(new_signal::NewSignalArgs),
    /// Create a new Tool with permission control
    NewTool(new_tool::NewToolArgs),
    /// Check development environment before coding
    EnvCheck,
    /// Run the axiom runtime
    Run(run::RunArgs),
    /// Run the axiom runtime in development mode
    Dev(run::DevArgs),
    /// Real-time runtime monitor
    Top(top::TopArgs),
    /// Trace signal flow by correlation ID
    Trace(trace::TraceArgs),
    /// Analyze causal chain for an entity
    Why(why::WhyArgs),
    /// Witness management commands
    Witness(witness::WitnessArgs),
    /// Cell management commands
    Cell(cell::CellArgs),
    /// Entropy management commands
    Entropy(entropy::EntropyArgs),
    /// Show usage heatmap
    Heatmap(heatmap::HeatmapArgs),
    /// Plugin management commands
    Plugin(plugin::PluginArgs),
    /// Initialize an axiom project (install hooks, generate constraints lock)
    Init,
    /// Install git hooks (configures core.hooksPath to hooks/)
    InstallHooks,
    /// Run preflight checks before starting a coding session
    Preflight(PreflightArgs),
    /// Run all quality gates (build/test/clippy/fmt/verify)
    Check,
    /// Verify architecture constraints (dependency direction, layer rules)
    Verify,
    /// Update the constraints lock file (after reviewing changes to .axiom/)
    UpdateConstraints,
    /// Show version information for all axiom crates
    Version,
}

#[derive(Args)]
pub struct PreflightArgs {
    /// Update constraints hash lock file after reviewing changes
    #[arg(long)]
    pub update_constraints: bool,
}

pub fn run(cli: &Cli) -> Result<ExitCode, anyhow::Error> {
    match &cli.command {
        Commands::New(args) => run_new(args),
        Commands::NewCrate(args) => run_new_crate(args),
        Commands::NewCell(args) => run_new_cell(args),
        Commands::NewSignal(args) => run_new_signal(args),
        Commands::NewTool(args) => run_new_tool(args),
        Commands::EnvCheck => run_env_check(),
        Commands::Run(args) => run_run(args),
        Commands::Dev(args) => run_dev(args),
        Commands::Top(args) => run_top(args),
        Commands::Trace(args) => run_trace(args),
        Commands::Why(args) => run_why(args),
        Commands::Witness(args) => run_witness(args),
        Commands::Cell(args) => run_cell(args),
        Commands::Entropy(args) => run_entropy(args),
        Commands::Heatmap(args) => run_heatmap(args),
        Commands::Plugin(args) => run_plugin(args),
        Commands::Init => init::run_init(),
        Commands::InstallHooks => install_hooks_only(),
        Commands::Preflight(args) => run_preflight(args),
        Commands::Check => run_check(),
        Commands::Verify => run_verify(),
        Commands::UpdateConstraints => run_update_constraints(),
        Commands::Version => {
            println!("axm {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn run_preflight(args: &PreflightArgs) -> Result<ExitCode, anyhow::Error> {
    println!("=== axiom preflight ===\n");

    if args.update_constraints {
        println!("Updating constraints lock file...");
        checks::constraints_hash::ConstraintsHashCheck::update_lock()?;
        println!("  ✓ constraints lock updated\n");
    }

    let checks_list = checks::preflight_checks();
    let (results, blocking) = checks::run_boxed_checks(&checks_list);
    checks::print_results(&results);

    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    println!("\n{}/{} checks passed", passed, total);

    if blocking {
        println!("\nBLOCKING FAILURES - fix before coding.");
        Ok(ExitCode::from(1))
    } else {
        println!("\nPreflight passed. Ready to code.");
        Ok(ExitCode::SUCCESS)
    }
}

fn run_check() -> Result<ExitCode, anyhow::Error> {
    println!("=== axiom check (full quality gates) ===\n");

    let checks_list = checks::all_checks();
    let (results, blocking) = checks::run_boxed_checks(&checks_list);
    checks::print_results(&results);

    let passed = results.iter().filter(|r| r.passed).count();
    let warnings = results.iter().filter(|r| !r.passed && !r.blocking).count();
    let failures = results.iter().filter(|r| !r.passed && r.blocking).count();

    println!(
        "\nResults: {} passed, {} warnings, {} blocking failures",
        passed, warnings, failures
    );

    if blocking {
        println!("\nBLOCKING FAILURES - do not commit or push.");
        Ok(ExitCode::from(1))
    } else if warnings > 0 {
        println!("\nWarning: non-blocking issues found.");
        Ok(ExitCode::from(0))
    } else {
        println!("\nAll gates passed.");
        Ok(ExitCode::SUCCESS)
    }
}

fn run_verify() -> Result<ExitCode, anyhow::Error> {
    println!("=== axiom verify (architecture constraints) ===\n");

    let checks_list = checks::verify_checks();
    let (results, blocking) = checks::run_boxed_checks(&checks_list);
    checks::print_results(&results);

    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    println!("\n{}/{} architecture checks passed", passed, total);

    if blocking {
        println!("\nARCHITECTURE VIOLATIONS - fix before proceeding.");
        Ok(ExitCode::from(1))
    } else {
        println!("\nArchitecture constraints satisfied.");
        Ok(ExitCode::SUCCESS)
    }
}

fn run_update_constraints() -> Result<ExitCode, anyhow::Error> {
    println!("Updating .axiom/.constraints.lock ...");
    checks::constraints_hash::ConstraintsHashCheck::update_lock()?;
    println!("  ✓ constraints lock updated.");
    Ok(ExitCode::SUCCESS)
}

fn install_hooks_only() -> Result<ExitCode, anyhow::Error> {
    println!("=== axiom install-hooks ===\n");
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    install_hooks(&cwd)?;
    println!("\nHooks installed successfully.");
    Ok(ExitCode::SUCCESS)
}
