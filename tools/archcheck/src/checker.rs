use crate::loader::Architecture;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

static DEP_RE: Lazy<Regex> =
    // foxguard: ignore[rs/no-unwrap-in-lib] — compile-time constants must be valid
    Lazy::new(|| Regex::new(r#"^[a-zA-Z0-9_-]+"#).expect("valid dep regex"));
static SECTION_RE: Lazy<Regex> =
    // foxguard: ignore[rs/no-unwrap-in-lib] — compile-time constants must be valid
    Lazy::new(|| Regex::new(r#"^\[(.+?)\]$"#).expect("valid section regex"));

#[derive(Debug, Clone)]
pub struct Violation {
    pub severity: Severity,
    pub category: String,
    pub message: String,
    pub file: Option<std::path::PathBuf>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Blocker,
    Warning,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Blocker => write!(f, "BLOCKER"),
            Severity::Warning => write!(f, "WARNING"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CrateManifest {
    pub name: String,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: HashMap<String, String>,
    pub build_dependencies: HashMap<String, String>,
}

pub fn check_all(arch: &Architecture, workspace_path: impl AsRef<Path>) -> Vec<Violation> {
    let workspace = workspace_path.as_ref();
    let mut violations = Vec::new();

    let manifests = collect_manifests(workspace);
    for manifest in manifests.values() {
        violations.extend(check_crate_registered(arch, &manifest.name));
        violations.extend(check_dependencies(arch, manifest, Severity::Blocker));
        violations.extend(check_build_dependencies(arch, manifest, Severity::Blocker));
        if arch.dev_dep_audit_enabled {
            violations.extend(check_dev_dependencies(arch, manifest, Severity::Blocker));
        }
    }

    violations
}

pub fn check_crate_registered(arch: &Architecture, crate_name: &str) -> Vec<Violation> {
    if !arch.crate_layers.contains_key(crate_name) {
        return vec![Violation {
            severity: Severity::Blocker,
            category: "unregistered-crate".into(),
            message: format!(
                "crate '{}' is not registered in [crate-layers]. \
                 Run: cargo xtask new_crate --name {} --layer <0-8>",
                crate_name,
                crate_name.strip_prefix("axiom-").unwrap_or(crate_name)
            ),
            file: None,
            line: None,
        }];
    }
    Vec::new()
}

pub fn check_dependencies(
    arch: &Architecture,
    manifest: &CrateManifest,
    _severity: Severity,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    for dep_name in manifest.dependencies.keys() {
        if dep_name.starts_with("axiom-") {
            violations.extend(check_internal_dep(arch, &manifest.name, dep_name));
        } else {
            violations.extend(check_third_party_dep(arch, &manifest.name, dep_name));
        }
    }

    violations
}

pub fn check_build_dependencies(
    arch: &Architecture,
    manifest: &CrateManifest,
    _severity: Severity,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    for dep_name in manifest.build_dependencies.keys() {
        if dep_name.starts_with("axiom-") {
            violations.extend(check_internal_dep(arch, &manifest.name, dep_name));
        } else {
            violations.extend(check_third_party_dep(arch, &manifest.name, dep_name));
        }
    }

    violations
}

pub fn check_dev_dependencies(
    arch: &Architecture,
    manifest: &CrateManifest,
    _severity: Severity,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    for dep_name in manifest.dev_dependencies.keys() {
        if dep_name.starts_with("axiom-") {
            continue;
        }
        violations.extend(check_third_party_dep(arch, &manifest.name, dep_name));
    }

    violations
}

fn check_internal_dep(arch: &Architecture, crate_name: &str, dep_name: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    let crate_layer = match arch.crate_layers.get(crate_name) {
        Some(&l) => l,
        None => return violations,
    };
    let dep_layer = match arch.crate_layers.get(dep_name) {
        Some(&l) => l,
        None => return violations,
    };

    if dep_layer < crate_layer {
        let is_exempt = arch
            .proc_macro_exemptions
            .get(crate_name)
            .map(|e| e.allowed_deps.contains(&dep_name.to_string()))
            .unwrap_or(false)
            || arch
                .reverse_dependency_exemptions
                .get(crate_name)
                .map(|e| e.allowed_deps.contains(&dep_name.to_string()))
                .unwrap_or(false);

        if !is_exempt {
            violations.push(Violation {
                severity: Severity::Blocker,
                category: "reverse-dependency".into(),
                message: format!(
                    "crate '{}' (layer {}) reverse-depends on '{}' (layer {})",
                    crate_name, crate_layer, dep_name, dep_layer
                ),
                file: None,
                line: None,
            });
        }
    }

    violations
}

fn check_third_party_dep(arch: &Architecture, crate_name: &str, dep_name: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    if let Some(reason) = arch.forbidden_deps.get(dep_name) {
        violations.push(Violation {
            severity: Severity::Blocker,
            category: "forbidden-dependency".into(),
            message: format!(
                "crate '{}' depends on forbidden dependency '{}'. Reason: {}",
                crate_name, dep_name, reason
            ),
            file: None,
            line: None,
        });
        return violations;
    }

    if !arch.audited_deps.contains_key(dep_name) {
        violations.push(Violation {
            severity: Severity::Blocker,
            category: "unaudited-dependency".into(),
            message: format!(
                "crate '{}' depends on un-audited dependency '{}'. \
                 Add to .axiom/architecture.toml [audited-deps] or remove dependency.",
                crate_name, dep_name
            ),
            file: None,
            line: None,
        });
    }

    violations
}

fn collect_manifests(workspace: &Path) -> HashMap<String, CrateManifest> {
    let mut manifests = HashMap::new();
    if !workspace.exists() {
        return manifests;
    }

    let entries = match std::fs::read_dir(workspace.join("crates")) {
        Ok(e) => e,
        Err(_) => return manifests,
    };

    let dep_re = &DEP_RE;
    let section_re = &SECTION_RE;

    for entry in entries.flatten() {
        let manifest_path = entry.path().join("Cargo.toml");
        if !manifest_path.exists() {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            let mut name = String::new();
            let mut deps = HashMap::new();
            let mut dev_deps = HashMap::new();
            let mut build_deps = HashMap::new();
            let mut current_section = "";
            let mut in_package_section = false;

            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || trimmed.is_empty() {
                    continue;
                }

                if let Some(caps) = section_re.captures(trimmed) {
                    let section = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    current_section = section;
                    in_package_section = section == "package";
                    continue;
                }

                if in_package_section && trimmed.starts_with("name =") {
                    if let Some(n) = trimmed.split('=').nth(1) {
                        name = n.trim().trim_matches('"').to_string();
                    }
                    continue;
                }

                if let Some(dep_name) = dep_re.find(trimmed).map(|m| m.as_str().to_string()) {
                    if dep_name.is_empty()
                        || dep_name == "package"
                        || dep_name == "version"
                        || dep_name == "edition"
                    {
                        continue;
                    }
                    match current_section {
                        "dependencies" => {
                            deps.insert(dep_name, String::new());
                        }
                        "dev-dependencies" => {
                            dev_deps.insert(dep_name, String::new());
                        }
                        "build-dependencies" => {
                            build_deps.insert(dep_name, String::new());
                        }
                        _ => {}
                    }
                }
            }

            if !name.is_empty() {
                manifests.insert(
                    name.clone(),
                    CrateManifest {
                        name,
                        dependencies: deps,
                        dev_dependencies: dev_deps,
                        build_dependencies: build_deps,
                    },
                );
            }
        }
    }

    manifests
}
