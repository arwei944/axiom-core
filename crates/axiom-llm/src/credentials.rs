//! Environment-based secrets for LLM providers (no multi-tenant IAM).
//!
//! **Never** hardcode API keys in source. Prefer:
//! - `AXIOM_LLM_API_KEY` — primary product key
//! - `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` — provider-specific fallbacks
//! - `AXIOM_LLM_MOCK=1` — force mock / offline path

use crate::types::LlmError;

pub const ENV_AXIOM_LLM_API_KEY: &str = "AXIOM_LLM_API_KEY";
pub const ENV_OPENAI_API_KEY: &str = "OPENAI_API_KEY";
pub const ENV_ANTHROPIC_API_KEY: &str = "ANTHROPIC_API_KEY";
pub const ENV_AXIOM_LLM_MOCK: &str = "AXIOM_LLM_MOCK";

/// Load a required secret from the environment.
pub fn require_env(name: &str) -> Result<String, LlmError> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        Ok(_) => Err(LlmError::InvalidRequest(format!("env `{name}` is empty"))),
        Err(_) => Err(LlmError::InvalidRequest(format!(
            "env `{name}` is not set — export the key or set {ENV_AXIOM_LLM_MOCK}=1"
        ))),
    }
}

pub fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

pub fn mock_mode() -> bool {
    matches!(
        std::env::var(ENV_AXIOM_LLM_MOCK).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

/// Resolve a product LLM API key from well-known env names.
pub fn resolve_llm_api_key() -> Result<String, LlmError> {
    if mock_mode() {
        return Err(LlmError::InvalidRequest(
            "mock mode active — no live credential".into(),
        ));
    }
    if let Some(k) = optional_env(ENV_AXIOM_LLM_API_KEY) {
        return Ok(k);
    }
    if let Some(k) = optional_env(ENV_OPENAI_API_KEY) {
        return Ok(k);
    }
    if let Some(k) = optional_env(ENV_ANTHROPIC_API_KEY) {
        return Ok(k);
    }
    Err(LlmError::InvalidRequest(format!(
        "no LLM API key in {ENV_AXIOM_LLM_API_KEY}/{ENV_OPENAI_API_KEY}/{ENV_ANTHROPIC_API_KEY}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_env_missing() {
        let name = "AXIOM_LLM_TEST_MISSING_KEY_XYZ";
        std::env::remove_var(name);
        let err = require_env(name).unwrap_err();
        assert!(err.to_string().contains("not set"), "{err}");
    }

    #[test]
    fn resolve_prefers_axiom_key() {
        let prev_a = std::env::var(ENV_AXIOM_LLM_API_KEY).ok();
        let prev_m = std::env::var(ENV_AXIOM_LLM_MOCK).ok();
        std::env::remove_var(ENV_AXIOM_LLM_MOCK);
        std::env::set_var(ENV_AXIOM_LLM_API_KEY, "sk-axiom-test");
        let k = resolve_llm_api_key().expect("key");
        assert_eq!(k, "sk-axiom-test");
        match prev_a {
            Some(v) => std::env::set_var(ENV_AXIOM_LLM_API_KEY, v),
            None => std::env::remove_var(ENV_AXIOM_LLM_API_KEY),
        }
        match prev_m {
            Some(v) => std::env::set_var(ENV_AXIOM_LLM_MOCK, v),
            None => std::env::remove_var(ENV_AXIOM_LLM_MOCK),
        }
    }
}
