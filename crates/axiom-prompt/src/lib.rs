//! Type-safe prompt template system for axiom-core.
//!
//! Provides:
//! - Type-safe template variables
//! - Template composition
//! - Version management
//! - Template registry
//! - Variable validation

pub mod template;
pub mod registry;
pub mod error;

pub use error::PromptError;
pub use template::{PromptTemplate, TemplateVariable, VariableType};