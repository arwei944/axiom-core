//! LLM client abstraction for axiom-core.
//!
//! Provides:
//! - Multi-provider LLM client (OpenAI, Anthropic, etc.)
//! - Mock provider for testing
//! - Automatic retry with exponential backoff
//! - Structured output with JSON Schema validation
//! - Token budget management

pub mod client;
pub mod mock;
pub mod types;

pub use client::{LlmClient, LlmProvider};
pub use types::{
    ChatMessage, ChatResponse, CompletionResponse, LlmError, MessageRole, TokenUsage,
};