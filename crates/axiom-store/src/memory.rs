//! In-memory event store implementation (for testing and development).

use crate::event::Event;
use crate::store::{BoxFuture, EventReceiver, EventStore, StoreError};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

struct Inner {
    events: RwLock<Vec<Event>>,
    event_index: RwLock<HashMap<String, usize>>,
    sequence: AtomicU64,
    sender: broadcast::Sender<Arc<Event>>,
}

pub struct MemoryStore {
    inner: Arc<Inner>,
}

impl MemoryStore {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self {
            inner: Arc::new(Inner {
                events: RwLock::new(Vec::new()),
                event_index: RwLock::new(HashMap::new()),
                sequence: AtomicU64::new(0),
                sender,
            }),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStore for MemoryStore {
    fn append<'a>(&'a self, mut event: Event) -> BoxFuture<'a, Result<u64, StoreError>> {
        Box::pin(async move {
            let event_id = event.event_id.clone();

            {
                let index = self.inner.event_index.read().await;
                if index.contains_key(&event_id) {
                    return Err(StoreError::DuplicateEvent(event_id));
                }
            }

            let seq = self.inner.sequence.fetch_add(1, Ordering::SeqCst) + 1;
            event.sequence_number = seq;

            {
                let mut events = self.inner.events.write().await;
                let mut index = self.inner.event_index.write().await;
                index.insert(event_id, events.len());
                let arc_event = Arc::new(event.clone());
                events.push(event);
                let _ = self.inner.sender.send(arc_event);
            }
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
                let index = self.inner.event_index.read().await;
                for event in &events {
                    if index.contains_key(&event.event_id) {
                        return Err(StoreError::DuplicateEvent(event.event_id.clone()));
                    }
                }
            }

            let mut events_mut = self.inner.events.write().await;
            let mut index = self.inner.event_index.write().await;

            for mut event in events {
                let seq = self.inner.sequence.fetch_add(1, Ordering::SeqCst) + 1;
                event.sequence_number = seq;
                let event_id = event.event_id.clone();
                let arc_event = Arc::new(event.clone());
                index.insert(event_id, events_mut.len());
                seqs.push(seq);
                let _ = self.inner.sender.send(arc_event);
                events_mut.push(event);
            }
            Ok(seqs)
        })
    }

    fn read<'a>(&'a self, aggregate_id: &'a str) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let events = self.inner.events.read().await;
            Ok(events
                .iter()
                .filter(|e| e.aggregate_id == aggregate_id)
                .cloned()
                .collect())
        })
    }

    fn read_all<'a>(&'a self) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let events = self.inner.events.read().await;
            Ok(events.clone())
        })
    }

    fn read_after<'a>(&'a self, after_ns: u64) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let events = self.inner.events.read().await;
            Ok(events
                .iter()
                .filter(|e| e.timestamp_ns > after_ns)
                .cloned()
                .collect())
        })
    }

    fn read_after_sequence<'a>(
        &'a self,
        seq: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let events = self.inner.events.read().await;
            Ok(events
                .iter()
                .filter(|e| e.sequence_number > seq)
                .cloned()
                .collect())
        })
    }

    fn read_range<'a>(
        &'a self,
        aggregate_id: &'a str,
        from_seq: u64,
        to_seq: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let events = self.inner.events.read().await;
            Ok(events
                .iter()
                .filter(|e| {
                    e.aggregate_id == aggregate_id
                        && e.sequence_number >= from_seq
                        && e.sequence_number <= to_seq
                })
                .cloned()
                .collect())
        })
    }

    fn read_by_correlation<'a>(
        &'a self,
        correlation_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let events = self.inner.events.read().await;
            Ok(events
                .iter()
                .filter(|e| e.correlation_id.as_str() == correlation_id)
                .cloned()
                .collect())
        })
    }

    fn read_by_cell_id<'a>(
        &'a self,
        cell_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let events = self.inner.events.read().await;
            Ok(events
                .iter()
                .filter(|e| e.cell_id.as_str() == cell_id)
                .cloned()
                .collect())
        })
    }

    fn read_by_time_range<'a>(
        &'a self,
        start_ns: u64,
        end_ns: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let events = self.inner.events.read().await;
            Ok(events
                .iter()
                .filter(|e| e.timestamp_ns >= start_ns && e.timestamp_ns <= end_ns)
                .cloned()
                .collect())
        })
    }

    fn latest_sequence<'a>(&'a self) -> BoxFuture<'a, Result<u64, StoreError>> {
        Box::pin(async move { Ok(self.inner.sequence.load(Ordering::SeqCst)) })
    }

    fn subscribe(&self) -> EventReceiver {
        self.inner.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventBuilder;
    use axiom_kernel::id::CorrelationId;

    #[tokio::test]
    async fn test_append_and_read_roundtrip() {
        let store = MemoryStore::new();
        let e = EventBuilder::new("a1", "evt1", serde_json::json!({}))
            .cell_id("c1")
            .build();
        let seq = store.append(e).await.unwrap();
        assert_eq!(seq, 1);
        let evts = store.read("a1").await.unwrap();
        assert_eq!(evts.len(), 1);
        assert_eq!(evts[0].sequence_number, 1);
    }

    #[tokio::test]
    async fn test_sequence_numbers_monotonic() {
        let store = MemoryStore::new();
        for i in 0..10 {
            let e = EventBuilder::new("a", &format!("e{}", i), serde_json::json!({})).build();
            let seq = store.append(e).await.unwrap();
            assert_eq!(seq, i + 1);
        }
        assert_eq!(store.latest_sequence().await.unwrap(), 10);
    }

    #[tokio::test]
    async fn test_batch_append_atomic() {
        let store = MemoryStore::new();
        let events: Vec<Event> = (0..5)
            .map(|i| {
                EventBuilder::new("batch", &format!("e{}", i), serde_json::json!({"i": i})).build()
            })
            .collect();
        let seqs = store.append_batch(events).await.unwrap();
        assert_eq!(seqs, vec![1, 2, 3, 4, 5]);
        assert_eq!(store.latest_sequence().await.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_batch_append_duplicate_rejects_all() {
        let store = MemoryStore::new();
        let e1 = EventBuilder::new("a", "e1", serde_json::json!({}))
            .event_id("dup-id")
            .build();
        store.append(e1).await.unwrap();

        let dup = EventBuilder::new("a", "e2", serde_json::json!({}))
            .event_id("dup-id")
            .build();
        let e3 = EventBuilder::new("a", "e3", serde_json::json!({})).build();
        let result = store.append_batch(vec![dup, e3]).await;
        assert!(result.is_err());
        assert_eq!(store.latest_sequence().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_read_range_boundaries() {
        let store = MemoryStore::new();
        for i in 0..10 {
            let e = EventBuilder::new("agg", "e", serde_json::json!({"i":i})).build();
            store.append(e).await.unwrap();
        }
        let range = store.read_range("agg", 3, 7).await.unwrap();
        assert_eq!(range.len(), 5);
        assert_eq!(range.first().unwrap().sequence_number, 3);
        assert_eq!(range.last().unwrap().sequence_number, 7);
    }

    #[tokio::test]
    async fn test_read_after_sequence() {
        let store = MemoryStore::new();
        for i in 0..5 {
            let e = EventBuilder::new("a", "e", serde_json::json!({"i":i})).build();
            store.append(e).await.unwrap();
        }
        let after = store.read_after_sequence(3).await.unwrap();
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].sequence_number, 4);
        assert_eq!(after[1].sequence_number, 5);
    }

    #[tokio::test]
    async fn test_duplicate_event_rejected() {
        let store = MemoryStore::new();
        let e1 = EventBuilder::new("a", "e", serde_json::json!({}))
            .event_id("dup")
            .build();
        store.append(e1).await.unwrap();
        let e2 = EventBuilder::new("a", "e", serde_json::json!({}))
            .event_id("dup")
            .build();
        assert!(matches!(
            store.append(e2).await.unwrap_err(),
            StoreError::DuplicateEvent(_)
        ));
    }

    #[tokio::test]
    async fn test_read_by_correlation() {
        let store = MemoryStore::new();
        let cid = CorrelationId::new("tx-1");
        let e1 = EventBuilder::new("a", "e", serde_json::json!({}))
            .correlation_id(cid.clone())
            .build();
        let e2 = EventBuilder::new("a", "e2", serde_json::json!({}))
            .correlation_id(CorrelationId::new("tx-2"))
            .build();
        store.append(e1).await.unwrap();
        store.append(e2).await.unwrap();
        let results = store.read_by_correlation("tx-1").await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_subscribe_receives_events() {
        let store = MemoryStore::new();
        let mut rx = store.subscribe();
        let e = EventBuilder::new("a", "e", serde_json::json!({})).build();
        store.append(e).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.aggregate_id, "a");
        assert_eq!(received.sequence_number, 1);
    }
}
