use std::process::Command;

use super::{Check, CheckResult};

const PROTECTED_BRANCHES: &[&str] = &["main", "master"];

pub struct BranchCheck;

impl Check for BranchCheck {
    fn name(&self) -> &'static str {
        "branch check (not on protected branch)"
    }

    fn blocking(&self) -> bool {
        true
    }

    fn run(&self) -> CheckResult {
        let output = match Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"]).output()
        {
            Ok(o) => o,
            Err(e) => {
                return CheckResult {
                    name: self.name(),
                    passed: false,
                    blocking: true,
                    message: format!("cannot run git: {e}"),
                }
            }
        };

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if PROTECTED_BRANCHES.contains(&branch.as_str()) {
            CheckResult {
                name: self.name(),
                passed: false,
                blocking: true,
                message: format!(
                    "currently on protected branch '{branch}'. Create a feature branch first: git checkout -b <branch-name>"
                ),
            }
        } else {
            CheckResult {
                name: self.name(),
                passed: true,
                blocking: true,
                message: format!("on branch '{branch}' (safe)"),
            }
        }
    }
}
