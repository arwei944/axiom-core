//! Axiom Core - Compatibility layer over axiom-kernel (deprecated)
//!
//! # Deprecation Notice
//! This crate is deprecated. All new code should use `axiom-kernel` directly.
//!
//! This crate only re-exports from `axiom-kernel` for backward compatibility.
//! The `axiom-core` runtime layer has been completely replaced by `axiom-kernel`.

#![allow(deprecated)]

pub use axiom_kernel::axiom::{
    AxiomKernel, AxiomViolation, DynAxiom, DynAxiomChain, DynLens, KernelError as AxiomError,
    KernelResult, Message, Projection, SignalHandler, State, ValidationError, ValidationResult,
    ValidationSeverity, ViolationAction,
};
pub use axiom_kernel::cell::{
    BoxHandleFuture, CellHandle, CellKernel, DynCell, DynHandleCell, RuntimeCellHandle,
    SupervisionStrategy,
};
pub use axiom_kernel::clock::{global_clock, set_global_clock, Clock, MockClock, SystemClock};
pub use axiom_kernel::codec::{JsonCodec, SignalCodec};
pub use axiom_kernel::context::{CellContext, OutgoingEnvelope, OutgoingWitness};
pub use axiom_kernel::entropy::{
    CellEntropy, EntropyLevel, EntropyScore, EntropySnapshot, EntropyWeights, CRITICAL_THRESHOLD,
    DEFAULT_HALF_LIFE_SECS, GREEN_THRESHOLD, RED_THRESHOLD, WEIGHT_AXIOM_VIOLATIONS,
    WEIGHT_CELL_RESTARTS, WEIGHT_CIRCUIT_BREAKS, WEIGHT_DROPPED_MESSAGES,
    WEIGHT_DUPLICATE_MESSAGES, WEIGHT_REJECTED_BY_GUARDIAN, WEIGHT_STALE_STATE_VIOLATIONS,
    WEIGHT_TIMEOUTS, YELLOW_THRESHOLD,
};
pub use axiom_kernel::gate;
pub use axiom_kernel::guard::{BoxedGuard, DynGuard, Guard};
pub use axiom_kernel::heatmap::collector::UsageSnapshot;
pub use axiom_kernel::heatmap::{HeatmapCollector, HeatmapExporter};
pub use axiom_kernel::id::{AxiomId, CellId, CorrelationId, LensId, MsgId, TraceId, WitnessId};
pub use axiom_kernel::layer::Layer;
pub use axiom_kernel::lens::LensKernel;
pub use axiom_kernel::plugin::{
    abi::{AxiomPlugin, PluginContext, PluginError, PluginKind, PluginMessage, PluginReply},
    composer::Composer,
    loader::NativePluginLoader,
    registry::PluginRegistry,
};
pub use axiom_kernel::registry::{
    CapabilityDescriptor, CapabilityDimension, CapabilityVersionRegistry, LensRegistry,
    WitnessRegistry, AXIOM_REGISTRY, CAPABILITY_REGISTRY, LENS_REGISTRY, MIGRATION_REGISTRY,
    WITNESS_REGISTRY,
};
pub use axiom_kernel::sealed::{
    AgentLayer, CanSendTo, ExecLayer, LayerMarker, OversightLayer, ValidateLayer,
};
pub use axiom_kernel::signal::{Signal, SignalEnvelope, SignalKernel, SignalKind, VectorClock};
pub use axiom_kernel::tool::{BoxedTool, DynTool, Tool};
pub use axiom_kernel::version::{
    Compatibility, CrateVersion, EventSchema, IdentityVersion, ProtocolVersion, SchemaVersion,
    SignalSchema, Version, VersionInfo, Versioned, WitnessSchema,
};
pub use axiom_kernel::witness::{
    TransitionOutcome, Witness, WitnessBuilder, WitnessEvent, WitnessGenerator, WitnessHash,
    WitnessKernel, WitnessKind, WitnessMetrics,
};

pub use axiom_macros::{
    axiom, capability, cell, guard, lens, migration, schema_version, signal, tool, SignalPayload,
};

pub type Result<T> = KernelResult<T>;
