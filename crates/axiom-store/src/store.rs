//! EventStore trait - abstraction for event storage.

use crate::event::Event;
use axiom_kernel::witness::Witness;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

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
    #[error("Witness chain broken: {0}")]
    WitnessChainBroken(String),
    #[error("Internal error: {message}")]
    Internal { message: String },
}

pub type EventSender = broadcast::Sender<Arc<Event>>;
pub type EventReceiver = broadcast::Receiver<Arc<Event>>;

pub trait EventStore: Send + Sync {
    fn append<'a>(&'a self, event: Event) -> BoxFuture<'a, Result<u64, StoreError>>;

    fn append_batch<'a>(
        &'a self,
        events: Vec<Event>,
    ) -> BoxFuture<'a, Result<Vec<u64>, StoreError>>;

    fn read<'a>(&'a self, aggregate_id: &'a str) -> BoxFuture<'a, Result<Vec<Event>, StoreError>>;

    fn read_all<'a>(&'a self) -> BoxFuture<'a, Result<Vec<Event>, StoreError>>;

    fn read_after<'a>(&'a self, after_ns: u64) -> BoxFuture<'a, Result<Vec<Event>, StoreError>>;

    fn read_after_sequence<'a>(&'a self, seq: u64)
        -> BoxFuture<'a, Result<Vec<Event>, StoreError>>;

    fn read_range<'a>(
        &'a self,
        aggregate_id: &'a str,
        from_seq: u64,
        to_seq: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>>;

    fn read_by_correlation<'a>(
        &'a self,
        correlation_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>>;

    fn read_by_cell_id<'a>(
        &'a self,
        cell_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>>;

    fn read_by_time_range<'a>(
        &'a self,
        start_ns: u64,
        end_ns: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>>;

    fn latest_sequence<'a>(&'a self) -> BoxFuture<'a, Result<u64, StoreError>>;

    fn subscribe(&self) -> EventReceiver;
}

pub fn verify_witness_chain(events: &[Event]) -> Result<(), StoreError> {
    let mut witnesses: Vec<Witness> = events
        .iter()
        .filter(|e| e.event_type == "witness")
        .filter_map(|e| serde_json::from_value(e.payload.clone()).ok())
        .collect();

    witnesses.sort_by_key(|w| w.timestamp_ns);

    for window in witnesses.windows(2) {
        let prev = &window[0];
        let curr = &window[1];
        if curr.prev_hash.as_ref() != Some(&prev.hash) {
            return Err(StoreError::WitnessChainBroken(format!(
                "witness {} prev_hash mismatch",
                curr.witness_id.as_str()
            )));
        }
    }
    Ok(())
}
