//! Product plugin host (commercial path) — registry + sandbox + hot-reload surface.
//!
//! Constitution: plugins are **not** a second runtime. They run under
//! [`PluginSandbox`] allow-lists; results feed Witness via the Cell path.

use axiom_kernel::plugin::abi::{
    AxiomPlugin, CapabilityDescriptor, PluginContext, PluginKind, PluginMessage, PluginReply,
    PluginResult,
};
use axiom_kernel::plugin::sandbox::{
    NativePluginSandbox, PluginSandbox, SandboxLimits, SandboxOutcome,
};
use axiom_kernel::plugin::PluginRegistry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Snapshot for surface / lens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSurfaceInfo {
    pub id: String,
    pub version: String,
    pub kind: String,
    pub refcount: u32,
    pub hot_reload: bool,
}

/// Built-in commercial echo plugin (in-process, no native dylib required).
#[derive(Clone)]
pub struct BuiltinEchoPlugin {
    caps: Vec<CapabilityDescriptor>,
}

impl BuiltinEchoPlugin {
    pub fn new() -> Self {
        Self {
            caps: vec![CapabilityDescriptor {
                name: "tool.echo".into(),
                version: "1.0.0".into(),
                description: Some("commercial path echo tool".into()),
            }],
        }
    }
}

impl Default for BuiltinEchoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl AxiomPlugin for BuiltinEchoPlugin {
    fn id(&self) -> &'static str {
        "builtin.echo"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn dependencies(&self) -> &[&'static str] {
        &[]
    }

    fn capabilities(&self) -> &[CapabilityDescriptor] {
        &self.caps
    }

    fn init(&mut self, _ctx: PluginContext) -> PluginResult<()> {
        Ok(())
    }

    fn handle_message(&mut self, msg: PluginMessage) -> PluginResult<PluginReply> {
        match msg {
            PluginMessage::CallTool { tool, input } => {
                if tool != "echo" && tool != "tool.echo" {
                    return Ok(PluginReply::Err(format!("unknown tool {tool}")));
                }
                let mut out = b"echo:".to_vec();
                out.extend_from_slice(&input);
                Ok(PluginReply::Ok(out))
            }
            PluginMessage::Custom { kind, payload } => {
                let mut out = format!("custom:{kind}:").into_bytes();
                out.extend_from_slice(&payload);
                Ok(PluginReply::Ok(out))
            }
            other => Ok(PluginReply::Err(format!("unsupported message: {other:?}"))),
        }
    }

    fn clone_box(&self) -> Box<dyn AxiomPlugin> {
        Box::new(self.clone())
    }
}

/// Commercial plugin host wrapping registry + sandbox.
pub struct ProductPluginHost {
    registry: Arc<PluginRegistry>,
    sandbox: NativePluginSandbox,
}

impl ProductPluginHost {
    pub fn new() -> Self {
        let limits = SandboxLimits::new()
            .with_memory(64)
            .with_cpu(1_000)
            .with_read_signals(&["tool:echo", "tool:tool.echo", "memory"])
            .with_write_signals(&["echo", "custom"]);
        Self {
            registry: Arc::new(PluginRegistry::new()),
            sandbox: NativePluginSandbox::new(limits),
        }
    }

    pub async fn boot_defaults(&self) -> PluginResult<()> {
        self.registry
            .register(Box::new(BuiltinEchoPlugin::new()))
            .await;
        Ok(())
    }

    pub fn registry(&self) -> Arc<PluginRegistry> {
        self.registry.clone()
    }

    /// Invoke echo under sandbox (product path).
    pub async fn invoke_echo(&self, payload: &str) -> PluginResult<String> {
        let mut plugin = self
            .registry
            .get("builtin.echo")
            .await
            .ok_or_else(|| {
                axiom_kernel::plugin::abi::PluginError::NotFound("builtin.echo".into())
            })?;

        let msg = PluginMessage::CallTool {
            tool: "echo".into(),
            input: payload.as_bytes().to_vec(),
        };
        // Sandbox enforces signal allow-list around handle_message.
        let reply = self.sandbox.handle_message(plugin.as_mut(), msg)?;
        match reply {
            PluginReply::Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
            PluginReply::Err(e) => Err(axiom_kernel::plugin::abi::PluginError::HandleFailed(e)),
        }
    }

    /// Hot-reload: replace builtin echo with a new instance (refcount gate).
    pub async fn hot_reload_echo(&self) -> PluginResult<()> {
        // Drain refs to 1 then upgrade.
        self.registry.release("builtin.echo").await;
        self.registry
            .upgrade(Box::new(BuiltinEchoPlugin::new()))
            .await
    }

    pub async fn list_surface(&self) -> Vec<PluginSurfaceInfo> {
        let all = self.registry.list_all().await;
        let mut out = Vec::new();
        for p in all {
            out.push(PluginSurfaceInfo {
                id: p.id().to_string(),
                version: p.version().to_string(),
                kind: format!("{:?}", PluginKind::Tool),
                refcount: 1,
                hot_reload: true,
            });
        }
        out
    }

    pub async fn plugin_ids(&self) -> Vec<String> {
        self.list_surface()
            .await
            .into_iter()
            .map(|p| p.id)
            .collect()
    }
}

impl Default for ProductPluginHost {
    fn default() -> Self {
        Self::new()
    }
}

/// Ensure sandbox denies disallowed write signals (path-driving).
pub fn sandbox_denies_shell() -> bool {
    let limits = SandboxLimits::new().with_write_signals(&["echo"]);
    let sb = NativePluginSandbox::new(limits);
    sb.enforce_write_signal("shell_raw") == SandboxOutcome::Denied
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_blocks_shell() {
        assert!(sandbox_denies_shell());
    }

    #[tokio::test]
    async fn echo_roundtrip() {
        let host = ProductPluginHost::new();
        host.boot_defaults().await.unwrap();
        let out = host.invoke_echo("hi").await.unwrap();
        assert!(out.contains("echo:"), "{out}");
        assert!(out.contains("hi"), "{out}");
    }

    #[tokio::test]
    async fn hot_reload_ok() {
        let host = ProductPluginHost::new();
        host.boot_defaults().await.unwrap();
        host.hot_reload_echo().await.unwrap();
        let ids = host.plugin_ids().await;
        assert!(ids.iter().any(|id| id == "builtin.echo"));
    }
}
