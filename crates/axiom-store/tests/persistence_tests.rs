//! Store persistence tests: SQLite, FileStore, integration, recovery, performance.

use axiom_core::version::VersionInfo;
use axiom_core::Versioned;
use axiom_store::event::EventBuilder;
use axiom_store::file_store::{FileStore, FileStoreConfig};
use axiom_store::memory::MemoryStore;
use axiom_store::snapshot::{FileSnapshotStore, FileSnapshotStoreConfig, SnapshotStore};
use axiom_store::store::{verify_witness_chain, EventStore};
use axiom_store::StoreFactory;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_memory_store_roundtrip() {
    let store = MemoryStore::new();
    let event = EventBuilder::new("agg-1", "test", json!({"x": 1}))
        .cell_id("c1")
        .build();
    let seq = store.append(event.clone()).await.unwrap();
    assert_eq!(seq, 1);
    let events = store.read("agg-1").await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence_number, 1);
}

#[tokio::test]
async fn test_memory_store_batch_append() {
    let store = MemoryStore::new();
    let events: Vec<_> = (0..10)
        .map(|i| EventBuilder::new("batch", &format!("e{i}"), json!({"i": i})).build())
        .collect();
    let seqs = store.append_batch(events).await.unwrap();
    assert_eq!(seqs.len(), 10);
    assert_eq!(store.latest_sequence().await.unwrap(), 10);
}

#[tokio::test]
async fn test_memory_store_read_range() {
    let store = MemoryStore::new();
    for i in 0..20 {
        let e = EventBuilder::new("agg", "e", json!({"i": i})).build();
        store.append(e).await.unwrap();
    }
    let range = store.read_range("agg", 5, 15).await.unwrap();
    assert_eq!(range.len(), 11);
    assert_eq!(range.first().unwrap().sequence_number, 5);
    assert_eq!(range.last().unwrap().sequence_number, 15);
}

#[tokio::test]
async fn test_memory_store_subscribe() {
    let store = MemoryStore::new();
    let mut rx = store.subscribe();
    let e = EventBuilder::new("agg", "e", json!({})).build();
    store.append(e).await.unwrap();
    let received = rx.recv().await.unwrap();
    assert_eq!(received.aggregate_id, "agg");
}

#[tokio::test]
async fn test_file_store_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = FileStoreConfig {
        data_dir: dir.path().join("events"),
        max_file_size_bytes: 1024 * 1024,
        max_files: 5,
        index_file: dir.path().join("index.json"),
    };
    let store = FileStore::connect(cfg).await.unwrap();
    let event = EventBuilder::new("file-agg", "file-event", json!({"hello": "world"}))
        .cell_id("c1")
        .build();
    let seq = store.append(event.clone()).await.unwrap();
    assert_eq!(seq, 1);
    let events = store.read("file-agg").await.unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn test_file_store_batch_and_latest_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = FileStoreConfig {
        data_dir: dir.path().join("events"),
        max_file_size_bytes: 1024 * 1024,
        max_files: 5,
        index_file: dir.path().join("index.json"),
    };
    let store = FileStore::connect(cfg).await.unwrap();
    let events: Vec<_> = (0..5)
        .map(|i| EventBuilder::new("batch", &format!("e{i}"), json!({"i": i})).build())
        .collect();
    let seqs = store.append_batch(events).await.unwrap();
    assert_eq!(seqs.len(), 5);
    assert_eq!(store.latest_sequence().await.unwrap(), 5);
}

#[tokio::test]
async fn test_file_store_rolls_when_full() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = FileStoreConfig {
        data_dir: dir.path().join("events"),
        max_file_size_bytes: 128,
        max_files: 3,
        index_file: dir.path().join("index.json"),
    };
    let store = FileStore::connect(cfg).await.unwrap();
    for i in 0..20 {
        let e = EventBuilder::new("roll", &format!("e{i}"), json!({"i": i})).build();
        store.append(e).await.unwrap();
    }
    let events = store.read("roll").await.unwrap();
    assert_eq!(events.len(), 20);
}

#[tokio::test]
async fn test_snapshot_file_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = FileSnapshotStoreConfig {
        snapshot_dir: dir.path().to_path_buf(),
        max_snapshots_per_aggregate: 3,
    };
    let store = FileSnapshotStore::connect(cfg).unwrap();
    let snap = axiom_store::Snapshot {
        aggregate_id: "snap-agg".into(),
        sequence_number: 7,
        state: json!({"counter": 7}),
        schema_version: 1,
        created_at_ns: 7,
        cell_id: "c1".into(),
        vector_clock: Default::default(),
    };
    store.save_snapshot(snap.clone()).await.unwrap();
    let loaded = store.load_latest_snapshot("snap-agg").await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().sequence_number, 7);
}

