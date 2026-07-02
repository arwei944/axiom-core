//! Tool trait and metadata definitions.

use async_trait::async_trait;
use serde_json::Value;

use crate::error::ToolError;

#[derive(Debug, Clone)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub schema: Value,
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub required_permission: Option<String>,
    pub version: String,
}

#[async_trait]
pub trait Tool: Send + Sync + 'static {
    fn info(&self) -> ToolInfo;

    async fn execute(&self, parameters: &Value) -> Result<Value, ToolError>;

    fn validate(&self, parameters: &Value) -> Result<(), ToolError> {
        let info = self.info();

        for param in &info.parameters {
            if param.required && parameters.get(&param.name).is_none() {
                return Err(ToolError::InvalidParameters(format!(
                    "required parameter '{}' is missing",
                    param.name
                )));
            }
        }

        Ok(())
    }
}

pub struct SimpleTool<F>
where
    F: Fn(&Value) -> Result<Value, ToolError> + Send + Sync + 'static,
{
    info: ToolInfo,
    handler: F,
}

impl<F> SimpleTool<F>
where
    F: Fn(&Value) -> Result<Value, ToolError> + Send + Sync + 'static,
{
    pub fn new(info: ToolInfo, handler: F) -> Self {
        Self { info, handler }
    }
}

#[async_trait]
impl<F> Tool for SimpleTool<F>
where
    F: Fn(&Value) -> Result<Value, ToolError> + Send + Sync + 'static,
{
    fn info(&self) -> ToolInfo {
        self.info.clone()
    }

    async fn execute(&self, parameters: &Value) -> Result<Value, ToolError> {
        self.validate(parameters)?;
        (self.handler)(parameters)
    }
}