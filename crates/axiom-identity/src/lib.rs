//! Identity and Skill system for axiom-core agents.
//!
//! Provides:
//! - Identity definition and mounting
//! - Skill system with activation conditions
//! - Progressive disclosure of capabilities
//! - Context-aware persona switching

pub mod identity;
pub mod skill;
pub mod agent;

pub use identity::{AgentIdentity, DisclosureLevel, IdentityError};
pub use skill::{Skill, SkillState, ActivationCondition};
pub use agent::AgentPersona;