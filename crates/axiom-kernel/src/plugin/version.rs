use crate::plugin::abi::PluginError;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVersion {
    pub id: String,
    pub version: semver::Version,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub id: String,
    pub version_req: semver::VersionReq,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryIndex {
    pub plugins: Vec<PluginVersion>,
}

impl Default for RepositoryIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl RepositoryIndex {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn add(&mut self, plugin: PluginVersion) {
        self.plugins.push(plugin);
    }

    pub fn resolve(&self, id: &str, version_req: &semver::VersionReq) -> Option<&PluginVersion> {
        self.plugins.iter().find(|p| p.id == id && version_req.matches(&p.version))
    }
}

pub fn load_index(path: &Path) -> Result<RepositoryIndex, PluginError> {
    let data = std::fs::read_to_string(path).map_err(|e| PluginError::LoadFailed(e.to_string()))?;
    let index: RepositoryIndex =
        serde_json::from_str(&data).map_err(|e| PluginError::LoadFailed(e.to_string()))?;
    Ok(index)
}
