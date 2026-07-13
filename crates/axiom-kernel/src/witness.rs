//! Witness - Immutable audit record for every state transition.
//!
//! Every state transition automatically produces a Witness, forming an
//! append-only SHA-256 hash chain.

use crate::axiom::KernelResult;
use crate::id::{CorrelationId, MsgId, TraceId, WitnessId};
use crate::layer::RuntimeTier;
use crate::signal::VectorClock;
use crate::version::{SchemaVersion, VersionInfo};
use crate::HeatmapCollector;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

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
        signal_layer: RuntimeTier,
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

const MAX_SUMMARY_LEN: usize = 512;
const MAX_REASON_LEN: usize = 1024;

pub struct WitnessBuilder {
    summary: String,
    outcome: TransitionOutcome,
    state_before: Option<WitnessHash>,
    state_after: Option<WitnessHash>,
    processing_time_us: u64,
}

impl WitnessBuilder {
    pub fn new() -> Self {
        Self {
            summary: String::new(),
            outcome: TransitionOutcome::Success,
            state_before: None,
            state_after: None,
            processing_time_us: 0,
        }
    }

    pub fn summary(mut self, s: impl Into<String>) -> Self {
        self.summary = s.into();
        if self.summary.len() > MAX_SUMMARY_LEN {
            self.summary.truncate(MAX_SUMMARY_LEN);
        }
        self
    }

    pub fn outcome(mut self, o: TransitionOutcome) -> Self {
        self.outcome = match o {
            TransitionOutcome::Failed { reason } => {
                let mut reason = reason;
                if reason.len() > MAX_REASON_LEN {
                    reason.truncate(MAX_REASON_LEN);
                }
                TransitionOutcome::Failed { reason }
            }
            TransitionOutcome::AxiomViolated { axiom_name, message } => {
                TransitionOutcome::AxiomViolated {
                    axiom_name,
                    message: {
                        let mut message = message;
                        if message.len() > MAX_REASON_LEN {
                            message.truncate(MAX_REASON_LEN);
                        }
                        message
                    },
                }
            }
            other => other,
        };
        self
    }

    pub fn failed(self, reason: impl Into<String>) -> Self {
        self.outcome(TransitionOutcome::Failed { reason: reason.into() })
    }

    pub fn axiom_violated(self, name: impl Into<String>, msg: impl Into<String>) -> Self {
        self.outcome(TransitionOutcome::AxiomViolated {
            axiom_name: name.into(),
            message: msg.into(),
        })
    }

    pub fn state_before(mut self, hash: WitnessHash) -> Self {
        self.state_before = Some(hash);
        self
    }

    pub fn state_after(mut self, hash: WitnessHash) -> Self {
        self.state_after = Some(hash);
        self
    }

    pub fn processing_time_us(mut self, us: u64) -> Self {
        self.processing_time_us = us;
        self
    }

    pub fn emit(self, ctx: &mut crate::context::CellContext<'_>) -> KernelResult<()> {
        let witness_id = crate::id::WitnessId::new(uuid::Uuid::new_v4().to_string());

        let correlation = ctx
            .current_correlation
            .clone()
            .unwrap_or_else(|| crate::id::CorrelationId::new("none"));
        let trace = ctx.current_trace.clone();
        let triggering = ctx.current_msg_id.clone();
        let timestamp = crate::clock::global_clock().now_ns();
        let vc = ctx.vector_clock.clone();
        let cell_id = ctx.cell_id.as_str().to_string();
        let version_info = crate::version::VersionInfo::current();

        let signal_fingerprint =
            match (&ctx.current_signal_type, ctx.current_schema_version, &ctx.current_payload) {
                (Some(st), Some(sv), Some(pl)) => {
                    #[cfg(feature = "sha2-id")]
                    {
                        use sha2::{Digest, Sha256};
                        let mut hasher = Sha256::new();
                        hasher.update(st.as_bytes());
                        hasher.update(sv.to_string().as_bytes());
                        hasher.update(serde_json::to_string(pl)?);
                        let result = hasher.finalize();
                        let mut fingerprint = [0u8; 32];
                        fingerprint.copy_from_slice(&result);
                        fingerprint
                    }

                    #[cfg(not(feature = "sha2-id"))]
                    {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::Hasher;
                        let mut hasher = DefaultHasher::new();
                        hasher.write(st.as_bytes());
                        hasher.write(sv.to_string().as_bytes());
                        hasher.write(serde_json::to_string(pl).unwrap_or_default().as_bytes());
                        let hash = hasher.finish();
                        let mut fingerprint = [0u8; 32];
                        fingerprint[0..8].copy_from_slice(&hash.to_be_bytes());
                        fingerprint
                    }
                }
                _ => [0u8; 32],
            };

        let prev_hash = ctx.last_witness_hash;

        let payload_size = serde_json::to_vec(&self.summary)
            .map_err(|e| crate::KernelError::WitnessSerialization {
                cell_id: ctx.cell_id.as_str().to_string(),
                message: format!("summary payload_size: {e}"),
            })?
            .len();

        let mut witness = Witness {
            witness_id,
            schema_version: crate::version::SchemaVersion::new(1),
            cell_id,
            correlation_id: correlation,
            trace_id: trace,
            triggering_msg_id: triggering,
            vector_clock: vc,
            timestamp_ns: timestamp,
            prev_hash,
            state_before_hash: self.state_before,
            state_after_hash: self.state_after,
            hash: WitnessHash::zero(),
            summary: self.summary,
            outcome: self.outcome,
            metrics: WitnessMetrics {
                processing_time_us: self.processing_time_us,
                signals_sent: ctx.outgoing.len() as u32,
                witnesses_produced: ctx.witnesses.len() as u32 + 1,
            },
            version_info,
            signal_fingerprint,
            payload_size_bytes: payload_size,
            kind: WitnessKind::StateTransition,
        };
        witness.hash = witness.compute_hash()?;
        ctx.last_witness_hash = Some(witness.hash);

        ctx.witnesses.push(crate::context::OutgoingWitness(witness));
        Ok(())
    }
}

