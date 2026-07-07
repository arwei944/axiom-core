//! Kernel integration for `axiom-mcp`.
//!
//! Provides adapters so MCP servers/tools can be registered as plugins
//! and invoked through the kernel runtime.

use crate::client::McpClient;

/// Adapter that exposes an `McpClient` through the kernel runtime.
pub struct McpKernelAdapter {
    client: Option<McpClient>,
}

impl McpKernelAdapter {
    pub fn new(client: Option<McpClient>) -> Self {
        Self { client }
    }

    pub fn client(&self) -> Option<&McpClient> {
        self.client.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_wraps_client() {
        let client = McpClient::new("http://localhost:3000").ok();
        let adapter = McpKernelAdapter::new(client);
        assert!(adapter.client().is_some());
    }
}
