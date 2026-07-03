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
    pub fn new(event_store: Arc<dyn EventStore>, snapshot_store: Arc<dyn SnapshotStore>) -> Self {
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

    pub async fn replay_by_cell<S: ReplayableState>(
        &self,
        cell_id: &str,
    ) -> Result<ReplayResult<S>, StoreError> {
        let all_events = self.event_store.read_by_cell_id(cell_id).await?;
        let latest_seq = all_events
            .iter()
            .map(|e| e.sequence_number)
            .max()
            .unwrap_or(0);
        self.replay_with_events::<S>(&all_events, latest_seq).await
    }

    pub async fn replay_by_time_range<S: ReplayableState>(
        &self,
        start_ns: u64,
        end_ns: u64,
    ) -> Result<ReplayResult<S>, StoreError> {
        let all_events = self.event_store.read_by_time_range(start_ns, end_ns).await?;
        let latest_seq = all_events
            .iter()
            .map(|e| e.sequence_number)
            .max()
            .unwrap_or(0);
        self.replay_with_events::<S>(&all_events, latest_seq).await
    }

    pub async fn replay_at_timestamp<S: ReplayableState>(
        &self,
        aggregate_id: &str,
        target_timestamp_ns: u64,
    ) -> Result<ReplayResult<S>, StoreError> {
        let all_events = self.event_store.read(aggregate_id).await?;
        let up_to_seq = all_events
            .iter()
            .filter(|e| e.timestamp_ns <= target_timestamp_ns)
            .map(|e| e.sequence_number)
            .max()
            .unwrap_or(0);
        self.replay_with_events::<S>(&all_events, up_to_seq).await
    }

    pub async fn replay_at_sequence<S: ReplayableState>(
        &self,
        aggregate_id: &str,
        up_to_seq: u64,
    ) -> Result<ReplayResult<S>, StoreError> {
        let all_events = self.event_store.read(aggregate_id).await?;
        self.replay_with_events::<S>(&all_events, up_to_seq).await
    }

    pub async fn diff_between<S: ReplayableState + Clone>(
        &self,
        aggregate_id: &str,
        from_seq: u64,
        to_seq: u64,
    ) -> Result<StateDiff<S>, StoreError> {
        let from = self.replay_to::<S>(aggregate_id, from_seq).await?;
        let to = self.replay_to::<S>(aggregate_id, to_seq).await?;
        Ok(StateDiff::compute(from, to))
    }

    pub async fn replay_by_correlation<S: ReplayableState>(
        &self,
        correlation_id: &str,
    ) -> Result<ReplayResult<S>, StoreError> {
        let all_events = self.event_store.read_by_correlation(correlation_id).await?;
        let latest_seq = all_events
            .iter()
            .map(|e| e.sequence_number)
            .max()
            .unwrap_or(0);
        self.replay_with_events::<S>(&all_events, latest_seq).await
    }

    async fn replay_with_events<S: ReplayableState>(
        &self,
        events: &[Event],
        up_to_seq: u64,
    ) -> Result<ReplayResult<S>, StoreError> {
        let start_ns = axiom_core::signal::now_ns();

        let snapshot = self
            .snapshot_store
            .load_snapshot_at("", up_to_seq)
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

        let applicable: Vec<&Event> = events
            .iter()
            .filter(|e| e.sequence_number > start_seq && e.sequence_number <= up_to_seq)
            .collect();
        let total_events = events.len() as u64;
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupValidation {
    pub validated_types: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff<S> {
    pub from_state: S,
    pub to_state: S,
    pub from_sequence: u64,
    pub to_sequence: u64,
    pub from_timestamp_ns: u64,
    pub to_timestamp_ns: u64,
    pub changed_fields: Vec<String>,
}

impl<S: ReplayableState + Clone> StateDiff<S> {
    pub fn compute(from: ReplayResult<S>, to: ReplayResult<S>) -> Self {
        let mut changed_fields = Vec::new();
        if from.state.to_snapshot() != to.state.to_snapshot() {
            changed_fields.push("state changed".to_string());
        }

        Self {
            from_state: from.state,
            to_state: to.state,
            from_sequence: from.last_sequence,
            to_sequence: to.last_sequence,
            from_timestamp_ns: 0,
            to_timestamp_ns: 0,
            changed_fields,
        }
    }
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
            if let Ok(event) = serde_json::from_value::<axiom_core::WitnessEvent>(
                serde_json::to_value(&w.summary).unwrap_or_default(),
            ) {
                if let Err(err) = state.apply_event("witness", &serde_json::to_value(&w).unwrap_or_default()) {
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
            if let Err(err) = state.apply_event("witness", &serde_json::to_value(&w).unwrap_or_default()) {
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
            let e = EventBuilder::new(agg, "increment", serde_json::json!({"by": 1})).build();
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
            let e = EventBuilder::new("c2", "increment", serde_json::json!({"by": 2})).build();
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

    #[tokio::test]
    async fn test_replay_at_timestamp() {
        let store = Arc::new(MemoryStore::new());
        let snaps = Arc::new(MemorySnapshotStore::new());
        let engine = ReplayEngine::new(store.clone(), snaps.clone());

        for i in 1..=5 {
            let e = EventBuilder::new("ts-agg", "increment", serde_json::json!({"by": 1}))
                .timestamp_ns(i * 1000)
                .build();
            store.append(e).await.unwrap();
        }

        let result: ReplayResult<CounterState> = engine
            .replay_at_timestamp("ts-agg", 3000)
            .await
            .unwrap();
        assert_eq!(result.state.count, 3);
    }

    #[tokio::test]
    async fn test_replay_at_sequence() {
        let store = Arc::new(MemoryStore::new());
        let snaps = Arc::new(MemorySnapshotStore::new());
        let engine = ReplayEngine::new(store.clone(), snaps.clone());

        for i in 1..=5 {
            let e = EventBuilder::new("seq-agg", "increment", serde_json::json!({"by": 1})).build();
            store.append(e).await.unwrap();
        }

        let result: ReplayResult<CounterState> = engine.replay_at_sequence("seq-agg", 3).await.unwrap();
        assert_eq!(result.state.count, 3);
    }

    #[tokio::test]
    async fn test_diff_between_sequences() {
        let store = Arc::new(MemoryStore::new());
        let snaps = Arc::new(MemorySnapshotStore::new());
        let engine = ReplayEngine::new(store.clone(), snaps.clone());

        for i in 1..=5 {
            let e = EventBuilder::new("diff-agg", "increment", serde_json::json!({"by": 1})).build();
            store.append(e).await.unwrap();
        }

        let diff = engine.diff_between::<CounterState>("diff-agg", 2, 5).await.unwrap();
        assert_eq!(diff.from_state.count, 2);
        assert_eq!(diff.to_state.count, 5);
        assert_eq!(diff.from_sequence, 2);
        assert_eq!(diff.to_sequence, 5);
    }

    #[tokio::test]
    async fn test_witness_replay_from_events() {
        use axiom_core::{Witness, WitnessEvent};

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
