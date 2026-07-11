pub mod axiom;
pub mod cell;
pub mod clock;
pub mod codec;
pub mod context;
pub mod entropy;
pub mod error;
pub mod gate;
pub mod guard;
pub mod heatmap;
pub mod id;
pub mod layer;
pub mod lens;
pub mod plugin;
pub mod registry;
pub mod sealed;
pub mod signal;
pub mod tool;
pub mod version;
pub mod witness;

pub use axiom::{
    AxiomKernel, AxiomViolation, DynAxiom, DynAxiomChain, DynLens, KernelError, KernelResult,
    Message, Projection, SignalHandler, State, ValidationError, ValidationResult,
    ValidationSeverity, ViolationAction,
};
pub use cell::{
    BoxHandleFuture, CellHandle, CellKernel, DynCell, DynHandleCell, RuntimeCellHandle,
    SupervisionStrategy,
};
pub use clock::{global_clock, set_global_clock, Clock, MockClock, SystemClock};
pub use codec::{JsonCodec, SignalCodec};
pub use context::{CellContext, OutgoingEnvelope, OutgoingWitness};
pub use entropy::{
    CellEntropy, EntropyLevel, EntropyScore, EntropySnapshot, EntropyWeights, CRITICAL_THRESHOLD,
    DEFAULT_HALF_LIFE_SECS, GREEN_THRESHOLD, RED_THRESHOLD, WEIGHT_AXIOM_VIOLATIONS,
    WEIGHT_CELL_RESTARTS, WEIGHT_CIRCUIT_BREAKS, WEIGHT_DROPPED_MESSAGES,
    WEIGHT_DUPLICATE_MESSAGES, WEIGHT_REJECTED_BY_GUARDIAN, WEIGHT_STALE_STATE_VIOLATIONS,
    WEIGHT_TIMEOUTS, YELLOW_THRESHOLD,
};
pub use guard::{BoxedGuard, DynGuard, Guard};
pub use heatmap::collector::UsageSnapshot;
pub use heatmap::{HeatmapCollector, HeatmapExporter};
pub use id::{AxiomId, CellId, CorrelationId, LensId, MsgId, TraceId, WitnessId};
#[allow(deprecated)]
pub use layer::{RuntimeTier, Layer};
pub use lens::LensKernel;
pub use plugin::{
    abi::{AxiomPlugin, PluginContext, PluginError, PluginKind, PluginMessage, PluginReply},
    composer::Composer,
    loader::NativePluginLoader,
    registry::PluginRegistry,
};
pub use registry::{
    CapabilityDescriptor, CapabilityDimension, CapabilityVersionRegistry, LensRegistry,
    RegistryGuard, WitnessRegistry, AXIOM_REGISTRY, CAPABILITY_REGISTRY, LENS_REGISTRY,
    MIGRATION_REGISTRY, WITNESS_REGISTRY, count_registered_axioms, is_axiom_registry_empty,
    registered_axioms, registered_migration_chains,
};
pub use sealed::{AgentTier, CanSendTo, ExecTier, OversightTier, RuntimeTierMarker, ValidateTier};
pub use signal::{Signal, SignalEnvelope, SignalKernel, SignalKind, VectorClock};
pub use tool::{BoxedTool, DynTool, Tool};
pub use version::{
    Compatibility, CrateVersion, EventSchema, IdentityVersion, ProtocolVersion, SchemaVersion,
    SignalSchema, Version, VersionInfo, Versioned, WitnessSchema,
};
pub use witness::{
    TransitionOutcome, Witness, WitnessBuilder, WitnessEvent, WitnessGenerator, WitnessHash,
    WitnessKernel, WitnessKind, WitnessMetrics,
};
