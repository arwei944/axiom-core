//! AgentCell - integrated agent runtime entity.
//!
//! Combines LLM, Tool, Memory, Planner, Prompt, and Identity into a single
//! agent that can process queries, execute plans, and manage context.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AgentError, AgentResult};

/// Agent runtime statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentStats {
    pub queries_processed: u64,
    pub tools_executed: u64,
    pub llm_calls: u64,
    pub plans_executed: u64,
    pub errors: u64,
    pub total_duration_ms: u64,
}

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: u32,
    pub memory_token_budget: usize,
    pub auto_summarize: bool,
    pub disclosure_level: axiom_identity::DisclosureLevel,
    pub planner_strategy: PlannerStrategy,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            memory_token_budget: 4000,
            auto_summarize: true,
            disclosure_level: axiom_identity::DisclosureLevel::Basic,
            planner_strategy: PlannerStrategy::ReAct,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerStrategy {
    ReAct,
    PlanAndExecute,
}

/// Integrated agent cell combining all toolchain components.
pub struct AgentCell {
    id: String,
    config: AgentConfig,
    llm: Option<Arc<axiom_llm::LlmClient>>,
    tools: Option<Arc<axiom_tool::ToolRegistry>>,
    memory: Arc<axiom_memory::WorkingMemory>,
    planner: Option<Arc<dyn axiom_planner::Planner>>,
    prompt_registry: Option<Arc<RwLock<axiom_prompt::registry::TemplateRegistry>>>,
    persona: Option<Arc<axiom_identity::AgentPersona>>,
    stats: Arc<RwLock<AgentStats>>,
    started: Arc<RwLock<bool>>,
}

