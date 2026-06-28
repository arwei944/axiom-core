//! In-memory event store implementation (for testing and development).

use crate::event::Event;
use crate::store::{EventStore, StoreError};
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

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

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

    async fn read_after(&self, after_ns: u64) -> Result<Vec<Event>, StoreError> {
        let events = self.events.read().unwrap();
        Ok(events
            .iter()
            .filter(|e| e.timestamp_ns > after_ns)
            .cloned()
            .collect())
    }
}
