//! Benchmark: SQLite event store write and query performance.
//!
//! Uses an in-memory SQLite database (single connection) so the benches are
//! hermetic and do not touch the filesystem. WAL mode and the standard
//! indexes are applied via `SqliteStore::connect` migrations, matching the
//! production schema.

use axiom_store::{Event, EventBuilder, EventStore, SqliteStore, SqliteStoreConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::runtime::Runtime;

/// Monotonic counter guaranteeing unique `event_id` values across all bench
/// iterations (the `events.event_id` column has a UNIQUE constraint).
static EVENT_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_event_id() -> String {
    format!("bench-evt-{}", EVENT_COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn make_event(aggregate_id: &str, cell_id: &str) -> Event {
    EventBuilder::new(aggregate_id, "bench-event", json!({"payload": "bench"}))
        .cell_id(cell_id)
        .event_id(&next_event_id())
        .build()
}

fn connect_memory_store(rt: &Runtime) -> SqliteStore {
    rt.block_on(async {
        let config = SqliteStoreConfig {
            database_url: "sqlite::memory:".to_string(),
            max_connections: 1,
            migration_timeout_ms: 5000,
        };
        SqliteStore::connect(config).await.expect("sqlite store connect")
    })
}

/// Single-event append — the dominant write path.
fn bench_sqlite_append_single(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = connect_memory_store(&rt);

    c.bench_function("sqlite_append_single", |b| {
        b.iter(|| {
            rt.block_on(async {
                let event = make_event("agg-1", "cell-1");
                let seq = store.append(event).await.unwrap();
                black_box(seq);
            });
        });
    });
}

/// 100 sequential single appends — sustained write throughput.
fn bench_sqlite_append_100_sequential(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = connect_memory_store(&rt);

    c.bench_function("sqlite_append_100_sequential", |b| {
        b.iter(|| {
            rt.block_on(async {
                for i in 0..100u64 {
                    let event = make_event("agg-batch", "cell-batch");
                    let _ = store.append(event).await.unwrap();
                    black_box(i);
                }
            });
        });
    });
}

/// Batch append of 100 events in a single transaction.
fn bench_sqlite_append_batch_100(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = connect_memory_store(&rt);

    c.bench_function("sqlite_append_batch_100", |b| {
        b.iter(|| {
            rt.block_on(async {
                let events: Vec<Event> = (0..100).map(|_| make_event("agg-b", "cell-b")).collect();
                let seqs = store.append_batch(events).await.unwrap();
                black_box(seqs);
            });
        });
    });
}

/// Read events by aggregate_id (indexed lookup). Store is pre-populated once.
fn bench_sqlite_read_by_aggregate(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = connect_memory_store(&rt);
    rt.block_on(async {
        for _ in 0..100 {
            store.append(make_event("agg-read", "cell-read")).await.unwrap();
        }
    });

    c.bench_function("sqlite_read_by_aggregate", |b| {
        b.iter(|| {
            rt.block_on(async {
                let events = store.read("agg-read").await.unwrap();
                black_box(events);
            });
        });
    });
}

/// Read events by cell_id (indexed lookup).
fn bench_sqlite_read_by_cell_id(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = connect_memory_store(&rt);
    rt.block_on(async {
        for _ in 0..100 {
            store.append(make_event("agg-cell", "cell-query")).await.unwrap();
        }
    });

    c.bench_function("sqlite_read_by_cell_id", |b| {
        b.iter(|| {
            rt.block_on(async {
                let events = store.read_by_cell_id("cell-query").await.unwrap();
                black_box(events);
            });
        });
    });
}

/// Read events by time range (indexed on timestamp_ns).
fn bench_sqlite_read_by_time_range(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = connect_memory_store(&rt);
    let (start_ns, end_ns) = rt.block_on(async {
        let mut first_ns = u64::MAX;
        let mut last_ns = 0u64;
        for _ in 0..100 {
            let event = make_event("agg-time", "cell-time");
            if event.timestamp_ns < first_ns {
                first_ns = event.timestamp_ns;
            }
            if event.timestamp_ns > last_ns {
                last_ns = event.timestamp_ns;
            }
            store.append(event).await.unwrap();
        }
        (first_ns, last_ns)
    });

    c.bench_function("sqlite_read_by_time_range", |b| {
        b.iter(|| {
            rt.block_on(async {
                let events = store.read_by_time_range(start_ns, end_ns).await.unwrap();
                black_box(events);
            });
        });
    });
}

criterion_group!(
    benches,
    bench_sqlite_append_single,
    bench_sqlite_append_100_sequential,
    bench_sqlite_append_batch_100,
    bench_sqlite_read_by_aggregate,
    bench_sqlite_read_by_cell_id,
    bench_sqlite_read_by_time_range
);
criterion_main!(benches);
