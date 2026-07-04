//! Property-based tests for axiom-store using proptest.

use axiom_store::event::EventBuilder;
use axiom_store::memory::MemoryStore;
use axiom_store::store::EventStore;
use proptest::prelude::*;
use tokio::runtime::Runtime;

fn run_async<F, T>(f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let rt = Runtime::new().unwrap();
    rt.block_on(f)
}

proptest! {
    #![proptest_config(ProptestConfig::with_source_file(file!()))]

    #[test]
    fn prop_append_and_read_roundtrip(
        aggregate_id in "[a-z0-9-]{1,30}",
        cell_id in "[a-z0-9-]{1,20}",
        event_type in "[a-z_]{1,20}",
    ) {
        let store = MemoryStore::new();
        let event = EventBuilder::new(&aggregate_id, &event_type, serde_json::json!({"key": cell_id}))
            .cell_id(&cell_id)
            .timestamp_ns(1000)
            .build();

        let seq = run_async(store.append(event.clone())).unwrap();
        prop_assert_eq!(seq, 1);

        let events = run_async(store.read(&aggregate_id)).unwrap();
        prop_assert_eq!(events.len(), 1);
        prop_assert_eq!(events[0].event_id.clone(), event.event_id);
        prop_assert_eq!(events[0].aggregate_id.clone(), event.aggregate_id);
        prop_assert_eq!(events[0].event_type.clone(), event.event_type);
    }

    #[test]
    fn prop_batch_append_preserves_order(
        count in 1usize..10usize,
    ) {
        let store = MemoryStore::new();
        let mut events = Vec::with_capacity(count);

        for i in 0..count {
            let event = EventBuilder::new(
                &format!("agg-{}", i % 3),
                "test_event",
                serde_json::json!({"index": i}),
            )
            .cell_id("cell-1")
            .timestamp_ns(1000 + i as u64)
            .build();
            events.push(event);
        }

        let seqs = run_async(store.append_batch(events.clone())).unwrap();
        for (i, seq) in seqs.iter().enumerate() {
            prop_assert_eq!(*seq, (i + 1) as u64);
        }

        let all_events = run_async(store.read_all()).unwrap();
        prop_assert_eq!(all_events.len(), count);
        for (i, event) in events.iter().enumerate() {
            prop_assert_eq!(all_events[i].event_id.clone(), event.event_id.clone());
        }
    }

    #[test]
    fn prop_read_after_sequence_returns_only_newer(
        count in 1usize..5usize,
    ) {
        let store = MemoryStore::new();

        for i in 0..count {
            let event = EventBuilder::new(
                "agg-1",
                "test",
                serde_json::json!({"i": i}),
            )
            .cell_id("cell-1")
            .timestamp_ns(1000 + i as u64)
            .build();
            run_async(store.append(event)).unwrap();
        }

        let mid = run_async(store.latest_sequence()).unwrap();
        let newer = run_async(store.read_after_sequence(mid)).unwrap();
        prop_assert!(newer.is_empty());

        let newer = run_async(store.read_after_sequence(mid / 2)).unwrap();
        prop_assert!(!newer.is_empty());
    }

    #[test]
    fn prop_duplicate_event_rejected(
        aggregate_id in "[a-z0-9-]{1,30}",
    ) {
        let store = MemoryStore::new();
        let event = EventBuilder::new(&aggregate_id, "test", serde_json::json!({}))
            .cell_id("cell-1")
            .build();

        run_async(store.append(event.clone())).unwrap();
        let result = run_async(store.append(event));
        prop_assert!(result.is_err());
    }

    #[test]
    fn prop_sequence_numbers_monotonic(
        count in 1usize..20usize,
    ) {
        let store = MemoryStore::new();
        let mut prev_seq = 0u64;

        for i in 0..count {
            let event = EventBuilder::new(
                &format!("agg-{}", i % 5),
                "test",
                serde_json::json!({"i": i}),
            )
            .cell_id("cell-1")
            .build();
            let seq = run_async(store.append(event)).unwrap();
            prop_assert!(seq > prev_seq);
            prev_seq = seq;
        }
    }
}
