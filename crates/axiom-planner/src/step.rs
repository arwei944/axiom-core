//! Step types for plan execution.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

impl StepStatus {
    pub fn is_finished(&self) -> bool {
        matches!(self, StepStatus::Completed | StepStatus::Failed | StepStatus::Skipped)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            StepStatus::Pending => "pending",
            StepStatus::InProgress => "in_progress",
            StepStatus::Completed => "completed",
            StepStatus::Failed => "failed",
            StepStatus::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub index: usize,
    pub description: String,
    pub status: StepStatus,
    pub tool_name: Option<String>,
    pub expected_output: Option<String>,
    pub actual_output: Option<String>,
    pub dependencies: Vec<usize>,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl PlanStep {
    pub fn new(index: usize, description: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            index,
            description: description.into(),
            status: StepStatus::Pending,
            tool_name: None,
            expected_output: None,
            actual_output: None,
            dependencies: Vec::new(),
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_name = Some(tool_name.into());
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<usize>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn mark_started(&mut self) {
        self.status = StepStatus::InProgress;
    }

    pub fn mark_completed(&mut self, output: impl Into<String>) {
        self.status = StepStatus::Completed;
        self.actual_output = Some(output.into());
    }

    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = StepStatus::Failed;
        self.actual_output = Some(error.into());
        self.retry_count += 1;
    }

    pub fn mark_skipped(&mut self, reason: impl Into<String>) {
        self.status = StepStatus::Skipped;
        self.actual_output = Some(reason.into());
    }
}