//! Type-safe tool invocation framework for axiom-kernel.
//!
//! Provides:
//! - Tool trait definition
//! - Tool registry with permission control
//! - Parameter validation
//! - Witness recording for tool invocations
//! - Tool composition

pub mod error;
pub mod kernel;
pub mod registry;
pub mod tool;

pub use error::ToolError;
pub use kernel::ToolKernelAdapter;
pub use registry::ToolRegistry;
pub use tool::{BoxToolFuture, Tool, ToolInfo, ToolParameter};
