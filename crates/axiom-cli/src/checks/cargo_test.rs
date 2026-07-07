use std::process::Command;

use super::{Check, CheckResult};

pub struct CargoTestCheck;

impl Check for CargoTestCheck {
    fn name(&self) -> &'static str {
        "cargo test"
    }

    fn blocking(&self) -> bool {
        true
    }

    fn run(&self) -> CheckResult {
        let output = Command::new("cargo").args(["test", "--workspace"]).output();

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let passed: Vec<&str> =
                    stdout.lines().filter(|l| l.contains("test result: ok")).collect();
                let total: usize = stdout
                    .lines()
                    .filter(|l| l.starts_with("test result:"))
                    .map(|l| {
                        l.split(|c: char| !c.is_ascii_digit())
                            .filter_map(|n| n.parse::<usize>().ok())
                            .next()
                            .unwrap_or(0)
                    })
                    .sum();
                CheckResult {
                    name: self.name(),
                    passed: true,
                    blocking: true,
                    message: format!("{} test suites passed ({} tests total)", passed.len(), total),
                }
            }
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let failures: Vec<&str> = stdout
                    .lines()
                    .filter(|l| l.contains("FAILED") || l.contains("panicked"))
                    .take(5)
                    .collect();
                CheckResult {
                    name: self.name(),
                    passed: false,
                    blocking: true,
                    message: if failures.is_empty() {
                        "tests failed".into()
                    } else {
                        format!("test failures:\n    {}", failures.join("\n    "))
                    },
                }
            }
            Err(e) => CheckResult {
                name: self.name(),
                passed: false,
                blocking: true,
                message: format!("failed to run cargo test: {e}"),
            },
        }
    }
}