impl Default for WitnessBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Witness {
    pub fn compute_hash(&self) -> KernelResult<WitnessHash> {
        #[cfg(feature = "sha2-id")]
        {
            use sha2::{Digest, Sha256};

            // Adapter that streams serialized bytes straight into the hasher,
            // avoiding the intermediate String allocations that the previous
            // `serde_json::to_string(..)?.as_bytes()` calls produced. The
            // hashed bytes are identical, so existing witness chains stay valid.
            struct HashWriter<'a> {
                inner: &'a mut Sha256,
            }
            impl<'a> std::io::Write for HashWriter<'a> {
                fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                    self.inner.update(bytes);
                    Ok(bytes.len())
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    Ok(())
                }
            }

            let mut hasher = Sha256::new();
            hasher.update(self.witness_id.as_str().as_bytes());
            hasher.update(self.cell_id.as_bytes());
            hasher.update(self.correlation_id.as_str().as_bytes());
            if let Some(ref t) = self.trace_id {
                hasher.update(t.as_str().as_bytes());
            }
            if let Some(ref t) = self.triggering_msg_id {
                hasher.update(t.as_str().as_bytes());
            }
            {
                let mut writer = HashWriter { inner: &mut hasher };
                serde_json::to_writer(&mut writer, &self.vector_clock)?;
            }
            hasher.update(self.timestamp_ns.to_be_bytes());
            hasher.update(self.schema_version.to_string().as_bytes());
            hasher.update(self.summary.as_bytes());
            {
                let mut writer = HashWriter { inner: &mut hasher };
                serde_json::to_writer(&mut writer, &self.outcome)?;
                serde_json::to_writer(&mut writer, &self.metrics)?;
            }
            if let Some(ref h) = self.prev_hash {
                hasher.update(h.0);
            }
            if let Some(ref h) = self.state_before_hash {
                hasher.update(h.0);
            }
            if let Some(ref h) = self.state_after_hash {
                hasher.update(h.0);
            }
            hasher.update(self.signal_fingerprint);
            hasher.update(self.payload_size_bytes.to_be_bytes());
            {
                let mut writer = HashWriter { inner: &mut hasher };
                serde_json::to_writer(&mut writer, &self.kind)?;
            }
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            Ok(WitnessHash(hash))
        }

        #[cfg(not(feature = "sha2-id"))]
        {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::Hasher;
            let mut hasher = DefaultHasher::new();
            hasher.write(self.witness_id.as_str().as_bytes());
            hasher.write(self.cell_id.as_bytes());
            hasher.write(self.correlation_id.as_str().as_bytes());
            if let Some(ref t) = self.trace_id {
                hasher.write(t.as_str().as_bytes());
            }
            if let Some(ref t) = self.triggering_msg_id {
                hasher.write(t.as_str().as_bytes());
            }
            hasher.write(serde_json::to_string(&self.vector_clock)?.as_bytes());
            hasher.write(self.timestamp_ns.to_be_bytes());
            hasher.write(self.schema_version.to_string().as_bytes());
            hasher.write(self.summary.as_bytes());
            let outcome_bytes = serde_json::to_string(&self.outcome)?;
            hasher.write(outcome_bytes.as_bytes());
            let metrics_bytes = serde_json::to_string(&self.metrics)?;
            hasher.write(metrics_bytes.as_bytes());
            if let Some(ref h) = self.prev_hash {
                hasher.write(&h.0);
            }
            if let Some(ref h) = self.state_before_hash {
                hasher.write(&h.0);
            }
            if let Some(ref h) = self.state_after_hash {
                hasher.write(&h.0);
            }
            hasher.write(&self.signal_fingerprint);
            hasher.write(self.payload_size_bytes.to_be_bytes());
            hasher.write(serde_json::to_string(&self.kind)?.as_bytes());
            let hash = hasher.finish();
            let mut result = [0u8; 32];
            result[0..8].copy_from_slice(&hash.to_be_bytes());
            Ok(WitnessHash(result))
        }
    }

    pub fn verify_chain_integrity(witnesses: &[Self]) -> bool {
        for window in witnesses.windows(2) {
            let prev = &window[0];
            let curr = &window[1];
            if curr.prev_hash.as_ref() != Some(&prev.hash) {
                return false;
            }
        }
        true
    }
}

