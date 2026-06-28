//! Snapshot mechanism for accelerating replay.

use crate::store::StoreError;
use async_trait::async_trait;
use axiom_core::signal::VectorClock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub aggregate_id: String,
    pub sequence_number: u64,
    pub state: serde_json::Value,
    pub schema_version: u32,
    pub created_at_ns: u64,
    pub cell_id: String,
    pub vector_clock: VectorClock,
}

#[async_trait]
pub trait SnapshotStore: Send + Sync {
    async fn save_snapshot(&self, snapshot: Snapshot) -> Result<(), StoreError>;
    async fn load_latest_snapshot(&self, aggregate_id: &str) -> Result<Option<Snapshot>, StoreError>;
    async fn load_snapshot_at(
        &self,
        aggregate_id: &str,
        seq: u64,
    ) -> Result<Option<Snapshot>, StoreError>;
}

#[derive(Debug, Clone)]
pub struct SnapshotPolicy {
    pub every_n_events: u64,
    pub max_snapshots_per_aggregate: usize,
}

impl Default for SnapshotPolicy {
    fn default() -> Self {
        Self {
            every_n_events: 100,
            max_snapshots_per_aggregate: 5,
        }
    }
}

pub struct MemorySnapshotStore {
    snapshots: Arc<RwLock<HashMap<String, Vec<Snapshot>>>>,
    policy: SnapshotPolicy,
}

impl MemorySnapshotStore {
    pub fn new() -> Self {
        Self::with_policy(SnapshotPolicy::default())
    }

    pub fn with_policy(policy: SnapshotPolicy) -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            policy,
        }
    }

    pub fn policy(&self) -> &SnapshotPolicy {
        &self.policy
    }

    pub fn should_snapshot(&self, events_since_last_snapshot: u64) -> bool {
        events_since_last_snapshot >= self.policy.every_n_events
    }
}

impl Default for MemorySnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SnapshotStore for MemorySnapshotStore {
    async fn save_snapshot(&self, snapshot: Snapshot) -> Result<(), StoreError> {
        let mut map = self.snapshots.write().await;
        let list = map
            .entry(snapshot.aggregate_id.clone())
            .or_insert_with(Vec::new);
        list.push(snapshot);
        list.sort_by_key(|s| s.sequence_number);

        if list.len() > self.policy.max_snapshots_per_aggregate {
            let excess = list.len() - self.policy.max_snapshots_per_aggregate;
            list.drain(0..excess);
        }
        Ok(())
    }

    async fn load_latest_snapshot(&self, aggregate_id: &str) -> Result<Option<Snapshot>, StoreError> {
        let map = self.snapshots.read().await;
        Ok(map
            .get(aggregate_id)
            .and_then(|list| list.last().cloned()))
    }

    async fn load_snapshot_at(
        &self,
        aggregate_id: &str,
        seq: u64,
    ) -> Result<Option<Snapshot>, StoreError> {
        let map = self.snapshots.read().await;
        Ok(map.get(aggregate_id).and_then(|list| {
            list.iter()
                .rev()
                .find(|s| s.sequence_number <= seq)
                .cloned()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(agg: &str, seq: u64) -> Snapshot {
        Snapshot {
            aggregate_id: agg.to_string(),
            sequence_number: seq,
            state: serde_json::json!({"seq": seq}),
            schema_version: 1,
            created_at_ns: seq * 1000,
            cell_id: "c1".to_string(),
            vector_clock: VectorClock::new(),
        }
    }

    #[tokio::test]
    async fn test_save_and_load_roundtrip() {
        let store = MemorySnapshotStore::new();
        store.save_snapshot(make_snapshot("a", 10)).await.unwrap();
        let snap = store.load_latest_snapshot("a").await.unwrap();
        assert!(snap.is_some());
        assert_eq!(snap.unwrap().sequence_number, 10);
    }

    #[tokio::test]
    async fn test_load_latest_returns_highest_seq() {
        let store = MemorySnapshotStore::new();
        store.save_snapshot(make_snapshot("a", 5)).await.unwrap();
        store.save_snapshot(make_snapshot("a", 10)).await.unwrap();
        store.save_snapshot(make_snapshot("a", 15)).await.unwrap();
        let snap = store.load_latest_snapshot("a").await.unwrap().unwrap();
        assert_eq!(snap.sequence_number, 15);
    }

    #[tokio::test]
    async fn test_load_snapshot_at() {
        let store = MemorySnapshotStore::new();
        store.save_snapshot(make_snapshot("a", 5)).await.unwrap();
        store.save_snapshot(make_snapshot("a", 10)).await.unwrap();
        store.save_snapshot(make_snapshot("a", 20)).await.unwrap();

        let snap = store.load_snapshot_at("a", 12).await.unwrap().unwrap();
        assert_eq!(snap.sequence_number, 10);

        let snap = store.load_snapshot_at("a", 20).await.unwrap().unwrap();
        assert_eq!(snap.sequence_number, 20);

        let snap = store.load_snapshot_at("a", 3).await.unwrap();
        assert!(snap.is_none());
    }

    #[tokio::test]
    async fn test_snapshot_policy_enforces_max_retention() {
        let policy = SnapshotPolicy {
            every_n_events: 10,
            max_snapshots_per_aggregate: 2,
        };
        let store = MemorySnapshotStore::with_policy(policy);
        store.save_snapshot(make_snapshot("a", 1)).await.unwrap();
        store.save_snapshot(make_snapshot("a", 2)).await.unwrap();
        store.save_snapshot(make_snapshot("a", 3)).await.unwrap();

        let map = store.snapshots.read().await;
        let list = map.get("a").unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].sequence_number, 2);
        assert_eq!(list[1].sequence_number, 3);
    }

    #[tokio::test]
    async fn test_snapshot_policy_should_snapshot() {
        let policy = SnapshotPolicy {
            every_n_events: 10,
            max_snapshots_per_aggregate: 5,
        };
        let store = MemorySnapshotStore::with_policy(policy);
        assert!(!store.should_snapshot(9));
        assert!(store.should_snapshot(10));
        assert!(store.should_snapshot(11));
    }
}
