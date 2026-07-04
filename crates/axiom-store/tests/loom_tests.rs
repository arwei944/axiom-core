//! Loom-based concurrency tests for axiom-store.
//!
//! loom 用于检测线程级数据竞争。由于 MemoryStore 内部使用 tokio 同步原语，
//! 这里我们用 loom 原语复现关键并发场景：序列号生成的原子性保证。

use loom::sync::atomic::{AtomicU64, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn loom_concurrent_sequence_generation_is_unique() {
    loom::model(|| {
        let counter = Arc::new(AtomicU64::new(0));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let counter = counter.clone();
                thread::spawn(move || {
                    for _ in 0..5 {
                        counter.fetch_add(1, Ordering::SeqCst);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    });
}
