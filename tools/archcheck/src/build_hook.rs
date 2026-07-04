//! Build hook for compile-time architecture enforcement.
//!
//! This module provides `check_current_crate()` which can be called from
//! any crate's `build.rs` to enforce architecture rules at compile time.
//!
//! Usage in build.rs:
//!   fn main() {
//!       archcheck::build_hook::check_current_crate("axiom-runtime");
//!   }

use std::env;
use std::path::Path;
use std::sync::OnceLock;

use crate::loader::Architecture;

static ARCHITECTURE_TOML: &str = include_str!("../../../.axiom/architecture.toml");

fn architecture() -> &'static Architecture {
    static ARCH: OnceLock<Architecture> = OnceLock::new();
    ARCH.get_or_init(|| {
        // foxguard: ignore[rs/no-unwrap-in-lib] — architecture TOML is bundled with the
        // crate; a parse failure here indicates a corrupted shipped resource.
        Architecture::from_toml_str(ARCHITECTURE_TOML)
            .expect("TOML parse error in ../../../.axiom/architecture.toml")
    })
}

fn crate_level_of(name: &str) -> Option<usize> {
    architecture()
        .crate_layers
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, l)| *l)
}

fn parse_local_axiom_deps(cargo_toml: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(cargo_toml) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    let mut section = "";
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
            continue;
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
    deps
}

fn parse_third_party_deps(cargo_toml: &Path) -> Vec<(String, String)> {
    let content = match std::fs::read_to_string(cargo_toml) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    let mut section = "";
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
            continue;
        }
        if (section == "dependencies"
            || section == "build-dependencies"
            || section == "dev-dependencies")
            && !trimmed.is_empty()
            && !trimmed.starts_with('#')
        {
            if let Some(dep_name) = trimmed
                .split(|c: char| c.is_whitespace() || c == '=')
                .next()
            {
                if !dep_name.is_empty() && !dep_name.starts_with("axiom-") {
                    deps.push((dep_name.to_string(), format!("Cargo.toml:{}", line_num + 1)));
                }
            }
        }
    }
    deps
}

fn check_dev_dependency_audit_enabled() -> bool {
    architecture().dev_dep_audit_enabled
}

