use std::process::Command;

use super::{Check, CheckResult};

pub struct GitStatusCheck;

impl Check for GitStatusCheck {
    fn name(&self) -> &'static str {
        "git working tree status"
    }

    fn blocking(&self) -> bool {
        false
    }

    fn run(&self) -> CheckResult {
        let output = match Command::new("git").args(["status", "--porcelain"]).output() {
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

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

        if lines.is_empty() {
            CheckResult {
                name: self.name(),
                passed: true,
                blocking: false,
                message: "working tree clean".into(),
            }
        } else {
            CheckResult {
                name: self.name(),
                passed: false,
                blocking: false,
                message: format!("{} uncommitted change(s)", lines.len()),
            }
        }
    }
}
