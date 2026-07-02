//! Prelude module for convenient imports.

pub use crate::agent::{AgentCell, AgentConfig, AgentStats};
pub use crate::builder::AgentBuilder;
pub use crate::error::{AgentError, AgentResult};

// Core types
pub use axiom_core::{CellId, Layer, Signal, SignalKind, Witness};

// LLM types
pub use axiom_llm::{LlmClient, CompletionResponse, ChatMessage, MessageRole};

// Tool types
pub use axiom_tool::{Tool, ToolRegistry, ToolInfo, ToolError};

// Memory types
pub use axiom_memory::{WorkingMemory, MemoryItem, MemoryItemType};

// Planner types
pub use axiom_planner::{Planner, ReActPlanner, PlanAndExecutePlanner, PlanningResult};

// Prompt types
pub use axiom_prompt::{PromptTemplate, TemplateVariable, VariableType};

// Identity types
pub use axiom_identity::{AgentIdentity, AgentPersona, Skill, DisclosureLevel};
