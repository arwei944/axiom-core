//! Witness - Immutable audit record for every state transition.
//!
//! Every state transition automatically produces a Witness, forming an
//! append-only SHA-256 hash chain. Each Witness carries:
//! - Before/after state hashes (for state integrity verification)
//! - Triggering signal reference
//! - Vector clock for causal ordering
//! - Correlation/trace IDs for full-trace reconstruction

use crate::context::CellContext;
use crate::id::{CorrelationId, MsgId, TraceId, WitnessId};
use crate::signal::VectorClock;
use crate::version::{SchemaVersion, Versioned, WitnessSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl Witness {
    #[cfg(feature = "sha2-id")]
    pub fn compute_hash(&self, prev_hash: &Option<WitnessHash>) -> WitnessHash {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.witness_id.as_str().as_bytes());
        hasher.update(self.cell_id.as_bytes());
        hasher.update(self.correlation_id.as_str().as_bytes());
        hasher.update(self.timestamp_ns.to_le_bytes());
        if let Some(ph) = prev_hash {
            hasher.update(ph.0);
        }
        if let Some(sbh) = &self.state_before_hash {
            hasher.update(sbh.0);
        }
        if let Some(sah) = &self.state_after_hash {
            hasher.update(sah.0);
        }
        hasher.update(self.summary.as_bytes());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        WitnessHash(hash)
    }

    pub fn verify_chain_integrity(witnesses: &[Witness]) -> bool {
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
        self
    }

    pub fn outcome(mut self, o: TransitionOutcome) -> Self {
        self.outcome = o;
        self
    }

    pub fn failed(self, reason: impl Into<String>) -> Self {
        self.outcome(TransitionOutcome::Failed {
            reason: reason.into(),
        })
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

    pub fn emit(self, ctx: &mut CellContext<'_>) {
        #[cfg(feature = "uuid")]
        let witness_id = WitnessId::generate();
        #[cfg(not(feature = "uuid"))]
        let witness_id = WitnessId::new({
            use std::time::{SystemTime, UNIX_EPOCH};
            format!(
                "wit-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )
        });

        let correlation = ctx
            .current_correlation
            .clone()
            .unwrap_or_else(|| CorrelationId::new("none"));
        let trace = ctx.current_trace.clone();
        let triggering = ctx.current_msg_id.clone();
        let timestamp = crate::signal::now_ns();
        let vc = ctx.vector_clock.clone();
        let cell_id = ctx.cell_id.as_str().to_string();

        let prev_hash = None;

        let mut witness = Witness {
            witness_id,
            schema_version: WitnessSchema::schema_version(),
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
        };

        #[cfg(feature = "sha2-id")]
        {
            witness.hash = witness.compute_hash(&witness.prev_hash);
        }

        ctx.add_witness(witness);
    }
}

impl Default for WitnessBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "sha2-id")]
    #[test]
    fn test_witness_hash_chain() {
        let w1 = Witness {
            witness_id: WitnessId::new("w1"),
            schema_version: WitnessSchema::schema_version(),
            cell_id: "cell-a".to_string(),
            correlation_id: CorrelationId::new("c1"),
            trace_id: None,
            triggering_msg_id: None,
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            prev_hash: None,
            state_before_hash: None,
            state_after_hash: None,
            hash: WitnessHash::zero(),
            summary: "first".to_string(),
            outcome: TransitionOutcome::Success,
            metrics: WitnessMetrics::default(),
        };
        let mut w1 = w1;
        w1.hash = w1.compute_hash(&None);

        let mut w2 = Witness {
            witness_id: WitnessId::new("w2"),
            schema_version: WitnessSchema::schema_version(),
            cell_id: "cell-a".to_string(),
            correlation_id: CorrelationId::new("c1"),
            trace_id: None,
            triggering_msg_id: None,
            vector_clock: VectorClock::new(),
            timestamp_ns: 2,
            prev_hash: Some(w1.hash.clone()),
            state_before_hash: None,
            state_after_hash: None,
            hash: WitnessHash::zero(),
            summary: "second".to_string(),
            outcome: TransitionOutcome::Success,
            metrics: WitnessMetrics::default(),
        };
        w2.hash = w2.compute_hash(&w2.prev_hash);

        assert!(Witness::verify_chain_integrity(&[w1, w2]));
    }
}
