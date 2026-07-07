//! Identity and Skill system for axiom-kernel agents.
//!
//! Provides:
//! - Identity definition and mounting
//! - Skill system with activation conditions
//! - Progressive disclosure of capabilities
//! - Context-aware persona switching

pub mod agent;
pub mod identity;
pub mod kernel;
pub mod skill;

pub use agent::AgentPersona;
pub use identity::{AgentIdentity, DisclosureLevel, IdentityError};
pub use kernel::IdentityKernelAdapter;
pub use skill::{ActivationCondition, Skill, SkillState};
