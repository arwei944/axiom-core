//! Append-only file-based event store implementation.

use crate::event::Event;
use crate::store::{BoxFuture, EventReceiver, EventStore, StoreError};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct FileStoreConfig {
    pub data_dir: PathBuf,
    pub max_file_size_bytes: u64,
    pub max_files: usize,
    pub index_file: PathBuf,
}

impl Default for FileStoreConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("events"),
            max_file_size_bytes: 50 * 1024 * 1024,
            max_files: 20,
            index_file: PathBuf::from("events/event_index.json"),
        }
    }
}

pub struct FileStore {
    config: FileStoreConfig,
    current_file: Arc<parking_lot::Mutex<Option<File>>>,
    current_path: Arc<parking_lot::Mutex<PathBuf>>,
    current_size: Arc<parking_lot::RwLock<u64>>,
    sequence: Arc<parking_lot::RwLock<u64>>,
    event_index: Arc<parking_lot::RwLock<HashMap<String, usize>>>,
    sender: broadcast::Sender<Arc<Event>>,
}

impl FileStore {
    pub async fn connect(config: FileStoreConfig) -> Result<Self, StoreError> {
        fs::create_dir_all(&config.data_dir)
            .map_err(|e| StoreError::Storage(format!("create data dir: {e}")))?;

        let store = Self {
            config: config.clone(),
            current_file: Arc::new(parking_lot::Mutex::new(None)),
            current_path: Arc::new(parking_lot::Mutex::new(PathBuf::new())),
            current_size: Arc::new(parking_lot::RwLock::new(0)),
            sequence: Arc::new(parking_lot::RwLock::new(0)),
            event_index: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            sender: broadcast::channel(1024).0,
        };

        store.recover_index().await?;
        store.roll_if_needed().await?;
        Ok(store)
    }

    async fn recover_index(&self) -> Result<(), StoreError> {
        if self.config.index_file.exists() {
            let data = fs::read(&self.config.index_file)
                .map_err(|e| StoreError::Storage(format!("read index: {e}")))?;
            let index: HashMap<String, usize> =
                serde_json::from_slice(&data).unwrap_or_default();
            *self.event_index.write() = index;
        }
        Ok(())
    }

    async fn persist_index(&self) -> Result<(), StoreError> {
        let index = self.event_index.read();
        let data = serde_json::to_vec(&*index)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;
        fs::write(&self.config.index_file, data)
            .map_err(|e| StoreError::Storage(format!("write index: {e}")))?;
        Ok(())
    }

    async fn roll_if_needed(&self) -> Result<(), StoreError> {
        let size = *self.current_size.read();
        if size >= self.config.max_file_size_bytes {
            self.roll_file().await?;
        }
        Ok(())
    }

    async fn roll_file(&self) -> Result<(), StoreError> {
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.config.data_dir).map_err(|e| StoreError::Storage(format!("read data dir: {e}")))? {
            let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {e}")))?;
            let path = entry.path();
            if path.extension().map(|e| e == "log").unwrap_or(false) {
                files.push(path);
            }
        }
        files.sort();
        while files.len() >= self.config.max_files {
            let oldest = files.remove(0);
            fs::remove_file(&oldest)
                .map_err(|e| StoreError::Storage(format!("remove old file: {e}")))?;
        }

        let path = self.next_file_path();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| StoreError::Storage(format!("open log file: {e}")))?;
        *self.current_file.lock() = Some(file);
        *self.current_path.lock() = path;
        *self.current_size.write() = 0;
        Ok(())
    }

    fn next_file_path(&self) -> PathBuf {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.config.data_dir.join(format!("events-{}.log", ts))
    }

    fn write_event(&self, event: &Event) -> Result<(), StoreError> {
        let mut file_guard = self.current_file.lock();
        if file_guard.is_none() {
            let path = self.next_file_path();
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| StoreError::Storage(format!("open log file: {e}")))?;
            *self.current_path.lock() = path.clone();
            *file_guard = Some(file);
        }
        let file = file_guard.as_mut().unwrap();
        let line = serde_json::to_string(event)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;
        writeln!(file, "{}", line)
            .map_err(|e| StoreError::Storage(format!("write event: {e}")))?;
        *self.current_size.write() += line.len() as u64 + 1;
        Ok(())
    }
}

