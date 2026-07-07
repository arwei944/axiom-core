use crate::plugin::abi::{PluginError, PluginKind, PluginResult};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Box<dyn crate::plugin::abi::AxiomPlugin>>>,
    kinds: RwLock<HashMap<PluginKind, Vec<String>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            kinds: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, plugin: Box<dyn crate::plugin::abi::AxiomPlugin>) {
        let id = plugin.id().to_string();
        let kind = PluginKind::Llm; // ponytail: default kind; real impl should detect from capabilities
        let mut kinds = self.kinds.write().await;
        kinds.entry(kind).or_default().push(id.clone());
        let mut plugins = self.plugins.write().await;
        plugins.insert(id, plugin);
    }

    pub async fn get(&self, id: &str) -> Option<Box<dyn crate::plugin::abi::AxiomPlugin>> {
        let mut plugins = self.plugins.write().await;
        plugins.remove(id)
    }

    pub async fn remove(&self, id: &str) -> Option<Box<dyn crate::plugin::abi::AxiomPlugin>> {
        let mut plugins = self.plugins.write().await;
        plugins.remove(id)
    }

    pub async fn get_all_by_kind(&self, kind: PluginKind) -> Vec<Box<dyn crate::plugin::abi::AxiomPlugin>> {
        let kinds = self.kinds.read().await;
        let plugins = self.plugins.read().await;
        if let Some(ids) = kinds.get(&kind) {
            ids.iter()
                .filter_map(|id| plugins.get(id).map(|p| p.clone_box()))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub async fn list_all(&self) -> Vec<Box<dyn crate::plugin::abi::AxiomPlugin>> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.clone_box()).collect()
    }

    pub async fn dependencies_resolved(&self, id: &str) -> bool {
        let plugins = self.plugins.read().await;
        let mut visited = HashSet::new();
        self.resolve(id, &mut visited, &mut Vec::new(), &plugins).is_ok()
    }

    pub async fn resolve_dependencies(&self) -> PluginResult<()> {
        let plugins = self.plugins.read().await;
        for (id, _) in plugins.iter() {
            let mut visited = HashSet::new();
            self.resolve(id, &mut visited, &mut Vec::new(), &plugins)?;
        }
        Ok(())
    }

    fn resolve<'a>(
        &self,
        id: &str,
        visited: &mut HashSet<String>,
        stack: &mut Vec<String>,
        plugins: &std::collections::HashMap<String, Box<dyn crate::plugin::abi::AxiomPlugin>>,
    ) -> PluginResult<()> {
        if stack.contains(&id.to_string()) {
            return Err(PluginError::DependencyCycle(id.to_string()));
        }
        if !visited.insert(id.to_string()) {
            return Ok(());
        }
        stack.push(id.to_string());
        if let Some(plugin) = plugins.get(id) {
            for dep in plugin.dependencies() {
                if !plugins.contains_key(*dep) {
                    return Err(PluginError::DependencyMissing(dep.to_string()));
                }
                self.resolve(dep, visited, stack, plugins)?;
            }
        }
        stack.pop();
        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
