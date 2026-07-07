//! Kernel integration for `axiom-llm`.
//!
//! Provides thin adapters so an `LlmClient` can be registered as a plugin
//! and invoked through the kernel runtime when needed.

use crate::client::LlmClient;

/// Adapter that exposes an `LlmClient` as an axiom plugin capability.
///
/// This is a minimal bridge; real implementations should register
/// provider-specific capabilities through `PluginRegistry`.
pub struct LlmKernelAdapter {
    client: LlmClient,
}

impl LlmKernelAdapter {
    pub fn new(client: LlmClient) -> Self {
        Self { client }
    }

    pub fn client(&self) -> &LlmClient {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_wraps_client() {
        let client = LlmClient::mock();
        let adapter = LlmKernelAdapter::new(client);
        assert!(adapter.client().remaining_budget() > 0);
    }
}
