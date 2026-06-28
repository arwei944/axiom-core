use std::process::Command;

use super::{Check, CheckResult};

pub struct CargoBuildCheck;

impl Check for CargoBuildCheck {
    fn name(&self) -> &'static str {
        "cargo build (warnings as errors)"
    }

    fn blocking(&self) -> bool {
        true
    }

    fn run(&self) -> CheckResult {
        let output = Command::new("cargo")
            .args(["build", "--workspace"])
            .env("RUSTFLAGS", "-D warnings")
            .output();

        match output {
            Ok(o) if o.status.success() => CheckResult {
                name: self.name(),
                passed: true,
                blocking: true,
                message: "build succeeded with zero warnings".into(),
            },
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let errors: Vec<&str> = stderr
                    .lines()
                    .filter(|l| l.contains("error") || l.contains("warning"))
                    .take(5)
                    .collect();
                CheckResult {
                    name: self.name(),
                    passed: false,
                    blocking: true,
                    message: if errors.is_empty() {
                        "build failed".into()
                    } else {
                        format!("build issues:\n    {}", errors.join("\n    "))
                    },
                }
            }
            Err(e) => CheckResult {
                name: self.name(),
                passed: false,
                blocking: true,
                message: format!("failed to run cargo build: {e}"),
            },
        }
    }
}
