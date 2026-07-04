//! End-to-end integration tests for the axiom-agent crate.
//!
//! These tests exercise the full agent pipeline: LLM + Tool + Memory +
//! Planner + Prompt + Identity working together.

use axiom_agent::*;
use axiom_identity::*;
use axiom_llm::LlmClient;
use axiom_memory::{MemoryItem, MemoryItemType};
use axiom_prompt::*;
use axiom_tool::tool::SimpleTool;
use axiom_tool::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

// --- Helper functions to create tools without async_trait ---

fn make_echo_tool() -> SimpleTool<impl Fn(&Value) -> Result<Value, ToolError>> {
    let info = ToolInfo {
        name: "echo".to_string(),
        description: "Echo back the input".to_string(),
        parameters: vec![ToolParameter {
            name: "message".to_string(),
            description: "Message to echo".to_string(),
            required: true,
            schema: json!({ "type": "string" }),
        }],
        required_permission: None,
        version: "1.0.0".to_string(),
    };
    SimpleTool::new(info, |params| {
        let msg = params["message"].as_str().unwrap_or("");
        Ok(json!({ "echo": msg }))
    })
}

fn make_calculator_tool() -> SimpleTool<impl Fn(&Value) -> Result<Value, ToolError>> {
    let info = ToolInfo {
        name: "calculate".to_string(),
        description: "Calculate a math expression".to_string(),
        parameters: vec![ToolParameter {
            name: "expression".to_string(),
            description: "Math expression".to_string(),
            required: true,
            schema: json!({ "type": "string" }),
        }],
        required_permission: None,
        version: "1.0.0".to_string(),
    };
    SimpleTool::new(info, |params| {
        let expr = params["expression"].as_str().unwrap_or("0");
        let result = if expr.contains("+") {
            let parts: Vec<&str> = expr.split('+').collect();
            let a: f64 = parts[0].trim().parse().unwrap_or(0.0);
            let b: f64 = parts[1].trim().parse().unwrap_or(0.0);
            a + b
        } else {
            0.0
        };
        Ok(json!({ "result": result }))
    })
}

// --- Tests ---

#[tokio::test]
async fn test_agent_basic_query() {
    let agent = AgentBuilder::new("test-agent")
        .with_llm(LlmClient::mock())
        .with_memory_budget(2000)
        .build_and_start()
        .unwrap();

    let response = agent.query("Hello, who are you?").await;
    assert!(response.is_ok());
    let resp = response.unwrap();
    assert!(!resp.is_empty());

    let stats = agent.stats();
    assert_eq!(stats.queries_processed, 1);
    assert!(stats.llm_calls > 0);
}

#[tokio::test]
async fn test_agent_not_started_error() {
    let agent = AgentBuilder::new("test-agent")
        .with_llm(LlmClient::mock())
        .build()
        .unwrap();

    let result = agent.query("Hello").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AgentError::NotStarted));
}

#[tokio::test]
async fn test_agent_with_tools() {
    let registry = ToolRegistry::new();
    registry.register(make_echo_tool());
    registry.register(make_calculator_tool());

    let agent = AgentBuilder::new("tool-agent")
        .with_llm(LlmClient::mock())
        .with_tools(registry)
        .build_and_start()
        .unwrap();

    let result = agent
        .execute_tool("echo", &json!({ "message": "test" }))
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["echo"], "test");

    let calc_result = agent
        .execute_tool("calculate", &json!({ "expression": "2 + 3" }))
        .await;
    assert!(calc_result.is_ok());
    assert_eq!(calc_result.unwrap()["result"], 5.0);

    let stats = agent.stats();
    assert_eq!(stats.tools_executed, 2);
}

