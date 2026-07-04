use std::path::Path;

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cargo_toml = Path::new(manifest_dir).join("Cargo.toml"); // foxguard: ignore[rs/no-path-traversal]

    let content = match std::fs::read_to_string(&cargo_toml) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "warning: cannot read archcheck Cargo.toml for self-check: {}",
                e
            );
            return;
        }
    };

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
                if dep_name == "async-trait" {
                    panic!(
                        "\n\n\
                        ╔══════════════════════════════════════════════════════════════╗\n\
                        ║  FORBIDDEN DEPENDENCY (SELF-CHECK)                         ║\n\
                        ╠══════════════════════════════════════════════════════════════╣\n\
                        ║  'async-trait' is FORBIDDEN in archcheck itself (R-004).   ║\n\
                        ║  Remove this dependency from tools/archcheck/Cargo.toml.   ║\n\
                        ╚══════════════════════════════════════════════════════════════╝\n\n"
                    );
                }
            }
        }
    }

    println!("cargo:rerun-if-changed=Cargo.toml");
}
