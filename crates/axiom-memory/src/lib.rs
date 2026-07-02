//! Working memory system for axiom-core agents.
//!
//! Provides:
//! - Working Memory with item-based storage
//! - Token budget awareness
//! - Auto-summarization when exceeding budget
//! - Memory retrieval with relevance scoring
//! - Memory item types (thought, observation, action, result)

pub mod memory;
pub mod item;

pub use item::{MemoryItem, MemoryItemType};
pub use memory::{MemoryError, WorkingMemory};