//! Handoff protocol as a **Signal payload** (not a second message bus).
//!
//! Semantics adapted from low-entropy-core handoff types; authority is Witness-only.

use serde::{Deserialize, Serialize};

/// Structured agent handoff request — the only legal inter-agent work transfer shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandoffRequest {
    pub token: String,
    pub source_agent: String,
    pub target_agent: String,
    /// Controlled action the workbench may execute (allow-list enforced).
    pub intent: String,
    /// Human-readable task body.
    pub payload: String,
    /// Optional deadline (ns wall clock); 0 = none.
    #[serde(default)]
    pub deadline_ns: u64,
    /// Permission tags (e.g. "workbench.execute").
    #[serde(default)]
    pub permissions: Vec<String>,
}

impl HandoffRequest {
    pub fn new(
        token: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
        intent: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            token: token.into(),
            source_agent: source.into(),
            target_agent: target.into(),
            intent: intent.into(),
            payload: payload.into(),
            deadline_ns: 0,
            permissions: vec!["workbench.execute".into()],
        }
    }

    pub fn validate_shape(&self) -> Result<(), String> {
        if self.token.trim().is_empty() {
            return Err("token required".into());
        }
        if self.source_agent.trim().is_empty() || self.target_agent.trim().is_empty() {
            return Err("source_agent and target_agent required".into());
        }
        if self.intent.trim().is_empty() {
            return Err("intent required".into());
        }
        if self.payload.trim().is_empty() {
            return Err("payload required".into());
        }
        Ok(())
    }
}

/// Result of a closed Workbench loop after handoff admit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandoffResult {
    pub success: bool,
    pub token: String,
    pub proposal: String,
    pub execution_summary: String,
    pub message: String,
}

/// Deterministic proposal produced by the controlled Workbench (no free-form LLM required).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkbenchProposal {
    pub plan_id: String,
    pub steps: Vec<String>,
    pub allowed_action: String,
}

/// Resource / capability envelope for the controlled Workbench sandbox (T11).
///
/// Unrestricted LLM code execution is **out of constitution scope**.
/// This struct is the product floor: allow-list + limits + optional WASM plugin id.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkbenchLimits {
    /// Max proposal steps that may execute.
    pub max_steps: u32,
    /// Max payload bytes accepted into sandbox.
    pub max_payload_bytes: usize,
    /// Wall-clock budget (ms) for the whole closed loop (advisory).
    pub timeout_ms: u64,
    /// When set, execute via named WASM/native plugin sandbox instead of in-process handlers.
    pub plugin_id: Option<String>,
    /// Memory ceiling advertised to sandbox (MB).
    pub memory_limit_mb: u64,
}

impl Default for WorkbenchLimits {
    fn default() -> Self {
        Self {
            max_steps: 16,
            max_payload_bytes: 64 * 1024,
            timeout_ms: 5_000,
            plugin_id: None,
            memory_limit_mb: 64,
        }
    }
}

impl WorkbenchLimits {
    pub fn commercial_default() -> Self {
        Self::default()
    }
}

/// Allow-listed workbench actions (U3 commercial floor — not unrestricted code exec).
pub fn is_allowed_intent(intent: &str) -> bool {
    matches!(
        intent,
        "echo" | "task_plan" | "summarize" | "validate_manifest" | "plugin_echo"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_token() {
        let h = HandoffRequest::new("", "a", "b", "echo", "x");
        assert!(h.validate_shape().is_err());
    }

    #[test]
    fn allow_list() {
        assert!(is_allowed_intent("echo"));
        assert!(!is_allowed_intent("shell_raw"));
    }
}