#[tokio::test]
async fn test_agent_memory_integration() {
    let agent = AgentBuilder::new("memory-agent")
        .with_llm(LlmClient::mock())
        .with_memory_budget(2000)
        .build_and_start()
        .unwrap();

    agent.remember(MemoryItem::new(
        MemoryItemType::Goal,
        "Answer user questions accurately",
    ));

    let _ = agent.query("What is your goal?").await.unwrap();

    let items = agent.memory_items();
    assert!(items.len() >= 2);

    let prompt = agent.memory_prompt();
    assert!(!prompt.is_empty());
}

#[tokio::test]
async fn test_agent_with_identity() {
    let identity = AgentIdentity::new("id-1", "TestBot")
        .with_description("A helpful test assistant")
        .with_tone("friendly")
        .with_traits(vec!["accurate".to_string(), "concise".to_string()]);

    let agent = AgentBuilder::new("identity-agent")
        .with_llm(LlmClient::mock())
        .with_identity(identity)
        .build_and_start()
        .unwrap();

    let response = agent.query("Who are you?").await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_agent_with_skill() {
    let skill = Skill::new("coding", "Code Assistant")
        .with_description("Help with coding tasks")
        .with_activation(ActivationCondition::KeywordTrigger(
            vec!["code".to_string()],
        ))
        .with_tools(vec!["echo".to_string()])
        .with_prompt_fragments(vec!["You can write code.".to_string()]);

    let agent = AgentBuilder::new("skill-agent")
        .with_llm(LlmClient::mock())
        .with_skill(skill)
        .build_and_start()
        .unwrap();

    let _ = agent.query("I need help with code").await.unwrap();

    let tools = agent.available_tools();
    assert!(tools.contains(&"echo".to_string()));
}

#[tokio::test]
async fn test_agent_with_planner() {
    let planner = Arc::new(axiom_planner::ReActPlanner::new().with_max_iterations(5));

    let agent = AgentBuilder::new("planner-agent")
        .with_llm(LlmClient::mock())
        .with_planner(planner)
        .build_and_start()
        .unwrap();

    let response = agent.query("Solve a problem").await;
    assert!(response.is_ok());

    let stats = agent.stats();
    assert!(stats.plans_executed > 0);
}

#[tokio::test]
async fn test_agent_with_prompt_templates() {
    let mut registry = axiom_prompt::registry::TemplateRegistry::new();

    let template = PromptTemplate::new("greeting", "Hello, {{name}}! I am {{agent_name}}.")
        .with_variable(TemplateVariable::new("name", VariableType::String))
        .with_variable(TemplateVariable::new("agent_name", VariableType::String));

    registry.register(template).unwrap();

    let agent = AgentBuilder::new("prompt-agent")
        .with_llm(LlmClient::mock())
        .with_prompt_registry(registry)
        .build_and_start()
        .unwrap();

    let mut values = HashMap::new();
    values.insert("name".to_string(), json!("Alice"));
    values.insert("agent_name".to_string(), json!("TestBot"));

    let rendered = agent.render_template("greeting", &values).unwrap();
    assert_eq!(rendered, "Hello, Alice! I am TestBot.");
}

#[tokio::test]
async fn test_agent_full_integration() {
    let tool_registry = ToolRegistry::new();
    tool_registry.register(make_echo_tool());

    let identity = AgentIdentity::new("full-agent", "FullAgent")
        .with_description("A fully configured agent")
        .with_tone("professional")
        .with_capabilities(vec!["echo".to_string()]);

    let agent = AgentBuilder::new("full-agent")
        .with_llm(LlmClient::mock())
        .with_tools(tool_registry)
        .with_identity(identity)
        .with_memory_budget(4000)
        .with_max_iterations(5)
        .with_planner_strategy(PlannerStrategy::ReAct)
        .build_and_start()
        .unwrap();

    let tool_result = agent
        .execute_tool("echo", &json!({ "message": "integration" }))
        .await
        .unwrap();
    assert_eq!(tool_result["echo"], "integration");

    let query_response = agent.query("Hello").await.unwrap();
    assert!(!query_response.is_empty());

    assert!(!agent.memory_items().is_empty());

    let stats = agent.stats();
    assert!(stats.queries_processed > 0);
    assert!(stats.tools_executed > 0);
    assert!(stats.llm_calls > 0);
}

#[tokio::test]
async fn test_agent_start_stop_lifecycle() {
    let agent = AgentBuilder::new("lifecycle-agent")
        .with_llm(LlmClient::mock())
        .build()
        .unwrap();

    assert!(!agent.is_started());

    agent.start().unwrap();
    assert!(agent.is_started());

    assert!(agent.start().is_err());

    agent.stop().unwrap();
    assert!(!agent.is_started());

    assert!(agent.stop().is_err());
}

#[tokio::test]
async fn test_agent_query_without_llm() {
    let agent = AgentCell::new("no-llm", AgentConfig::default());

    let result = agent.start();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AgentError::NotConfigured(_)));
}

