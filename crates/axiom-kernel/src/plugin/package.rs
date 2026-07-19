use crate::plugin::abi::PluginError;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub version: String,
    pub description: Option<String>,
    pub kind: crate::plugin::abi::PluginKind,
    pub entry: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPackage {
    pub manifest: PluginManifest,
    pub wasm_bytes: Vec<u8>,
    pub signature: Option<Vec<u8>>,
    /// Native/package ABI version (P3-3).
    #[serde(default = "default_abi_version")]
    pub abi_version: u32,
}

fn default_abi_version() -> u32 {
    CURRENT_ABI_VERSION
}

/// Supported plugin package ABI.
pub const CURRENT_ABI_VERSION: u32 = 1;
/// Exported WASM symbol name for ABI (P1-5).
pub const WASM_ABI_VERSION_EXPORT: &str = "axiom_abi_version";

const AXIOM_PLUGIN_MAGIC: &[u8] = b"AXMP";
const AXIOM_PLUGIN_VERSION: u32 = 1;

pub fn check_abi_compatible(package_abi: u32) -> Result<(), PluginError> {
    if package_abi != CURRENT_ABI_VERSION {
        return Err(PluginError::LoadFailed(format!(
            "ABI mismatch: package={package_abi} host={CURRENT_ABI_VERSION}"
        )));
    }
    Ok(())
}

pub fn pack(
    manifest: PluginManifest,
    wasm_bytes: Vec<u8>,
    signature: Option<Vec<u8>>,
) -> Result<Vec<u8>, PluginError> {
    let package = PluginPackage {
        manifest,
        wasm_bytes,
        signature,
        abi_version: CURRENT_ABI_VERSION,
    };
    let json = serde_json::to_vec(&package).map_err(|e| PluginError::LoadFailed(e.to_string()))?;
    let mut out = Vec::new();
    out.extend_from_slice(AXIOM_PLUGIN_MAGIC);
    out.extend_from_slice(&AXIOM_PLUGIN_VERSION.to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&json);
    Ok(out)
}

pub fn unpack(data: &[u8]) -> Result<PluginPackage, PluginError> {
    if data.len() < 16 {
        return Err(PluginError::LoadFailed("package too short".into()));
    }
    if &data[0..4] != AXIOM_PLUGIN_MAGIC {
        return Err(PluginError::LoadFailed("invalid magic".into()));
    }
    let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    if version != AXIOM_PLUGIN_VERSION {
        return Err(PluginError::LoadFailed(format!("unsupported version: {version}")));
    }
    let json_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    if data.len() < 12 + json_len {
        return Err(PluginError::LoadFailed("package truncated".into()));
    }
    let json = &data[12..12 + json_len];
    let package: PluginPackage =
        serde_json::from_slice(json).map_err(|e| PluginError::LoadFailed(e.to_string()))?;
    check_abi_compatible(package.abi_version)?;
    Ok(package)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::abi::PluginKind;

    #[test]
    fn abi_mismatch_rejected() {
        assert!(check_abi_compatible(CURRENT_ABI_VERSION).is_ok());
        assert!(check_abi_compatible(99).is_err());
    }

    #[test]
    fn pack_unpack_roundtrip() {
        let m = PluginManifest {
            id: "p".into(),
            version: "0.1.0".into(),
            description: None,
            kind: PluginKind::Tool,
            entry: "e".into(),
            dependencies: vec![],
        };
        let bytes = pack(m, vec![1, 2, 3], None).unwrap();
        let pkg = unpack(&bytes).unwrap();
        assert_eq!(pkg.abi_version, CURRENT_ABI_VERSION);
    }
}

pub fn pack_to_file(
    manifest: PluginManifest,
    wasm_bytes: Vec<u8>,
    signature: Option<Vec<u8>>,
    path: &Path,
) -> Result<(), PluginError> {
    let bytes = pack(manifest, wasm_bytes, signature)?;
    std::fs::write(path, bytes).map_err(|e| PluginError::LoadFailed(e.to_string()))?;
    Ok(())
}

pub fn unpack_from_file(path: &Path) -> Result<PluginPackage, PluginError> {
    let data = std::fs::read(path).map_err(|e| PluginError::LoadFailed(e.to_string()))?;
    unpack(&data)
}
