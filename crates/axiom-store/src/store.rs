//! EventStore trait - abstraction for event storage.

use crate::event::Event;
use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Event not found: {0}")]
    NotFound(String),
}

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, event: Event) -> Result<(), StoreError>;
    async fn read(&self, aggregate_id: &str) -> Result<Vec<Event>, StoreError>;
    async fn read_all(&self) -> Result<Vec<Event>, StoreError>;
}
