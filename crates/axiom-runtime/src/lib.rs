//! Axiom Runtime - Tokio-based runtime with supervision tree and L2 gates.

pub mod bus;
pub mod dlq;
pub mod entropy_gov;
pub mod guardian;
pub mod interceptors;
pub mod loop_detector;
pub mod mailbox;
pub mod runtime;
pub mod supervisor;

pub use bus::{BusInterceptor, InterceptDecision, MessageBus};
pub use dlq::{DeadLetter, DeadLetterQueue};
pub use entropy_gov::EntropyGovernor;
pub use guardian::ArchitectureGuardian;
pub use interceptors::{
    ComplianceInterceptor, HopLimitInterceptor, IdempotencyInterceptor, LoopDetectInterceptor,
    OversightReportInterceptor, ResourceInterceptor, SchemaVersionInterceptor,
};
pub use loop_detector::LoopDetector;
pub use mailbox::Mailbox;
pub use runtime::{AxiomRuntime, CellRegistration, RuntimeBuilder, RuntimeConfig, RuntimeHealth};
pub use supervisor::Supervisor;
