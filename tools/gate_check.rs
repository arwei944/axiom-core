// tools/gate_check.rs — Shared build.rs gate check logic.
// Included via include!() from each crate's build.rs.
//
// Usage in build.rs:
//   fn main() {
//       gate_check("axiom-runtime");
//   }
//   include!("../../tools/gate_check.rs");

use std::fs;
use std::path::Path;

const CRATE_LAYERS: &[(&str, usize)] = &[
    ("axiom-cli", 0),
    ("axiom-viz", 1),
    ("axiom-agent", 2),
    ("axiom-oversight", 3),
    ("axiom-runtime", 4),
    ("axiom-store", 5),
    ("axiom-macros", 6),
    ("axiom-core", 7),
];

const FORBIDDEN_DEPS: &[&str] = &["async-trait"];

const AUDITED_DEPS: &[&str] = &[
    "tokio", "serde", "serde_json", "thiserror", "anyhow", "tracing",
    "tracing-subscriber", "sha2", "uuid", "futures", "clap", "ratatui",
    "crossterm", "syn", "quote", "proc-macro2", "linkme", "trybuild", "regex",
    "parking_lot",
];

fn crate_level_of(name: &str) -> Option<usize> {
    CRATE_LAYERS.iter().find(|(n, _)| *n == name).map(|(_, l)| *l)
}

fn parse_local_axiom_deps(cargo_toml: &Path) -> Vec<String> {
    let content = match fs::read_to_string(cargo_toml) {
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
    let content = match fs::read_to_string(cargo_toml) {
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
                if !dep_name.is_empty() && !dep_name.starts_with("axiom-") {
                    deps.push(dep_name.to_string());
                }
            }
        }
    }
    deps
}

#[allow(dead_code)]
fn gate_check(crate_name: &str) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cargo_toml = Path::new(manifest_dir).join("Cargo.toml");

    let local_deps = parse_local_axiom_deps(&cargo_toml);
    if let Some(level) = crate_level_of(crate_name) {
        for dep in &local_deps {
            if dep == "axiom-macros" {
                continue;
            }
            if let Some(dep_level) = crate_level_of(dep) {
                if dep_level < level {
                    panic!(
                        "\n\n\
                        ╔══════════════════════════════════════════════════════════════╗\n\
                        ║  ARCHITECTURE VIOLATION: REVERSE DEPENDENCY                 ║\n\
                        ╠══════════════════════════════════════════════════════════════╣\n\
                        ║  {crate_name:20} (level {level}) depends on                ║\n\
                        ║  {dep:20} (level {dep_level}) which is a HIGHER layer       ║\n\
                        ║                                                              ║\n\
                        ║  Rule: crates may only depend on same-level or lower-level   ║\n\
                        ║  crates (higher level index). See gate.rs CRATE_LAYERS.      ║\n\
                        ╚══════════════════════════════════════════════════════════════╝\n\n",
                    );
                }
            }
        }
    }

    let third_party = parse_third_party_deps(&cargo_toml);
    for dep in &third_party {
        if FORBIDDEN_DEPS.contains(&dep.as_str()) {
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
        if !AUDITED_DEPS.contains(&dep.as_str()) {
            panic!(
                "\n\n\
                ╔══════════════════════════════════════════════════════════════╗\n\
                ║  UNAUDITED DEPENDENCY                                        ║\n\
                ╠══════════════════════════════════════════════════════════════╣\n\
                ║  '{dep}' has not been audited (R-022).                      ║\n\
                ║  Either:                                                     ║\n\
                ║  1. Add it to AUDITED_DEPS in gate.rs if reviewed           ║\n\
                ║  2. Remove it if unnecessary                                ║\n\
                ╚══════════════════════════════════════════════════════════════╝\n\n",
            );
        }
    }

    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=../../tools/gate_check.rs");
}
