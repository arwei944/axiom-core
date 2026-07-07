//! Type-safe prompt template system for axiom-kernel.
//!
//! Provides:
//! - Type-safe template variables
//! - Template composition
//! - Version management
//! - Template registry
//! - Variable validation

pub mod error;
pub mod kernel;
pub mod registry;
pub mod template;

pub use error::PromptError;
pub use kernel::PromptKernelAdapter;
pub use template::{PromptTemplate, TemplateVariable, VariableType};
