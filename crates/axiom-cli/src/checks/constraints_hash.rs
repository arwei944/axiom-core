use std::collections::HashMap;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::{Check, CheckResult};

const CONSTRAINT_FILES: &[&str] = &[
    ".axiom/AGENTS.md",
    ".axiom/preflight.md",
    ".axiom/rules/axiom-builder-rules.md",
    ".axiom/tools.md",
    ".axiom/identity/axiom-builder.md",
    ".axiom/skills/axiom-builder-skills.md",
];

const LOCK_FILE: &str = ".axiom/.constraints.lock";

fn compute_hash(path: &Path) -> Result<String, std::io::Error> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

fn parse_lock(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((file, hash)) = line.split_once(':') {
            map.insert(file.trim().to_string(), hash.trim().to_string());
        }
    }
    map
}

pub struct ConstraintsHashCheck;

impl ConstraintsHashCheck {
    pub fn update_lock() -> Result<(), std::io::Error> {
        let mut entries = Vec::new();
        for file in CONSTRAINT_FILES {
            let path = Path::new(file);
            let hash = compute_hash(path)?;
            entries.push(format!("{file}:{hash}"));
        }
        entries.sort();
        let content = entries.join("\n") + "\n";
        fs::write(LOCK_FILE, content)
    }
}

impl Check for ConstraintsHashCheck {
    fn name(&self) -> &'static str {
        "constraints integrity (hash check)"
    }

    fn blocking(&self) -> bool {
        true
    }

    fn run(&self) -> CheckResult {
        let lock_path = Path::new(LOCK_FILE);
        if !lock_path.exists() {
            return CheckResult {
                name: self.name(),
                passed: false,
                blocking: false,
                message: "constraints lock file not found; run `axm preflight --update-constraints` to generate it".into(),
            };
        }

        let lock_content = match fs::read_to_string(lock_path) {
            Ok(c) => c,
            Err(e) => {
                return CheckResult {
                    name: self.name(),
                    passed: false,
                    blocking: true,
                    message: format!("cannot read lock file: {e}"),
                }
            }
        };
        let expected = parse_lock(&lock_content);

        let mut mismatches = Vec::new();
        for file in CONSTRAINT_FILES {
            let path = Path::new(file);
            if !path.exists() {
                mismatches.push(format!("{file}: file missing"));
                continue;
            }
            match compute_hash(path) {
                Ok(hash) => match expected.get(*file) {
                    Some(expected_hash) if expected_hash == &hash => {}
                    Some(_) => mismatches.push(format!("{file}: hash mismatch (tampered?)")),
                    None => mismatches.push(format!("{file}: not in lock file")),
                },
                Err(e) => mismatches.push(format!("{file}: cannot read: {e}")),
            }
        }

        if mismatches.is_empty() {
            CheckResult {
                name: self.name(),
                passed: true,
                blocking: true,
                message: "all constraint files verified".into(),
            }
        } else {
            CheckResult {
                name: self.name(),
                passed: false,
                blocking: true,
                message: format!(
                    "constraint violations detected:\n    {}",
                    mismatches.join("\n    ")
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lock() {
        let content = "file1:abc123\nfile2:def456\n";
        let map = parse_lock(content);
        assert_eq!(map.get("file1").unwrap(), "abc123");
        assert_eq!(map.get("file2").unwrap(), "def456");
    }
}
