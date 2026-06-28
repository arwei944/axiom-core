//! Axiom Agent - complete toolkit for agent development.
//!
//! This is a fascade crate that re-exports the agent toolchain.
//! Individual tool crates (axiom-llm, axiom-tool, axiom-memory, etc.)
//! will be added as they are implemented.
//!
//! # Included tools (planned)
//! - LLM client abstraction (multi-provider, retry, cache, structured output)
//! - Tool registry & calling framework
//! - Memory system (working/episodic/semantic/procedural)
//! - Planner abstraction (ReAct, Plan-and-Execute, ToT)
//! - Type-safe prompt templates
//! - RAG components (ingest, chunk, retrieve, rerank)
//! - Evaluation framework (Golden Set, LLM-as-Judge)
//! - Testing utilities (Mock LLM, deterministic replay, chaos injection)

pub use axiom_core;
pub use axiom_runtime;