/// Check current crate architecture compliance at compile time.
///
/// This function should be called from each crate's `build.rs` to enforce
/// architecture rules. It will panic with a descriptive message if violations are found.
pub fn check_current_crate(crate_name: &str) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cargo_toml = Path::new(manifest_dir).join("Cargo.toml"); // foxguard: ignore[rs/no-path-traversal]

    // Check [dependencies] and [build-dependencies] for internal deps
    let local_deps = parse_local_axiom_deps(&cargo_toml);
    if let Some(level) = crate_level_of(crate_name) {
        let arch = architecture();
        for dep in &local_deps {
            // Skip self
            if dep == crate_name {
                continue;
            }

            // Check proc-macro exemption
            if dep == "axiom-macros" {
                let is_exempt = arch
                    .proc_macro_exemptions
                    .iter()
                    .any(|(k, v)| k == crate_name && v.allowed_deps.contains(dep));
                if is_exempt {
                    continue;
                }
            }

            // Check reverse dependency exemption
            if let Some(dep_level) = crate_level_of(dep) {
                if dep_level < level {
                    let is_exempt = arch
                        .reverse_dependency_exemptions
                        .iter()
                        .any(|(k, v)| k == crate_name && v.allowed_deps.contains(dep));
                    if is_exempt {
                        continue;
                    }
                    panic!(
                        "\n\n\
                        ╔══════════════════════════════════════════════════════════════╗\n\
                        ║  ARCHITECTURE VIOLATION: REVERSE DEPENDENCY                 ║\n\
                        ╠══════════════════════════════════════════════════════════════╣\n\
                        ║  {crate_name:20} (level {level}) depends on                ║\n\
                        ║  {dep:20} (level {dep_level}) which is a HIGHER layer       ║\n\
                        ║                                                              ║\n\
                        ║  Rule: crates may only depend on same-level or lower-level   ║\n\
                        ║  crates (higher level index). See .axiom/architecture.toml.  ║\n\
                        ║                                                              ║\n\
                        ║  Fix:                                                        ║\n\
                        ║  1. Remove the dependency if possible                        ║\n\
                        ║  2. Add a reverse-dependency exemption in architecture.toml   ║\n\
                        ╚══════════════════════════════════════════════════════════════╝\n\n",
                    );
                }
            }
        }
    }

    // Check third-party deps in [dependencies] and [build-dependencies]
    let third_party = parse_third_party_deps(&cargo_toml);
    let arch = architecture();
    for (dep, location) in &third_party {
        if arch.forbidden_deps.contains_key(dep) {
            let reason = arch
                .forbidden_deps
                .get(dep)
                .map_or("No reason provided", |v| v.as_str());
            panic!(
                "\n\n\
                ╔══════════════════════════════════════════════════════════════╗\n\
                ║  FORBIDDEN DEPENDENCY                                        ║\n\
                ╠══════════════════════════════════════════════════════════════╣\n\
                ║  '{dep}' is FORBIDDEN in axiom crates.                       ║\n\
                ║  Location: {location:45} ║\n\
                ║  Reason: {reason:49} ║\n\
                ║                                                              ║\n\
                ║  Fix: Remove this dependency from Cargo.toml.                 ║\n\
                ╚══════════════════════════════════════════════════════════════╝\n\n",
            );
        }
        if !arch.audited_deps.contains_key(dep) {
            panic!(
                "\n\n\
                ╔══════════════════════════════════════════════════════════════╗\n\
                ║  UNAUDITED DEPENDENCY                                        ║\n\
                ╠══════════════════════════════════════════════════════════════╣\n\
                ║  '{dep}' has not been audited (R-022).                       ║\n\
                ║  Location: {location:45} ║\n\
                ║                                                              ║\n\
                ║  Either:                                                     ║\n\
                ║  1. Add it to audited-deps in .axiom/architecture.toml      ║\n\
                ║  2. Remove it if unnecessary                                 ║\n\
                ╚══════════════════════════════════════════════════════════════╝\n\n",
            );
        }
    }

    // Check [dev-dependencies] if audit is enabled
    if check_dev_dependency_audit_enabled() {
        // Re-parse to get only dev-dependencies
        let content = match std::fs::read_to_string(&cargo_toml) {
            Ok(c) => c,
            Err(_) => return,
        };
        let mut dev_dep_names = Vec::new();
        let mut section = "";
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
                continue;
            }
            if section == "dev-dependencies" && !trimmed.is_empty() && !trimmed.starts_with('#') {
                if let Some(dep_name) = trimmed
                    .split(|c: char| c.is_whitespace() || c == '=')
                    .next()
                {
                    if !dep_name.is_empty() && !dep_name.starts_with("axiom-") {
                        dev_dep_names.push(dep_name.to_string());
                    }
                }
            }
        }

        for dep in &dev_dep_names {
            if arch.forbidden_deps.contains_key(dep) {
                panic!(
                    "\n\n\
                    ╔══════════════════════════════════════════════════════════════╗\n\
                    ║  FORBIDDEN DEV-DEPENDENCY                                    ║\n\
                    ╠══════════════════════════════════════════════════════════════╣\n\
                    ║  '{dep}' is FORBIDDEN in dev-dependencies.                  ║\n\
                    ║  Remove it from [dev-dependencies] in Cargo.toml.            ║\n\
                    ╚══════════════════════════════════════════════════════════════╝\n\n",
                );
            }
            if !arch.audited_deps.contains_key(dep) {
                panic!(
                    "\n\n\
                    ╔══════════════════════════════════════════════════════════════╗\n\
                    ║  UNAUDITED DEV-DEPENDENCY                                    ║\n\
                    ╠══════════════════════════════════════════════════════════════╣\n\
                    ║  '{dep}' has not been audited in dev-dependencies (R-022).   ║\n\
                    ║                                                              ║\n\
                    ║  Either:                                                     ║\n\
                    ║  1. Add it to audited-deps in .axiom/architecture.toml      ║\n\
                    ║  2. Remove it if unnecessary                                 ║\n\
                    ║  3. Disable dev-dependency audit in architecture.toml        ║\n\
                    ╚══════════════════════════════════════════════════════════════╝\n\n",
                );
            }
        }
    }

    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=../../tools/archcheck/src/build_hook.rs");
}
