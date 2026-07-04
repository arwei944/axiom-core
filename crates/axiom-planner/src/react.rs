//! ReAct (Reason + Act) planner implementation.

use std::sync::Arc;

use crate::planner::{BoxPlannerFuture, Planner, PlannerError, PlanningResult};
use crate::step::PlanStep;

pub struct ReActPlanner {
    max_iterations: u32,
    llm_client: Option<Arc<axiom_llm::LlmClient>>,
    tool_registry: Option<Arc<axiom_tool::ToolRegistry>>,
    memory: Option<Arc<axiom_memory::WorkingMemory>>,
}

impl ReActPlanner {
    pub fn new() -> Self {
        Self {
            max_iterations: 10,
            llm_client: None,
            tool_registry: None,
            memory: None,
        }
    }

    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_llm(mut self, client: Arc<axiom_llm::LlmClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    pub fn with_tools(mut self, registry: Arc<axiom_tool::ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    pub fn with_memory(mut self, memory: Arc<axiom_memory::WorkingMemory>) -> Self {
        self.memory = Some(memory);
        self
    }

    fn think(&self, step: &mut PlanStep, thought: impl Into<String>) {
        step.description = format!("Thought: {}", thought.into());
    }

    async fn act(
        &self,
        step: &mut PlanStep,
        action: &str,
        action_input: &serde_json::Value,
    ) -> Result<(), PlannerError> {
        step.tool_name = Some(action.to_string());

        if let Some(registry) = &self.tool_registry {
            let result = registry
                .execute(action, action_input)
                .await
                .map_err(|e| PlannerError::ToolError(e.to_string()))?;
            step.mark_completed(result.to_string());
        } else {
            step.mark_completed(format!("Mock result for action: {}", action));
        }

        Ok(())
    }

    fn is_final_answer(&self, text: &str) -> Option<String> {
        let lower = text.to_lowercase();
        if lower.contains("final answer:") || lower.contains("answer:") {
            let answer = text
                .split_once("Final Answer:")
                .or_else(|| text.split_once("final answer:"))
                .or_else(|| text.split_once("Answer:"))
                .or_else(|| text.split_once("answer:"))
                .map(|(_, rest)| rest.trim().to_string());
            answer
        } else {
            None
        }
    }
}

impl Default for ReActPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Planner for ReActPlanner {
    fn name(&self) -> &str {
        "react"
    }

    fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    fn plan_and_execute<'a>(&'a self, goal: &'a str, context: &'a str) -> BoxPlannerFuture<'a> {
        Box::pin(async move {
            let start = std::time::Instant::now();
            let mut steps: Vec<PlanStep> = Vec::new();
            let mut scratchpad = String::new();

            if let Some(mem) = &self.memory {
                mem.add(axiom_memory::MemoryItem::goal(goal));
                if !context.is_empty() {
                    mem.add(axiom_memory::MemoryItem::observation(context));
                }
            }

            for iteration in 0..self.max_iterations {
                let thought = format!("Iteration {} - thinking about: {}", iteration + 1, goal);
                let mut step = PlanStep::new(iteration as usize, &thought);
                step.mark_started();

                let prompt = if !scratchpad.is_empty() {
                    format!(
                        "Goal: {}\nContext: {}\n\nScratchpad:\n{}\n\nContinue with next Thought/Action or provide Final Answer.",
                        goal, context, scratchpad
                    )
                } else {
                    format!(
                        "Goal: {}\nContext: {}\n\nUse ReAct pattern: Thought -> Action -> Observation -> ... -> Final Answer",
                        goal, context
                    )
                };

                let response = if let Some(llm) = &self.llm_client {
                    llm.complete(&prompt)
                        .await
                        .map_err(|e| PlannerError::LlmError(e.to_string()))?
                        .text
                } else {
                    if iteration == 0 {
                        format!(
                            "Thought: I need to understand the goal first.\nAction: {}\nAction Input: {{\"query\": \"{}\"}}",
                            if self.tool_registry.is_some() { "search" } else { "mock_tool" },
                            goal
                        )
                    } else {
                        format!(
                            "Final Answer: Based on the analysis, the answer for '{}' is ready.",
                            goal
                        )
                    }
                };

                if let Some(answer) = self.is_final_answer(&response) {
                    step.mark_completed(&answer);
                    steps.push(step);

                    let duration = start.elapsed().as_millis() as u64;
                    return Ok(PlanningResult {
                        success: true,
                        steps,
                        final_output: Some(answer),
                        iterations: iteration + 1,
                        total_duration_ms: duration,
                    });
                }

                let action_name = extract_action(&response).unwrap_or("unknown");
                let action_input = extract_action_input(&response);

                self.think(&mut step, extract_thought(&response).unwrap_or(&thought));
                self.act(&mut step, action_name, &action_input).await?;

                scratchpad.push_str(&format!(
                    "\nThought: {}\nAction: {} with {}\nObservation: {}",
                    extract_thought(&response).unwrap_or("thinking"),
                    action_name,
                    action_input,
                    step.actual_output.as_deref().unwrap_or("")
                ));

                steps.push(step);
            }

            Ok(PlanningResult::failure(
                steps,
                "Max iterations reached without final answer",
            ))
        })
    }
}

fn extract_thought(text: &str) -> Option<&str> {
    let lower = text.to_lowercase();
    if let Some(pos) = lower.find("thought:") {
        let rest = &text[pos + "thought:".len()..];
        let end = rest.find("\n").unwrap_or(rest.len());
        Some(rest[..end].trim())
    } else {
        None
    }
}

fn extract_action(text: &str) -> Option<&str> {
    let lower = text.to_lowercase();
    if let Some(pos) = lower.find("action:") {
        let rest = &text[pos + "action:".len()..];
        let end = rest.find("\n").unwrap_or(rest.len());
        Some(rest[..end].trim())
    } else {
        None
    }
}

fn extract_action_input(text: &str) -> serde_json::Value {
    let lower = text.to_lowercase();
    if let Some(pos) = lower.find("action input:") {
        let rest = &text[pos + "action input:".len()..];
        let end = rest.find("\n").unwrap_or(rest.len());
        let input_str = rest[..end].trim();
        serde_json::from_str(input_str).unwrap_or(serde_json::json!({ "input": input_str }))
    } else {
        serde_json::Value::Null
    }
}
