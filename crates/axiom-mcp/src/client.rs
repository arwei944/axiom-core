//! MCP Client implementation for connecting to external MCP servers.

use std::sync::Arc;

use parking_lot::RwLock;
use reqwest::Client as ReqwestClient;
use url::Url;

use crate::protocol::{McpCapability, McpError, McpRequest, McpResponse};

pub struct McpClient {
    base_url: Url,
    client: ReqwestClient,
    capabilities: Arc<RwLock<Option<McpCapability>>>,
}

impl McpClient {
    pub fn new(base_url: &str) -> Result<Self, McpError> {
        let url = Url::parse(base_url).map_err(|e| McpError::Connection(e.to_string()))?;

        let client = ReqwestClient::new();

        Ok(Self { base_url: url, client, capabilities: Arc::new(RwLock::new(None)) })
    }

    pub async fn capabilities(&self) -> Result<McpCapability, McpError> {
        if let Some(cap) = self.capabilities.read().clone() {
            return Ok(cap);
        }

        let url =
            self.base_url.join("capabilities").map_err(|e| McpError::Connection(e.to_string()))?;

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| McpError::Connection(e.to_string()))?;

        let response: McpResponse =
            response.json().await.map_err(|e| McpError::Serialization(e.to_string()))?;

        match response {
            McpResponse::Capabilities(cap) => {
                *self.capabilities.write() = Some(cap.clone());
                Ok(cap)
            }
            McpResponse::Error(e) => Err(McpError::Server(e.message)),
            _ => Err(McpError::InvalidRequest("unexpected response".into())),
        }
    }

    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let tool_call = crate::protocol::McpToolCall {
            tool_name: tool_name.to_string(),
            arguments: arguments.clone(),
            id: uuid::Uuid::new_v4().to_string(),
        };

        let url = self.base_url.join("tool").map_err(|e| McpError::Connection(e.to_string()))?;

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&McpRequest::ToolCall(tool_call))
            .send()
            .await
            .map_err(|e| McpError::Connection(e.to_string()))?;

        let response: McpResponse =
            response.json().await.map_err(|e| McpError::Serialization(e.to_string()))?;

        match response {
            McpResponse::ToolResult(result) => match result.result {
                Ok(value) => Ok(value),
                Err(e) => Err(McpError::ToolExecution(e.message)),
            },
            McpResponse::Error(e) => Err(McpError::Server(e.message)),
            _ => Err(McpError::InvalidRequest("unexpected response".into())),
        }
    }

    pub async fn list_tools(&self) -> Result<Vec<String>, McpError> {
        let cap = self.capabilities().await?;
        Ok(cap.tools.into_iter().map(|t| t.name).collect())
    }

    pub async fn shutdown(&self) -> Result<(), McpError> {
        let url =
            self.base_url.join("shutdown").map_err(|e| McpError::Connection(e.to_string()))?;

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&McpRequest::Shutdown)
            .send()
            .await
            .map_err(|e| McpError::Connection(e.to_string()))?;

        let response: McpResponse =
            response.json().await.map_err(|e| McpError::Serialization(e.to_string()))?;

        match response {
            McpResponse::Shutdown => Ok(()),
            McpResponse::Error(e) => Err(McpError::Server(e.message)),
            _ => Err(McpError::InvalidRequest("unexpected response".into())),
        }
    }
}
