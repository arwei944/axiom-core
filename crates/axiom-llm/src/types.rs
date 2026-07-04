//! LLM types and data structures.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("rate limited")]
    RateLimited,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("timeout")]
    Timeout,
    #[error("out of budget")]
    OutOfBudget,
    #[error("unknown error")]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub text: String,
    pub usage: TokenUsage,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub usage: TokenUsage,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 10000,
            backoff_factor: 2.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub total_budget: u64,
    pub used_tokens: u64,
}

impl TokenBudget {
    pub fn new(budget: u64) -> Self {
        Self {
            total_budget: budget,
            used_tokens: 0,
        }
    }

    pub fn can_use(&self, tokens: u64) -> bool {
        self.used_tokens + tokens <= self.total_budget
    }

    pub fn record_usage(&mut self, tokens: u64) -> Result<(), LlmError> {
        if !self.can_use(tokens) {
            return Err(LlmError::OutOfBudget);
        }
        self.used_tokens += tokens;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.used_tokens = 0;
    }

    pub fn remaining(&self) -> u64 {
        self.total_budget - self.used_tokens
    }
}
