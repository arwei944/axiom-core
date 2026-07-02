//! MCP protocol data structures and error types.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("tool execution failed: {0}")]
    ToolExecution(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("requires approval: {0}")]
    RequiresApproval(String),
    #[error("timeout")]
    Timeout,
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("server error: {0}")]
    Server(String),
    #[error("unknown error")]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub requires_permission: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCapability {
    pub name: String,
    pub protocol_version: String,
    pub tools: Vec<McpTool>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCall {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub id: String,
    pub result: Result<serde_json::Value, McpErrorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpErrorInfo {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl From<McpError> for McpErrorInfo {
    fn from(e: McpError) -> Self {
        let code = match &e {
            McpError::Connection(_) => "connection_error",
            McpError::ToolNotFound(_) => "tool_not_found",
            McpError::InvalidRequest(_) => "invalid_request",
            McpError::ToolExecution(_) => "execution_error",
            McpError::PermissionDenied(_) => "permission_denied",
            McpError::RequiresApproval(_) => "requires_approval",
            McpError::Timeout => "timeout",
            McpError::Serialization(_) => "serialization_error",
            McpError::Server(_) => "server_error",
            McpError::Unknown => "unknown",
        };
        McpErrorInfo {
            code: code.to_string(),
            message: e.to_string(),
            details: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpRequest {
    Capabilities,
    ToolCall(McpToolCall),
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpResponse {
    Capabilities(McpCapability),
    ToolResult(McpToolResult),
    Shutdown,
    Error(McpErrorInfo),
}