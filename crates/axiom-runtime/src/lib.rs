//! Axiom Runtime - Tokio-based runtime with supervision tree and L2 gates.

pub mod bus;
pub mod entropy_gov;
pub mod guardian;
pub mod mailbox;
pub mod runtime;
pub mod supervisor;

pub use bus::MessageBus;
pub use entropy_gov::EntropyGovernor;
pub use guardian::ArchitectureGuardian;
pub use mailbox::Mailbox;
pub use runtime::{AxiomRuntime, RuntimeConfig, RuntimeHealth};
pub use supervisor::Supervisor;
