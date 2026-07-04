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

        fn execute<'a>(&'a self, _params: &'a serde_json::Value) -> axiom_tool::BoxToolFuture<'a> {
            Box::pin(async move { Ok(serde_json::json!({ "result": "found info" })) })
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

    let result = planner.plan_and_execute("Test goal", "Test context").await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.steps.is_empty());
}
