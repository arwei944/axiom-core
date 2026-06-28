//! In-memory event store implementation (for testing).

use crate::event::Event;
use crate::store::{EventStore, StoreError};
use async_trait::async_trait;
use std::sync::RwLock;

pub struct MemoryStore {
    events: RwLock<Vec<Event>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait]
impl EventStore for MemoryStore {
    async fn append(&self, event: Event) -> Result<(), StoreError> {
        self.events.write().unwrap().push(event);
        Ok(())
    }

    async fn read(&self, aggregate_id: &str) -> Result<Vec<Event>, StoreError> {
        let events = self.events.read().unwrap();
        Ok(events
            .iter()
            .filter(|e| e.aggregate_id == aggregate_id)
            .cloned()
            .collect())
    }

    async fn read_all(&self) -> Result<Vec<Event>, StoreError> {
        Ok(self.events.read().unwrap().clone())
    }
}
