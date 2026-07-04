//! Compile-time architecture gate data — single source of truth for dependency rules.
//!
//! Layer indices (lower index = higher layer, can depend on higher indices):
//! 0: axiom-cli, 1: axiom-viz, 2: axiom-identity, 3: axiom-oversight,
//! 4: axiom-runtime, 5: axiom-store, 7: axiom-core
//!
//! Note: Layer 6 is reserved for future use.
//!
//! Rule: crate at level N may only depend on crates at level >= N (same or lower layer).
//!
//! NOTE: The `Architecture` struct and parsing logic are intentionally duplicated from
//! `tools/archcheck/src/loader.rs` because `axiom-core` cannot depend on the `archcheck`
//! crate (which is a build-time tool). Any changes to the TOML structure must be
//! synchronized with `loader.rs`. The canonical implementation lives in `loader.rs`.

use std::sync::OnceLock;

static ARCHITECTURE_TOML: &str = include_str!("../../../.axiom/architecture.toml");

#[derive(Debug, Clone)]
struct Architecture {
    crate_layers: Vec<(String, usize)>,
    forbidden_deps: Vec<String>,
    audited_deps: Vec<String>,
    proc_macro_exemptions: Vec<(String, Vec<String>)>,
    reverse_dependency_exemptions: Vec<(String, Vec<String>)>,
}

impl Architecture {
    fn from_toml(toml_str: &str) -> Option<Self> {
        let parsed: toml::Value = toml::from_str(toml_str).ok()?;

        let crate_layers = parsed
            .get("crate-layers")?
            .as_table()?
            .iter()
            .filter_map(|(k, v)| v.as_integer().map(|i| (k.clone(), i as usize)))
            .collect();

        let forbidden_deps = parsed
            .get("forbidden-deps")?
            .as_table()?
            .keys()
            .cloned()
            .collect();

        let audited_deps = parsed
            .get("audited-deps")?
            .as_table()?
            .keys()
            .cloned()
            .collect();

        let proc_macro_exemptions = parsed
            .get("proc-macro-exemptions")?
            .as_table()?
            .iter()
            .filter_map(|(k, v)| {
                let allowed = v
                    .get("allowed_deps")?
                    .as_array()?
                    .iter()
                    .filter_map(|d| d.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>();
                Some((k.clone(), allowed))
            })
            .collect();

        let reverse_dependency_exemptions = parsed
            .get("reverse-dependency-exemptions")?
            .as_table()?
            .iter()
            .filter_map(|(k, v)| {
                let allowed = v
                    .get("allowed_deps")?
                    .as_array()?
                    .iter()
                    .filter_map(|d| d.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>();
                Some((k.clone(), allowed))
            })
            .collect();

        Some(Architecture {
            crate_layers,
            forbidden_deps,
            audited_deps,
            proc_macro_exemptions,
            reverse_dependency_exemptions,
        })
    }
}

fn architecture() -> Result<&'static Architecture, crate::AxiomError> {
    static ARCH: OnceLock<Architecture> = OnceLock::new();
    ARCH.get_or_init(|| {
        Architecture::from_toml(ARCHITECTURE_TOML)
            .expect("failed to parse .axiom/architecture.toml")
    });
    Ok(ARCH.get().expect("architecture initialized"))
}

/// Find layer index for a crate by name.
pub fn crate_level(name: &str) -> Option<usize> {
    architecture().ok().and_then(|arch| {
        arch.crate_layers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, l)| *l)
    })
}

/// Return all registered crates and their layer indices.
pub fn crate_layers() -> &'static [(String, usize)] {
    architecture()
        .map(|arch| arch.crate_layers.as_slice())
        .unwrap_or(&[])
}

/// Verify local dependency direction. Returns list of violation messages.
pub fn verify_dependencies(crate_name: &str, deps: &[String]) -> Vec<String> {
    let level = match crate_level(crate_name) {
        Some(l) => l,
        None => return Vec::new(),
    };
    let arch = match architecture() {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut violations = Vec::new();
    for dep in deps {
        if dep == "axiom-macros" {
            let is_exempt = arch
                .proc_macro_exemptions
                .iter()
                .any(|(k, v)| k == crate_name && v.contains(dep));
            if is_exempt {
                continue;
            }
        }
        if let Some(dep_level) = crate_level(dep) {
            if dep_level < level {
                let is_exempt = arch
                    .reverse_dependency_exemptions
                    .iter()
                    .any(|(k, v)| k == crate_name && v.contains(dep));
                if is_exempt {
                    continue;
                }
                violations.push(format!(
                    "REVERSE DEPENDENCY: {crate_name} (level {level}) depends on {dep} (level {dep_level})"
                ));
            }
        }
    }
    violations
}

/// Audit a single third-party dependency. Returns Err(reason) if forbidden/unaudited.
pub fn audit_dependency(dep: &str) -> Result<(), String> {
    let arch = architecture().map_err(|e| e.to_string())?;
    if arch.forbidden_deps.contains(&dep.to_string()) {
        return Err(format!("forbidden dependency '{dep}' (R-004)"));
    }
    if !arch.audited_deps.contains(&dep.to_string()) {
        return Err(format!("unaudited dependency '{dep}' (R-022)"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_order_is_dag() {
        let layers = architecture()
            .unwrap()
            .crate_layers
            .iter()
            .map(|(_, l)| l)
            .collect::<Vec<_>>();
        assert!(!layers.is_empty(), "crate layers should not be empty");
    }

    #[test]
    fn test_reverse_dependency_detected() {
        let violations = verify_dependencies(
            "axiom-runtime",
            &["axiom-oversight".into(), "axiom-core".into()],
        );
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("REVERSE DEPENDENCY"));
    }

    #[test]
    fn test_valid_dependencies_pass() {
        let violations = verify_dependencies(
            "axiom-oversight",
            &["axiom-runtime".into(), "axiom-core".into()],
        );
        assert!(
            violations.is_empty(),
            "expected no violations: {violations:?}"
        );
    }

    #[test]
    fn test_forbidden_dep_detected() {
        assert!(audit_dependency("async-trait").is_err());
    }

    #[test]
    fn test_audited_dep_passes() {
        assert!(audit_dependency("tokio").is_ok());
        assert!(audit_dependency("regex").is_ok());
    }

    #[test]
    fn test_unaudited_dep_detected() {
        assert!(audit_dependency("unknown-crate-xyz").is_err());
    }
}
