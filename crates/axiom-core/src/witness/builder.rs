use crate::context::CellContext;
use crate::version::Versioned;

const MAX_SUMMARY_LEN: usize = 512;
const MAX_REASON_LEN: usize = 1024;

pub struct WitnessBuilder {
    summary: String,
    outcome: super::def::TransitionOutcome,
    state_before: Option<super::def::WitnessHash>,
    state_after: Option<super::def::WitnessHash>,
    processing_time_us: u64,
}

impl WitnessBuilder {
    pub fn new() -> Self {
        Self {
            summary: String::new(),
            outcome: super::def::TransitionOutcome::Success,
            state_before: None,
            state_after: None,
            processing_time_us: 0,
        }
    }

    pub fn summary(mut self, s: impl Into<String>) -> Self {
        self.summary = super::hash::truncate(&s.into(), MAX_SUMMARY_LEN);
        self
    }

    pub fn outcome(mut self, o: super::def::TransitionOutcome) -> Self {
        self.outcome = match o {
            super::def::TransitionOutcome::Failed { reason } => {
                super::def::TransitionOutcome::Failed {
                    reason: super::hash::truncate(&reason, MAX_REASON_LEN),
                }
            }
            super::def::TransitionOutcome::AxiomViolated {
                axiom_name,
                message,
            } => super::def::TransitionOutcome::AxiomViolated {
                axiom_name,
                message: super::hash::truncate(&message, MAX_REASON_LEN),
            },
            other => other,
        };
        self
    }

    pub fn failed(self, reason: impl Into<String>) -> Self {
        self.outcome(super::def::TransitionOutcome::Failed {
            reason: reason.into(),
        })
    }

    pub fn axiom_violated(self, name: impl Into<String>, msg: impl Into<String>) -> Self {
        self.outcome(super::def::TransitionOutcome::AxiomViolated {
            axiom_name: name.into(),
            message: msg.into(),
        })
    }

    pub fn state_before(mut self, hash: super::def::WitnessHash) -> Self {
        self.state_before = Some(hash);
        self
    }

    pub fn state_after(mut self, hash: super::def::WitnessHash) -> Self {
        self.state_after = Some(hash);
        self
    }

    pub fn processing_time_us(mut self, us: u64) -> Self {
        self.processing_time_us = us;
        self
    }

    pub fn emit(self, ctx: &mut CellContext<'_>) -> crate::Result<()> {
        #[cfg(feature = "uuid")]
        let witness_id = crate::id::WitnessId::generate();
        #[cfg(not(feature = "uuid"))]
        let witness_id = crate::id::WitnessId::new({
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
            .unwrap_or_else(|| crate::id::CorrelationId::new("none"));
        let trace = ctx.current_trace.clone();
        let triggering = ctx.current_msg_id.clone();
        let timestamp = crate::clock::global_clock().now_ns();
        let vc = ctx.vector_clock.clone();
        let cell_id = ctx.cell_id.as_str().to_string();
        let version_info = crate::version::VersionInfo::current();

        // Use real signal data from CellContext for fingerprint
        let signal_fingerprint = match (
            &ctx.current_signal_type,
            ctx.current_schema_version,
            &ctx.current_payload,
        ) {
            (Some(st), Some(sv), Some(pl)) => super::hash::compute_signal_fingerprint(st, sv, pl)?,
            _ => [0u8; 32],
        };

        // Chain from last witness hash in CellContext
        let prev_hash = ctx.last_witness_hash;

        let payload_size = serde_json::to_vec(&self.summary)
            .map_err(|e| crate::AxiomError::WitnessSerialization {
                cell_id: ctx.cell_id.as_str().to_string(),
                message: format!("summary payload_size: {e}"),
            })?
            .len();

        let mut witness = super::def::Witness {
            witness_id,
            schema_version: crate::version::WitnessSchema::schema_version(),
            cell_id,
            correlation_id: correlation,
            trace_id: trace,
            triggering_msg_id: triggering,
            vector_clock: vc,
            timestamp_ns: timestamp,
            prev_hash,
            state_before_hash: self.state_before,
            state_after_hash: self.state_after,
            hash: super::def::WitnessHash::zero(),
            summary: self.summary,
            outcome: self.outcome,
            metrics: super::def::WitnessMetrics {
                processing_time_us: self.processing_time_us,
                signals_sent: ctx.outgoing.len() as u32,
                witnesses_produced: ctx.witnesses.len() as u32 + 1,
            },
            version_info,
            signal_fingerprint,
            payload_size_bytes: payload_size,
            kind: super::def::WitnessKind::StateTransition,
        };

        #[cfg(feature = "sha2-id")]
        {
            witness.hash = witness.compute_hash(&witness.prev_hash)?;
        }

        // Update last_witness_hash for chaining
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
