//! Prelude module for convenient imports.

pub use crate::agent::{AgentCell, AgentConfig, AgentStats};
pub use crate::builder::AgentBuilder;
pub use crate::error::{AgentError, AgentResult};

// Core types
pub use axiom_kernel::{CellId, RuntimeTier, Signal, SignalKind, Witness};

// LLM types
pub use axiom_llm::{ChatMessage, CompletionResponse, LlmClient, MessageRole};

// Tool types
pub use axiom_tool::{Tool, ToolError, ToolInfo, ToolRegistry};

// Memory types
pub use axiom_memory::{MemoryItem, MemoryItemType, WorkingMemory};

// Planner types
pub use axiom_planner::{PlanAndExecutePlanner, Planner, PlanningResult, ReActPlanner};

// Prompt types
pub use axiom_prompt::{PromptTemplate, TemplateVariable, VariableType};

// Identity types
pub use axiom_identity::{AgentIdentity, AgentPersona, DisclosureLevel, Skill};
