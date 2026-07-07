use std::process::Command;

use super::{Check, CheckResult};

pub struct CargoClippyCheck;

impl Check for CargoClippyCheck {
    fn name(&self) -> &'static str {
        "cargo clippy (warnings as errors)"
    }

    fn blocking(&self) -> bool {
        true
    }

    fn run(&self) -> CheckResult {
        let output =
            Command::new("cargo").args(["clippy", "--workspace", "--", "-D", "warnings"]).output();

        match output {
            Ok(o) if o.status.success() => CheckResult {
                name: self.name(),
                passed: true,
                blocking: true,
                message: "clippy passed with zero warnings".into(),
            },
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let issues: Vec<&str> = stderr
                    .lines()
                    .filter(|l| l.contains("error") || l.contains("warning"))
                    .take(5)
                    .collect();
                CheckResult {
                    name: self.name(),
                    passed: false,
                    blocking: true,
                    message: if issues.is_empty() {
                        "clippy failed".into()
                    } else {
                        format!("clippy issues:\n    {}", issues.join("\n    "))
                    },
                }
            }
            Err(e) => CheckResult {
                name: self.name(),
                passed: false,
                blocking: true,
                message: format!("failed to run cargo clippy: {e}"),
            },
        }
    }
}
