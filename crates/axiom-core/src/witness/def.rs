use crate::id::{CorrelationId, MsgId, TraceId, WitnessId};
use crate::layer::Layer;
use crate::signal::VectorClock;
use crate::version::{SchemaVersion, VersionInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WitnessKind {
    StateTransition,
    ToolInvocation,
    GuardCheck,
    SignalEmission,
    CellStartup,
    CellShutdown,
    LensProjection,
    CacheHit,
    CacheMiss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WitnessEvent {
    ToolExecuted {
        tool_name: String,
        parameters: serde_json::Value,
        timestamp: u64,
    },
    GuardChecked {
        guard_name: String,
        signal_type: String,
        signal_layer: Layer,
        passed: bool,
        timestamp: u64,
    },
    StateChanged {
        from: String,
        to: String,
        timestamp: u64,
    },
    SignalSent {
        signal_type: String,
        target_cell: Option<String>,
        timestamp: u64,
    },
    LensProjected {
        lens_id: String,
        input_hash: [u8; 32],
        output_hash: [u8; 32],
        event_count: usize,
        projection_time_ms: u64,
        was_cached: bool,
        timestamp: u64,
    },
}

pub trait WitnessGenerator {
    fn generate_witness(&self, event: WitnessEvent) -> Witness;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessHash(pub [u8; 32]);

impl WitnessHash {
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    #[cfg(feature = "sha2-id")]
    pub fn from_bytes_sha2(data: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Self(hash)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    pub witness_id: WitnessId,
    pub schema_version: SchemaVersion,
    pub cell_id: String,
    pub correlation_id: CorrelationId,
    pub trace_id: Option<TraceId>,
    pub triggering_msg_id: Option<MsgId>,
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
    pub prev_hash: Option<WitnessHash>,
    pub state_before_hash: Option<WitnessHash>,
    pub state_after_hash: Option<WitnessHash>,
    pub hash: WitnessHash,
    pub summary: String,
    pub outcome: TransitionOutcome,
    pub metrics: WitnessMetrics,
    pub version_info: VersionInfo,
    pub signal_fingerprint: [u8; 32],
    pub payload_size_bytes: usize,
    pub kind: WitnessKind,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WitnessMetrics {
    pub processing_time_us: u64,
    pub signals_sent: u32,
    pub witnesses_produced: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionOutcome {
    Success,
    Failed { reason: String },
    AxiomViolated { axiom_name: String, message: String },
}
