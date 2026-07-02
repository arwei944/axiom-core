//! Type-safe tool invocation framework for axiom-core.
//!
//! Provides:
//! - Tool trait definition
//! - Tool registry with permission control
//! - Parameter validation
//! - Witness recording for tool invocations
//! - Tool composition

pub mod registry;
pub mod tool;
pub mod error;

pub use error::ToolError;
pub use registry::ToolRegistry;
pub use tool::{Tool, ToolInfo, ToolParameter};