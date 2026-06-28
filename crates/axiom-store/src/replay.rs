//! Replay engine - rebuilds aggregate state from event log.

use crate::event::Event;
use crate::snapshot::{Snapshot, SnapshotStore};
use crate::store::{EventStore, StoreError};
use axiom_core::signal::VectorClock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ReplayResult<S> {
    pub state: S,
    pub last_sequence: u64,
    pub vector_clock: VectorClock,
    pub events_replayed: u64,
    pub total_events_for_aggregate: u64,
    pub snapshot_used: bool,
    pub replay_duration_ms: u64,
}

pub trait ReplayableState: Default + Send + Sync + 'static {
    fn apply_event(
        &mut self,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), StoreError>;
    fn current_schema_version() -> u32;
    fn to_snapshot(&self) -> serde_json::Value;
    fn from_snapshot(value: &serde_json::Value) -> Result<Self, StoreError>
    where
        Self: Sized;
}

pub struct ReplayEngine {
    event_store: Arc<dyn EventStore>,
    snapshot_store: Arc<dyn SnapshotStore>,
}

impl ReplayEngine {
    pub fn new(
        event_store: Arc<dyn EventStore>,
        snapshot_store: Arc<dyn SnapshotStore>,
    ) -> Self {
        Self {
            event_store,
            snapshot_store,
        }
    }

    pub async fn replay_aggregate<S: ReplayableState>(
        &self,
        aggregate_id: &str,
    ) -> Result<ReplayResult<S>, StoreError> {
        let latest = self.event_store.latest_sequence().await?;
        self.replay_to(aggregate_id, latest).await
    }

    pub async fn replay_to<S: ReplayableState>(
        &self,
        aggregate_id: &str,
        up_to_seq: u64,
    ) -> Result<ReplayResult<S>, StoreError> {
        let start_ns = axiom_core::signal::now_ns();

        let snapshot = self
            .snapshot_store
            .load_snapshot_at(aggregate_id, up_to_seq)
            .await?;

        let (mut state, start_seq, mut vc, snapshot_used) = if let Some(snap) = snapshot {
            if snap.schema_version > S::current_schema_version() {
                return Err(StoreError::Storage(format!(
                    "snapshot schema version {} is newer than current {}",
                    snap.schema_version,
                    S::current_schema_version()
                )));
            }
            let state = S::from_snapshot(&snap.state)?;
            (state, snap.sequence_number, snap.vector_clock.clone(), true)
        } else {
            (S::default(), 0, VectorClock::new(), false)
        };

        let all_events = self.event_store.read(aggregate_id).await?;
        let applicable: Vec<&Event> = all_events
            .iter()
            .filter(|e| e.sequence_number > start_seq && e.sequence_number <= up_to_seq)
            .collect();
        let total_events = all_events.len() as u64;
        let events_replayed = applicable.len() as u64;

        for event in &applicable {
            vc.merge(&event.vector_clock);
            state.apply_event(&event.event_type, &event.payload)?;
        }

        let last_sequence = applicable
            .last()
            .map(|e| e.sequence_number)
            .unwrap_or(start_seq);

        let duration_ms = (axiom_core::signal::now_ns() - start_ns) / 1_000_000;

        Ok(ReplayResult {
            state,
            last_sequence,
            vector_clock: vc,
            events_replayed,
            total_events_for_aggregate: total_events,
            snapshot_used,
            replay_duration_ms: duration_ms,
        })
    }

    pub async fn save_snapshot<S: ReplayableState>(
        &self,
        aggregate_id: &str,
        cell_id: &str,
        state: &S,
        sequence_number: u64,
        vector_clock: VectorClock,
    ) -> Result<(), StoreError> {
        let snapshot = Snapshot {
            aggregate_id: aggregate_id.to_string(),
            sequence_number,
            state: state.to_snapshot(),
            schema_version: S::current_schema_version(),
            created_at_ns: axiom_core::signal::now_ns(),
            cell_id: cell_id.to_string(),
            vector_clock,
        };
        self.snapshot_store.save_snapshot(snapshot).await
    }

    pub async fn subscribe(&self) -> crate::store::EventReceiver {
        self.event_store.subscribe()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupValidation {
    pub validated_types: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub async fn validate_migration_chains_at_startup(
    _store: &dyn EventStore,
) -> Result<StartupValidation, StoreError> {
    Ok(StartupValidation {
        validated_types: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventBuilder;
    use crate::memory::MemoryStore;
    use crate::snapshot::MemorySnapshotStore;

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

    async fn populate(store: &MemoryStore, agg: &str) {
        for _ in 1..=5 {
            let e = EventBuilder::new(agg, "increment", serde_json::json!({"by": 1}))
                .build();
            store.append(e).await.unwrap();
        }
        let e = EventBuilder::new(agg, "set_name", serde_json::json!({"name": "test"})).build();
        store.append(e).await.unwrap();
    }

    #[tokio::test]
    async fn test_replay_from_scratch() {
        let store = Arc::new(MemoryStore::new());
        let snaps = Arc::new(MemorySnapshotStore::new());
        let engine = ReplayEngine::new(store.clone(), snaps.clone());

        populate(&store, "c1").await;

        let result: ReplayResult<CounterState> = engine.replay_aggregate("c1").await.unwrap();
        assert_eq!(result.state.count, 5);
        assert_eq!(result.state.name, "test");
        assert_eq!(result.events_replayed, 6);
        assert!(!result.snapshot_used);
        assert_eq!(result.last_sequence, 6);
    }

    #[tokio::test]
    async fn test_replay_from_snapshot_accelerates() {
        let store = Arc::new(MemoryStore::new());
        let snaps = Arc::new(MemorySnapshotStore::new());
        let engine = ReplayEngine::new(store.clone(), snaps.clone());

        populate(&store, "c2").await;

        let mid: ReplayResult<CounterState> = engine.replay_to("c2", 3).await.unwrap();
        engine
            .save_snapshot("c2", "cell", &mid.state, 3, mid.vector_clock.clone())
            .await
            .unwrap();

        for _ in 0..5 {
            let e = EventBuilder::new("c2", "increment", serde_json::json!({"by": 2}))
                .build();
            store.append(e).await.unwrap();
        }

        let full: ReplayResult<CounterState> = engine.replay_aggregate("c2").await.unwrap();
        assert_eq!(full.state.count, 15);
        assert!(full.snapshot_used);
        assert!(full.events_replayed < full.total_events_for_aggregate);
    }

    #[tokio::test]
    async fn test_replay_to_point_in_time() {
        let store = Arc::new(MemoryStore::new());
        let snaps = Arc::new(MemorySnapshotStore::new());
        let engine = ReplayEngine::new(store.clone(), snaps.clone());

        populate(&store, "c3").await;

        let result: ReplayResult<CounterState> = engine.replay_to("c3", 3).await.unwrap();
        assert_eq!(result.state.count, 3);
        assert_eq!(result.state.name, "");
        assert_eq!(result.last_sequence, 3);
    }

    #[tokio::test]
    async fn test_snapshot_roundtrip() {
        let state = CounterState {
            count: 42,
            name: "alice".into(),
        };
        let snap_val = state.to_snapshot();
        let restored = CounterState::from_snapshot(&snap_val).unwrap();
        assert_eq!(restored, state);
    }
}
