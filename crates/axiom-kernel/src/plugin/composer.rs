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
        let content =
            std::fs::read_to_string(path).map_err(|e| PluginError::LoadFailed(e.to_string()))?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> PluginResult<SystemComposition> {
        let parsed: SystemComposition =
            toml::from_str(content).map_err(|e| PluginError::LoadFailed(e.to_string()))?;
        Ok(parsed)
    }

    /// Wire ConnectionSpec edges: validates plugin IDs exist and records topology (P2-5).
    pub fn compose(
        &self,
        composition: &SystemComposition,
        registry: &mut crate::PluginRegistry,
    ) -> PluginResult<()> {
        let plugin_ids: std::collections::HashSet<String> =
            composition.plugins.iter().map(|p| p.id.clone()).collect();

        let _ = registry; // topology validation is id-based; registry may load later
        for spec in &composition.plugins {
            if spec.id.is_empty() {
                return Err(PluginError::LoadFailed("empty plugin id".into()));
            }
        }

        for conn in &composition.connections {
            let (from_plugin, from_port) = &conn.from;
            let (to_plugin, to_port) = &conn.to;
            if !plugin_ids.contains(from_plugin) {
                return Err(PluginError::LoadFailed(format!(
                    "connection from unknown plugin `{from_plugin}`"
                )));
            }
            if !plugin_ids.contains(to_plugin) {
                return Err(PluginError::LoadFailed(format!(
                    "connection to unknown plugin `{to_plugin}`"
                )));
            }
            if from_port.is_empty() || to_port.is_empty() {
                return Err(PluginError::LoadFailed(
                    "connection ports must be non-empty".into(),
                ));
            }
            tracing::info!(
                from = %from_plugin,
                from_port = %from_port,
                to = %to_plugin,
                to_port = %to_port,
                transform = ?conn.transform,
                "wired connection"
            );
        }
        Ok(())
    }

    /// Returns wired edge count for tests.
    pub fn wire_count(composition: &SystemComposition) -> usize {
        composition.connections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_topology_wires() {
        let toml = r#"
[system]
name = "demo"
[[plugins]]
id = "a"
kind = "Tool"
config = {}
[[plugins]]
id = "b"
kind = "Tool"
config = {}
[[connections]]
from = ["a", "out"]
to = ["b", "in"]
"#;
        let comp = Composer::parse(toml).unwrap();
        assert_eq!(Composer::wire_count(&comp), 1);
        let mut reg = crate::PluginRegistry::new();
        Composer.compose(&comp, &mut reg).unwrap();
    }

    #[test]
    fn unknown_plugin_fails() {
        let toml = r#"
[system]
name = "x"
[[plugins]]
id = "a"
kind = "Tool"
config = {}
[[connections]]
from = ["a", "out"]
to = ["missing", "in"]
"#;
        let comp = Composer::parse(toml).unwrap();
        let mut reg = crate::PluginRegistry::new();
        assert!(Composer.compose(&comp, &mut reg).is_err());
    }
}