impl AgentCell {
    /// Create a new AgentCell with the given ID and config.
    pub fn new(id: impl Into<String>, config: AgentConfig) -> Self {
        let memory = Arc::new(axiom_memory::WorkingMemory::new(config.memory_token_budget));
        Self {
            id: id.into(),
            config,
            llm: None,
            tools: None,
            memory,
            planner: None,
            prompt_registry: None,
            persona: None,
            stats: Arc::new(RwLock::new(AgentStats::default())),
            started: Arc::new(RwLock::new(false)),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    pub fn is_started(&self) -> bool {
        *self.started.read()
    }

    pub fn stats(&self) -> AgentStats {
        self.stats.read().clone()
    }

    /// Set the LLM client.
    pub fn with_llm(mut self, llm: Arc<axiom_llm::LlmClient>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Set the tool registry.
    pub fn with_tools(mut self, tools: Arc<axiom_tool::ToolRegistry>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the planner.
    pub fn with_planner(mut self, planner: Arc<dyn axiom_planner::Planner>) -> Self {
        self.planner = Some(planner);
        self
    }

    /// Set the prompt registry.
    pub fn with_prompt_registry(
        mut self,
        registry: axiom_prompt::registry::TemplateRegistry,
    ) -> Self {
        self.prompt_registry = Some(Arc::new(RwLock::new(registry)));
        self
    }

    /// Set the agent persona (identity + skills).
    pub fn with_persona(mut self, persona: axiom_identity::AgentPersona) -> Self {
        self.persona = Some(Arc::new(persona));
        self
    }

    /// Start the agent.
    pub fn start(&self) -> AgentResult<()> {
        if *self.started.read() {
            return Err(AgentError::AlreadyStarted);
        }

        if self.llm.is_none() {
            return Err(AgentError::NotConfigured("LLM client not set".into()));
        }

        *self.started.write() = true;

        if let Some(persona) = &self.persona {
            let identity = persona.identity();
            self.memory
                .add(axiom_memory::MemoryItem::new(
                    axiom_memory::MemoryItemType::System,
                    format!(
                        "Agent {} started. Role: {}",
                        identity.name, identity.description
                    ),
                ));
        }

        tracing::info!(agent_id = %self.id, "Agent started");
        Ok(())
    }

    /// Stop the agent gracefully.
    pub fn stop(&self) -> AgentResult<()> {
        if !*self.started.read() {
            return Err(AgentError::NotStarted);
        }

        *self.started.write() = false;

        let stats = self.stats.read();
        tracing::info!(
            agent_id = %self.id,
            queries_processed = stats.queries_processed,
            tools_executed = stats.tools_executed,
            llm_calls = stats.llm_calls,
            errors = stats.errors,
            "Agent stopped gracefully"
        );
        Ok(())
    }

    /// Process a user query and return a response.
    pub async fn query(&self, user_input: &str) -> AgentResult<String> {
        if !*self.started.read() {
            return Err(AgentError::NotStarted);
        }

        let start = std::time::Instant::now();
        self.stats.write().queries_processed += 1;

        tracing::debug!(
            agent_id = %self.id,
            input_len = user_input.len(),
            "Processing query"
        );

        // Record user input in memory
        self.memory
            .add(axiom_memory::MemoryItem::observation(user_input));

        // Update skills based on context
        if let Some(persona) = &self.persona {
            let newly_activated = persona.update_skills_for_context(user_input, true);
            if !newly_activated.is_empty() {
                tracing::info!(
                    agent_id = %self.id,
                    skills = ?newly_activated,
                    "Skills activated"
                );
            }
        }

        // Build the system prompt
        let system_prompt = self.build_system_prompt();

        // Build the full prompt
        let memory_context = self.memory.render_with_limit(self.config.memory_token_budget);
        let full_prompt = format!(
            "{}\n\nContext:\n{}\n\nUser: {}\n\nAssistant:",
            system_prompt, memory_context, user_input
        );

        // Use planner if available, otherwise direct LLM call
        let response = if let Some(planner) = &self.planner {
            self.stats.write().plans_executed += 1;
            tracing::debug!(agent_id = %self.id, "Using planner for query");

            match planner.plan_and_execute(user_input, &memory_context).await {
                Ok(result) => {
                    if result.success {
                        result
                            .final_output
                            .unwrap_or_else(|| "Task completed.".to_string())
                    } else {
                        self.stats.write().errors += 1;
                        tracing::warn!(
                            agent_id = %self.id,
                            iterations = result.iterations,
                            "Planning did not succeed"
                        );
                        result
                            .final_output
                            .unwrap_or_else(|| "Planning failed.".to_string())
                    }
                }
                Err(e) => {
                    self.stats.write().errors += 1;
                    tracing::error!(
                        agent_id = %self.id,
                        error = %e,
                        "Planner error, falling back to direct LLM"
                    );
                    // Fallback to direct LLM if planner fails
                    if let Some(llm) = &self.llm {
                        self.stats.write().llm_calls += 1;
                        let completion = llm.complete(&full_prompt).await.map_err(AgentError::from)?;
                        completion.text
                    } else {
                        return Err(AgentError::from(e));
                    }
                }
            }
        } else if let Some(llm) = &self.llm {
            self.stats.write().llm_calls += 1;
            let completion = llm.complete(&full_prompt).await.map_err(|e| {
                self.stats.write().errors += 1;
                tracing::error!(
                    agent_id = %self.id,
                    error = %e,
                    "LLM completion failed"
                );
                AgentError::from(e)
            })?;
            completion.text
        } else {
            return Err(AgentError::NotConfigured(
                "No LLM client or planner available".into(),
            ));
        };

        // Record response in memory
        self.memory
            .add(axiom_memory::MemoryItem::result(&response));

        // Update stats
        let duration_ms = start.elapsed().as_millis() as u64;
        {
            let mut stats = self.stats.write();
            stats.total_duration_ms += duration_ms;
        }

        tracing::debug!(
            agent_id = %self.id,
            duration_ms = duration_ms,
            response_len = response.len(),
            "Query completed"
        );

        Ok(response)
    }

    /// Execute a tool by name.
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &Value,
    ) -> AgentResult<Value> {
        let tools = self
            .tools
            .as_ref()
            .ok_or_else(|| AgentError::NotConfigured("Tool registry not set".into()))?;

        self.stats.write().tools_executed += 1;

        tracing::debug!(
            agent_id = %self.id,
            tool = tool_name,
            "Executing tool"
        );

        let result = tools
            .execute(tool_name, parameters)
            .await
            .map_err(|e| {
                self.stats.write().errors += 1;
                tracing::error!(
                    agent_id = %self.id,
                    tool = tool_name,
                    error = %e,
                    "Tool execution failed"
                );
                AgentError::from(e)
            })?;

        tracing::debug!(
            agent_id = %self.id,
            tool = tool_name,
            "Tool execution succeeded"
        );

        Ok(result)
    }

    /// Add a memory item to working memory.
    pub fn remember(&self, item: axiom_memory::MemoryItem) {
        self.memory.add(item);
    }

    /// Get current working memory items.
    pub fn memory_items(&self) -> Vec<axiom_memory::MemoryItem> {
        self.memory.all()
    }

    /// Render memory as a context prompt.
    pub fn memory_prompt(&self) -> String {
        self.memory.render_as_prompt()
    }

    /// Get available tools from the persona's active skills.
    pub fn available_tools(&self) -> Vec<String> {
        self.persona
            .as_ref()
            .map(|p| p.available_tools())
            .unwrap_or_default()
    }

    /// Register a prompt template.
    pub fn register_template(
        &self,
        template: axiom_prompt::PromptTemplate,
    ) -> AgentResult<()> {
        if let Some(registry) = &self.prompt_registry {
            registry
                .write()
                .register(template)
                .map_err(AgentError::from)?;
        }
        Ok(())
    }

    /// Render a prompt template by name.
    pub fn render_template(
        &self,
        name: &str,
        values: &HashMap<String, Value>,
    ) -> AgentResult<String> {
        let registry = self
            .prompt_registry
            .as_ref()
            .ok_or_else(|| AgentError::NotConfigured("Prompt registry not set".into()))?;

        registry
            .read()
            .render_latest(name, values)
            .map_err(AgentError::from)
    }

    fn build_system_prompt(&self) -> String {
        if let Some(persona) = &self.persona {
            persona.build_prompt(self.config.disclosure_level)
        } else {
            "You are a helpful AI assistant.".to_string()
        }
    }
}

impl std::fmt::Debug for AgentCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentCell")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("started", &*self.started.read())
            .field("stats", &*self.stats.read())
            .finish()
    }
}
