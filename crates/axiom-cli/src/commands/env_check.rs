use std::process::{Command, ExitCode};

use anyhow::Result;

pub fn run_env_check() -> Result<ExitCode> {
    println!("=== axiom env-check ===\n");

    let mut all_passed = true;

    all_passed &= check_rust_version();
    all_passed &= check_rustfmt();
    all_passed &= check_clippy();
    all_passed &= check_git_hooks();
    all_passed &= check_dependencies();
    all_passed &= check_env_vars();

    println!();

    if all_passed {
        println!("✅ All environment checks passed! Ready to code.");
        Ok(ExitCode::SUCCESS)
    } else {
        println!("❌ Some checks failed. Fix issues before coding.");
        Ok(ExitCode::from(1))
    }
}

fn check_rust_version() -> bool {
    print!("Checking Rust version... ");
    
    let output = match Command::new("rustc")
        .arg("--version")
        .output() {
            Ok(o) => o,
            Err(_) => {
                println!("❌ Rust not found");
                return false;
            }
        };

    if !output.status.success() {
        println!("❌ Rust command failed");
        return false;
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = version_str
        .split_whitespace()
        .nth(1)
        .unwrap_or("0.0.0");

    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        let major: u32 = parts[0].parse().unwrap_or(0);
        let minor: u32 = parts[1].parse().unwrap_or(0);

        if major >= 1 && minor >= 75 {
            println!("✅ {} (>= 1.75 required)", version);
            return true;
        }
    }

    println!("❌ {} (< 1.75 required)", version);
    false
}

fn check_rustfmt() -> bool {
    print!("Checking rustfmt... ");
    
    let output = match Command::new("cargo")
        .arg("fmt")
        .arg("--version")
        .output() {
            Ok(o) => o,
            Err(_) => {
                println!("❌ not found (run: rustup component add rustfmt)");
                return false;
            }
        };

    if output.status.success() {
        println!("✅ installed");
        true
    } else {
        println!("❌ not found (run: rustup component add rustfmt)");
        false
    }
}

fn check_clippy() -> bool {
    print!("Checking clippy... ");
    
    let output = match Command::new("cargo")
        .arg("clippy")
        .arg("--version")
        .output() {
            Ok(o) => o,
            Err(_) => {
                println!("❌ not found (run: rustup component add clippy)");
                return false;
            }
        };

    if output.status.success() {
        println!("✅ installed");
        true
    } else {
        println!("❌ not found (run: rustup component add clippy)");
        false
    }
}

fn check_git_hooks() -> bool {
    print!("Checking git hooks... ");
    
    let output = match Command::new("git")
        .arg("config")
        .arg("core.hooksPath")
        .output() {
            Ok(o) => o,
            Err(_) => {
                println!("❌ not a git repository");
                return false;
            }
        };

    if output.status.success() {
        let hooks_path = String::from_utf8_lossy(&output.stdout).to_string();
        if hooks_path.trim() == "hooks" {
            println!("✅ configured");
            true
        } else {
            println!("❌ not configured (run: axm install-hooks)");
            false
        }
    } else {
        println!("❌ not a git repository");
        false
    }
}

fn check_dependencies() -> bool {
    print!("Checking dependencies... ");
    
    let output = match Command::new("cargo")
        .arg("check")
        .arg("--workspace")
        .arg("--quiet")
        .output() {
            Ok(o) => o,
            Err(_) => {
                println!("❌ cargo check failed");
                return false;
            }
        };

    if output.status.success() {
        println!("✅ up to date");
        true
    } else {
        println!("❌ missing or outdated (run: cargo check --workspace)");
        false
    }
}

fn check_env_vars() -> bool {
    print!("Checking environment variables... ");
    
    let required_vars = ["AXIOM_ENV"];
    let mut all_set = true;

    for var in required_vars {
        if std::env::var(var).is_err() {
            all_set = false;
            break;
        }
    }

    if all_set {
        println!("✅ all set");
        true
    } else {
        println!("⚠ optional variables not set (AXIOM_ENV)");
        true
    }
}
