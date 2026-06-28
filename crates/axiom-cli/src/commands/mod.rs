use std::process::ExitCode;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};

use crate::checks;

mod init;
use init::install_hooks;

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
