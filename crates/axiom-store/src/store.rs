//! EventStore trait - abstraction for event storage.

use crate::event::Event;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Event not found: {0}")]
    NotFound(String),
    #[error("Version conflict: expected {expected}, got {actual}")]
    VersionConflict { expected: u64, actual: u64 },
    #[error("Duplicate event: {0}")]
    DuplicateEvent(String),
    #[error("Migration chain gap for {event_type}: no migration from v{from} to v{current}")]
    MigrationChainGap {
        event_type: String,
        from: u32,
        current: u32,
    },
}

pub type EventSender = broadcast::Sender<Arc<Event>>;
pub type EventReceiver = broadcast::Receiver<Arc<Event>>;

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, event: Event) -> Result<u64, StoreError>;

    async fn append_batch(&self, events: Vec<Event>) -> Result<Vec<u64>, StoreError>;

    async fn read(&self, aggregate_id: &str) -> Result<Vec<Event>, StoreError>;

    async fn read_all(&self) -> Result<Vec<Event>, StoreError>;

    async fn read_after(&self, after_ns: u64) -> Result<Vec<Event>, StoreError>;

    async fn read_after_sequence(&self, seq: u64) -> Result<Vec<Event>, StoreError>;

    async fn read_range(
        &self,
        aggregate_id: &str,
        from_seq: u64,
        to_seq: u64,
    ) -> Result<Vec<Event>, StoreError>;

    async fn read_by_correlation(&self, correlation_id: &str) -> Result<Vec<Event>, StoreError>;

    async fn latest_sequence(&self) -> Result<u64, StoreError>;

    fn subscribe(&self) -> EventReceiver;
}
