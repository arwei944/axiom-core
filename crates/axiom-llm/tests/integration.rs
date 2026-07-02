use std::sync::Arc;

use axiom_llm::*;
use serde_json::json;

#[tokio::test]
async fn test_mock_completion() {
    let client = LlmClient::mock();

    let result = client.complete("Hello, world!").await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.text, "Mock completion response");
    assert_eq!(response.model, "mock-model");
    assert_eq!(response.usage.total_tokens, 30);
}

#[tokio::test]
async fn test_mock_chat() {
    let client = LlmClient::mock();

    let messages = vec![
        ChatMessage {
            role: MessageRole::System,
            content: "You are a helpful assistant.".to_string(),
            name: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: MessageRole::User,
            content: "Hello!".to_string(),
            name: None,
            tool_call_id: None,
        },
    ];

    let result = client.chat(&messages).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.message.role, MessageRole::Assistant);
    assert_eq!(response.message.content, "Mock chat response");
    assert_eq!(response.usage.total_tokens, 80);
}

#[tokio::test]
async fn test_custom_response() {
    let provider = Arc::new(mock::MockProvider::new());
    provider.set_completion_response("Custom response");
    let client = LlmClient::new(provider.clone());

    let response = client.complete("test").await.unwrap();
    assert_eq!(response.text, "Custom response");
    assert_eq!(provider.call_count(), 1);
}

#[tokio::test]
async fn test_token_budget() {
    let client = LlmClient::mock().with_token_budget(100);

    let response = client.complete("test").await.unwrap();
    assert_eq!(response.usage.total_tokens, 30);
    assert_eq!(client.remaining_budget(), 70);

    let response = client.complete("test").await.unwrap();
    assert_eq!(client.remaining_budget(), 40);

    let response = client.complete("test").await.unwrap();
    assert_eq!(client.remaining_budget(), 10);

    let result = client.complete("test").await;
    assert!(result.is_err());
    if let Err(LlmError::OutOfBudget) = result {
    } else {
        panic!("Expected OutOfBudget error");
    }
}

#[tokio::test]
async fn test_structured_output() {
    let provider = Arc::new(mock::MockProvider::new());
    provider.set_completion_response(&json!({"name": "test", "value": 42}).to_string());
    let client = LlmClient::new(provider);

    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "value": {"type": "number"}
        }
    });

    let result: Result<serde_json::Value, _> = client.structured_output("test", &schema).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["name"], "test");
    assert_eq!(value["value"], 42);
}

#[tokio::test]
async fn test_retry_on_failure() {
    let provider = Arc::new(mock::MockProvider::new());
    provider.set_fail_n_times(2);

    let retry_config = types::RetryConfig {
        max_retries: 3,
        initial_delay_ms: 10,
        max_delay_ms: 100,
        backoff_factor: 2.0,
    };

    let client = LlmClient::new(provider.clone()).with_retry_config(retry_config);

    let result = client.complete("test").await;
    assert!(result.is_ok());
    assert_eq!(provider.call_count(), 3);
}

#[tokio::test]
async fn test_retry_exhausted() {
    let provider = Arc::new(mock::MockProvider::new());
    provider.set_fail_n_times(5);

    let retry_config = types::RetryConfig {
        max_retries: 2,
        initial_delay_ms: 10,
        max_delay_ms: 100,
        backoff_factor: 2.0,
    };

    let client = LlmClient::new(provider.clone()).with_retry_config(retry_config);

    let result = client.complete("test").await;
    assert!(result.is_err());
    assert_eq!(provider.call_count(), 3);
}

#[tokio::test]
async fn test_reset_budget() {
    let client = LlmClient::mock().with_token_budget(100);

    let _ = client.complete("test").await.unwrap();
    assert_eq!(client.remaining_budget(), 70);

    client.reset_budget();
    assert_eq!(client.remaining_budget(), 100);
}