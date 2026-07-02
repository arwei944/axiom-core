//! Benchmark: Mailbox push/pop throughput and latency.

use axiom_bench::common::make_signal;
use axiom_runtime::mailbox::Mailbox;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn bench_mailbox_push(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("mailbox_push_single", |b| {
        b.iter(|| {
            let mailbox = Mailbox::new(1024);
            let env = make_signal("Bench", "src", "dst");
            rt.block_on(async {
                mailbox.push(env).await.unwrap();
            });
            black_box(mailbox);
        });
    });
}

fn bench_mailbox_push_pop_cycle(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("mailbox_push_pop_cycle", |b| {
        b.iter(|| {
            let mailbox = Mailbox::new(1024);
            let env = make_signal("Bench", "src", "dst");
            rt.block_on(async {
                mailbox.push(env).await.unwrap();
                let _ = mailbox.pop().await;
            });
            black_box(());
        });
    });
}

fn bench_mailbox_batch_push_100(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("mailbox_batch_push_100", |b| {
        b.iter(|| {
            let mailbox = Mailbox::new(1024);
            rt.block_on(async {
                for _ in 0..100 {
                    let env = make_signal("Bench", "src", "dst");
                    mailbox.push(env).await.unwrap();
                }
            });
            black_box(mailbox);
        });
    });
}

fn bench_mailbox_batch_push_pop_100(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("mailbox_batch_push_pop_100", |b| {
        b.iter(|| {
            let mailbox = Arc::new(Mailbox::new(1024));
            rt.block_on(async {
                for _ in 0..100 {
                    let env = make_signal("Bench", "src", "dst");
                    mailbox.push(env).await.unwrap();
                }
                for _ in 0..100 {
                    let _ = mailbox.pop().await;
                }
            });
            black_box(());
        });
    });
}

fn bench_mailbox_drain_100(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("mailbox_drain_100", |b| {
        b.iter(|| {
            let mailbox = Mailbox::new(1024);
            rt.block_on(async {
                for _ in 0..100 {
                    let env = make_signal("Bench", "src", "dst");
                    mailbox.push(env).await.unwrap();
                }
                let _ = mailbox.drain().await;
            });
            black_box(());
        });
    });
}

criterion_group!(
    benches,
    bench_mailbox_push,
    bench_mailbox_push_pop_cycle,
    bench_mailbox_batch_push_100,
    bench_mailbox_batch_push_pop_100,
    bench_mailbox_drain_100
);
criterion_main!(benches);
