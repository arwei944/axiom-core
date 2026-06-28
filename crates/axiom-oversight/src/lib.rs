//! Axiom Oversight - Layer 0 meta-governance layer.
//!
//! The Oversight Layer is the "prefrontal cortex" of the system.
//! Unlike the supervision tree (which handles Cell crashes),
//! Oversight handles architecture compliance, entropy governance,
//! intent auditing, resource management, and global deadlock detection.
//!
//! # Oversight Cells
//! - **EntropyGovernor**: monitors system/cell entropy, triggers de-entropy actions
//! - **ArchitectureGuardian**: detects illegal cross-layer calls and dependency violations
//! - **IntentAuditor**: checks agent output for intent drift
//! - **ResourceManager**: token budgets, API rate limits, fair scheduling
//! - **LoopDetector**: global message loop detection, handoff limit enforcement
//! - **ComplianceGuard**: PII detection, dangerous operation approval
//! - **OversightOversight**: meta-oversight - supervises the overseers

pub mod architecture_guardian;
pub mod entropy_governor;
