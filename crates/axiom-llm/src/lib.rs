//! LLM client abstraction for axiom-kernel.
//!
//! Provides:
//! - Multi-provider LLM client (OpenAI, Anthropic, etc.)
//! - Mock provider for testing
//! - Automatic retry with exponential backoff
//! - Structured output with JSON Schema validation
//! - Token budget management

pub mod claude_provider;
pub mod client;
pub mod config;
pub mod kernel;
pub mod mock;
pub mod openai_provider;
pub mod types;

use std::future::Future;
use std::pin::Pin;

pub type BoxLlmFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub use claude_provider::ClaudeProvider;
pub use client::{LlmClient, LlmProvider};
pub use config::{LlmConfig, LlmProviderType};
pub use kernel::LlmKernelAdapter;
pub use mock::MockProvider;
pub use openai_provider::OpenAIProvider;
pub use types::{ChatMessage, ChatResponse, CompletionResponse, LlmError, MessageRole, TokenUsage};
