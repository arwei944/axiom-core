//! Plan-and-Execute planner implementation.

use std::sync::Arc;

use crate::planner::{BoxPlannerFuture, Planner, PlannerError, PlanningResult};
use crate::step::{PlanStep, StepStatus};

pub struct PlanAndExecutePlanner {
    max_iterations: u32,
    max_plan_steps: usize,
    llm_client: Option<Arc<axiom_llm::LlmClient>>,
    tool_registry: Option<Arc<axiom_tool::ToolRegistry>>,
    memory: Option<Arc<axiom_memory::WorkingMemory>>,
    replan_on_failure: bool,
}

impl PlanAndExecutePlanner {
    pub fn new() -> Self {
        Self {
            max_iterations: 5,
            max_plan_steps: 10,
            llm_client: None,
            tool_registry: None,
            memory: None,
            replan_on_failure: true,
        }
    }

    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_max_plan_steps(mut self, max: usize) -> Self {
        self.max_plan_steps = max;
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

    pub fn with_replan(mut self, enabled: bool) -> Self {
        self.replan_on_failure = enabled;
        self
    }

    async fn create_plan(&self, goal: &str, context: &str) -> Result<Vec<PlanStep>, PlannerError> {
        if let Some(llm) = &self.llm_client {
            let prompt = format!(
                "Create a plan to accomplish the following goal. Output as a numbered list of steps.\n\n\
                 Goal: {}\nContext: {}\n\n\
                 Format each step as: <number>. <description>",
                goal, context
            );

            let response = llm
                .complete(&prompt)
                .await
                .map_err(|e| PlannerError::LlmError(e.to_string()))?;

            Ok(parse_steps_from_text(&response.text, self.max_plan_steps))
        } else {
            Ok(self.mock_plan(goal))
        }
    }

    fn mock_plan(&self, goal: &str) -> Vec<PlanStep> {
        vec![
            PlanStep::new(0, format!("Understand the goal: {}", goal)),
            PlanStep::new(1, "Gather relevant information").with_dependencies(vec![0]),
            PlanStep::new(2, "Analyze the information").with_dependencies(vec![1]),
            PlanStep::new(3, "Formulate final answer").with_dependencies(vec![2]),
        ]
    }

    async fn execute_step(&self, step: &mut PlanStep) -> Result<(), PlannerError> {
        step.mark_started();

        if let Some(tool_name) = &step.tool_name {
            if let Some(registry) = &self.tool_registry {
                let params = serde_json::json!({ "step": step.description });
                let result = registry
                    .execute(tool_name, &params)
                    .await
                    .map_err(|e| PlannerError::ToolError(e.to_string()))?;
                step.mark_completed(result.to_string());
            } else {
                step.mark_completed(format!("Completed: {}", step.description));
            }
        } else {
            step.mark_completed(format!("Completed: {}", step.description));
        }

        if let Some(mem) = &self.memory {
            mem.add(axiom_memory::MemoryItem::action(&step.description).with_importance(0.6));
            if let Some(output) = &step.actual_output {
                mem.add(axiom_memory::MemoryItem::result(output));
            }
        }

        Ok(())
    }

    fn dependencies_satisfied(&self, step: &PlanStep, steps: &[PlanStep]) -> bool {
        step.dependencies.iter().all(|&dep_idx| {
            steps
                .get(dep_idx)
                .map(|s| s.status == StepStatus::Completed)
                .unwrap_or(false)
        })
    }

    async fn replan(
        &self,
        steps: &mut Vec<PlanStep>,
        failed_idx: usize,
        goal: &str,
    ) -> Result<(), PlannerError> {
        if !self.replan_on_failure {
            return Ok(());
        }

        let completed: Vec<String> = steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .map(|s| s.description.clone())
            .collect();

        let failed = &steps[failed_idx];
        let error = failed.actual_output.as_deref().unwrap_or("unknown error");

        if let Some(llm) = &self.llm_client {
            let prompt = format!(
                "The plan failed at step {}: {}\nError: {}\n\nCompleted steps:\n{}\n\n\
                 Original goal: {}\n\nProvide revised steps to continue from here (numbered list).",
                failed_idx + 1,
                failed.description,
                error,
                completed
                    .iter()
                    .enumerate()
                    .map(|(i, s)| format!("{}. {}", i + 1, s))
                    .collect::<Vec<_>>()
                    .join("\n"),
                goal
            );

            let response = llm
                .complete(&prompt)
                .await
                .map_err(|e| PlannerError::LlmError(e.to_string()))?;

            let new_steps = parse_steps_from_text(&response.text, self.max_plan_steps);
            let base_idx = steps.len();

            for (i, mut new_step) in new_steps.into_iter().enumerate() {
                new_step.index = base_idx + i;
                if i == 0 {
                    new_step.dependencies = vec![failed_idx];
                } else {
                    new_step.dependencies = vec![base_idx + i - 1];
                }
                steps.push(new_step);
            }
        } else {
            let retry_idx = steps.len();
            let mut retry_step = PlanStep::new(retry_idx, format!("Retry: {}", failed.description));
            retry_step.tool_name = failed.tool_name.clone();
            retry_step.dependencies = vec![failed_idx];
            steps.push(retry_step);
        }

        Ok(())
    }
}

impl Default for PlanAndExecutePlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Planner for PlanAndExecutePlanner {
    fn name(&self) -> &str {
        "plan_and_execute"
    }

    fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    fn plan_and_execute<'a>(&'a self, goal: &'a str, context: &'a str) -> BoxPlannerFuture<'a> {
        Box::pin(async move {
            let start = std::time::Instant::now();

            if let Some(mem) = &self.memory {
                mem.add(axiom_memory::MemoryItem::goal(goal));
                if !context.is_empty() {
                    mem.add(axiom_memory::MemoryItem::observation(context));
                }
            }

            let mut steps = self.create_plan(goal, context).await?;
            let mut iteration = 0;

            if let Some(mem) = &self.memory {
                mem.add(
                    axiom_memory::MemoryItem::plan(format!(
                        "Plan created with {} steps",
                        steps.len()
                    ))
                    .with_importance(0.7),
                );
            }

            loop {
                iteration += 1;
                if iteration > self.max_iterations {
                    return Ok(PlanningResult::failure(steps, "Max iterations reached"));
                }

                let mut all_done = true;
                let mut progress_made = false;

                for i in 0..steps.len() {
                    if steps[i].status.is_finished() {
                        continue;
                    }

                    all_done = false;

                    if !self.dependencies_satisfied(&steps[i], &steps) {
                        continue;
                    }

                    match self.execute_step(&mut steps[i]).await {
                        Ok(_) => {
                            progress_made = true;
                        }
                        Err(e) => {
                            steps[i].mark_failed(e.to_string());
                            self.replan(&mut steps, i, goal).await?;
                            progress_made = true;
                            break;
                        }
                    }
                }

                if all_done {
                    let all_completed = steps.iter().all(|s| s.status == StepStatus::Completed);
                    let final_output = if all_completed {
                        steps
                            .last()
                            .and_then(|s| s.actual_output.clone())
                            .unwrap_or_else(|| "Task completed successfully".to_string())
                    } else {
                        "Plan did not complete successfully".to_string()
                    };

                    let duration = start.elapsed().as_millis() as u64;
                    return Ok(PlanningResult {
                        success: all_completed,
                        steps,
                        final_output: Some(final_output),
                        iterations: iteration,
                        total_duration_ms: duration,
                    });
                }

                if !progress_made {
                    return Ok(PlanningResult::failure(
                        steps,
                        "No progress made - possible deadlock in dependencies",
                    ));
                }
            }
        })
    }
}

fn parse_steps_from_text(text: &str, max_steps: usize) -> Vec<PlanStep> {
    let mut steps = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some((num_str, rest)) = trimmed.split_once('.') {
            if let Ok(_num) = num_str.trim().parse::<usize>() {
                let description = rest.trim().to_string();
                if !description.is_empty() {
                    let mut step = PlanStep::new(steps.len(), description);
                    if !steps.is_empty() {
                        step.dependencies = vec![steps.len() - 1];
                    }
                    steps.push(step);
                    if steps.len() >= max_steps {
                        break;
                    }
                }
            }
        }
    }

    if steps.is_empty() {
        steps.push(PlanStep::new(0, text.to_string()));
    }

    steps
}
