use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};

pub fn run_init() -> Result<ExitCode> {
    println!("=== axiom init ===\n");

    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    let cargo_toml = cwd.join("Cargo.toml");
    if !cargo_toml.exists() {
        anyhow::bail!(
            "Cargo.toml not found in current directory. Run 'axm init' from the project root."
        );
    }

    let axiom_dir = cwd.join(".axiom");
    if !axiom_dir.exists() {
        anyhow::bail!(
            ".axiom/ directory not found. Ensure this is an axiom project (check .axiom/ exists)."
        );
    }

    install_hooks(&cwd)?;
    update_constraints_lock()?;

    println!("\nInitialized axiom project. Hooks installed, constraints lock updated.");
    println!("Ready to code with full gate protection.");
    Ok(ExitCode::SUCCESS)
}

pub(crate) fn install_hooks(project_root: &Path) -> Result<()> {
    let hooks_src = project_root.join("hooks");
    if !hooks_src.exists() {
        anyhow::bail!(
            "hooks/ directory not found in project root. Expected at {}.",
            hooks_src.display()
        );
    }

    for hook_name in &["pre-commit", "pre-push"] {
        let hook_path = hooks_src.join(hook_name);
        if !hook_path.exists() {
            anyhow::bail!("Required hook '{}' not found in hooks/", hook_name);
        }
    }

    let hooks_abs = hooks_src
        .canonicalize()
        .context("Failed to resolve hooks/ absolute path")?;
    let status = std::process::Command::new("git")
        .args(["config", "core.hooksPath", hooks_abs.to_str().unwrap()])
        .current_dir(project_root)
        .status()
        .context("Failed to run 'git config core.hooksPath'")?;

    if !status.success() {
        anyhow::bail!(
            "git config core.hooksPath failed with exit code: {:?}",
            status.code()
        );
    }

    println!("  ✓ configured core.hooksPath -> hooks/");
    println!("  ✓ hooks active: pre-commit, pre-push");

    Ok(())
}

fn update_constraints_lock() -> Result<()> {
    crate::checks::constraints_hash::ConstraintsHashCheck::update_lock()
        .context("Failed to update constraints lock")?;
    println!("  ✓ constraints lock updated");
    Ok(())
}
