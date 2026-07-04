use crate::clock::global_clock;
use crate::id::WitnessId;
use crate::version::{VersionInfo, Versioned, WitnessSchema};

impl super::def::Witness {
    pub fn new(
        cell_id: crate::id::CellId,
        kind: super::def::WitnessKind,
        event: super::def::WitnessEvent,
        _layer: crate::layer::Layer,
    ) -> Self {
        #[cfg(feature = "uuid")]
        let witness_id = WitnessId::generate();
        #[cfg(not(feature = "uuid"))]
        let witness_id = WitnessId::new(format!("wit-{}", global_clock().now_ns()));

        let event_summary = match &event {
            super::def::WitnessEvent::ToolExecuted { tool_name, .. } => {
                format!("tool {} executed", tool_name)
            }
            super::def::WitnessEvent::GuardChecked {
                guard_name, passed, ..
            } => {
                format!(
                    "guard {} check {}",
                    guard_name,
                    if *passed { "passed" } else { "failed" }
                )
            }
            super::def::WitnessEvent::StateChanged { from, to, .. } => {
                format!("state changed: {} -> {}", from, to)
            }
            super::def::WitnessEvent::SignalSent { signal_type, .. } => {
                format!("signal {} sent", signal_type)
            }
            super::def::WitnessEvent::LensProjected {
                lens_id,
                was_cached,
                ..
            } => {
                format!("lens {} projected (cached: {})", lens_id, was_cached)
            }
        };

        Self {
            witness_id,
            schema_version: WitnessSchema::schema_version(),
            cell_id: cell_id.as_str().to_string(),
            correlation_id: crate::id::CorrelationId::new("none"),
            trace_id: None,
            triggering_msg_id: None,
            vector_clock: crate::signal::VectorClock::new(),
            timestamp_ns: global_clock().now_ns(),
            prev_hash: None,
            state_before_hash: None,
            state_after_hash: None,
            hash: super::def::WitnessHash::zero(),
            summary: event_summary,
            outcome: super::def::TransitionOutcome::Success,
            metrics: super::def::WitnessMetrics::default(),
            version_info: VersionInfo::current(),
            signal_fingerprint: [0u8; 32],
            payload_size_bytes: 0,
            kind,
        }
    }

    #[cfg(feature = "sha2-id")]
    pub fn compute_hash(
        &self,
        prev_hash: &Option<super::def::WitnessHash>,
    ) -> crate::Result<super::def::WitnessHash> {
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
        hasher.update(self.signal_fingerprint);
        hasher.update(self.payload_size_bytes.to_le_bytes());
        let vi_bytes = serde_json::to_vec(&self.version_info).map_err(|e| {
            crate::AxiomError::WitnessSerialization {
                cell_id: self.cell_id.clone(),
                message: format!("version_info: {e}"),
            }
        })?;
        hasher.update(&vi_bytes);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Ok(super::def::WitnessHash(hash))
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
