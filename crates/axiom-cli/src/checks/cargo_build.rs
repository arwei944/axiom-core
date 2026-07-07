use std::process::Command;

use super::{Check, CheckResult};

pub struct CargoBuildCheck;

impl Check for CargoBuildCheck {
    fn name(&self) -> &'static str {
        "cargo check (warnings as errors)"
    }

    fn blocking(&self) -> bool {
        true
    }

    fn run(&self) -> CheckResult {
        let output = Command::new("cargo")
            .args(["check", "--workspace", "--all-targets"])
            .env("RUSTFLAGS", "-D warnings")
            .output();

        match output {
            Ok(o) if o.status.success() => CheckResult {
                name: self.name(),
                passed: true,
                blocking: true,
                message: "check passed with zero warnings".into(),
            },
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let issues: Vec<&str> = stderr
                    .lines()
                    .filter(|l| l.starts_with("error") || l.starts_with("warning"))
                    .take(5)
                    .collect();
                CheckResult {
                    name: self.name(),
                    passed: false,
                    blocking: true,
                    message: if issues.is_empty() {
                        "check failed".into()
                    } else {
                        format!("check issues:\n    {}", issues.join("\n    "))
                    },
                }
            }
            Err(e) => CheckResult {
                name: self.name(),
                passed: false,
                blocking: true,
                message: format!("failed to run cargo check: {e}"),
            },
        }
    }
}
