use std::sync::Arc;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::types::{
    ChatMessage, ChatResponse, CompletionResponse, LlmError, MessageRole, TokenUsage,
};
use crate::LlmProvider;

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: Arc<Client>,
}

#[derive(Serialize)]
struct OpenAICompletionRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    max_tokens: Option<u32>,
    temperature: Option<f64>,
}

#[derive(Deserialize)]
struct OpenAICompletionChoice {
    text: String,
}

#[derive(Deserialize)]
struct OpenAICompletionResponse {
    choices: Vec<OpenAICompletionChoice>,
    usage: OpenAITokenUsage,
    model: String,
}

#[derive(Deserialize)]
struct OpenAITokenUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[derive(Serialize)]
struct OpenAIChatRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAIChatMessage<'a>>,
    max_tokens: Option<u32>,
    temperature: Option<f64>,
}

#[derive(Serialize)]
struct OpenAIChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct OpenAIChatChoice {
    message: OpenAIChatMessageResponse,
}

#[derive(Deserialize)]
struct OpenAIChatMessageResponse {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChatChoice>,
    usage: OpenAITokenUsage,
    model: String,
}

#[derive(Deserialize)]
struct OpenAIErrorResponse {
    error: OpenAIErrorDetails,
}

#[derive(Deserialize)]
struct OpenAIErrorDetails {
    message: String,
    r#type: String,
}

impl OpenAIProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: "gpt-4o-mini".to_string(),
            base_url: "https://api.openai.com".to_string(),
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

impl LlmProvider for OpenAIProvider {
    fn complete<'a>(
        &'a self,
        prompt: &'a str,
    ) -> crate::BoxLlmFuture<'a, Result<CompletionResponse, LlmError>> {
        Box::pin(async move {
            let request = OpenAICompletionRequest {
                model: &self.model,
                prompt,
                max_tokens: Some(2048),
                temperature: Some(0.7),
            };

            let url = format!("{}/v1/completions", self.base_url);
            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| LlmError::Connection(e.to_string()))?;

            if !response.status().is_success() {
                let error_body = response.text().await.unwrap_or_default();
                if let Ok(error) = serde_json::from_str::<OpenAIErrorResponse>(&error_body) {
                    if error.error.r#type.contains("rate_limit") {
                        return Err(LlmError::RateLimited);
                    }
                    return Err(LlmError::ApiError(error.error.message));
                }
                return Err(LlmError::ApiError(error_body));
            }

            let result: OpenAICompletionResponse =
                response.json().await.map_err(|e| LlmError::Serialization(e.to_string()))?;

            Ok(CompletionResponse {
                text: result.choices.into_iter().map(|c| c.text).collect(),
                usage: TokenUsage {
                    prompt_tokens: result.usage.prompt_tokens,
                    completion_tokens: result.usage.completion_tokens,
                    total_tokens: result.usage.total_tokens,
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
            let openai_messages: Vec<OpenAIChatMessage<'a>> = messages
                .iter()
                .map(|m| OpenAIChatMessage {
                    role: match m.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::Tool => "tool",
                    },
                    content: &m.content,
                })
                .collect();

            let request = OpenAIChatRequest {
                model: &self.model,
                messages: openai_messages,
                max_tokens: Some(4096),
                temperature: Some(0.7),
            };

            let url = format!("{}/v1/chat/completions", self.base_url);
            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| LlmError::Connection(e.to_string()))?;

            if !response.status().is_success() {
                let error_body = response.text().await.unwrap_or_default();
                if let Ok(error) = serde_json::from_str::<OpenAIErrorResponse>(&error_body) {
                    if error.error.r#type.contains("rate_limit") {
                        return Err(LlmError::RateLimited);
                    }
                    return Err(LlmError::ApiError(error.error.message));
                }
                return Err(LlmError::ApiError(error_body));
            }

            let result: OpenAIChatResponse =
                response.json().await.map_err(|e| LlmError::Serialization(e.to_string()))?;

            let choice = result
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| LlmError::ApiError("no choices returned".to_string()))?;

            Ok(ChatResponse {
                message: ChatMessage {
                    role: match choice.message.role.as_str() {
                        "system" => MessageRole::System,
                        "user" => MessageRole::User,
                        "assistant" => MessageRole::Assistant,
                        "tool" => MessageRole::Tool,
                        _ => MessageRole::Assistant,
                    },
                    content: choice.message.content,
                    name: None,
                    tool_call_id: None,
                },
                usage: TokenUsage {
                    prompt_tokens: result.usage.prompt_tokens,
                    completion_tokens: result.usage.completion_tokens,
                    total_tokens: result.usage.total_tokens,
                },
                model: result.model,
            })
        })
    }
}
