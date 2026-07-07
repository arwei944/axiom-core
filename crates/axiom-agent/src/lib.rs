//! Axiom Agent - complete toolkit for agent development.
//!
//! This is a fascade crate that re-exports the agent toolchain and provides
//! an integrated `AgentCell` that combines all toolchain components into a
//! single runtime entity.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                    AgentCell                         │
//! │  ┌───────────┐  ┌──────────┐  ┌──────────────────┐  │
//! │  │ Identity  │→ │ Prompt   │→ │     Planner      │  │
//! │  │ /Skill    │  │ Template │  │ (ReAct/PlanExec) │  │
//! │  └───────────┘  └──────────┘  └────────┬─────────┘  │
//! │                                         │            │
//! │  ┌───────────┐  ┌──────────┐  ┌────────▼─────────┐  │
//! │  │  Memory   │← │   LLM    │← │   Tool Registry  │  │
//! │  │ (Working) │  │  Client  │  │   (Execute)      │  │
//! │  └───────────┘  └──────────┘  └──────────────────┘  │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```no_run
//! use axiom_agent::AgentBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let agent = AgentBuilder::new("my-agent")
//!     .with_llm(axiom_llm::LlmClient::mock())
//!     .with_memory_budget(4000)
//!     .build()?;
//! # Ok(())
//! # }
//! ```

pub mod agent;
pub mod builder;
pub mod error;
pub mod prelude;

// Re-export all toolchain crates
pub use axiom_identity;
pub use axiom_kernel;
pub use axiom_llm;
pub use axiom_memory;
pub use axiom_planner;
pub use axiom_prompt;
pub use axiom_runtime;
pub use axiom_tool;

// Re-export key types from this crate
pub use agent::{AgentCell, AgentConfig, AgentStats, PlannerStrategy};
pub use builder::AgentBuilder;
pub use error::{AgentError, AgentResult};
