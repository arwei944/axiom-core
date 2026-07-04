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

use std::future::Future;
use std::pin::Pin;

pub type BoxLlmFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub use client::{LlmClient, LlmProvider};
pub use types::{ChatMessage, ChatResponse, CompletionResponse, LlmError, MessageRole, TokenUsage};