impl EventStore for FileStore {
    fn append<'a>(&'a self, event: Event) -> BoxFuture<'a, Result<u64, StoreError>> {
        Box::pin(async move {
            let event_id = event.event_id.clone();
            {
                let index = self.event_index.read();
                if index.contains_key(&event_id) {
                    return Err(StoreError::DuplicateEvent(event_id));
                }
            }

            let seq = {
                let mut s = self.sequence.write();
                *s += 1;
                *s
            };

            self.write_event(&event)?;
            self.event_index.write().insert(event_id, seq as usize);
            self.persist_index().await?;
            self.roll_if_needed().await?;

            let arc_event = Arc::new(event.clone());
            let _ = self.sender.send(arc_event);
            Ok(seq)
        })
    }

    fn append_batch<'a>(
        &'a self,
        events: Vec<Event>,
    ) -> BoxFuture<'a, Result<Vec<u64>, StoreError>> {
        Box::pin(async move {
            let mut seqs = Vec::with_capacity(events.len());
            {
                let index = self.event_index.read();
                for event in &events {
                    if index.contains_key(&event.event_id) {
                        return Err(StoreError::DuplicateEvent(event.event_id.clone()));
                    }
                }
            }

            for mut event in events {
                let seq = {
                    let mut s = self.sequence.write();
                    *s += 1;
                    *s
                };
                event.sequence_number = seq;
                self.write_event(&event)?;
                self.event_index.write().insert(event.event_id.clone(), seq as usize);
                seqs.push(seq);
                let arc_event = Arc::new(event.clone());
                let _ = self.sender.send(arc_event);
            }

            self.persist_index().await?;
            self.roll_if_needed().await?;
            Ok(seqs)
        })
    }

    fn read<'a>(&'a self, aggregate_id: &'a str) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let mut out = Vec::new();
            let mut files = Vec::new();
            for entry in fs::read_dir(&self.config.data_dir).map_err(|e| StoreError::Storage(format!("read data dir: {e}")))? {
                let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {e}")))?;
                let path = entry.path();
                if path.extension().map(|e| e == "log").unwrap_or(false) {
                    files.push(path);
                }
            }
            files.sort();

            for file_path in files {
                let file = File::open(&file_path).map_err(|e| StoreError::Storage(format!("open log: {e}")))?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let line = line.map_err(|e| StoreError::Storage(format!("read line: {e}")))?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<Event>(&line) {
                        if event.aggregate_id == aggregate_id {
                            out.push(event);
                        }
                    }
                }
            }
            out.sort_by_key(|e| e.sequence_number);
            Ok(out)
        })
    }

    fn read_all<'a>(&'a self) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let mut out = Vec::new();
            let mut files = Vec::new();
            for entry in fs::read_dir(&self.config.data_dir).map_err(|e| StoreError::Storage(format!("read data dir: {e}")))? {
                let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {e}")))?;
                let path = entry.path();
                if path.extension().map(|e| e == "log").unwrap_or(false) {
                    files.push(path);
                }
            }
            files.sort();

            for file_path in files {
                let file = File::open(&file_path).map_err(|e| StoreError::Storage(format!("open log: {e}")))?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let line = line.map_err(|e| StoreError::Storage(format!("read line: {e}")))?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<Event>(&line) {
                        out.push(event);
                    }
                }
            }
            out.sort_by_key(|e| e.sequence_number);
            Ok(out)
        })
    }

    fn read_after<'a>(&'a self, after_ns: u64) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let mut out = Vec::new();
            let mut files = Vec::new();
            for entry in fs::read_dir(&self.config.data_dir).map_err(|e| StoreError::Storage(format!("read data dir: {e}")))? {
                let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {e}")))?;
                let path = entry.path();
                if path.extension().map(|e| e == "log").unwrap_or(false) {
                    files.push(path);
                }
            }
            files.sort();

            for file_path in files {
                let file = File::open(&file_path).map_err(|e| StoreError::Storage(format!("open log: {e}")))?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let line = line.map_err(|e| StoreError::Storage(format!("read line: {e}")))?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<Event>(&line) {
                        if event.timestamp_ns > after_ns {
                            out.push(event);
                        }
                    }
                }
            }
            out.sort_by_key(|e| e.timestamp_ns);
            Ok(out)
        })
    }

    fn read_after_sequence<'a>(
        &'a self,
        seq: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let mut out = Vec::new();
            let mut files = Vec::new();
            for entry in fs::read_dir(&self.config.data_dir).map_err(|e| StoreError::Storage(format!("read data dir: {e}")))? {
                let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {e}")))?;
                let path = entry.path();
                if path.extension().map(|e| e == "log").unwrap_or(false) {
                    files.push(path);
                }
            }
            files.sort();

            for file_path in files {
                let file = File::open(&file_path).map_err(|e| StoreError::Storage(format!("open log: {e}")))?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let line = line.map_err(|e| StoreError::Storage(format!("read line: {e}")))?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<Event>(&line) {
                        if event.sequence_number > seq {
                            out.push(event);
                        }
                    }
                }
            }
            out.sort_by_key(|e| e.sequence_number);
            Ok(out)
        })
    }

    fn read_range<'a>(
        &'a self,
        aggregate_id: &'a str,
        from_seq: u64,
        to_seq: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let mut out = Vec::new();
            let mut files = Vec::new();
            for entry in fs::read_dir(&self.config.data_dir).map_err(|e| StoreError::Storage(format!("read data dir: {e}")))? {
                let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {e}")))?;
                let path = entry.path();
                if path.extension().map(|e| e == "log").unwrap_or(false) {
                    files.push(path);
                }
            }
            files.sort();

            for file_path in files {
                let file = File::open(&file_path).map_err(|e| StoreError::Storage(format!("open log: {e}")))?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let line = line.map_err(|e| StoreError::Storage(format!("read line: {e}")))?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<Event>(&line) {
                        if event.aggregate_id == aggregate_id
                            && event.sequence_number >= from_seq
                            && event.sequence_number <= to_seq
                        {
                            out.push(event);
                        }
                    }
                }
            }
            out.sort_by_key(|e| e.sequence_number);
            Ok(out)
        })
    }

    fn read_by_correlation<'a>(
        &'a self,
        correlation_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let mut out = Vec::new();
            let mut files = Vec::new();
            for entry in fs::read_dir(&self.config.data_dir).map_err(|e| StoreError::Storage(format!("read data dir: {e}")))? {
                let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {e}")))?;
                let path = entry.path();
                if path.extension().map(|e| e == "log").unwrap_or(false) {
                    files.push(path);
                }
            }
            files.sort();

            for file_path in files {
                let file = File::open(&file_path).map_err(|e| StoreError::Storage(format!("open log: {e}")))?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let line = line.map_err(|e| StoreError::Storage(format!("read line: {e}")))?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<Event>(&line) {
                        if event.correlation_id.as_str() == correlation_id {
                            out.push(event);
                        }
                    }
                }
            }
            out.sort_by_key(|e| e.sequence_number);
            Ok(out)
        })
    }

    fn read_by_cell_id<'a>(
        &'a self,
        cell_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let mut out = Vec::new();
            let mut files = Vec::new();
            for entry in fs::read_dir(&self.config.data_dir).map_err(|e| StoreError::Storage(format!("read data dir: {e}")))? {
                let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {e}")))?;
                let path = entry.path();
                if path.extension().map(|e| e == "log").unwrap_or(false) {
                    files.push(path);
                }
            }
            files.sort();

            for file_path in files {
                let file = File::open(&file_path).map_err(|e| StoreError::Storage(format!("open log: {e}")))?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let line = line.map_err(|e| StoreError::Storage(format!("read line: {e}")))?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<Event>(&line) {
                        if event.cell_id == cell_id {
                            out.push(event);
                        }
                    }
                }
            }
            out.sort_by_key(|e| e.sequence_number);
            Ok(out)
        })
    }

    fn read_by_time_range<'a>(
        &'a self,
        start_ns: u64,
        end_ns: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let mut out = Vec::new();
            let mut files = Vec::new();
            for entry in fs::read_dir(&self.config.data_dir).map_err(|e| StoreError::Storage(format!("read data dir: {e}")))? {
                let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {e}")))?;
                let path = entry.path();
                if path.extension().map(|e| e == "log").unwrap_or(false) {
                    files.push(path);
                }
            }
            files.sort();

            for file_path in files {
                let file = File::open(&file_path).map_err(|e| StoreError::Storage(format!("open log: {e}")))?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let line = line.map_err(|e| StoreError::Storage(format!("read line: {e}")))?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(event) = serde_json::from_str::<Event>(&line) {
                        if event.timestamp_ns >= start_ns && event.timestamp_ns <= end_ns {
                            out.push(event);
                        }
                    }
                }
            }
            out.sort_by_key(|e| e.timestamp_ns);
            Ok(out)
        })
    }

    fn latest_sequence<'a>(&'a self) -> BoxFuture<'a, Result<u64, StoreError>> {
        Box::pin(async move { Ok(*self.sequence.read()) })
    }

    fn subscribe(&self) -> EventReceiver {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventBuilder;
    use std::sync::OnceLock;
    use tempfile::tempdir;

    static DIR: OnceLock<PathBuf> = OnceLock::new();

    fn test_dir() -> PathBuf {
        DIR.get_or_init(|| tempdir().unwrap().into_path()).clone()
    }

    #[tokio::test]
    async fn test_file_store_roundtrip() {
        let dir = tempdir().unwrap();
        let cfg = FileStoreConfig {
            data_dir: dir.path().to_path_buf(),
            max_file_size_bytes: 1024 * 1024,
            max_files: 5,
            index_file: dir.path().join("index.json"),
        };
        let store = FileStore::connect(cfg).await.unwrap();
        let e = EventBuilder::new("a1", "evt1", serde_json::json!({}))
            .cell_id("c1")
            .build();
        let seq = store.append(e).await.unwrap();
        assert_eq!(seq, 1);
        let evts = store.read("a1").await.unwrap();
        assert_eq!(evts.len(), 1);
    }
}
