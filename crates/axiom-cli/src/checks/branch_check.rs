use std::process::Command;

use super::{Check, CheckResult};

const PROTECTED_BRANCHES: &[&str] = &["main", "master"];

pub struct BranchCheck;

impl Check for BranchCheck {
    fn name(&self) -> &'static str {
        "branch check (not on protected branch)"
    }

    fn blocking(&self) -> bool {
        false
    }

    fn run(&self) -> CheckResult {
        let output = match Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"]).output()
        {
            Ok(o) => o,
            Err(e) => {
                return CheckResult {
                    name: self.name(),
                    passed: false,
                    blocking: false,
                    message: format!("cannot run git: {e}"),
                }
            }
        };

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if PROTECTED_BRANCHES.contains(&branch.as_str()) {
            CheckResult {
                name: self.name(),
                passed: false,
                blocking: false,
                message: format!(
                    "currently on protected branch '{branch}'. Consider using a feature branch: git checkout -b <branch-name>"
                ),
            }
        } else {
            CheckResult {
                name: self.name(),
                passed: true,
                blocking: false,
                message: format!("on branch '{branch}' (safe)"),
            }
        }
    }
}
