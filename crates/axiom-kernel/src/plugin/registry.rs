use crate::plugin::abi::{PluginError, PluginKind, PluginResult};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Box<dyn crate::plugin::abi::AxiomPlugin>>>,
    kinds: RwLock<HashMap<PluginKind, Vec<String>>>,
    /// Refcount for hot-reload (P3-2).
    refcounts: RwLock<HashMap<String, u32>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            kinds: RwLock::new(HashMap::new()),
            refcounts: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, plugin: Box<dyn crate::plugin::abi::AxiomPlugin>) {
        let id = plugin.id().to_string();
        let kind = Self::detect_plugin_kind(plugin.capabilities());
        let mut kinds = self.kinds.write().await;
        kinds.entry(kind).or_default().push(id.clone());
        let mut plugins = self.plugins.write().await;
        plugins.insert(id.clone(), plugin);
        let mut rc = self.refcounts.write().await;
        *rc.entry(id).or_insert(0) += 1;
    }

    /// Hot-upgrade: replace instance when refcount allows (P3-2).
    pub async fn upgrade(
        &self,
        plugin: Box<dyn crate::plugin::abi::AxiomPlugin>,
    ) -> PluginResult<()> {
        let id = plugin.id().to_string();
        let rc = self.refcounts.read().await.get(&id).copied().unwrap_or(0);
        if rc > 1 {
            return Err(PluginError::LoadFailed(format!(
                "plugin {id} still has {rc} refs; drain before upgrade"
            )));
        }
        self.register(plugin).await;
        Ok(())
    }

    pub async fn acquire(&self, id: &str) -> bool {
        let plugins = self.plugins.read().await;
        if !plugins.contains_key(id) {
            return false;
        }
        drop(plugins);
        let mut rc = self.refcounts.write().await;
        *rc.entry(id.to_string()).or_insert(0) += 1;
        true
    }

    pub async fn release(&self, id: &str) {
        let mut rc = self.refcounts.write().await;
        if let Some(c) = rc.get_mut(id) {
            *c = c.saturating_sub(1);
        }
    }

    fn detect_plugin_kind(capabilities: &[crate::plugin::abi::CapabilityDescriptor]) -> PluginKind {
        let cap_names: Vec<&str> = capabilities.iter().map(|c| c.name.as_str()).collect();
        if cap_names.iter().any(|n| {
            n.contains("llm") || n.contains("chat") || n.contains("gpt") || n.contains("claude")
        }) {
            PluginKind::Llm
        } else if cap_names.iter().any(|n| n.contains("memory") || n.contains("storage")) {
            PluginKind::Memory
        } else if cap_names.iter().any(|n| n.contains("tool") || n.contains("function")) {
            PluginKind::Tool
        } else if cap_names.iter().any(|n| n.contains("mcp") || n.contains("model")) {
            PluginKind::Mcp
        } else if cap_names.iter().any(|n| n.contains("plan") || n.contains("task")) {
            PluginKind::Planner
        } else if cap_names.iter().any(|n| n.contains("alert") || n.contains("notify")) {
            PluginKind::Alert
        } else if cap_names
            .iter()
            .any(|n| n.contains("viz") || n.contains("visual") || n.contains("graph"))
        {
            PluginKind::Viz
        } else if cap_names
            .iter()
            .any(|n| n.contains("govern") || n.contains("entropy") || n.contains("policy"))
        {
            PluginKind::Governance
        } else {
            PluginKind::Llm
        }
    }

    pub async fn get(&self, id: &str) -> Option<Box<dyn crate::plugin::abi::AxiomPlugin>> {
        let plugins = self.plugins.read().await;
        plugins.get(id).map(|p| p.clone_box())
    }

    pub async fn remove(&self, id: &str) -> Option<Box<dyn crate::plugin::abi::AxiomPlugin>> {
        let mut plugins = self.plugins.write().await;
        plugins.remove(id)
    }

    pub async fn get_all_by_kind(
        &self,
        kind: PluginKind,
    ) -> Vec<Box<dyn crate::plugin::abi::AxiomPlugin>> {
        let kinds = self.kinds.read().await;
        let plugins = self.plugins.read().await;
        if let Some(ids) = kinds.get(&kind) {
            ids.iter().filter_map(|id| plugins.get(id).map(|p| p.clone_box())).collect()
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

    fn resolve(
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
