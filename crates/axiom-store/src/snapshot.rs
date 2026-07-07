//! Snapshot mechanism for accelerating replay.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::store::{BoxFuture, StoreError};
use axiom_kernel::signal::VectorClock;
use serde::{Deserialize, Serialize};
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

pub trait SnapshotStore: Send + Sync {
    fn save_snapshot<'a>(&'a self, snapshot: Snapshot) -> BoxFuture<'a, Result<(), StoreError>>;
    fn load_latest_snapshot<'a>(
        &'a self,
        aggregate_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<Snapshot>, StoreError>>;
    fn load_snapshot_at<'a>(
        &'a self,
        aggregate_id: &'a str,
        seq: u64,
    ) -> BoxFuture<'a, Result<Option<Snapshot>, StoreError>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "kebab-case")]
pub enum SnapshotPolicy {
    Never,
    EveryN { n: u64 },
    EveryDuration { duration_ms: u64 },
    OnStateSize { bytes: usize },
}

impl Default for SnapshotPolicy {
    fn default() -> Self {
        SnapshotPolicy::EveryN { n: 100 }
    }
}

impl SnapshotPolicy {
    pub fn should_snapshot(&self, events_since: u64, state_size_bytes: usize) -> bool {
        match self {
            SnapshotPolicy::Never => false,
            SnapshotPolicy::EveryN { n } => events_since >= *n,
            SnapshotPolicy::EveryDuration { .. } => false,
            SnapshotPolicy::OnStateSize { bytes } => state_size_bytes >= *bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRetention {
    pub max_snapshots_per_aggregate: usize,
}

impl Default for SnapshotRetention {
    fn default() -> Self {
        Self {
            max_snapshots_per_aggregate: 5,
        }
    }
}

pub struct MemorySnapshotStore {
    snapshots: Arc<RwLock<HashMap<String, Vec<Snapshot>>>>,
    policy: SnapshotPolicy,
    retention: SnapshotRetention,
}

impl MemorySnapshotStore {
    pub fn new() -> Self {
        Self::with_policy(SnapshotPolicy::default())
    }

    pub fn with_policy(policy: SnapshotPolicy) -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            policy,
            retention: SnapshotRetention::default(),
        }
    }

    pub fn with_retention(mut self, retention: SnapshotRetention) -> Self {
        self.retention = retention;
        self
    }

    pub fn policy(&self) -> &SnapshotPolicy {
        &self.policy
    }

    pub fn should_snapshot(
        &self,
        events_since_last_snapshot: u64,
        state_size_bytes: usize,
    ) -> bool {
        self.policy
            .should_snapshot(events_since_last_snapshot, state_size_bytes)
    }
}

impl Default for MemorySnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotStore for MemorySnapshotStore {
    fn save_snapshot<'a>(&'a self, snapshot: Snapshot) -> BoxFuture<'a, Result<(), StoreError>> {
        Box::pin(async move {
            let mut map = self.snapshots.write().await;
            let list = map
                .entry(snapshot.aggregate_id.clone())
                .or_insert_with(Vec::new);
            list.push(snapshot);
            list.sort_by_key(|s| s.sequence_number);

            while list.len() > self.retention.max_snapshots_per_aggregate {
                list.remove(0);
            }
            Ok(())
        })
    }

    fn load_latest_snapshot<'a>(
        &'a self,
        aggregate_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<Snapshot>, StoreError>> {
        Box::pin(async move {
            let map = self.snapshots.read().await;
            Ok(map.get(aggregate_id).and_then(|list| list.last().cloned()))
        })
    }

    fn load_snapshot_at<'a>(
        &'a self,
        aggregate_id: &'a str,
        seq: u64,
    ) -> BoxFuture<'a, Result<Option<Snapshot>, StoreError>> {
        Box::pin(async move {
            let map = self.snapshots.read().await;
            Ok(map.get(aggregate_id).and_then(|list| {
                list.iter()
                    .rev()
                    .find(|s| s.sequence_number <= seq)
                    .cloned()
            }))
        })
    }
}

#[derive(Debug, Clone)]
pub struct FileSnapshotStoreConfig {
    pub snapshot_dir: PathBuf,
    pub max_snapshots_per_aggregate: usize,
}

impl Default for FileSnapshotStoreConfig {
    fn default() -> Self {
        Self {
            snapshot_dir: PathBuf::from("snapshots"),
            max_snapshots_per_aggregate: 5,
        }
    }
}

pub struct FileSnapshotStore {
    config: FileSnapshotStoreConfig,
}

impl FileSnapshotStore {
    pub fn connect(config: FileSnapshotStoreConfig) -> Result<Self, StoreError> {
        fs::create_dir_all(&config.snapshot_dir)
            .map_err(|e| StoreError::Storage(format!("create snapshot dir: {e}")))?;
        Ok(Self { config })
    }

    fn snapshot_path(&self, aggregate_id: &str, seq: u64) -> PathBuf {
        self.config
            .snapshot_dir
            .join(format!("{}-{}.snap", aggregate_id, seq))
    }

    fn enforce_retention(&self, aggregate_id: &str) -> Result<(), StoreError> {
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.config.snapshot_dir)
            .map_err(|e| StoreError::Storage(format!("read snapshot dir: {e}")))?
        {
            let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {e}")))?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(aggregate_id) && name.ends_with(".snap") {
                    files.push(path);
                }
            }
        }
        files.sort();
        while files.len() > self.config.max_snapshots_per_aggregate {
            let oldest = files.remove(0);
            fs::remove_file(&oldest)
                .map_err(|e| StoreError::Storage(format!("remove old snapshot: {e}")))?;
        }
        Ok(())
    }
}

