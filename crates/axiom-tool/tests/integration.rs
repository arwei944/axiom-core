use std::sync::Arc;

use axiom_tool::*;
use serde_json::json;

struct EchoTool;

#[async_trait::async_trait]
impl Tool for EchoTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "echo".to_string(),
            description: "Echo back the input message".to_string(),
            parameters: vec![ToolParameter {
                name: "message".to_string(),
                description: "The message to echo".to_string(),
                required: true,
                schema: json!({ "type": "string" }),
            }],
            required_permission: None,
            version: "1.0.0".to_string(),
        }
    }

    async fn execute(&self, parameters: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        self.validate(parameters)?;

        let message = parameters
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("message must be a string".into()))?;

        Ok(json!({ "echo": message }))
    }
}

struct AddTool;

#[async_trait::async_trait]
impl Tool for AddTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "add".to_string(),
            description: "Add two numbers".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "a".to_string(),
                    description: "First number".to_string(),
                    required: true,
                    schema: json!({ "type": "number" }),
                },
                ToolParameter {
                    name: "b".to_string(),
                    description: "Second number".to_string(),
                    required: true,
                    schema: json!({ "type": "number" }),
                },
            ],
            required_permission: None,
            version: "1.0.0".to_string(),
        }
    }

    async fn execute(&self, parameters: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        self.validate(parameters)?;

        let a = parameters
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidParameters("a must be a number".into()))?;

        let b = parameters
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidParameters("b must be a number".into()))?;

        Ok(json!({ "result": a + b }))
    }
}

#[tokio::test]
async fn test_register_and_execute() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool);

    assert!(registry.contains("echo"));
    assert_eq!(registry.tool_count(), 1);

    let result = registry.execute("echo", &json!({ "message": "hello" })).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["echo"], "hello");
}

#[tokio::test]
async fn test_tool_not_found() {
    let registry = ToolRegistry::new();

    let result = registry.execute("nonexistent", &json!({})).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ToolError::NotFound(_)));
}

#[tokio::test]
async fn test_missing_required_parameter() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool);

    let result = registry.execute("echo", &json!({})).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ToolError::InvalidParameters(_)));
}

#[tokio::test]
async fn test_multiple_tools() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool);
    registry.register(AddTool);

    assert_eq!(registry.tool_count(), 2);

    let tools = registry.list();
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().any(|t| t.name == "echo"));
    assert!(tools.iter().any(|t| t.name == "add"));

    let add_result = registry.execute("add", &json!({ "a": 2, "b": 3 })).await;
    assert!(add_result.is_ok());
    assert_eq!(add_result.unwrap()["result"], 5.0);
}

#[tokio::test]
async fn test_invocation_history() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool);

    let _ = registry.execute("echo", &json!({ "message": "first" })).await;
    let _ = registry.execute("echo", &json!({ "message": "second" })).await;

    let history = registry.history();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].tool_name, "echo");
    assert_eq!(history[1].tool_name, "echo");

    let tool_history = registry.history_for_tool("echo");
    assert_eq!(tool_history.len(), 2);
}

#[tokio::test]
async fn test_simple_tool() {
    let registry = ToolRegistry::new();

    let info = ToolInfo {
        name: "greet".to_string(),
        description: "Greet someone".to_string(),
        parameters: vec![ToolParameter {
            name: "name".to_string(),
            description: "Name to greet".to_string(),
            required: true,
            schema: json!({ "type": "string" }),
        }],
        required_permission: None,
        version: "1.0.0".to_string(),
    };

    let greet_tool = tool::SimpleTool::new(info, |params| {
        let name = params["name"].as_str().unwrap_or("world");
        Ok(json!({ "greeting": format!("Hello, {}!", name) }))
    });

    registry.register(greet_tool);

    let result = registry.execute("greet", &json!({ "name": "Alice" })).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["greeting"], "Hello, Alice!");
}

#[tokio::test]
async fn test_caller_recording() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool);

    let _ = registry
        .execute_with_caller("echo", &json!({ "message": "test" }), Some("test-user"))
        .await;

    let history = registry.history();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].caller, Some("test-user".to_string()));
}

#[tokio::test]
async fn test_clear_history() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool);

    let _ = registry.execute("echo", &json!({ "message": "test" })).await;
    assert_eq!(registry.history().len(), 1);

    registry.clear_history();
    assert_eq!(registry.history().len(), 0);
}

#[tokio::test]
async fn test_history_max_size() {
    let registry = ToolRegistry::new().with_max_history(3);
    registry.register(EchoTool);

    for i in 0..5 {
        let _ = registry
            .execute("echo", &json!({ "message": format!("msg-{}", i) }))
            .await;
    }

    let history = registry.history();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].parameters["message"], "msg-2");
    assert_eq!(history[2].parameters["message"], "msg-4");
}