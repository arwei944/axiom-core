use axiom_kernel::plugin::abi::{AxiomPlugin, CapabilityDescriptor, PluginMessage, PluginReply};
use axiom_kernel::plugin::registry::PluginRegistry;
use axiom_kernel::plugin::sandbox::{
    NativePluginSandbox, PluginSandbox, SandboxLimits, SandboxOutcome, WasmPluginSandbox,
};

#[allow(dead_code)]
struct DummyPlugin;

impl axiom_kernel::plugin::abi::AxiomPlugin for DummyPlugin {
    fn id(&self) -> &'static str {
        "dummy"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn dependencies(&self) -> &[&'static str] {
        &[]
    }

    fn capabilities(&self) -> &[CapabilityDescriptor] {
        &[]
    }

    fn init(
        &mut self,
        _ctx: axiom_kernel::plugin::abi::PluginContext,
    ) -> axiom_kernel::plugin::abi::PluginResult<()> {
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn axiom_kernel::plugin::abi::AxiomPlugin> {
        Box::new(DummyPlugin)
    }

    fn handle_message(
        &mut self,
        _msg: PluginMessage,
    ) -> axiom_kernel::plugin::abi::PluginResult<axiom_kernel::plugin::abi::PluginReply> {
        Ok(axiom_kernel::plugin::abi::PluginReply::Ok(Vec::new()))
    }
}

#[allow(dead_code)]
struct SandboxedPlugin;

impl axiom_kernel::plugin::abi::AxiomPlugin for SandboxedPlugin {
    fn id(&self) -> &'static str {
        "sandboxed"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn dependencies(&self) -> &[&'static str] {
        &[]
    }

    fn capabilities(&self) -> &[CapabilityDescriptor] {
        &[]
    }

    fn init(
        &mut self,
        _ctx: axiom_kernel::plugin::abi::PluginContext,
    ) -> axiom_kernel::plugin::abi::PluginResult<()> {
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn axiom_kernel::plugin::abi::AxiomPlugin> {
        Box::new(SandboxedPlugin)
    }

    fn handle_message(
        &mut self,
        _msg: PluginMessage,
    ) -> axiom_kernel::plugin::abi::PluginResult<axiom_kernel::plugin::abi::PluginReply> {
        Ok(axiom_kernel::plugin::abi::PluginReply::Ok(Vec::new()))
    }
}

#[test]
fn wasm_sandbox_allows_permitted_signal() {
    let sandbox = WasmPluginSandbox::new(SandboxLimits::new().with_write_signals(&["memory"]));

    let outcome = sandbox.enforce_write_signal("memory");
    assert_eq!(outcome, SandboxOutcome::Allowed);
}

#[test]
fn wasm_sandbox_denies_unpermitted_signal() {
    let sandbox = WasmPluginSandbox::new(SandboxLimits::new().with_write_signals(&["memory"]));

    let outcome = sandbox.enforce_write_signal("network");
    assert_eq!(outcome, SandboxOutcome::Denied);
}

#[test]
fn native_sandbox_network_default_disabled() {
    let sandbox = NativePluginSandbox::new(SandboxLimits::new());
    let outcome = sandbox.enforce_network();
    assert_eq!(outcome, SandboxOutcome::Denied);
}

#[test]
fn native_sandbox_network_can_be_enabled() {
    let sandbox = NativePluginSandbox::new(SandboxLimits::new().allow_network());
    let outcome = sandbox.enforce_network();
    assert_eq!(outcome, SandboxOutcome::Allowed);
}

#[test]
fn sandbox_limits_builder() {
    let limits = SandboxLimits::new()
        .with_memory(128)
        .with_cpu(50)
        .with_read_signals(&["memory", "axiom"])
        .with_write_signals(&["memory"])
        .allow_network();

    assert_eq!(limits.memory_limit_mb, Some(128));
    assert_eq!(limits.cpu_time_limit_ms, Some(50));
    assert_eq!(limits.read_signals, vec!["memory", "axiom"]);
    assert_eq!(limits.write_signals, vec!["memory"]);
    assert!(limits.network);
}

#[tokio::test]
async fn registry_register_plugin() {
    let registry = PluginRegistry::new();
    registry.register(Box::new(DummyPlugin)).await;
    let plugins = registry.list_all().await;
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].id(), "dummy");
}

#[tokio::test]
async fn registry_remove_plugin() {
    let registry = PluginRegistry::new();
    registry.register(Box::new(DummyPlugin)).await;
    let removed = registry.remove("dummy").await;
    assert!(removed.is_some());
    let plugins = registry.list_all().await;
    assert_eq!(plugins.len(), 0);
}

#[tokio::test]
async fn registry_list_all_by_kind() {
    let registry = PluginRegistry::new();
    registry.register(Box::new(DummyPlugin)).await;
    registry.register(Box::new(SandboxedPlugin)).await;
    let llm_plugins = registry.get_all_by_kind(axiom_kernel::plugin::abi::PluginKind::Llm).await;
    assert!(llm_plugins.len() >= 1);
}

#[tokio::test]
async fn plugin_handle_message_works() {
    let mut plugin = Box::new(SandboxedPlugin);
    let result = plugin.handle_message(PluginMessage::SendSignal {
        signal: "memory".to_string(),
        payload: Vec::new(),
    });
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), PluginReply::Ok(_)));
}
