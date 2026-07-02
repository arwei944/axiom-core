//! Benchmark: Signal envelope creation and serialization overhead.

use axiom_bench::common::make_signal;
use axiom_core::signal::SignalEnvelope;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_signal_creation(c: &mut Criterion) {
    c.bench_function("signal_creation", |b| {
        b.iter(|| {
            let env = make_signal("BenchSignal", "src", "dst");
            black_box(env);
        });
    });
}

fn bench_signal_serialization(c: &mut Criterion) {
    let env = make_signal("BenchSignal", "src", "dst");
    c.bench_function("signal_serialize_json", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&env)).unwrap();
            black_box(json);
        });
    });
}

fn bench_signal_deserialization(c: &mut Criterion) {
    let env = make_signal("BenchSignal", "src", "dst");
    let json = serde_json::to_string(&env).unwrap();
    c.bench_function("signal_deserialize_json", |b| {
        b.iter(|| {
            let env: SignalEnvelope = serde_json::from_str(black_box(&json)).unwrap();
            black_box(env);
        });
    });
}

fn bench_signal_batch_creation(c: &mut Criterion) {
    c.bench_function("signal_batch_100", |b| {
        b.iter(|| {
            let batch: Vec<SignalEnvelope> = (0..100)
                .map(|i| make_signal("BenchSignal", &format!("src-{i}"), "dst"))
                .collect();
            black_box(batch);
        });
    });
}

criterion_group!(benches, bench_signal_creation, bench_signal_serialization, bench_signal_deserialization, bench_signal_batch_creation);
criterion_main!(benches);
