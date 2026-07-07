use axiom_kernel::plugin::abi::{
    AxiomPlugin, PluginContext, PluginKind, PluginMessage, PluginReply, PluginResult,
};
use axiom_kernel::plugin::registry::PluginRegistry;

#[derive(Default)]
struct TestPlugin {
    id: &'static str,
    deps: &'static [&'static str],
}

impl AxiomPlugin for TestPlugin {
    fn id(&self) -> &'static str {
        self.id
    }
    fn version(&self) -> &'static str {
        "0.1.0"
    }
    fn dependencies(&self) -> &[&'static str] {
        self.deps
    }
    fn capabilities(&self) -> &[axiom_kernel::plugin::abi::CapabilityDescriptor] {
        &[]
    }
    fn init(&mut self, _ctx: PluginContext) -> PluginResult<()> {
        Ok(())
    }
    fn handle_message(&mut self, _msg: PluginMessage) -> PluginResult<PluginReply> {
        Ok(PluginReply::Ok(Vec::new()))
    }
    fn clone_box(&self) -> Box<dyn AxiomPlugin> {
        Box::new(TestPlugin { id: self.id, deps: self.deps })
    }
}

#[tokio::test]
async fn test_registry_register_and_get() {
    let registry = PluginRegistry::new();
    let plugin = Box::new(TestPlugin { id: "p1", deps: &[] });
    registry.register(plugin).await;
    let found = registry.get("p1").await;
    assert!(found.is_some());
    assert_eq!(found.unwrap().id(), "p1");
}

#[tokio::test]
async fn test_registry_list_all() {
    let registry = PluginRegistry::new();
    registry.register(Box::new(TestPlugin { id: "p1", deps: &[] })).await;
    registry.register(Box::new(TestPlugin { id: "p2", deps: &[] })).await;
    let all = registry.list_all().await;
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_registry_get_all_by_kind() {
    let registry = PluginRegistry::new();
    registry.register(Box::new(TestPlugin { id: "p1", deps: &[] })).await;
    registry.register(Box::new(TestPlugin { id: "p2", deps: &[] })).await;
    let tools = registry.get_all_by_kind(PluginKind::Llm).await;
    assert_eq!(tools.len(), 2);
}

#[tokio::test]
async fn test_registry_resolve_dependencies_cycle() {
    let registry = PluginRegistry::new();
    registry.register(Box::new(TestPlugin { id: "a", deps: &["b"] })).await;
    registry.register(Box::new(TestPlugin { id: "b", deps: &["a"] })).await;
    let result = registry.resolve_dependencies().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_registry_dependencies_resolved_true() {
    let registry = PluginRegistry::new();
    registry.register(Box::new(TestPlugin { id: "a", deps: &["b"] })).await;
    registry.register(Box::new(TestPlugin { id: "b", deps: &[] })).await;
    assert!(registry.dependencies_resolved("a").await);
}
