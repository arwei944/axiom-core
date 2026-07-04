use std::process::Command;

fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let output = match Command::new(&rustc).arg("--version").output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Warning: failed to run rustc --version: {}", e);
            archcheck::build_hook::check_current_crate(env!("CARGO_PKG_NAME"));
            return;
        }
    };

    let version_str = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Warning: rustc --version output is not valid UTF-8");
            archcheck::build_hook::check_current_crate(env!("CARGO_PKG_NAME"));
            return;
        }
    };

    let min_version = (1, 75, 0);
    let parsed = parse_rustc_version(&version_str);
    match parsed {
        Some((major, minor, patch)) if (major, minor, patch) >= min_version => {}
        Some((major, minor, patch)) => {
            panic!(
                "axiom-core requires Rust >= {}.{}.{} (found {}.{}.{}). \
                 axiom-core uses native async fn in traits stabilized in Rust 1.75.",
                min_version.0, min_version.1, min_version.2, major, minor, patch
            );
        }
        None => {
            eprintln!(
                "Warning: could not parse rustc version from '{}'. \
                 Build may fail if rustc < 1.75.",
                version_str.trim()
            );
        }
    }

    archcheck::build_hook::check_current_crate(env!("CARGO_PKG_NAME"));
}

fn parse_rustc_version(output: &str) -> Option<(u32, u32, u32)> {
    let rest = output.strip_prefix("rustc ")?;
    let ver_str = rest.split_whitespace().next()?;
    let mut parts = ver_str.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    let patch_part = parts.next()?;
    let patch: u32 = patch_part
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    Some((major, minor, patch))
}
