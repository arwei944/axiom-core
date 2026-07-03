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
            if let Some(dep_name) = trimmed.split(|c: char| c.is_whitespace() || c == '=').next() {
                if dep_name.starts_with("axiom-") {
                    deps.push(dep_name.to_string());
                }
            }
        }
    }
    deps
}

fn parse_third_party_deps(cargo_toml: &Path) -> Vec<String> {
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
        if (section == "dependencies" || section == "build-dependencies" || section == "dev-dependencies")
            && !trimmed.is_empty()
            && !trimmed.starts_with('#')
        {
            if let Some(dep_name) = trimmed.split(|c: char| c.is_whitespace() || c == '=').next() {
                if !dep_name.is_empty() && !dep_name.starts_with("axiom-") {
                    deps.push(dep_name.to_string());
                }
            }
        }
    }
    deps
}

/// Check current crate architecture compliance at compile time.
///
/// This function should be called from each crate's `build.rs` to enforce
/// architecture rules. It will panic with a descriptive message if violations are found.
pub fn check_current_crate(crate_name: &str) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cargo_toml = Path::new(manifest_dir).join("Cargo.toml");

    let local_deps = parse_local_axiom_deps(&cargo_toml);
    if let Some(level) = crate_level_of(crate_name) {
        let arch = architecture();
        for dep in &local_deps {
            if dep == "axiom-macros" {
                let is_exempt = arch.proc_macro_exemptions.iter()
                    .any(|(k, v)| k == crate_name && v.allowed_deps.contains(dep));
                if is_exempt {
                    continue;
                }
            }
            if let Some(dep_level) = crate_level_of(dep) {
                if dep_level < level {
                    let is_exempt = arch.reverse_dependency_exemptions.iter()
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
                        ╚══════════════════════════════════════════════════════════════╝\n\n",
                    );
                }
            }
        }
    }

    let third_party = parse_third_party_deps(&cargo_toml);
    let arch = architecture();
    for dep in &third_party {
        if arch.forbidden_deps.contains_key(dep) {
            panic!(
                "\n\n\
                ╔══════════════════════════════════════════════════════════════╗\n\
                ║  FORBIDDEN DEPENDENCY                                        ║\n\
                ╠══════════════════════════════════════════════════════════════╣\n\
                ║  '{dep}' is FORBIDDEN in axiom crates (R-004).              ║\n\
                ║  Reason: Rust 1.75+ supports native async fn in traits.     ║\n\
                ║  Remove this dependency from Cargo.toml.                    ║\n\
                ╚══════════════════════════════════════════════════════════════╝\n\n",
            );
        }
        if !arch.audited_deps.contains_key(dep) {
            panic!(
                "\n\n\
                ╔══════════════════════════════════════════════════════════════╗\n\
                ║  UNAUDITED DEPENDENCY                                        ║\n\
                ╠══════════════════════════════════════════════════════════════╣\n\
                ║  '{dep}' has not been audited (R-022).                      ║\n\
                ║  Either:                                                     ║\n\
                ║  1. Add it to audited-deps in .axiom/architecture.toml      ║\n\
                ║  2. Remove it if unnecessary                                ║\n\
                ╚══════════════════════════════════════════════════════════════╝\n\n",
            );
        }
    }

    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=../../tools/archcheck/src/build_hook.rs");
}