impl SnapshotStore for FileSnapshotStore {
    fn save_snapshot<'a>(&'a self, snapshot: Snapshot) -> BoxFuture<'a, Result<(), StoreError>> {
        Box::pin(async move {
            let path = self.snapshot_path(&snapshot.aggregate_id, snapshot.sequence_number);
            let data = serde_json::to_vec(&snapshot)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;
            let mut compressed = Vec::new();
            {
                let mut encoder = snap::write::FrameEncoder::new(&mut compressed);
                std::io::Write::write_all(&mut encoder, &data)
                    .map_err(|e| StoreError::Storage(format!("snapshot compression: {e}")))?;
            }
            fs::write(&path, compressed)
                .map_err(|e| StoreError::Storage(format!("write snapshot: {e}")))?;
            self.enforce_retention(&snapshot.aggregate_id)?;
            Ok(())
        })
    }

    fn load_latest_snapshot<'a>(
        &'a self,
        aggregate_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<Snapshot>, StoreError>> {
        Box::pin(async move {
            let mut files = Vec::new();
            for entry in fs::read_dir(&self.config.snapshot_dir)
                .map_err(|e| StoreError::Storage(format!("read snapshot dir: {e}")))?
            {
                let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {e}")))?;
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(aggregate_id) && name.ends_with(".snap") {
                        files.push(path);
                    }
                }
            }
            files.sort();
            let latest = files.last();
            if let Some(path) = latest {
                let data = fs::read(path)
                    .map_err(|e| StoreError::Storage(format!("read snapshot: {e}")))?;
                let mut decoder = snap::read::FrameDecoder::new(&data[..]);
                let mut decompressed = Vec::new();
                std::io::Read::read_to_end(&mut decoder, &mut decompressed)
                    .map_err(|e| StoreError::Storage(format!("snapshot decompression: {e}")))?;
                let snap = serde_json::from_slice(&decompressed)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                Ok(Some(snap))
            } else {
                Ok(None)
            }
        })
    }

    fn load_snapshot_at<'a>(
        &'a self,
        aggregate_id: &'a str,
        seq: u64,
    ) -> BoxFuture<'a, Result<Option<Snapshot>, StoreError>> {
        Box::pin(async move {
            let path = self.snapshot_path(aggregate_id, seq);
            if !path.exists() {
                return Ok(None);
            }
            let data =
                fs::read(&path).map_err(|e| StoreError::Storage(format!("read snapshot: {e}")))?;
            let mut decoder = snap::read::FrameDecoder::new(&data[..]);
            let mut decompressed = Vec::new();
            std::io::Read::read_to_end(&mut decoder, &mut decompressed)
                .map_err(|e| StoreError::Storage(format!("snapshot decompression: {e}")))?;
            let snap = serde_json::from_slice(&decompressed)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;
            Ok(Some(snap))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
    async fn test_file_snapshot_roundtrip() {
        let dir = tempdir().unwrap();
        let cfg = FileSnapshotStoreConfig {
            snapshot_dir: dir.path().to_path_buf(),
            max_snapshots_per_aggregate: 5,
        };
        let store = FileSnapshotStore::connect(cfg).unwrap();
        store.save_snapshot(make_snapshot("a", 10)).await.unwrap();
        let snap = store.load_latest_snapshot("a").await.unwrap();
        assert!(snap.is_some());
        assert_eq!(snap.unwrap().sequence_number, 10);
    }

    #[tokio::test]
    async fn test_file_snapshot_load_at() {
        let dir = tempdir().unwrap();
        let cfg = FileSnapshotStoreConfig {
            snapshot_dir: dir.path().to_path_buf(),
            max_snapshots_per_aggregate: 5,
        };
        let store = FileSnapshotStore::connect(cfg).unwrap();
        store.save_snapshot(make_snapshot("a", 5)).await.unwrap();
        store.save_snapshot(make_snapshot("a", 10)).await.unwrap();

        let snap = store.load_snapshot_at("a", 10).await.unwrap();
        assert!(snap.is_some());
        assert_eq!(snap.unwrap().sequence_number, 10);

        let snap = store.load_snapshot_at("a", 7).await.unwrap();
        assert!(snap.is_none());
    }

    #[tokio::test]
    async fn test_file_snapshot_retention() {
        let dir = tempdir().unwrap();
        let cfg = FileSnapshotStoreConfig {
            snapshot_dir: dir.path().to_path_buf(),
            max_snapshots_per_aggregate: 2,
        };
        let store = FileSnapshotStore::connect(cfg).unwrap();
        store.save_snapshot(make_snapshot("a", 1)).await.unwrap();
        store.save_snapshot(make_snapshot("a", 2)).await.unwrap();
        store.save_snapshot(make_snapshot("a", 3)).await.unwrap();

        let mut count = 0;
        for entry in fs::read_dir(dir.path()).unwrap() {
            if entry
                .unwrap()
                .path()
                .extension()
                .map(|e| e == "snap")
                .unwrap_or(false)
            {
                count += 1;
            }
        }
        assert_eq!(count, 2);
    }
}
