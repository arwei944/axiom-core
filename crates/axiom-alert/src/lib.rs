//! Axiom Alert - alerting system for Axiom.
//!
//! Provides:
//! - Alert rules and evaluation
//! - Threshold and window conditions
//! - Routing, escalation, silence
//! - Governance integration

pub mod alert;
pub mod governance;
pub mod router;
pub mod silence;
pub mod store;
pub mod threshold;

pub use alert::{Alert, AlertRule, AlertSink, AlertStatus, Severity};
pub use threshold::{Threshold, Window, WindowKind};
pub use router::AlertRouter;
pub use silence::Silence;
pub use store::MemoryAlertStore;
pub use governance::GovernanceMapper;
