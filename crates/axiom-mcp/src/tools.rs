//! Tool registry and type-safe tool definitions for MCP.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::Value;

use crate::protocol::{McpError, McpTool};

pub type BoxMcpFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, McpError>> + Send + 'a>>;

pub trait AxiomTool: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    fn output_schema(&self) -> Option<Value>;
    fn requires_permission(&self) -> bool;
    fn execute<'a>(&'a self, arguments: &'a Value) -> BoxMcpFuture<'a>;
}

pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn AxiomTool>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register<T: AxiomTool>(&self, tool: T) {
        self.tools.write().insert(tool.name().to_string(), Arc::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn AxiomTool>> {
        self.tools.read().get(name).cloned()
    }

    pub fn list(&self) -> Vec<McpTool> {
        self.tools
            .read()
            .values()
            .map(|t| McpTool {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.input_schema(),
                output_schema: t.output_schema(),
                requires_permission: t.requires_permission(),
            })
            .collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tools.read().contains_key(name)
    }

    pub async fn execute(&self, name: &str, arguments: &Value) -> Result<Value, McpError> {
        let tool = self.get(name)
            .ok_or_else(|| McpError::ToolNotFound(name.to_string()))?;

        tool.execute(arguments).await
    }
}

pub struct CellListTool {
    runtime: Arc<dyn RuntimeAccessor>,
}

impl CellListTool {
    pub fn new(runtime: Arc<dyn RuntimeAccessor>) -> Self {
        Self { runtime }
    }
}

impl AxiomTool for CellListTool {
    fn name(&self) -> &str {
        "cell_list"
    }

    fn description(&self) -> &str {
        "List all cells in the runtime with their status"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn output_schema(&self) -> Option<Value> {
        Some(serde_json::json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "layer": { "type": "string" },
                    "state": { "type": "string" },
                    "messages_processed": { "type": "integer" },
                    "errors": { "type": "integer" },
                    "restarts": { "type": "integer" }
                }
            }
        }))
    }

    fn requires_permission(&self) -> bool {
        false
    }

    fn execute<'a>(&'a self, _arguments: &'a Value) -> BoxMcpFuture<'a> {
        Box::pin(async move {
            self.runtime.list_cells().await.map_err(|e| McpError::ToolExecution(e.to_string()))
        })
    }
}

pub struct CellStatusTool {
    runtime: Arc<dyn RuntimeAccessor>,
}

impl CellStatusTool {
    pub fn new(runtime: Arc<dyn RuntimeAccessor>) -> Self {
        Self { runtime }
    }
}

impl AxiomTool for CellStatusTool {
    fn name(&self) -> &str {
        "cell_status"
    }

    fn description(&self) -> &str {
        "Get detailed status of a specific cell"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "cell_id": { "type": "string" }
            },
            "required": ["cell_id"]
        })
    }

    fn output_schema(&self) -> Option<Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "layer": { "type": "string" },
                "state": { "type": "string" },
                "version": { "type": "string" },
                "messages_processed": { "type": "integer" },
                "errors": { "type": "integer" },
                "restarts": { "type": "integer" },
                "uptime": { "type": "integer" },
                "mailbox_depth": { "type": "integer" }
            }
        }))
    }

    fn requires_permission(&self) -> bool {
        false
    }

    fn execute<'a>(&'a self, arguments: &'a Value) -> BoxMcpFuture<'a> {
        Box::pin(async move {
            let cell_id = arguments.get("cell_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidRequest("cell_id is required".into()))?;

            self.runtime.cell_status(cell_id).await.map_err(|e| McpError::ToolExecution(e.to_string()))
        })
    }
}

pub struct EntropyStatusTool {
    runtime: Arc<dyn RuntimeAccessor>,
}

impl EntropyStatusTool {
    pub fn new(runtime: Arc<dyn RuntimeAccessor>) -> Self {
        Self { runtime }
    }
}

impl AxiomTool for EntropyStatusTool {
    fn name(&self) -> &str {
        "entropy_status"
    }

    fn description(&self) -> &str {
        "Get current entropy levels for the system"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn output_schema(&self) -> Option<Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "global_level": { "type": "string" },
                "global_entropy": { "type": "number" },
                "cells": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "level": { "type": "string" },
                            "entropy": { "type": "number" }
                        }
                    }
                }
            }
        }))
    }

    fn requires_permission(&self) -> bool {
        false
    }

    fn execute<'a>(&'a self, _arguments: &'a Value) -> BoxMcpFuture<'a> {
        Box::pin(async move {
            self.runtime.entropy_status().await.map_err(|e| McpError::ToolExecution(e.to_string()))
        })
    }
}

pub trait RuntimeAccessor: Send + Sync + 'static {
    fn list_cells<'a>(&'a self) -> BoxMcpFuture<'a>;
    fn cell_status<'a>(&'a self, cell_id: &'a str) -> BoxMcpFuture<'a>;
    fn entropy_status<'a>(&'a self) -> BoxMcpFuture<'a>;
}