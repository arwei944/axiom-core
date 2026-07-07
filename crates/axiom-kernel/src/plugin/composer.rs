use crate::plugin::abi::{PluginError, PluginKind, PluginResult};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct SystemComposition {
    pub system: SystemSpec,
    pub plugins: Vec<PluginSpec>,
    pub connections: Vec<ConnectionSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SystemSpec {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginSpec {
    pub id: String,
    pub kind: PluginKind,
    pub config: toml::Value,
    #[serde(default)]
    pub instance: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionSpec {
    pub from: (String, String),
    pub to: (String, String),
    #[serde(default)]
    pub transform: Option<String>,
}

pub struct Composer;

impl Composer {
    pub fn from_file(path: impl AsRef<Path>) -> PluginResult<SystemComposition> {
        let content = std::fs::read_to_string(path).map_err(|e| PluginError::LoadFailed(e.to_string()))?;
        Self::from_str(&content)
    }

    pub fn from_str(content: &str) -> PluginResult<SystemComposition> {
        let parsed: SystemComposition =
            toml::from_str(content).map_err(|e| PluginError::LoadFailed(e.to_string()))?;
        Ok(parsed)
    }

    pub fn compose(&self, _composition: &SystemComposition, _registry: &mut crate::PluginRegistry) -> PluginResult<()> {
        for spec in &_composition.plugins {
            let _ = spec.id.clone();
        }
        for conn in &_composition.connections {
            let _ = conn.from;
            let _ = conn.to;
        }
        Ok(())
    }
}
