//! Benchmark: EntropyGovernor event recording and decision throughput.
//!
//! The EntropyGovernorCell is the oversight hot path: every disorder event
//! (dropped message, axiom violation, cell restart, ...) flows through
//! `record`, and the dispatch loop consults `take_action` / `snapshot` to
//! decide throttling. These benches isolate that cost.

use axiom_runtime::{EntropyEvent, EntropyGovernorCell};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Record a single AxiomViolation event into a fresh green governor.
fn bench_entropy_record_single(c: &mut Criterion) {
    c.bench_function("entropy_record_single", |b| {
        b.iter(|| {
            let gov = EntropyGovernorCell::default();
            gov.record(EntropyEvent::AxiomViolation { cell_id: "c1".into() });
            black_box(());
        });
    });
}

/// Record 100 mixed events against one governor to measure sustained ingest.
fn bench_entropy_record_batch_100(c: &mut Criterion) {
    c.bench_function("entropy_record_batch_100", |b| {
        b.iter(|| {
            let gov = EntropyGovernorCell::default();
            for i in 0..100u32 {
                let cell = format!("cell-{i}");
                gov.record(EntropyEvent::DroppedMessage { cell_id: cell.clone() });
                gov.record(EntropyEvent::Timeout { cell_id: cell });
            }
            black_box(gov);
        });
    });
}

/// Measure `take_action` on a green governor — the common dispatch-loop path
/// that snapshots, classifies the level, and short-circuits via cooldown.
fn bench_entropy_take_action_green(c: &mut Criterion) {
    let gov = EntropyGovernorCell::default();

    c.bench_function("entropy_take_action_green", |b| {
        b.iter(|| {
            let action = gov.take_action();
            black_box(action);
        });
    });
}

/// Measure `snapshot` — the read path that clones global/per-cell scores.
fn bench_entropy_snapshot(c: &mut Criterion) {
    let gov = EntropyGovernorCell::default();
    for i in 0..10u32 {
        gov.record(EntropyEvent::AxiomViolation { cell_id: format!("cell-{i}") });
    }

    c.bench_function("entropy_snapshot", |b| {
        b.iter(|| {
            let snap = gov.snapshot();
            black_box(snap);
        });
    });
}

/// Measure `decay_tick` over a populated per-cell map.
fn bench_entropy_decay_tick(c: &mut Criterion) {
    let gov = EntropyGovernorCell::default();
    for i in 0..50u32 {
        gov.record(EntropyEvent::AxiomViolation { cell_id: format!("cell-{i}") });
    }

    c.bench_function("entropy_decay_tick", |b| {
        b.iter(|| {
            gov.decay_tick();
            black_box(());
        });
    });
}

criterion_group!(
    benches,
    bench_entropy_record_single,
    bench_entropy_record_batch_100,
    bench_entropy_take_action_green,
    bench_entropy_snapshot,
    bench_entropy_decay_tick
);
criterion_main!(benches);
