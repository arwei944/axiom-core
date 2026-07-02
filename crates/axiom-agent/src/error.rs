//! Unified error types for the agent crate.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Planner error: {0}")]
    Planner(String),

    #[error("Prompt error: {0}")]
    Prompt(String),

    #[error("Identity error: {0}")]
    Identity(String),

    #[error("Agent not configured: {0}")]
    NotConfigured(String),

    #[error("Agent already started")]
    AlreadyStarted,

    #[error("Agent not started")]
    NotStarted,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Runtime error: {0}")]
    Runtime(String),
}

impl From<axiom_llm::LlmError> for AgentError {
    fn from(e: axiom_llm::LlmError) -> Self {
        AgentError::Llm(e.to_string())
    }
}

impl From<axiom_tool::ToolError> for AgentError {
    fn from(e: axiom_tool::ToolError) -> Self {
        AgentError::Tool(e.to_string())
    }
}

impl From<axiom_memory::MemoryError> for AgentError {
    fn from(e: axiom_memory::MemoryError) -> Self {
        AgentError::Memory(e.to_string())
    }
}

impl From<axiom_planner::PlannerError> for AgentError {
    fn from(e: axiom_planner::PlannerError) -> Self {
        AgentError::Planner(e.to_string())
    }
}

impl From<axiom_prompt::PromptError> for AgentError {
    fn from(e: axiom_prompt::PromptError) -> Self {
        AgentError::Prompt(e.to_string())
    }
}

impl From<axiom_identity::IdentityError> for AgentError {
    fn from(e: axiom_identity::IdentityError) -> Self {
        AgentError::Identity(e.to_string())
    }
}

pub type AgentResult<T> = Result<T, AgentError>;
