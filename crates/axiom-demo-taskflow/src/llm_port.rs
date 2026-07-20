//! Env-credential LLM propose Port (commercial floor).
//!
//! Credentials come **only** from environment variables — never from source.
//! When `AXIOM_LLM_MOCK=1` (or key missing with mock default in tests), the Port
//! returns a deterministic proposal without network I/O.

use axiom_isa::{IsaError, IsaResult, Port};
use std::env;

/// Env var for API key (OpenAI-compatible or provider-agnostic product key).
pub const ENV_LLM_API_KEY: &str = "AXIOM_LLM_API_KEY";
/// When `1`/`true`, never call network; always mock.
pub const ENV_LLM_MOCK: &str = "AXIOM_LLM_MOCK";

/// Thin secrets helper — product code should not scatter `env::var` for keys.
pub struct EnvSecrets;

impl EnvSecrets {
    pub fn require(name: &str) -> Result<String, String> {
        match env::var(name) {
            Ok(v) if !v.trim().is_empty() => Ok(v),
            Ok(_) => Err(format!("env `{name}` is empty")),
            Err(_) => Err(format!("env `{name}` is not set")),
        }
    }

    pub fn optional(name: &str) -> Option<String> {
        env::var(name).ok().filter(|v| !v.trim().is_empty())
    }

    pub fn mock_mode() -> bool {
        matches!(
            env::var(ENV_LLM_MOCK).as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
        )
    }
}

#[derive(Debug, Clone)]
pub struct LlmProposeInput {
    pub intent: String,
    pub payload: String,
}

#[derive(Debug, Clone)]
pub struct LlmProposeOutput {
    pub text: String,
    pub model: String,
    pub mock: bool,
}

/// Port: propose text for workbench-style flows using env credentials or mock.
pub struct EnvLlmProposePort {
    /// When true, never require a real key (unit tests / CI).
    pub force_mock: bool,
}

impl Default for EnvLlmProposePort {
    fn default() -> Self {
        Self { force_mock: false }
    }
}

impl EnvLlmProposePort {
    pub fn mock_only() -> Self {
        Self { force_mock: true }
    }

    pub fn from_env() -> Self {
        Self {
            force_mock: EnvSecrets::mock_mode(),
        }
    }
}

impl Port<LlmProposeInput, LlmProposeOutput> for EnvLlmProposePort {
    fn name(&self) -> &str {
        "env_llm_propose"
    }

    fn call(&mut self, input: LlmProposeInput) -> IsaResult<LlmProposeOutput> {
        let mock = self.force_mock || EnvSecrets::mock_mode();
        if mock {
            return Ok(LlmProposeOutput {
                text: format!(
                    "mock-propose intent={} body={}",
                    input.intent,
                    input.payload.chars().take(64).collect::<String>()
                ),
                model: "mock-env".into(),
                mock: true,
            });
        }

        // Real path: require credential (network call intentionally not wired in floor —
        // missing key fails closed; present key returns a non-network stub that proves
        // credential loading without shipping secrets or depending on live APIs in CI).
        let key = EnvSecrets::require(ENV_LLM_API_KEY).map_err(|e| {
            IsaError::port(
                "env_llm_propose",
                format!("credential error: {e} (set {ENV_LLM_MOCK}=1 for mock)"),
            )
        })?;

        // Do not log the key. Fingerprint only for proof-of-load in tests.
        let fingerprint = key.chars().take(4).collect::<String>();
        Ok(LlmProposeOutput {
            text: format!(
                "credential-loaded fp={fingerprint}… intent={} (network provider not invoked in product floor)",
                input.intent
            ),
            model: "env-credential-stub".into(),
            mock: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_isa::Port;

    #[test]
    fn mock_port_never_needs_key() {
        let mut p = EnvLlmProposePort::mock_only();
        let out = p
            .call(LlmProposeInput {
                intent: "summarize".into(),
                payload: "hello".into(),
            })
            .expect("mock");
        assert!(out.mock);
        assert!(out.text.contains("summarize"));
    }

    #[test]
    fn missing_key_errors_when_not_mock() {
        // Ensure mock env is not forcing mock for this process snapshot.
        let prev_mock = env::var(ENV_LLM_MOCK).ok();
        let prev_key = env::var(ENV_LLM_API_KEY).ok();
        env::remove_var(ENV_LLM_MOCK);
        env::remove_var(ENV_LLM_API_KEY);

        let mut p = EnvLlmProposePort {
            force_mock: false,
        };
        let err = p
            .call(LlmProposeInput {
                intent: "echo".into(),
                payload: "x".into(),
            })
            .expect_err("must fail without key");
        let msg = err.to_string();
        assert!(
            msg.contains("credential") || msg.contains(ENV_LLM_API_KEY) || msg.contains("not set"),
            "{msg}"
        );

        // restore
        match prev_mock {
            Some(v) => env::set_var(ENV_LLM_MOCK, v),
            None => env::remove_var(ENV_LLM_MOCK),
        }
        match prev_key {
            Some(v) => env::set_var(ENV_LLM_API_KEY, v),
            None => env::remove_var(ENV_LLM_API_KEY),
        }
    }

    #[test]
    fn with_key_loads_without_network() {
        let prev_mock = env::var(ENV_LLM_MOCK).ok();
        let prev_key = env::var(ENV_LLM_API_KEY).ok();
        env::remove_var(ENV_LLM_MOCK);
        env::set_var(ENV_LLM_API_KEY, "sk-test-not-real-0000");

        let mut p = EnvLlmProposePort {
            force_mock: false,
        };
        let out = p
            .call(LlmProposeInput {
                intent: "task_plan".into(),
                payload: "ship".into(),
            })
            .expect("key present");
        assert!(!out.mock);
        assert!(out.text.contains("credential-loaded"), "{:?}", out.text);

        match prev_mock {
            Some(v) => env::set_var(ENV_LLM_MOCK, v),
            None => env::remove_var(ENV_LLM_MOCK),
        }
        match prev_key {
            Some(v) => env::set_var(ENV_LLM_API_KEY, v),
            None => env::remove_var(ENV_LLM_API_KEY),
        }
    }
}
