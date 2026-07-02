use axiom_planner::*;
use std::sync::Arc;

#[tokio::test]
async fn test_react_planner_basic() {
    let planner = ReActPlanner::new().with_max_iterations(5);

    let result = planner
        .plan_and_execute("What is 2 + 2?", "Basic math question")
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.steps.is_empty());
    assert!(result.iterations > 0);
}

#[tokio::test]
async fn test_react_with_tools() {
    let registry = Arc::new(axiom_tool::ToolRegistry::new());

    struct SearchTool;
    #[async_trait::async_trait]
    impl axiom_tool::Tool for SearchTool {
        fn info(&self) -> axiom_tool::ToolInfo {
            axiom_tool::ToolInfo {
                name: "search".to_string(),
                description: "Search for information".to_string(),
                parameters: vec![],
                required_permission: None,
                version: "1.0.0".to_string(),
            }
        }

        async fn execute(
            &self,
            _params: &serde_json::Value,
        ) -> Result<serde_json::Value, axiom_tool::ToolError> {
            Ok(serde_json::json!({ "result": "found info" }))
        }
    }

    registry.register(SearchTool);

    let planner = ReActPlanner::new()
        .with_max_iterations(5)
        .with_tools(registry);

    let result = planner
        .plan_and_execute("Calculate something", "Need to use search tool")
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.steps.is_empty());
}

#[tokio::test]
async fn test_plan_execute_basic() {
    let planner = PlanAndExecutePlanner::new().with_max_iterations(5);

    let result = planner
        .plan_and_execute("Write a report", "Report about Rust")
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.steps.is_empty());
    assert_eq!(result.steps.len(), 4);
}

#[tokio::test]
async fn test_plan_step_status_transitions() {
    let mut step = PlanStep::new(0, "Test step");

    assert_eq!(step.status, step::StepStatus::Pending);
    assert!(!step.status.is_finished());

    step.mark_started();
    assert_eq!(step.status, step::StepStatus::InProgress);

    step.mark_completed("Done");
    assert_eq!(step.status, step::StepStatus::Completed);
    assert!(step.status.is_finished());
    assert_eq!(step.actual_output.as_deref(), Some("Done"));
}

#[tokio::test]
async fn test_plan_step_failed_and_retry() {
    let mut step = PlanStep::new(0, "Flaky step").with_max_retries(3);

    step.mark_failed("Error 1");
    assert_eq!(step.status, step::StepStatus::Failed);
    assert_eq!(step.retry_count, 1);
    assert!(step.can_retry());

    step.mark_failed("Error 2");
    assert_eq!(step.retry_count, 2);
    assert!(step.can_retry());

    step.mark_failed("Error 3");
    assert_eq!(step.retry_count, 3);
    assert!(!step.can_retry());
}

#[tokio::test]
async fn test_plan_step_skipped() {
    let mut step = PlanStep::new(0, "Skip me");

    step.mark_skipped("Not needed");
    assert_eq!(step.status, step::StepStatus::Skipped);
    assert!(step.status.is_finished());
}

#[tokio::test]
async fn test_plan_execute_with_memory() {
    use axiom_memory::WorkingMemory;

    let memory = Arc::new(WorkingMemory::new(4000));
    let planner = PlanAndExecutePlanner::new()
        .with_max_iterations(5)
        .with_memory(memory.clone());

    let _result = planner
        .plan_and_execute("Test goal", "Test context")
        .await
        .unwrap();

    assert!(memory.item_count() > 0);
    assert!(!memory
        .filter_by_type(axiom_memory::MemoryItemType::Goal)
        .is_empty());
}

#[tokio::test]
async fn test_planner_strategy_display() {
    assert_eq!(planner::PlannerStrategy::ReAct.to_string(), "react");
    assert_eq!(
        planner::PlannerStrategy::PlanAndExecute.to_string(),
        "plan_and_execute"
    );
}

#[tokio::test]
async fn test_planning_result_success() {
    let steps = vec![PlanStep::new(0, "Step 1")];
    let result = PlanningResult::success(steps, "All done");

    assert!(result.success);
    assert_eq!(result.final_output.as_deref(), Some("All done"));
}

#[tokio::test]
async fn test_planning_result_failure() {
    let steps = vec![PlanStep::new(0, "Step 1")];
    let result = PlanningResult::failure(steps, "Something went wrong");

    assert!(!result.success);
    assert_eq!(result.final_output.as_deref(), Some("Something went wrong"));
}

#[tokio::test]
async fn test_react_max_iterations() {
    let planner = ReActPlanner::new().with_max_iterations(1);

    let result = planner
        .plan_and_execute("Long running task", "Will take many steps")
        .await
        .unwrap();

    assert!(result.iterations <= 1);
}

#[tokio::test]
async fn test_step_dependencies() {
    let mut step = PlanStep::new(2, "Step 2");
    step = step.with_dependencies(vec![0, 1]);

    assert_eq!(step.dependencies, vec![0, 1]);
}

#[tokio::test]
async fn test_step_with_tool() {
    let step = PlanStep::new(0, "Use search tool").with_tool("search");

    assert_eq!(step.tool_name.as_deref(), Some("search"));
}

#[tokio::test]
async fn test_step_status_strings() {
    assert_eq!(step::StepStatus::Pending.as_str(), "pending");
    assert_eq!(step::StepStatus::InProgress.as_str(), "in_progress");
    assert_eq!(step::StepStatus::Completed.as_str(), "completed");
    assert_eq!(step::StepStatus::Failed.as_str(), "failed");
    assert_eq!(step::StepStatus::Skipped.as_str(), "skipped");
}