#[tokio::test]
async fn test_snapshot_retention_enforced() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = FileSnapshotStoreConfig {
        snapshot_dir: dir.path().to_path_buf(),
        max_snapshots_per_aggregate: 2,
    };
    let store = FileSnapshotStore::connect(cfg).unwrap();
    for i in 1..=4 {
        let snap = axiom_store::Snapshot {
            aggregate_id: "ret".into(),
            sequence_number: i,
            state: json!({"i": i}),
            schema_version: 1,
            created_at_ns: i,
            cell_id: "c1".into(),
            vector_clock: Default::default(),
        };
        store.save_snapshot(snap).await.unwrap();
    }
    let count = std::fs::read_dir(dir.path())
        .unwrap()
        .filter(|e| e.as_ref().unwrap().path().extension().map(|x| x == "snap").unwrap_or(false))
        .count();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_verify_witness_chain_accepts_valid_chain() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = FileStoreConfig {
        data_dir: dir.path().join("events"),
        max_file_size_bytes: 1024 * 1024,
        max_files: 5,
        index_file: dir.path().join("index.json"),
    };
    let store = Arc::new(FileStore::connect(cfg).await.unwrap());
    let witness_a = axiom_core::witness::Witness {
        witness_id: axiom_core::id::WitnessId::new("w1"),
        schema_version: axiom_core::version::WitnessSchema::schema_version(),
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
        summary: "first".into(),
        outcome: axiom_core::witness::TransitionOutcome::Success,
        metrics: Default::default(),
        version_info: VersionInfo::current(),
        signal_fingerprint: [0; 32],
        payload_size_bytes: 0,
        kind: axiom_core::witness::WitnessKind::StateTransition,
    };
    let mut witness_b = witness_a.clone();
    witness_b.witness_id = axiom_core::id::WitnessId::new("w2");
    witness_b.timestamp_ns = 2;
    witness_b.prev_hash = Some(witness_a.hash.clone());
    witness_b.hash = axiom_core::witness::WitnessHash([2; 32]);

    let payload_a = json!(witness_a.clone());
    let payload_b = json!(witness_b.clone());
    let event_a = EventBuilder::new("c1", "witness", payload_a)
        .cell_id("c1")
        .build();
    let event_b = EventBuilder::new("c1", "witness", payload_b)
        .cell_id("c1")
        .build();
    store.append_batch(vec![event_a, event_b]).await.unwrap();

    let all = store.read_all().await.unwrap();
    assert!(verify_witness_chain(&all).is_ok());
}

#[tokio::test]
async fn test_store_factory_memory_default() {
    let factory = StoreFactory::from_config(axiom_store::StoreConfig::Memory)
        .await
        .unwrap();
    let store = factory.event_store();
    let event = EventBuilder::new("agg", "e", json!({})).build();
    let seq = store.append(event).await.unwrap();
    assert_eq!(seq, 1);
}

#[tokio::test]
async fn test_store_factory_file() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = axiom_store::StoreConfig::File(FileStoreConfig {
        data_dir: dir.path().join("events"),
        max_file_size_bytes: 1024 * 1024,
        max_files: 5,
        index_file: dir.path().join("index.json"),
    });
    let factory = StoreFactory::from_config(cfg).await.unwrap();
    let store = factory.event_store();
    let event = EventBuilder::new("agg", "e", json!({})).build();
    let seq = store.append(event).await.unwrap();
    assert_eq!(seq, 1);
}

#[tokio::test]
async fn test_performance_append_throughput() {
    let store = MemoryStore::new();
    let events: Vec<_> = (0..1000)
        .map(|i| EventBuilder::new("perf", &format!("e{i}"), json!({"i": i})).build())
        .collect();
    let start = Instant::now();
    let seqs = store.append_batch(events).await.unwrap();
    let elapsed = start.elapsed();
    assert_eq!(seqs.len(), 1000);
    assert!(elapsed < Duration::from_millis(500), "append batch took {:?}", elapsed);
}

#[tokio::test]
async fn test_performance_read_latency() {
    let store = MemoryStore::new();
    for i in 0..500 {
        let e = EventBuilder::new("perf-read", &format!("e{i}"), json!({"i": i})).build();
        store.append(e).await.unwrap();
    }
    let start = Instant::now();
    let events = store.read("perf-read").await.unwrap();
    let elapsed = start.elapsed();
    assert_eq!(events.len(), 500);
    assert!(elapsed < Duration::from_millis(100), "read took {:?}", elapsed);
}
