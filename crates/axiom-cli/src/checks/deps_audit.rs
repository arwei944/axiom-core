use std::fs;
use std::path::Path;

use super::{Check, CheckResult};

fn parse_deps_from_cargo(path: &Path) -> Result<Vec<String>, std::io::Error> {
    let content = fs::read_to_string(path)?;
    let mut deps = Vec::new();
    let mut in_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]"
                || trimmed == "[dev-dependencies]"
                || trimmed == "[build-dependencies]";
            continue;
        }
        if in_deps && !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some(name) = trimmed.split(|c: char| c.is_whitespace() || c == '=').next() {
                if !name.is_empty() && !name.starts_with("axiom-") {
                    deps.push(name.to_string());
                }
            }
        }
    }
    Ok(deps)
}

fn collect_cargo_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let mut results = Vec::new();
    fn walk(dir: &Path, results: &mut Vec<std::path::PathBuf>) -> Result<(), std::io::Error> {
        if !dir.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().map(|n| n == "target").unwrap_or(false) {
                    continue;
                }
                walk(&path, results)?;
            } else if path.file_name().map(|n| n == "Cargo.toml").unwrap_or(false) {
                results.push(path);
            }
        }
        Ok(())
    }
    walk(dir, &mut results)?;
    Ok(results)
}

pub struct DepsAuditCheck;

impl Check for DepsAuditCheck {
    fn name(&self) -> &'static str {
        "third-party dependency audit"
    }

    fn blocking(&self) -> bool {
        true
    }

    fn run(&self) -> CheckResult {
        let mut violations = Vec::new();
        let cargo_files = match collect_cargo_files(Path::new(".")) {
            Ok(f) => f,
            Err(e) => {
                return CheckResult {
                    name: self.name(),
                    passed: false,
                    blocking: true,
                    message: format!("cannot scan Cargo.toml files: {e}"),
                }
            }
        };

        for cargo_path in cargo_files {
            match parse_deps_from_cargo(&cargo_path) {
                Ok(deps) => {
                    for dep in deps {
                        if let Err(reason) = axiom_kernel::gate::audit_dependency(&dep) {
                            violations.push(format!("{}: {}", cargo_path.display(), reason));
                        }
                    }
                }
                Err(e) => {
                    violations.push(format!("{}: parse error: {e}", cargo_path.display()));
                }
            }
        }

        if violations.is_empty() {
            CheckResult {
                name: self.name(),
                passed: true,
                blocking: true,
                message: "all dependencies audited".into(),
            }
        } else {
            CheckResult {
                name: self.name(),
                passed: false,
                blocking: true,
                message: format!(
                    "{} dependency violation(s):\n    {}",
                    violations.len(),
                    violations.join("\n    ")
                ),
            }
        }
    }
}
