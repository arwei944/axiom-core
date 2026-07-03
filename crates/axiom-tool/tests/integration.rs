use std::sync::Arc;

use axiom_tool::{BoxToolFuture, *};
use serde_json::json;

struct EchoTool;

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

    fn execute<'a>(&'a self, parameters: &'a serde_json::Value) -> BoxToolFuture<'a> {
        Box::pin(async move {
            self.validate(parameters)?;

            let message = parameters
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters("message must be a string".into()))?;

            Ok(json!({ "echo": message }))
        })
    }
}

struct AddTool;

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

    fn execute<'a>(&'a self, parameters: &'a serde_json::Value) -> BoxToolFuture<'a> {
        Box::pin(async move {
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
        })
    }
}
