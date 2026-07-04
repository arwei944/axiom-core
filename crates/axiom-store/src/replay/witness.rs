//! Witness replay utilities.

use crate::event::Event;
use crate::replay::ReplayableState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessReplayResult<S> {
    pub state: S,
    pub witnesses_consumed: u64,
    pub chain_valid: bool,
    pub replay_duration_ms: u64,
}

pub struct WitnessReplay;

impl WitnessReplay {
    pub fn replay<S: ReplayableState + Default>(
        witnesses: &[axiom_core::Witness],
    ) -> WitnessReplayResult<S> {
        let start = std::time::Instant::now();
        let chain_valid = axiom_core::Witness::verify_chain_integrity(witnesses);
        let mut state = S::default();

        for w in witnesses {
            if let Ok(_event) = serde_json::from_value::<axiom_core::WitnessEvent>(
                serde_json::to_value(&w.summary).unwrap_or_default(),
            ) {
                if let Err(err) =
                    state.apply_event("witness", &serde_json::to_value(w).unwrap_or_default())
                {
                    tracing::warn!(witness_id = ?w.witness_id, error = ?err, "witness replay apply_event failed");
                }
            }
        }

        WitnessReplayResult {
            state,
            witnesses_consumed: witnesses.len() as u64,
            chain_valid,
            replay_duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    pub fn replay_from_events<S: ReplayableState + Default>(
        events: &[Event],
    ) -> WitnessReplayResult<S> {
        let start = std::time::Instant::now();
        let mut witnesses = Vec::new();

        for e in events {
            if let Ok(w) = serde_json::from_value::<axiom_core::Witness>(e.payload.clone()) {
                witnesses.push(w);
            }
        }

        let chain_valid = axiom_core::Witness::verify_chain_integrity(&witnesses);
        let mut state = S::default();

        for w in &witnesses {
            if let Err(err) =
                state.apply_event("witness", &serde_json::to_value(w).unwrap_or_default())
            {
                tracing::warn!(witness_id = ?w.witness_id, error = ?err, "witness replay apply_event failed");
            }
        }

        WitnessReplayResult {
            state,
            witnesses_consumed: witnesses.len() as u64,
            chain_valid,
            replay_duration_ms: start.elapsed().as_millis() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventBuilder;
    use crate::memory::MemoryStore;
    use crate::replay::ReplayableState;
    use crate::store::EventStore;
    use crate::StoreError;
    use std::sync::Arc;

    #[derive(Debug, Clone, Default, PartialEq)]
    struct CounterState {
        count: i64,
        name: String,
    }

    impl ReplayableState for CounterState {
        fn apply_event(
            &mut self,
            event_type: &str,
            payload: &serde_json::Value,
        ) -> Result<(), StoreError> {
            match event_type {
                "increment" => {
                    let by = payload.get("by").and_then(|v| v.as_i64()).unwrap_or(1);
                    self.count += by;
                }
                "set_name" => {
                    self.name = payload
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }
                _ => {}
            }
            Ok(())
        }

        fn current_schema_version() -> u32 {
            1
        }

        fn to_snapshot(&self) -> serde_json::Value {
            serde_json::json!({"count": self.count, "name": self.name})
        }

        fn from_snapshot(value: &serde_json::Value) -> Result<Self, StoreError> {
            Ok(CounterState {
                count: value.get("count").and_then(|v| v.as_i64()).unwrap_or(0),
                name: value
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
        }
    }

    #[tokio::test]
    async fn test_witness_replay_from_events() {
        use axiom_core::Witness;

        let store = Arc::new(MemoryStore::new());
        let witness = Witness {
            witness_id: axiom_core::id::WitnessId::new("w1"),
            schema_version: axiom_core::version::SchemaVersion::new(1),
            cell_id: "c1".into(),
            correlation_id: axiom_core::id::CorrelationId::new("corr"),
            trace_id: None,
            triggering_msg_id: None,
            vector_clock: Default::default(),
            timestamp_ns: 1,
            prev_hash: None,
            state_before_hash: None,
            state_after_hash: None,
            hash: axiom_core::witness::WitnessHash([1; 32]),
            summary: "test".into(),
            outcome: axiom_core::witness::TransitionOutcome::Success,
            metrics: Default::default(),
            version_info: axiom_core::version::VersionInfo::current(),
            signal_fingerprint: [0; 32],
            payload_size_bytes: 0,
            kind: axiom_core::witness::WitnessKind::StateTransition,
        };

        let payload = serde_json::to_value(&witness).unwrap();
        let event = EventBuilder::new("c1", "witness", payload).build();
        store.append(event).await.unwrap();

        let events = store.read("c1").await.unwrap();
        let result = WitnessReplay::replay_from_events::<CounterState>(&events);
        assert!(result.chain_valid);
        assert_eq!(result.witnesses_consumed, 1);
    }
}
