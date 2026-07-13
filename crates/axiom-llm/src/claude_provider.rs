use std::sync::Arc;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::types::{
    ChatMessage, ChatResponse, CompletionResponse, LlmError, MessageRole, TokenUsage,
};
use crate::LlmProvider;

#[derive(Debug, Clone)]
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: Arc<Client>,
}

#[derive(Serialize)]
struct ClaudeCompletionRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    max_tokens_to_sample: u32,
    temperature: Option<f64>,
}

#[derive(Deserialize)]
struct ClaudeCompletionResponse {
    completion: String,
    #[allow(dead_code)]
    stop_reason: Option<String>,
    model: String,
    usage: ClaudeTokenUsage,
}

#[derive(Deserialize)]
struct ClaudeTokenUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Serialize)]
struct ClaudeChatRequest<'a> {
    model: &'a str,
    messages: Vec<ClaudeChatMessage<'a>>,
    max_tokens: u32,
    temperature: Option<f64>,
}

#[derive(Serialize)]
struct ClaudeChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ClaudeChatResponse {
    content: Vec<ClaudeChatContent>,
    model: String,
    usage: ClaudeTokenUsage,
}

#[derive(Deserialize)]
struct ClaudeChatContent {
    r#type: String,
    text: String,
}

#[derive(Deserialize)]
struct ClaudeErrorResponse {
    error: ClaudeErrorDetails,
}

#[derive(Deserialize)]
struct ClaudeErrorDetails {
    message: String,
    r#type: String,
}

impl ClaudeProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            client: Arc::new(Client::new()),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }
}

impl LlmProvider for ClaudeProvider {
    fn complete<'a>(
        &'a self,
        prompt: &'a str,
    ) -> crate::BoxLlmFuture<'a, Result<CompletionResponse, LlmError>> {
        Box::pin(async move {
            let request = ClaudeCompletionRequest {
                model: &self.model,
                prompt,
                max_tokens_to_sample: 2048,
                temperature: Some(0.7),
            };

            let url = format!("{}/v1/complete", self.base_url);
            let response = self
                .client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .json(&request)
                .send()
                .await
                .map_err(|e| LlmError::Connection(e.to_string()))?;

            if !response.status().is_success() {
                let error_body = response.text().await.unwrap_or_default();
                if let Ok(error) = serde_json::from_str::<ClaudeErrorResponse>(&error_body) {
                    if error.error.r#type.contains("rate_limit") {
                        return Err(LlmError::RateLimited);
                    }
                    return Err(LlmError::ApiError(error.error.message));
                }
                return Err(LlmError::ApiError(error_body));
            }

            let result: ClaudeCompletionResponse =
                response.json().await.map_err(|e| LlmError::Serialization(e.to_string()))?;

            Ok(CompletionResponse {
                text: result.completion,
                usage: TokenUsage {
                    prompt_tokens: result.usage.input_tokens,
                    completion_tokens: result.usage.output_tokens,
                    total_tokens: result.usage.input_tokens + result.usage.output_tokens,
                },
                model: result.model,
            })
        })
    }

    fn chat<'a>(
        &'a self,
        messages: &'a [ChatMessage],
    ) -> crate::BoxLlmFuture<'a, Result<ChatResponse, LlmError>> {
        Box::pin(async move {
            let claude_messages: Vec<ClaudeChatMessage<'a>> = messages
                .iter()
                .map(|m| ClaudeChatMessage {
                    role: match m.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::Tool => "tool",
                    },
                    content: &m.content,
                })
                .collect();

            let request = ClaudeChatRequest {
                model: &self.model,
                messages: claude_messages,
                max_tokens: 4096,
                temperature: Some(0.7),
            };

            let url = format!("{}/v1/messages", self.base_url);
            let response = self
                .client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .json(&request)
                .send()
                .await
                .map_err(|e| LlmError::Connection(e.to_string()))?;

            if !response.status().is_success() {
                let error_body = response.text().await.unwrap_or_default();
                if let Ok(error) = serde_json::from_str::<ClaudeErrorResponse>(&error_body) {
                    if error.error.r#type.contains("rate_limit") {
                        return Err(LlmError::RateLimited);
                    }
                    return Err(LlmError::ApiError(error.error.message));
                }
                return Err(LlmError::ApiError(error_body));
            }

            let result: ClaudeChatResponse =
                response.json().await.map_err(|e| LlmError::Serialization(e.to_string()))?;

            let content = result
                .content
                .into_iter()
                .find(|c| c.r#type == "text")
                .ok_or_else(|| LlmError::ApiError("no text content returned".to_string()))?;

            Ok(ChatResponse {
                message: ChatMessage {
                    role: MessageRole::Assistant,
                    content: content.text,
                    name: None,
                    tool_call_id: None,
                },
                usage: TokenUsage {
                    prompt_tokens: result.usage.input_tokens,
                    completion_tokens: result.usage.output_tokens,
                    total_tokens: result.usage.input_tokens + result.usage.output_tokens,
                },
                model: result.model,
            })
        })
    }
}
