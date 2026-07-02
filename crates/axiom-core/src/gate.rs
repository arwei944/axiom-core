//! Compile-time architecture gate data — single source of truth for dependency rules.
//!
//! Layer indices (lower index = higher layer, can depend on higher indices):
//! 0: axiom-cli, 1: axiom-viz, 2: axiom-agent, 3: axiom-oversight,
//! 4: axiom-runtime, 5: axiom-store, 6: axiom-macros, 7: axiom-core
//!
//! Rule: crate at level N may only depend on crates at level >= N (same or lower layer).

/// (crate_name, layer_index). Lower index = higher layer.
pub const CRATE_LAYERS: &[(&str, usize)] = &[
    ("axiom-cli", 0),
    ("axiom-viz", 1),
    ("axiom-agent", 2),
    ("axiom-oversight", 3),
    ("axiom-runtime", 4),
    ("axiom-store", 5),
    ("axiom-macros", 6),
    ("axiom-core", 7),
];

/// Third-party dependencies that are FORBIDDEN in any axiom crate.
/// R-004: async-trait is banned (Rust 1.75+ supports native async fn in traits).
pub const FORBIDDEN_DEPS: &[&str] = &["async-trait"];

/// Third-party dependencies that have been audited and are allowed.
pub const AUDITED_DEPS: &[&str] = &[
    "tokio",
    "serde",
    "serde_json",
    "thiserror",
    "anyhow",
    "tracing",
    "tracing-subscriber",
    "sha2",
    "uuid",
    "futures",
    "clap",
    "ratatui",
    "crossterm",
    "syn",
    "quote",
    "proc-macro2",
    "linkme",
    "trybuild",
    "regex",
    "parking_lot",
];

/// Find layer index for a crate by name.
pub fn crate_level(name: &str) -> Option<usize> {
    CRATE_LAYERS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, l)| *l)
}

/// Verify local dependency direction. Returns list of violation messages.
pub fn verify_dependencies(crate_name: &str, deps: &[String]) -> Vec<String> {
    let level = match crate_level(crate_name) {
        Some(l) => l,
        None => return Vec::new(),
    };
    let mut violations = Vec::new();
    for dep in deps {
        if dep == "axiom-macros" {
            continue;
        }
        if let Some(dep_level) = crate_level(dep) {
            if dep_level < level {
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
    if FORBIDDEN_DEPS.contains(&dep) {
        return Err(format!("forbidden dependency '{dep}' (R-004)"));
    }
    if !AUDITED_DEPS.contains(&dep) {
        return Err(format!("unaudited dependency '{dep}' (R-022)"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_order_is_dag() {
        for (name, level) in CRATE_LAYERS {
            assert!(*level < CRATE_LAYERS.len(), "level out of range for {name}");
        }
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
