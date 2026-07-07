//! MCP (Model Context Protocol) bridge for axiom-kernel.
//!
//! Provides:
//! - MCP Client: Connect to external MCP servers and call tools
//! - MCP Server: Expose axiom capabilities as MCP tools
//! - Tool Registry: Type-safe tool definitions
//! - Security Layer: Permission → Rules → Axiom → Human-in-the-loop

pub mod client;
pub mod kernel;
pub mod protocol;
pub mod security;
pub mod server;
pub mod tools;

pub use client::McpClient;
pub use kernel::McpKernelAdapter;
pub use protocol::{McpCapability, McpError, McpTool, McpToolCall, McpToolResult};
pub use security::{
    ApprovalManager, ApprovalRequest, ApprovalStatus, PermissionLevel, SecurityContext,
    SecurityManager, ToolPermission,
};
pub use server::McpServer;
pub use tools::{AxiomTool, ToolRegistry};
