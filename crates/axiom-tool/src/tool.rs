//! Tool trait and metadata definitions.

use std::future::Future;
use std::pin::Pin;

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

pub type BoxToolFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, ToolError>> + Send + 'a>>;

pub trait Tool: Send + Sync + 'static {
    fn info(&self) -> ToolInfo;

    fn execute<'a>(&'a self, parameters: &'a Value) -> BoxToolFuture<'a>;

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

impl<F> Tool for SimpleTool<F>
where
    F: Fn(&Value) -> Result<Value, ToolError> + Send + Sync + 'static,
{
    fn info(&self) -> ToolInfo {
        self.info.clone()
    }

    fn execute<'a>(&'a self, parameters: &'a Value) -> BoxToolFuture<'a> {
        Box::pin(async move {
            self.validate(parameters)?;
            (self.handler)(parameters)
        })
    }
}
