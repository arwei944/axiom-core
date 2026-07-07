use std::process::Command;

use super::{Check, CheckResult};

pub struct CargoFmtCheck;

impl Check for CargoFmtCheck {
    fn name(&self) -> &'static str {
        "cargo fmt --check"
    }

    fn blocking(&self) -> bool {
        true
    }

    fn run(&self) -> CheckResult {
        let output = Command::new("cargo").args(["fmt", "--all", "--", "--check"]).output();

        match output {
            Ok(o) if o.status.success() => CheckResult {
                name: self.name(),
                passed: true,
                blocking: true,
                message: "all files formatted correctly".into(),
            },
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let stdout = String::from_utf8_lossy(&o.stdout);
                let detail = if !stdout.is_empty() {
                    stdout.lines().take(5).collect::<Vec<_>>().join("\n    ")
                } else {
                    stderr.lines().take(5).collect::<Vec<_>>().join("\n    ")
                };
                CheckResult {
                    name: self.name(),
                    passed: false,
                    blocking: true,
                    message: format!("formatting errors found:\n    {}", detail),
                }
            }
            Err(e) => CheckResult {
                name: self.name(),
                passed: false,
                blocking: true,
                message: format!("failed to run cargo fmt: {e}"),
            },
        }
    }
}
