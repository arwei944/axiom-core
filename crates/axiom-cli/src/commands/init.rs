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

fn install_hooks(project_root: &Path) -> Result<()> {
    let hooks_src = project_root.join("hooks");
    if !hooks_src.exists() {
        println!("  ⚠ hooks/ directory not found in project root, skipping hook installation.");
        return Ok(());
    }

    let git_dir = project_root.join(".git");
    if !git_dir.exists() {
        println!("  ⚠ .git/ directory not found, skipping hook installation.");
        return Ok(());
    }

    let git_hooks = git_dir.join("hooks");
    std::fs::create_dir_all(&git_hooks).context("Failed to create .git/hooks/ directory")?;

    for hook_name in &["pre-commit", "pre-push"] {
        let src = hooks_src.join(hook_name);
        let dst = git_hooks.join(hook_name);
        if src.exists() {
            std::fs::copy(&src, &dst)
                .with_context(|| format!("Failed to copy {} to .git/hooks/", hook_name))?;
            set_executable_permission(&dst)?;
            println!("  ✓ installed {}", hook_name);
        }
    }

    Ok(())
}

#[cfg(unix)]
fn set_executable_permission(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(path, perms).context("Failed to set executable permission")?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable_permission(_path: &Path) -> Result<()> {
    Ok(())
}

fn update_constraints_lock() -> Result<()> {
    crate::checks::constraints_hash::ConstraintsHashCheck::update_lock()
        .context("Failed to update constraints lock")?;
    println!("  ✓ constraints lock updated");
    Ok(())
}
