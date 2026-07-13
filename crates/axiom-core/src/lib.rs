//! Axiom Core - Compatibility layer over axiom-kernel (deprecated)
//!
//! # Deprecation Notice
//! This crate is deprecated. All new code should use `axiom-kernel` directly.
//!
//! This crate only re-exports from `axiom-kernel` for backward compatibility.
//! The `axiom-core` runtime layer has been completely replaced by `axiom-kernel`.
//!
//! ## Migration Guide
//! See https://github.com/arwei944/axiom-core/blob/main/docs/MIGRATION.md for migration instructions.

#![allow(deprecated)]

#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::axiom::{
    AxiomKernel, AxiomViolation, DynAxiom, DynAxiomChain, DynLens, KernelError as AxiomError,
    KernelResult, Message, Projection, SignalHandler, State, ValidationError, ValidationResult,
    ValidationSeverity, ViolationAction,
};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::cell` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::cell::{
    BoxHandleFuture, CellHandle, CellKernel, DynCell, DynHandleCell, RuntimeCellHandle,
    SupervisionStrategy,
};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::clock` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::clock::{global_clock, set_global_clock, Clock, MockClock, SystemClock};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::codec` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::codec::{JsonCodec, SignalCodec};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::context` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::context::{CellContext, OutgoingEnvelope, OutgoingWitness};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::entropy` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::entropy::{
    CellEntropy, EntropyLevel, EntropyScore, EntropySnapshot, EntropyWeights, CRITICAL_THRESHOLD,
    DEFAULT_HALF_LIFE_SECS, GREEN_THRESHOLD, RED_THRESHOLD, WEIGHT_AXIOM_VIOLATIONS,
    WEIGHT_CELL_RESTARTS, WEIGHT_CIRCUIT_BREAKS, WEIGHT_DROPPED_MESSAGES,
    WEIGHT_DUPLICATE_MESSAGES, WEIGHT_REJECTED_BY_GUARDIAN, WEIGHT_STALE_STATE_VIOLATIONS,
    WEIGHT_TIMEOUTS, YELLOW_THRESHOLD,
};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::gate` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::gate;
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::guard` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::guard::{BoxedGuard, DynGuard, Guard};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::heatmap` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::heatmap::collector::UsageSnapshot;
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::heatmap` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::heatmap::{HeatmapCollector, HeatmapExporter};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::id` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::id::{AxiomId, CellId, CorrelationId, LensId, MsgId, TraceId, WitnessId};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::layer` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::layer::RuntimeTier;
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::lens` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::lens::LensKernel;
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::plugin` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::plugin::{
    abi::{AxiomPlugin, PluginContext, PluginError, PluginKind, PluginMessage, PluginReply},
    composer::Composer,
    loader::NativePluginLoader,
    registry::PluginRegistry,
};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::registry` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::registry::{
    CapabilityDescriptor, CapabilityDimension, CapabilityVersionRegistry, LensRegistry,
    WitnessRegistry, AXIOM_REGISTRY, CAPABILITY_REGISTRY, LENS_REGISTRY, MIGRATION_REGISTRY,
    WITNESS_REGISTRY,
};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::sealed` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::sealed::{
    AgentTier, CanSendTo, ExecTier, OversightTier, RuntimeTierMarker, ValidateTier,
};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::signal` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::signal::{Signal, SignalEnvelope, SignalKernel, SignalKind, VectorClock};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::tool` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::tool::{BoxedTool, DynTool, Tool};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::version` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::version::{
    Compatibility, CrateVersion, EventSchema, IdentityVersion, ProtocolVersion, SchemaVersion,
    SignalSchema, Version, VersionInfo, Versioned, WitnessSchema,
};
#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-kernel::witness` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_kernel::witness::{
    TransitionOutcome, Witness, WitnessBuilder, WitnessEvent, WitnessGenerator, WitnessHash,
    WitnessKernel, WitnessKind, WitnessMetrics,
};

#[deprecated(
    since = "0.4.0",
    note = "Use `axiom-macros` instead. See MIGRATION.md for migration instructions."
)]
pub use axiom_macros::{
    axiom, capability, cell, guard, lens, migration, schema_version, signal, tool, SignalPayload,
};

#[deprecated(
    since = "0.4.0",
    note = "Use `axiom_kernel::KernelResult` instead. See MIGRATION.md for migration instructions."
)]
pub type Result<T> = KernelResult<T>;
