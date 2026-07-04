//! AgentBuilder - fluent builder for constructing AgentCell instances.

use std::sync::Arc;

use crate::agent::{AgentCell, AgentConfig, PlannerStrategy};
use crate::error::AgentResult;

/// Builder for creating configured AgentCell instances.
///
/// # Example
///
/// ```no_run
/// use axiom_agent::AgentBuilder;
/// use axiom_agent::PlannerStrategy;
/// use axiom_llm::LlmClient;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let agent = AgentBuilder::new("my-agent")
///     .with_llm(LlmClient::mock())
///     .with_memory_budget(8000)
///     .with_planner_strategy(PlannerStrategy::ReAct)
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct AgentBuilder {
    id: String,
    config: AgentConfig,
    llm: Option<Arc<axiom_llm::LlmClient>>,
    tools: Option<Arc<axiom_tool::ToolRegistry>>,
    planner: Option<Arc<dyn axiom_planner::Planner>>,
    prompt_registry: Option<axiom_prompt::registry::TemplateRegistry>,
    persona: Option<axiom_identity::AgentPersona>,
}

impl AgentBuilder {
    /// Create a new builder with the given agent ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            config: AgentConfig::default(),
            llm: None,
            tools: None,
            planner: None,
            prompt_registry: None,
            persona: None,
        }
    }

    /// Set the agent configuration.
    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the maximum planning iterations.
    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.config.max_iterations = max;
        self
    }

    /// Set the memory token budget.
    pub fn with_memory_budget(mut self, budget: usize) -> Self {
        self.config.memory_token_budget = budget;
        self
    }

    /// Enable or disable auto-summarization.
    pub fn with_auto_summarize(mut self, enabled: bool) -> Self {
        self.config.auto_summarize = enabled;
        self
    }

    /// Set the disclosure level.
    pub fn with_disclosure_level(mut self, level: axiom_identity::DisclosureLevel) -> Self {
        self.config.disclosure_level = level;
        self
    }

    /// Set the planner strategy.
    pub fn with_planner_strategy(mut self, strategy: PlannerStrategy) -> Self {
        self.config.planner_strategy = strategy;
        self
    }

    /// Set the LLM client.
    pub fn with_llm(mut self, llm: axiom_llm::LlmClient) -> Self {
        self.llm = Some(Arc::new(llm));
        self
    }

    /// Set the LLM client from Arc.
    pub fn with_llm_arc(mut self, llm: Arc<axiom_llm::LlmClient>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Set the tool registry.
    pub fn with_tools(mut self, tools: axiom_tool::ToolRegistry) -> Self {
        self.tools = Some(Arc::new(tools));
        self
    }

    /// Set the tool registry from Arc.
    pub fn with_tools_arc(mut self, tools: Arc<axiom_tool::ToolRegistry>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set a custom planner.
    pub fn with_planner(mut self, planner: Arc<dyn axiom_planner::Planner>) -> Self {
        self.planner = Some(planner);
        self
    }

    /// Set the prompt template registry.
    pub fn with_prompt_registry(
        mut self,
        registry: axiom_prompt::registry::TemplateRegistry,
    ) -> Self {
        self.prompt_registry = Some(registry);
        self
    }

    /// Set the agent persona (identity + skills).
    pub fn with_persona(mut self, persona: axiom_identity::AgentPersona) -> Self {
        self.persona = Some(persona);
        self
    }

    /// Set the agent identity (creates a persona if not already set).
    pub fn with_identity(mut self, identity: axiom_identity::AgentIdentity) -> Self {
        if self.persona.is_some() {
            let persona = self
                .persona
                .take()
                .expect("persona must be set when identity is provided"); // foxguard: ignore[rs/no-unwrap-in-lib]
            persona.set_identity(identity);
            self.persona = Some(persona);
        } else {
            self.persona = Some(axiom_identity::AgentPersona::new(identity));
        }
        self
    }

    /// Add a skill to the agent persona.
    pub fn with_skill(mut self, skill: axiom_identity::Skill) -> Self {
        if self.persona.is_none() {
            let identity = axiom_identity::AgentIdentity::new(&self.id, &self.id);
            self.persona = Some(axiom_identity::AgentPersona::new(identity));
        }
        if let Some(persona) = &self.persona {
            persona.add_skill(skill);
        }
        self
    }

    /// Build the AgentCell.
    pub fn build(self) -> AgentResult<AgentCell> {
        let mut agent = AgentCell::new(self.id, self.config);

        if let Some(llm) = self.llm {
            agent = agent.with_llm(llm);
        }

        if let Some(tools) = self.tools {
            agent = agent.with_tools(tools);
        }

        // Use custom planner if provided
        if let Some(planner) = self.planner {
            agent = agent.with_planner(planner);
        }

        if let Some(registry) = self.prompt_registry {
            agent = agent.with_prompt_registry(registry);
        }

        if let Some(persona) = self.persona {
            agent = agent.with_persona(persona);
        }

        Ok(agent)
    }

    /// Build and start the AgentCell.
    pub fn build_and_start(self) -> AgentResult<AgentCell> {
        let agent = self.build()?;
        agent.start()?;
        Ok(agent)
    }
}

impl std::fmt::Debug for AgentBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentBuilder")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("has_llm", &self.llm.is_some())
            .field("has_tools", &self.tools.is_some())
            .field("has_planner", &self.planner.is_some())
            .field("has_prompt_registry", &self.prompt_registry.is_some())
            .field("has_persona", &self.persona.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let agent = AgentBuilder::new("test")
            .with_llm(axiom_llm::LlmClient::mock())
            .with_memory_budget(2000)
            .build()
            .unwrap();

        assert_eq!(agent.id(), "test");
        assert!(!agent.is_started());
    }

    #[test]
    fn test_builder_missing_llm() {
        let result = AgentBuilder::new("test").build();
        assert!(result.is_ok()); // Build succeeds, start fails

        let agent = result.unwrap();
        let start_result = agent.start();
        assert!(start_result.is_err());
    }

    #[test]
    fn test_builder_with_identity() {
        let identity =
            axiom_identity::AgentIdentity::new("id-1", "TestBot").with_description("A test bot");

        let agent = AgentBuilder::new("test")
            .with_llm(axiom_llm::LlmClient::mock())
            .with_identity(identity)
            .build()
            .unwrap();

        assert_eq!(agent.id(), "test");
    }

    #[test]
    fn test_builder_with_skill() {
        let skill = axiom_identity::Skill::new("s1", "Coding")
            .with_activation(axiom_identity::ActivationCondition::Always);

        let agent = AgentBuilder::new("test")
            .with_llm(axiom_llm::LlmClient::mock())
            .with_skill(skill)
            .build()
            .unwrap();

        assert_eq!(agent.available_tools().len(), 0);
    }

    #[test]
    fn test_builder_planner_strategy() {
        let agent = AgentBuilder::new("test")
            .with_llm(axiom_llm::LlmClient::mock())
            .with_planner_strategy(PlannerStrategy::ReAct)
            .build()
            .unwrap();

        assert!(agent.config().planner_strategy == PlannerStrategy::ReAct);
    }
}
