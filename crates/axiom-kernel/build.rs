use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".into());
    let parts: Vec<u16> = version.split('.').filter_map(|part| part.parse::<u16>().ok()).collect();

    let major = parts.first().copied().unwrap_or(0);
    let minor = parts.get(1).copied().unwrap_or(1);
    let patch = parts.get(2).copied().unwrap_or(0);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("version_generated.rs");
    let content = format!(
        "impl Version {{ pub const CURRENT: Self = Self::new({major}, {minor}, {patch}); }}\n"
    );

    fs::write(&dest_path, content).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
