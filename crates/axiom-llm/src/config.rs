use std::env;

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub default_provider: LlmProviderType,
    pub default_model: String,
    pub request_timeout_ms: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProviderType {
    OpenAI,
    Claude,
    Mock,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            default_provider: Self::detect_default_provider(),
            default_model: "gpt-4o-mini".to_string(),
            request_timeout_ms: 60000,
            max_retries: 3,
        }
    }
}

impl LlmConfig {
    fn detect_default_provider() -> LlmProviderType {
        if env::var("OPENAI_API_KEY").is_ok() {
            return LlmProviderType::OpenAI;
        }
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            return LlmProviderType::Claude;
        }
        LlmProviderType::Mock
    }

    pub fn from_env() -> Self {
        Self::default()
    }

    pub fn openai_api_key(&self) -> Option<&str> {
        self.openai_api_key.as_deref()
    }

    pub fn anthropic_api_key(&self) -> Option<&str> {
        self.anthropic_api_key.as_deref()
    }

    pub fn has_openai_key(&self) -> bool {
        self.openai_api_key.is_some()
    }

    pub fn has_anthropic_key(&self) -> bool {
        self.anthropic_api_key.is_some()
    }
}
