//! Tool registry with permission control and invocation history.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::Value;

use crate::error::ToolError;
use crate::tool::{Tool, ToolInfo};

#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub id: String,
    pub tool_name: String,
    pub parameters: Value,
    pub result: Result<Value, String>,
    pub timestamp: u64,
    pub duration_ms: u64,
    pub caller: Option<String>,
}

pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
    history: Arc<RwLock<Vec<ToolInvocation>>>,
    max_history: usize,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 1000,
        }
    }

    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    pub fn register<T: Tool>(&self, tool: T) {
        let info = tool.info();
        self.tools.write().insert(info.name.clone(), Arc::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().get(name).cloned()
    }

    pub fn list(&self) -> Vec<ToolInfo> {
        self.tools.read().values().map(|t| t.info()).collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tools.read().contains_key(name)
    }

    pub async fn execute(&self, name: &str, parameters: &Value) -> Result<Value, ToolError> {
        self.execute_with_caller(name, parameters, None).await
    }

    pub async fn execute_with_caller(
        &self,
        name: &str,
        parameters: &Value,
        caller: Option<&str>,
    ) -> Result<Value, ToolError> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        let start = std::time::Instant::now();
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let result = tool.execute(parameters).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        let invocation = ToolInvocation {
            id,
            tool_name: name.to_string(),
            parameters: parameters.clone(),
            result: result.clone().map_err(|e| e.to_string()),
            timestamp,
            duration_ms,
            caller: caller.map(|s| s.to_string()),
        };

        self.record_invocation(invocation);

        result
    }

    fn record_invocation(&self, invocation: ToolInvocation) {
        let mut history = self.history.write();
        history.push(invocation);
        if history.len() > self.max_history {
            let excess = history.len() - self.max_history;
            history.drain(0..excess);
        }
    }

    pub fn history(&self) -> Vec<ToolInvocation> {
        self.history.read().clone()
    }

    pub fn history_for_tool(&self, tool_name: &str) -> Vec<ToolInvocation> {
        self.history
            .read()
            .iter()
            .filter(|inv| inv.tool_name == tool_name)
            .cloned()
            .collect()
    }

    pub fn clear_history(&self) {
        self.history.write().clear();
    }

    pub fn tool_count(&self) -> usize {
        self.tools.read().len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
