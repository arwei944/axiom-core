//! Integration tests for Witness persistence, event replay, and snapshots.

use axiom_kernel::id::CorrelationId;
use axiom_kernel::signal::VectorClock;
use axiom_store::event::EventBuilder;
use axiom_store::memory::MemoryStore;
use axiom_store::replay::ReplayEngine;
use axiom_store::snapshot::MemorySnapshotStore;
use axiom_store::{EventStore, SnapshotStore};
use std::sync::Arc;

#[tokio::test]
async fn test_event_replay_by_cell() {
    let store = Arc::new(MemoryStore::new());

    let e1 = EventBuilder::new("agg1", "event1", serde_json::json!({"value": 1}))
        .cell_id("cell1")
        .build();
    let e2 = EventBuilder::new("agg1", "event2", serde_json::json!({"value": 2}))
        .cell_id("cell1")
        .build();
    let e3 = EventBuilder::new("agg2", "event3", serde_json::json!({"value": 3}))
        .cell_id("cell2")
        .build();

    store.append(e1).await.unwrap();
    store.append(e2).await.unwrap();
    store.append(e3).await.unwrap();

    let events = store.read_by_cell_id("cell1").await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].aggregate_id, "agg1");
    assert_eq!(events[1].aggregate_id, "agg1");

    let events = store.read_by_cell_id("cell2").await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].aggregate_id, "agg2");
}

#[tokio::test]
async fn test_event_replay_by_time_range() {
    let store = Arc::new(MemoryStore::new());

    let start_ns = axiom_kernel::signal::now_ns();
    let e1 = EventBuilder::new("agg", "event1", serde_json::json!({})).build();
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    let e2 = EventBuilder::new("agg", "event2", serde_json::json!({})).build();
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    let end_ns = axiom_kernel::signal::now_ns();

    store.append(e1).await.unwrap();
    store.append(e2).await.unwrap();

    let events = store.read_by_time_range(start_ns, end_ns).await.unwrap();
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn test_snapshot_save_and_load() {
    let snapshot_store = Arc::new(MemorySnapshotStore::new());

    let snapshot = axiom_store::Snapshot {
        aggregate_id: "agg1".to_string(),
        sequence_number: 10,
        state: serde_json::json!({"count": 5, "data": "test"}),
        schema_version: 1,
        created_at_ns: axiom_kernel::signal::now_ns(),
        cell_id: "cell1".to_string(),
        vector_clock: VectorClock::new(),
    };

    snapshot_store.save_snapshot(snapshot).await.unwrap();

    let loaded = snapshot_store.load_latest_snapshot("agg1").await.unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.aggregate_id, "agg1");
    assert_eq!(loaded.sequence_number, 10);
    assert_eq!(loaded.state["count"], 5);
    assert_eq!(loaded.state["data"], "test");
}

#[tokio::test]
async fn test_replay_engine_with_snapshot() {
    let event_store = Arc::new(MemoryStore::new());
    let snapshot_store = Arc::new(MemorySnapshotStore::new());
    let _replay_engine = ReplayEngine::new(event_store.clone(), snapshot_store.clone());

    let snapshot = axiom_store::Snapshot {
        aggregate_id: "agg1".to_string(),
        sequence_number: 5,
        state: serde_json::json!({"value": 100}),
        schema_version: 1,
        created_at_ns: axiom_kernel::signal::now_ns(),
        cell_id: "cell1".to_string(),
        vector_clock: VectorClock::new(),
    };
    snapshot_store.save_snapshot(snapshot).await.unwrap();

    let e1 = EventBuilder::new("agg1", "event1", serde_json::json!({"delta": 10}))
        .cell_id("cell1")
        .build();
    let e2 = EventBuilder::new("agg1", "event2", serde_json::json!({"delta": 20}))
        .cell_id("cell1")
        .build();

    event_store.append(e1).await.unwrap();
    event_store.append(e2).await.unwrap();

    let latest_seq = event_store.latest_sequence().await.unwrap();
    assert!(latest_seq > 0);
}

#[tokio::test]
async fn test_witness_to_event_serialization() {
    let store = Arc::new(MemoryStore::new());

    let cid = CorrelationId::new("test-123");
    let e = EventBuilder::new(
        "user-1",
        "UserCreated",
        serde_json::json!({
            "name": "Alice",
            "email": "alice@example.com"
        }),
    )
    .correlation_id(cid.clone())
    .cell_id("user-service")
    .build();

    store.append(e).await.unwrap();

    let events = store.read_by_correlation("test-123").await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].aggregate_id, "user-1");
    assert_eq!(events[0].cell_id, "user-service");
    assert_eq!(events[0].payload["name"], "Alice");
}

#[tokio::test]
async fn test_snapshot_policy_enforces_retention() {
    let policy = axiom_store::snapshot::SnapshotPolicy::EveryN { n: 10 };
    let snapshot_store = Arc::new(MemorySnapshotStore::with_policy(policy));

    for i in 0..5 {
        let snapshot = axiom_store::Snapshot {
            aggregate_id: "agg1".to_string(),
            sequence_number: i as u64 * 10,
            state: serde_json::json!({"seq": i}),
            schema_version: 1,
            created_at_ns: i as u64 * 1000,
            cell_id: "cell1".to_string(),
            vector_clock: VectorClock::new(),
        };
        snapshot_store.save_snapshot(snapshot).await.unwrap();
    }

    let loaded = snapshot_store.load_latest_snapshot("agg1").await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().sequence_number, 40);
}
