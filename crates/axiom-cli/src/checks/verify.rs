use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::checks::{Check, CheckResult};

fn parse_local_deps(cargo_path: &Path) -> Result<(String, Vec<String>), std::io::Error> {
    let content = fs::read_to_string(cargo_path)?;
    let mut name = String::new();
    let mut deps = Vec::new();
    let mut section = "";

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
            continue;
        }
        if section == "package" && trimmed.starts_with("name") {
            if let Some(val) = trimmed.split('=').nth(1) {
                name = val.trim().trim_matches('"').trim_matches('\'').to_string();
            }
        }
        if (section == "dependencies" || section == "build-dependencies")
            && !trimmed.is_empty()
            && !trimmed.starts_with('#')
        {
            if let Some(dep_name) = trimmed
                .split(|c: char| c.is_whitespace() || c == '=')
                .next()
            {
                if dep_name.starts_with("axiom-") {
                    deps.push(dep_name.to_string());
                }
            }
        }
    }
    if name.is_empty() {
        name = cargo_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
    }
    Ok((name, deps))
}

fn collect_crates() -> Result<HashMap<String, Vec<String>>, std::io::Error> {
    let crates_dir = Path::new("crates");
    let mut result = HashMap::new();
    if !crates_dir.exists() {
        return Ok(result);
    }
    for entry in fs::read_dir(crates_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let cargo_path = path.join("Cargo.toml");
            if cargo_path.exists() {
                let (name, deps) = parse_local_deps(&cargo_path)?;
                result.insert(name, deps);
            }
        }
    }
    Ok(result)
}

pub struct VerifyCheck;

impl Check for VerifyCheck {
    fn name(&self) -> &'static str {
        "architecture dependency verification"
    }

    fn blocking(&self) -> bool {
        true
    }

    fn run(&self) -> CheckResult {
        let crates = match collect_crates() {
            Ok(c) => c,
            Err(e) => {
                return CheckResult {
                    name: self.name(),
                    passed: false,
                    blocking: true,
                    message: format!("cannot scan crates: {e}"),
                }
            }
        };

        let order: HashMap<&str, usize> = axiom_core::gate::crate_layers()
            .iter()
            .map(|(n, l)| (n.as_str(), *l))
            .collect();

        let mut violations = Vec::new();
        let max_order = axiom_core::gate::crate_layers().len();

        for (crate_name, deps) in &crates {
            let crate_level = order.get(crate_name.as_str()).copied().unwrap_or(max_order);
            for dep in deps {
                if dep == "axiom-macros" {
                    continue;
                }
                let dep_level = order.get(dep.as_str()).copied().unwrap_or(max_order);
                if dep_level < crate_level {
                    violations.push(format!(
                        "{crate_name} (level {crate_level}) depends on {dep} (level {dep_level}) - REVERSE DEPENDENCY"
                    ));
                }
            }
        }

        if violations.is_empty() {
            CheckResult {
                name: self.name(),
                passed: true,
                blocking: true,
                message: format!("dependency direction verified ({} crates)", crates.len()),
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_dep_order_matches_gate_constants() {
        for (name, level) in axiom_core::gate::crate_layers() {
            assert!(*level <= 8, "unexpected level for {name}");
        }
        assert_eq!(axiom_core::gate::crate_level("axiom-core"), Some(7));
        assert_eq!(axiom_core::gate::crate_level("axiom-cli"), Some(0));
    }
}
