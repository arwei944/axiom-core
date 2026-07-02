//! Mock LLM provider for testing.

use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::types::{ChatMessage, ChatResponse, CompletionResponse, LlmError, MessageRole, TokenUsage};

pub struct MockProvider {
    completion_response: Arc<RwLock<String>>,
    chat_response: Arc<RwLock<String>>,
    call_count: Arc<RwLock<u64>>,
    fail_count: Arc<RwLock<u32>>,
    fail_n_times: Arc<RwLock<u32>>,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self {
            completion_response: Arc::new(RwLock::new("Mock completion response".to_string())),
            chat_response: Arc::new(RwLock::new("Mock chat response".to_string())),
            call_count: Arc::new(RwLock::new(0)),
            fail_count: Arc::new(RwLock::new(0)),
            fail_n_times: Arc::new(RwLock::new(0)),
        }
    }
}

impl MockProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_completion_response(&self, response: &str) {
        *self.completion_response.write() = response.to_string();
    }

    pub fn set_chat_response(&self, response: &str) {
        *self.chat_response.write() = response.to_string();
    }

    pub fn set_fail_n_times(&self, n: u32) {
        *self.fail_n_times.write() = n;
        *self.fail_count.write() = 0;
    }

    pub fn call_count(&self) -> u64 {
        *self.call_count.read()
    }

    fn check_fail(&self) -> Result<(), LlmError> {
        *self.call_count.write() += 1;

        let mut fail_count = self.fail_count.write();
        let fail_n_times = *self.fail_n_times.read();
        if *fail_count < fail_n_times {
            *fail_count += 1;
            return Err(LlmError::RateLimited);
        }
        Ok(())
    }
}

#[async_trait]
impl super::LlmProvider for MockProvider {
    async fn complete(&self, _prompt: &str) -> Result<CompletionResponse, LlmError> {
        self.check_fail()?;

        Ok(CompletionResponse {
            text: self.completion_response.read().clone(),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
            model: "mock-model".to_string(),
        })
    }

    async fn chat(&self, _messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
        self.check_fail()?;

        Ok(ChatResponse {
            message: ChatMessage {
                role: MessageRole::Assistant,
                content: self.chat_response.read().clone(),
                name: None,
                tool_call_id: None,
            },
            usage: TokenUsage {
                prompt_tokens: 50,
                completion_tokens: 30,
                total_tokens: 80,
            },
            model: "mock-model".to_string(),
        })
    }
}