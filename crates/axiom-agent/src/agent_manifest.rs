use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub role: String,
    pub instructions: String,
    pub capabilities: Vec<Capability>,
    pub dependencies: Vec<Dependency>,
    pub memory_config: MemoryConfig,
    pub planner_config: PlannerConfig,
    pub constraints: Constraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub activation: ActivationCondition,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivationCondition {
    Always,
    Keyword(String),
    Intent(String),
    Context(String),
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub description: String,
    pub r#type: String,
    pub required: bool,
    pub default: Option<String>,
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub source: DependencySource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencySource {
    Builtin,
    Plugin(String),
    Remote(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_tokens: usize,
    pub auto_summarize: bool,
    pub retention_policy: RetentionPolicy,
    pub recall_strategy: RecallStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionPolicy {
    Recent,
    Importance,
    Semantic,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecallStrategy {
    Exact,
    Semantic,
    Fuzzy,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfig {
    pub strategy: PlannerStrategy,
    pub max_iterations: u32,
    pub timeout_seconds: u64,
    pub replan_on_failure: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerStrategy {
    ReAct,
    PlanAndExecute,
    ChainOfThought,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    pub forbidden_actions: HashSet<String>,
    pub required_actions: HashSet<String>,
    pub max_tool_calls: Option<u32>,
    pub min_confidence: f64,
    pub disclosure_level: DisclosureLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisclosureLevel {
    None,
    Basic,
    Detailed,
    Full,
}

impl Default for AgentManifest {
    fn default() -> Self {
        Self {
            id: "agent".to_string(),
            name: "Agent".to_string(),
            version: "1.0.0".to_string(),
            description: "Axiom Agent".to_string(),
            role: "AI Assistant".to_string(),
            instructions: "You are a helpful AI assistant.".to_string(),
            capabilities: Vec::new(),
            dependencies: Vec::new(),
            memory_config: MemoryConfig {
                max_tokens: 4000,
                auto_summarize: true,
                retention_policy: RetentionPolicy::Hybrid,
                recall_strategy: RecallStrategy::Hybrid,
            },
            planner_config: PlannerConfig {
                strategy: PlannerStrategy::Auto,
                max_iterations: 10,
                timeout_seconds: 60,
                replan_on_failure: true,
            },
            constraints: Constraints {
                forbidden_actions: HashSet::new(),
                required_actions: HashSet::new(),
                max_tool_calls: None,
                min_confidence: 0.5,
                disclosure_level: DisclosureLevel::Basic,
            },
        }
    }
}

impl AgentManifest {
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}