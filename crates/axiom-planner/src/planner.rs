//! Planner trait and common types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::step::PlanStep;

#[derive(Debug, thiserror::Error)]
pub enum PlannerError {
    #[error("planning failed: {0}")]
    PlanningFailed(String),
    #[error("execution error: {0}")]
    ExecutionError(String),
    #[error("max iterations reached")]
    MaxIterationsReached,
    #[error("llm error: {0}")]
    LlmError(String),
    #[error("tool error: {0}")]
    ToolError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningResult {
    pub success: bool,
    pub steps: Vec<PlanStep>,
    pub final_output: Option<String>,
    pub iterations: u32,
    pub total_duration_ms: u64,
}

impl PlanningResult {
    pub fn success(steps: Vec<PlanStep>, output: impl Into<String>) -> Self {
        Self {
            success: true,
            steps,
            final_output: Some(output.into()),
            iterations: 0,
            total_duration_ms: 0,
        }
    }

    pub fn failure(steps: Vec<PlanStep>, reason: impl Into<String>) -> Self {
        Self {
            success: false,
            steps,
            final_output: Some(reason.into()),
            iterations: 0,
            total_duration_ms: 0,
        }
    }
}

#[async_trait]
pub trait Planner: Send + Sync {
    fn name(&self) -> &str;
    async fn plan_and_execute(&self, goal: &str, context: &str) -> Result<PlanningResult, PlannerError>;
    fn max_iterations(&self) -> u32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannerStrategy {
    ReAct,
    PlanAndExecute,
}

impl fmt::Display for PlannerStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlannerStrategy::ReAct => write!(f, "react"),
            PlannerStrategy::PlanAndExecute => write!(f, "plan_and_execute"),
        }
    }
}