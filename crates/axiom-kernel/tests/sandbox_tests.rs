use axiom_kernel::plugin::sandbox::{NativePluginSandbox, PluginSandbox, SandboxLimits, WasmPluginSandbox};
use axiom_kernel::plugin::abi::{PluginMessage, PluginReply};
use axiom_kernel::plugin::registry::PluginRegistry;

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

    fn capabilities(&self) -> &[axiom_kernel::plugin::abi::PluginCapabilityDescriptor] {
        &[]
    }

    fn init(&mut self, _ctx: axiom_kernel::plugin::abi::PluginContext) -> axiom_kernel::plugin::abi::PluginResult<()> {
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn axiom_kernel::plugin::abi::AxiomPlugin> {
        Box::new(DummyPlugin)
    }

    fn handle_message(&mut self, _msg: PluginMessage) -> axiom_kernel::plugin::abi::PluginResult<axiom_kernel::plugin::abi::PluginReply> {
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

    fn capabilities(&self) -> &[axiom_kernel::plugin::abi::PluginCapabilityDescriptor] {
        &[]
    }

    fn init(&mut self, _ctx: axiom_kernel::plugin::abi::PluginContext) -> axiom_kernel::plugin::abi::PluginResult<()> {
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn axiom_kernel::plugin::abi::AxiomPlugin> {
        Box::new(SandboxedPlugin)
    }

    fn handle_message(&mut self, _msg: PluginMessage) -> axiom_kernel::plugin::abi::PluginResult<axiom_kernel::plugin::abi::PluginReply> {
        Ok(axiom_kernel::plugin::abi::PluginReply::Ok(Vec::new()))
    }
}

#[test]
fn wasm_sandbox_allows_permitted_signal() {
    let sandbox = WasmPluginSandbox::new(
        SandboxLimits::new()
            .with_write_signals(&["memory"]),
    );

    let outcome = sandbox.enforce_write_signal("memory");
    assert_eq!(outcome, axiom_kernel::plugin::sandbox::SandboxOutcome::Allowed);
}

#[test]
fn wasm_sandbox_denies_unpermitted_signal() {
    let sandbox = WasmPluginSandbox::new(
        SandboxLimits::new()
            .with_write_signals(&["memory"]),
    );

    let outcome = sandbox.enforce_write_signal("network");
    assert_eq!(outcome, axiom_kernel::plugin::sandbox::SandboxOutcome::Denied);
}

#[test]
fn native_sandbox_network_default_disabled() {
    let sandbox = NativePluginSandbox::new(SandboxLimits::new());
    let outcome = sandbox.enforce_network();
    assert_eq!(outcome, axiom_kernel::plugin::sandbox::SandboxOutcome::Denied);
}

#[test]
fn native_sandbox_network_can_be_enabled() {
    let sandbox = NativePluginSandbox::new(SandboxLimits::new().allow_network());
    let outcome = sandbox.enforce_network();
    assert_eq!(outcome, axiom_kernel::plugin::sandbox::SandboxOutcome::Allowed);
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
async fn register_with_sandbox_verifies_capabilities() {
    let registry = PluginRegistry::new();
    let manifest = axiom_kernel::plugin::package::PluginManifest {
        id: "cap-plugin".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        kind: axiom_kernel::plugin::abi::PluginKind::Llm,
        entry: "entry".to_string(),
        dependencies: Vec::new(),
        abi_version: axiom_kernel::version::AbiVersion::CURRENT,
        permissions: axiom_kernel::plugin::package::PluginPermissions::default(),
        required_capabilities: Vec::new(),
        conflicts: Vec::new(),
    };

    let result = registry
        .register_with_sandbox(Box::new(DummyPlugin), Some(manifest), None)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn register_with_sandbox_detects_conflicts() {
    let registry = PluginRegistry::new();

    let manifest_b = axiom_kernel::plugin::package::PluginManifest {
        id: "plugin-b".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        kind: axiom_kernel::plugin::abi::PluginKind::Llm,
        entry: "entry".to_string(),
        dependencies: Vec::new(),
        abi_version: axiom_kernel::version::AbiVersion::CURRENT,
        permissions: axiom_kernel::plugin::package::PluginPermissions::default(),
        required_capabilities: Vec::new(),
        conflicts: Vec::new(),
    };

    let manifest_a = axiom_kernel::plugin::package::PluginManifest {
        id: "plugin-a".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        kind: axiom_kernel::plugin::abi::PluginKind::Llm,
        entry: "entry".to_string(),
        dependencies: Vec::new(),
        abi_version: axiom_kernel::version::AbiVersion::CURRENT,
        permissions: axiom_kernel::plugin::package::PluginPermissions::default(),
        required_capabilities: Vec::new(),
        conflicts: vec!["plugin-b".to_string()],
    };

    let _ = registry
        .register_with_sandbox(Box::new(DummyPlugin), Some(manifest_b), None)
        .await;

    let result = registry
        .register_with_sandbox(Box::new(DummyPlugin), Some(manifest_a), None)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn handle_message_respects_sandbox() {
    let registry = PluginRegistry::new();

    let manifest = axiom_kernel::plugin::package::PluginManifest {
        id: "sandboxed-plugin".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        kind: axiom_kernel::plugin::abi::PluginKind::Llm,
        entry: "entry".to_string(),
        dependencies: Vec::new(),
        abi_version: axiom_kernel::version::AbiVersion::CURRENT,
        permissions: axiom_kernel::plugin::package::PluginPermissions::default(),
        required_capabilities: Vec::new(),
        conflicts: Vec::new(),
    };

    let _ = registry
        .register_with_sandbox(Box::new(SandboxedPlugin), Some(manifest), Some(SandboxLimits::new().with_write_signals(&["memory"])))
        .await;

    let key = axiom_kernel::plugin::registry::PluginKey::new("sandboxed", "1.0.0");
    let instance = registry.get(key).await.unwrap();

    let allowed = instance
        .handle_message(PluginMessage::SendSignal {
            signal: "memory".to_string(),
            payload: Vec::new(),
        })
        .unwrap();

    assert!(matches!(allowed, PluginReply::Ok(_)));

    let denied = instance
        .handle_message(PluginMessage::SendSignal {
            signal: "network".to_string(),
            payload: Vec::new(),
        });

    assert!(denied.is_err());
}
