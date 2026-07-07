//! LLM client abstraction.

use std::sync::Arc;

use parking_lot::RwLock;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::mock::MockProvider;
use crate::types::{
    ChatMessage, ChatResponse, CompletionResponse, LlmError, RetryConfig, TokenBudget, TokenUsage,
};

pub trait LlmProvider: Send + Sync + 'static {
    fn complete<'a>(
        &'a self,
        prompt: &'a str,
    ) -> crate::BoxLlmFuture<'a, Result<CompletionResponse, LlmError>>;
    fn chat<'a>(
        &'a self,
        messages: &'a [ChatMessage],
    ) -> crate::BoxLlmFuture<'a, Result<ChatResponse, LlmError>>;
}

pub struct LlmClient {
    provider: Arc<dyn LlmProvider>,
    retry_config: RetryConfig,
    token_budget: Arc<RwLock<TokenBudget>>,
}

impl LlmClient {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider,
            retry_config: RetryConfig::default(),
            token_budget: Arc::new(RwLock::new(TokenBudget::new(u64::MAX))),
        }
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    pub fn with_token_budget(mut self, budget: u64) -> Self {
        self.token_budget = Arc::new(RwLock::new(TokenBudget::new(budget)));
        self
    }

    pub fn mock() -> Self {
        Self::new(Arc::new(MockProvider::default()))
    }

    fn is_retryable(error: &LlmError) -> bool {
        matches!(error, LlmError::Connection(_) | LlmError::RateLimited | LlmError::Timeout)
    }

    async fn with_retry<F, Fut, T>(&self, f: F) -> Result<T, LlmError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, LlmError>>,
    {
        let mut attempts = 0;
        let mut delay_ms = self.retry_config.initial_delay_ms;

        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !Self::is_retryable(&e) || attempts >= self.retry_config.max_retries {
                        return Err(e);
                    }
                    attempts += 1;
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms as f64 * self.retry_config.backoff_factor) as u64;
                    delay_ms = delay_ms.min(self.retry_config.max_delay_ms);
                }
            }
        }
    }

    pub async fn complete(&self, prompt: &str) -> Result<CompletionResponse, LlmError> {
        let provider = self.provider.clone();
        let prompt_owned = prompt.to_string();

        let result = self
            .with_retry(move || {
                let p = provider.clone();
                let prompt = prompt_owned.clone();
                async move { p.complete(&prompt).await }
            })
            .await?;

        self.token_budget.write().record_usage(result.usage.total_tokens)?;

        Ok(result)
    }

    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
        let provider = self.provider.clone();
        let messages_owned = messages.to_vec();

        let result = self
            .with_retry(move || {
                let p = provider.clone();
                let msgs = messages_owned.clone();
                async move { p.chat(&msgs).await }
            })
            .await?;

        self.token_budget.write().record_usage(result.usage.total_tokens)?;

        Ok(result)
    }

    pub async fn structured_output<T>(&self, prompt: &str, _schema: &Value) -> Result<T, LlmError>
    where
        T: DeserializeOwned,
    {
        let response = self.complete(prompt).await?;

        serde_json::from_str(&response.text).map_err(|e| LlmError::Validation(e.to_string()))
    }

    pub async fn structured_chat<T>(
        &self,
        messages: &[ChatMessage],
        _schema: &Value,
    ) -> Result<T, LlmError>
    where
        T: DeserializeOwned,
    {
        let response = self.chat(messages).await?;

        serde_json::from_str(&response.message.content)
            .map_err(|e| LlmError::Validation(e.to_string()))
    }

    pub fn token_usage(&self) -> TokenUsage {
        let budget = self.token_budget.read();
        TokenUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: budget.used_tokens }
    }

    pub fn remaining_budget(&self) -> u64 {
        self.token_budget.read().remaining()
    }

    pub fn reset_budget(&self) {
        self.token_budget.write().reset();
    }
}
