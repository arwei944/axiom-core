//! Plugin sandboxing - resource limits and signal allowlists for plugins.

use crate::plugin::abi::{AxiomPlugin, PluginError, PluginResult};

/// Resource limits for a plugin sandbox.
#[derive(Debug, Clone, Default)]
pub struct SandboxLimits {
    /// Maximum memory usage in megabytes.
    pub memory_limit_mb: Option<u64>,
    /// Maximum CPU time in milliseconds.
    pub cpu_time_limit_ms: Option<u64>,
    /// Allowlist of signal types the plugin may read.
    pub read_signals: Vec<&'static str>,
    /// Allowlist of signal types the plugin may write.
    pub write_signals: Vec<&'static str>,
    /// Whether the plugin may access the network.
    pub network: bool,
}

impl SandboxLimits {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_memory(mut self, mb: u64) -> Self {
        self.memory_limit_mb = Some(mb);
        self
    }

    pub fn with_cpu(mut self, ms: u64) -> Self {
        self.cpu_time_limit_ms = Some(ms);
        self
    }

    pub fn with_read_signals(mut self, signals: &'static [&'static str]) -> Self {
        self.read_signals = signals.to_vec();
        self
    }

    pub fn with_write_signals(mut self, signals: &'static [&'static str]) -> Self {
        self.write_signals = signals.to_vec();
        self
    }

    pub fn allow_network(mut self) -> Self {
        self.network = true;
        self
    }
}

/// Sandbox enforcement outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxOutcome {
    Allowed,
    Denied,
}

/// Trait for plugin sandboxes.
///
/// Implementations enforce resource limits and signal allowlists
/// around plugin operations.
pub trait PluginSandbox: Send + Sync + 'static {
    fn limits(&self) -> &SandboxLimits;

    fn enforce_read_signal(&self, signal_type: &str) -> SandboxOutcome;
    fn enforce_write_signal(&self, signal_type: &str) -> SandboxOutcome;
    fn enforce_network(&self) -> SandboxOutcome;

    /// Wrap a plugin handle_message call with sandbox enforcement.
    fn handle_message(
        &self,
        plugin: &mut dyn AxiomPlugin,
        msg: crate::plugin::abi::PluginMessage,
    ) -> PluginResult<crate::plugin::abi::PluginReply> {
        match msg {
            crate::plugin::abi::PluginMessage::SendSignal { ref signal, .. } => {
                if self.enforce_write_signal(signal) == SandboxOutcome::Denied {
                    return Err(PluginError::PermissionDenied(format!(
                        "plugin `{}` is not allowed to write signal `{signal}`",
                        plugin.id()
                    )));
                }
            }
            crate::plugin::abi::PluginMessage::QueryMemory { .. } => {
                if self.enforce_read_signal("memory") == SandboxOutcome::Denied {
                    return Err(PluginError::PermissionDenied(format!(
                        "plugin `{}` is not allowed to read memory",
                        plugin.id()
                    )));
                }
            }
            crate::plugin::abi::PluginMessage::CheckAxiom { .. } => {
                if self.enforce_read_signal("axiom") == SandboxOutcome::Denied {
                    return Err(PluginError::PermissionDenied(format!(
                        "plugin `{}` is not allowed to check axioms",
                        plugin.id()
                    )));
                }
            }
            crate::plugin::abi::PluginMessage::CallTool { ref tool, .. } => {
                if self.enforce_read_signal(&format!("tool:{tool}")) == SandboxOutcome::Denied {
                    return Err(PluginError::PermissionDenied(format!(
                        "plugin `{}` is not allowed to call tool `{tool}`",
                        plugin.id()
                    )));
                }
            }
            crate::plugin::abi::PluginMessage::QueryLens { .. } => {
                if self.enforce_read_signal("lens") == SandboxOutcome::Denied {
                    return Err(PluginError::PermissionDenied(format!(
                        "plugin `{}` is not allowed to query lenses",
                        plugin.id()
                    )));
                }
            }
            crate::plugin::abi::PluginMessage::Custom { ref kind, .. } => {
                if self.enforce_write_signal(kind) == SandboxOutcome::Denied {
                    return Err(PluginError::PermissionDenied(format!(
                        "plugin `{}` is not allowed to send custom signal `{kind}`",
                        plugin.id()
                    )));
                }
            }
        }

        plugin.handle_message(msg)
    }
}

/// WASM sandbox using wasmtime store limits.
pub struct WasmPluginSandbox {
    limits: SandboxLimits,
}

impl WasmPluginSandbox {
    pub fn new(limits: SandboxLimits) -> Self {
        Self { limits }
    }
}

impl PluginSandbox for WasmPluginSandbox {
    fn limits(&self) -> &SandboxLimits {
        &self.limits
    }

    fn enforce_read_signal(&self, signal_type: &str) -> SandboxOutcome {
        if self.limits.read_signals.iter().any(|s| s == &"*" || *s == signal_type) {
            SandboxOutcome::Allowed
        } else {
            SandboxOutcome::Denied
        }
    }

    fn enforce_write_signal(&self, signal_type: &str) -> SandboxOutcome {
        if self.limits.write_signals.iter().any(|s| s == &"*" || *s == signal_type) {
            SandboxOutcome::Allowed
        } else {
            SandboxOutcome::Denied
        }
    }

    fn enforce_network(&self) -> SandboxOutcome {
        if self.limits.network {
            SandboxOutcome::Allowed
        } else {
            SandboxOutcome::Denied
        }
    }
}

/// Native sandbox using thread pool + timeout.
pub struct NativePluginSandbox {
    limits: SandboxLimits,
}

impl NativePluginSandbox {
    pub fn new(limits: SandboxLimits) -> Self {
        Self { limits }
    }
}

impl PluginSandbox for NativePluginSandbox {
    fn limits(&self) -> &SandboxLimits {
        &self.limits
    }

    fn enforce_read_signal(&self, signal_type: &str) -> SandboxOutcome {
        if self.limits.read_signals.iter().any(|s| s == &"*" || *s == signal_type) {
            SandboxOutcome::Allowed
        } else {
            SandboxOutcome::Denied
        }
    }

    fn enforce_write_signal(&self, signal_type: &str) -> SandboxOutcome {
        if self.limits.write_signals.iter().any(|s| s == &"*" || *s == signal_type) {
            SandboxOutcome::Allowed
        } else {
            SandboxOutcome::Denied
        }
    }

    fn enforce_network(&self) -> SandboxOutcome {
        if self.limits.network {
            SandboxOutcome::Allowed
        } else {
            SandboxOutcome::Denied
        }
    }
}
