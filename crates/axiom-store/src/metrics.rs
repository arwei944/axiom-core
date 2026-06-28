//! Store health metrics and metered decorator.

use crate::event::Event;
use crate::store::{BoxFuture, EventReceiver, EventStore, StoreError};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreHealth {
    pub total_events: u64,
    pub total_snapshots: u64,
    pub oldest_event_ns: Option<u64>,
    pub newest_event_ns: Option<u64>,
    pub store_size_bytes: Option<u64>,
    pub write_latency_p50_ms: f64,
    pub write_latency_p99_ms: f64,
    pub read_latency_p50_ms: f64,
    pub error_count: u64,
}

impl Default for StoreHealth {
    fn default() -> Self {
        Self {
            total_events: 0,
            total_snapshots: 0,
            oldest_event_ns: None,
            newest_event_ns: None,
            store_size_bytes: None,
            write_latency_p50_ms: 0.0,
            write_latency_p99_ms: 0.0,
            read_latency_p50_ms: 0.0,
            error_count: 0,
        }
    }
}

struct LatencyTracker {
    samples: std::sync::Mutex<Vec<f64>>,
}

impl LatencyTracker {
    fn new() -> Self {
        Self {
            samples: std::sync::Mutex::new(Vec::with_capacity(128)),
        }
    }

    fn record(&self, ms: f64) {
        let mut s = self.samples.lock().unwrap();
        if s.len() >= 100 {
            s.remove(0);
        }
        s.push(ms);
    }

    fn percentile(&self, p: f64) -> f64 {
        let mut s = self.samples.lock().unwrap();
        if s.is_empty() {
            return 0.0;
        }
        s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((s.len() - 1) as f64 * p) as usize;
        s[idx.min(s.len() - 1)]
    }
}

pub struct StoreMetrics {
    write_latency: LatencyTracker,
    read_latency: LatencyTracker,
    error_count: AtomicU64,
}

impl StoreMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            write_latency: LatencyTracker::new(),
            read_latency: LatencyTracker::new(),
            error_count: AtomicU64::new(0),
        })
    }

    fn record_write(&self, dur_ms: f64) {
        self.write_latency.record(dur_ms);
    }

    fn record_read(&self, dur_ms: f64) {
        self.read_latency.record(dur_ms);
    }

    fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn write_p50_ms(&self) -> f64 {
        self.write_latency.percentile(0.5)
    }

    pub fn write_p99_ms(&self) -> f64 {
        self.write_latency.percentile(0.99)
    }

    pub fn read_p50_ms(&self) -> f64 {
        self.read_latency.percentile(0.5)
    }

    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }
}

pub struct MeteredStore<S> {
    inner: S,
    metrics: Arc<StoreMetrics>,
}

impl<S> MeteredStore<S> {
    pub fn new(inner: S, metrics: Arc<StoreMetrics>) -> Self {
        Self { inner, metrics }
    }

    pub fn metrics(&self) -> &StoreMetrics {
        &self.metrics
    }
}

impl<S: EventStore> EventStore for MeteredStore<S> {
    fn append<'a>(&'a self, event: Event) -> BoxFuture<'a, Result<u64, StoreError>> {
        Box::pin(async move {
            let start = Instant::now();
            let res = self.inner.append(event).await;
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            if res.is_ok() {
                self.metrics.record_write(ms);
            } else {
                self.metrics.record_error();
            }
            res
        })
    }

    fn append_batch<'a>(
        &'a self,
        events: Vec<Event>,
    ) -> BoxFuture<'a, Result<Vec<u64>, StoreError>> {
        Box::pin(async move {
            let start = Instant::now();
            let res = self.inner.append_batch(events).await;
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            if res.is_ok() {
                self.metrics.record_write(ms);
            } else {
                self.metrics.record_error();
            }
            res
        })
    }

    fn read<'a>(&'a self, aggregate_id: &'a str) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let start = Instant::now();
            let res = self.inner.read(aggregate_id).await;
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            if res.is_ok() {
                self.metrics.record_read(ms);
            } else {
                self.metrics.record_error();
            }
            res
        })
    }

    fn read_all<'a>(&'a self) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let start = Instant::now();
            let res = self.inner.read_all().await;
            self.metrics
                .record_read(start.elapsed().as_secs_f64() * 1000.0);
            if res.is_err() {
                self.metrics.record_error();
            }
            res
        })
    }

    fn read_after<'a>(&'a self, after_ns: u64) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let start = Instant::now();
            let res = self.inner.read_after(after_ns).await;
            self.metrics
                .record_read(start.elapsed().as_secs_f64() * 1000.0);
            if res.is_err() {
                self.metrics.record_error();
            }
            res
        })
    }

    fn read_after_sequence<'a>(
        &'a self,
        seq: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let start = Instant::now();
            let res = self.inner.read_after_sequence(seq).await;
            self.metrics
                .record_read(start.elapsed().as_secs_f64() * 1000.0);
            if res.is_err() {
                self.metrics.record_error();
            }
            res
        })
    }

    fn read_range<'a>(
        &'a self,
        aggregate_id: &'a str,
        from_seq: u64,
        to_seq: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let start = Instant::now();
            let res = self.inner.read_range(aggregate_id, from_seq, to_seq).await;
            self.metrics
                .record_read(start.elapsed().as_secs_f64() * 1000.0);
            if res.is_err() {
                self.metrics.record_error();
            }
            res
        })
    }

    fn read_by_correlation<'a>(
        &'a self,
        correlation_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let start = Instant::now();
            let res = self.inner.read_by_correlation(correlation_id).await;
            self.metrics
                .record_read(start.elapsed().as_secs_f64() * 1000.0);
            if res.is_err() {
                self.metrics.record_error();
            }
            res
        })
    }

    fn latest_sequence<'a>(&'a self) -> BoxFuture<'a, Result<u64, StoreError>> {
        Box::pin(async move { self.inner.latest_sequence().await })
    }

    fn subscribe(&self) -> EventReceiver {
        self.inner.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventBuilder;
    use crate::memory::MemoryStore;

    #[tokio::test]
    async fn test_metered_store_records_latencies() {
        let inner = MemoryStore::new();
        let metrics = StoreMetrics::new();
        let metered = MeteredStore::new(inner, metrics.clone());

        for _ in 0..5 {
            let e = EventBuilder::new("a", "e", serde_json::json!({})).build();
            metered.append(e).await.unwrap();
        }
        let _ = metered.read("a").await.unwrap();

        assert!(metrics.write_p50_ms() >= 0.0);
        assert!(metrics.read_p50_ms() >= 0.0);
        assert_eq!(metrics.error_count(), 0);
    }

    #[test]
    fn test_latency_tracker_percentiles() {
        let tracker = LatencyTracker::new();
        for i in 1..=100 {
            tracker.record(i as f64);
        }
        let p50 = tracker.percentile(0.5);
        let p99 = tracker.percentile(0.99);
        assert!(
            (p50 - 50.0).abs() < 2.0,
            "p50 should be around 50, got {}",
            p50
        );
        assert!(p99 >= 98.0, "p99 should be high, got {}", p99);
    }
}