// ============================================================
// WitnessKernel - simple in-memory witness store
// ============================================================

#[derive(Debug, Default)]
pub struct WitnessKernel {
    witnesses: RwLock<Vec<Witness>>,
}

impl WitnessKernel {
    pub fn new() -> Self {
        Self { witnesses: RwLock::new(Vec::new()) }
    }

    pub fn with_heatmap(_heatmap: std::sync::Arc<RwLock<HeatmapCollector>>) -> Self {
        Self::new()
    }

    pub async fn record(&self, witness: Witness) {
        let mut store = self.witnesses.write().await;
        store.push(witness);
    }

    pub async fn verify_chain(&self) -> Result<(), Vec<String>> {
        let store = self.witnesses.read().await;
        let mut errors = Vec::new();
        for window in store.windows(2) {
            let prev = &window[0];
            let curr = &window[1];
            if curr.prev_hash.as_ref() != Some(&prev.hash) {
                errors.push(format!(
                    "chain break: {} -> {}",
                    prev.witness_id.as_str(),
                    curr.witness_id.as_str()
                ));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub async fn get_recent(&self, limit: usize) -> Vec<Witness> {
        let store = self.witnesses.read().await;
        store.iter().rev().take(limit).cloned().collect()
    }

    pub async fn len(&self) -> usize {
        let store = self.witnesses.read().await;
        store.len()
    }

    pub async fn is_empty(&self) -> bool {
        let store = self.witnesses.read().await;
        store.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash_differs_on_outcome() {
        let w1 = Witness {
            witness_id: WitnessId::new("w1"),
            schema_version: SchemaVersion::new(1),
            cell_id: "c1".into(),
            correlation_id: CorrelationId::new("corr"),
            trace_id: None,
            triggering_msg_id: None,
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            prev_hash: None,
            state_before_hash: None,
            state_after_hash: None,
            hash: WitnessHash::zero(),
            summary: "same summary".into(),
            outcome: TransitionOutcome::Success,
            metrics: WitnessMetrics {
                processing_time_us: 10,
                signals_sent: 1,
                witnesses_produced: 1,
            },
            version_info: crate::version::VersionInfo::current(),
            signal_fingerprint: [0u8; 32],
            payload_size_bytes: 0,
            kind: WitnessKind::StateTransition,
        };

        let mut w2 = w1.clone();
        w2.outcome = TransitionOutcome::Failed { reason: "boom".into() };

        let h1 = w1.compute_hash().unwrap();
        let h2 = w2.compute_hash().unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_compute_hash_is_deterministic() {
        let w = Witness {
            witness_id: WitnessId::new("w-det"),
            schema_version: SchemaVersion::new(1),
            cell_id: "c1".into(),
            correlation_id: CorrelationId::new("corr"),
            trace_id: None,
            triggering_msg_id: None,
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            prev_hash: None,
            state_before_hash: None,
            state_after_hash: None,
            hash: WitnessHash::zero(),
            summary: "deterministic".into(),
            outcome: TransitionOutcome::Success,
            metrics: WitnessMetrics {
                processing_time_us: 10,
                signals_sent: 1,
                witnesses_produced: 1,
            },
            version_info: crate::version::VersionInfo::current(),
            signal_fingerprint: [0u8; 32],
            payload_size_bytes: 0,
            kind: WitnessKind::StateTransition,
        };

        let h = w.compute_hash().unwrap();
        assert_eq!(h, w.compute_hash().unwrap());
    }
}
