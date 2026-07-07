//! Recovery workflow validation for SQLite + snapshots.

use axiom_kernel::clock::global_clock;
use axiom_kernel::signal::VectorClock;
use axiom_store::event::EventBuilder;
use axiom_store::snapshot::MemorySnapshotStore;
use axiom_store::snapshot::SnapshotStore;
use axiom_store::sqlite::SqliteStore;
use axiom_store::store::EventStore;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Simulate recovery workflow:
/// 1. Write events to SQLite
/// 2. Save snapshot
/// 3. Verify snapshot can be loaded
#[tokio::test]
async fn test_snapshot_recovery_workflow() {
    let db_path = format!("file:test_recovery_{}.db?mode=memory", uuid::Uuid::new_v4());
    let pool = sqlx::sqlite::SqlitePool::connect(&db_path).await.expect("sqlite pool");
    let store = SqliteStore::connect_with_pool(pool).await.expect("sqlite store");

    // Write events
    for i in 0..5 {
        let e =
            EventBuilder::new("agg1", format!("evt-{}", i).as_str(), serde_json::json!({"i": i}))
                .cell_id("cell1")
                .build();
        store.append(e).await.unwrap();
    }

    let latest_seq = store.latest_sequence().await.unwrap();
    assert_eq!(latest_seq, 5);

    // Save snapshot
    let snapshot_store = Arc::new(RwLock::new(MemorySnapshotStore::new()));
    let snapshot = axiom_store::Snapshot {
        aggregate_id: "agg1".to_string(),
        sequence_number: latest_seq,
        state: serde_json::json!({"recovered": true}),
        schema_version: 1,
        created_at_ns: global_clock().now_ns(),
        cell_id: "cell1".to_string(),
        vector_clock: VectorClock::new(),
    };
    snapshot_store.write().await.save_snapshot(snapshot).await.unwrap();

    // Verify snapshot recovery
    let loaded = snapshot_store.read().await.load_latest_snapshot("agg1").await.unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.sequence_number, 5);
    assert_eq!(loaded.state["recovered"], true);
}