#[tokio::test]
async fn test_agent_stats_tracking() {
    let agent = AgentBuilder::new("stats-agent")
        .with_llm(LlmClient::mock())
        .build_and_start()
        .unwrap();

    let initial = agent.stats();
    assert_eq!(initial.queries_processed, 0);

    agent.query("Question 1").await.unwrap();
    agent.query("Question 2").await.unwrap();
    agent.query("Question 3").await.unwrap();

    let stats = agent.stats();
    assert_eq!(stats.queries_processed, 3);
    assert!(stats.llm_calls >= 3);
}

#[tokio::test]
async fn test_agent_multiple_queries_memory_growth() {
    let agent = AgentBuilder::new("growth-agent")
        .with_llm(LlmClient::mock())
        .with_memory_budget(10000)
        .build_and_start()
        .unwrap();

    let initial_count = agent.memory_items().len();

    agent.query("First question").await.unwrap();
    assert!(agent.memory_items().len() > initial_count);

    let after_first = agent.memory_items().len();

    agent.query("Second question").await.unwrap();
    assert!(agent.memory_items().len() > after_first);
}

#[tokio::test]
async fn test_agent_config_defaults() {
    let config = AgentConfig::default();
    assert_eq!(config.max_iterations, 10);
    assert_eq!(config.memory_token_budget, 4000);
    assert!(config.auto_summarize);
    assert_eq!(config.disclosure_level, DisclosureLevel::Basic);
    assert_eq!(config.planner_strategy, PlannerStrategy::ReAct);
}

#[tokio::test]
async fn test_agent_debug_format() {
    let agent = AgentBuilder::new("debug-agent")
        .with_llm(LlmClient::mock())
        .build()
        .unwrap();

    let debug_str = format!("{:?}", agent);
    assert!(debug_str.contains("debug-agent"));
    assert!(debug_str.contains("AgentCell"));
}

#[tokio::test]
async fn test_agent_disclosure_levels() {
    let identity = AgentIdentity::new("id", "Bot")
        .with_description("Test")
        .with_traits(vec!["smart".to_string()]);

    let agent = AgentBuilder::new("disclosure-agent")
        .with_llm(LlmClient::mock())
        .with_identity(identity)
        .with_disclosure_level(DisclosureLevel::Full)
        .build_and_start()
        .unwrap();

    let response = agent.query("Tell me about yourself").await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_agent_tool_not_found() {
    let registry = ToolRegistry::new();

    let agent = AgentBuilder::new("error-agent")
        .with_llm(LlmClient::mock())
        .with_tools(registry)
        .build_and_start()
        .unwrap();

    let result = agent.execute_tool("nonexistent", &json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_agent_re_export_crates() {
    let _ = axiom_agent::axiom_core::Layer::Agent;
    let _ = axiom_agent::axiom_llm::LlmClient::mock();
    let _ = axiom_agent::axiom_tool::ToolRegistry::new();
    let _ = axiom_agent::axiom_memory::WorkingMemory::new(1000);
    let _ = axiom_agent::axiom_identity::AgentIdentity::new("id", "name");
    let _ = axiom_agent::axiom_prompt::PromptTemplate::new("t", "template");
}
