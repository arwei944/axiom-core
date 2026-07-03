use std::sync::Arc;

use axiom_mcp::{client::McpClient, server::McpServer, tools::{ToolRegistry, AxiomTool, BoxMcpFuture}, protocol::{McpCapability, McpError}};
use serde_json::Value;
use tokio;

struct TestTool;

impl AxiomTool for TestTool {
    fn name(&self) -> &str {
        "test_tool"
    }

    fn description(&self) -> &str {
        "Test tool for integration testing"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            },
            "required": ["message"]
        })
    }

    fn output_schema(&self) -> Option<Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "echo": { "type": "string" }
            }
        }))
    }

    fn requires_permission(&self) -> bool {
        false
    }

    fn execute<'a>(&'a self, arguments: &'a Value) -> BoxMcpFuture<'a> {
        Box::pin(async move {
            let message = arguments.get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidRequest("message is required".into()))?;

            Ok(serde_json::json!({ "echo": message }))
        })
    }
}

#[tokio::test]
async fn test_mcp_capabilities() {
    let registry = Arc::new(ToolRegistry::new());
    registry.register(TestTool);

    let cap = registry.list();
    assert!(!cap.is_empty());
    assert_eq!(cap[0].name, "test_tool");
}

#[tokio::test]
async fn test_mcp_tool_execution() {
    let registry = Arc::new(ToolRegistry::new());
    registry.register(TestTool);

    let result = registry.execute("test_tool", &serde_json::json!({ "message": "hello" })).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.get("echo").unwrap().as_str().unwrap(), "hello");
}

#[tokio::test]
async fn test_mcp_tool_not_found() {
    let registry = Arc::new(ToolRegistry::new());

    let result = registry.execute("non_existent", &serde_json::json!({})).await;

    assert!(result.is_err());
    if let Err(McpError::ToolNotFound(_)) = result {
        // expected
    } else {
        panic!("expected ToolNotFound error");
    }
}
