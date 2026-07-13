//! Benchmark: Bus dispatch and interceptor chain overhead.

use axiom_bench::common::make_signal;
use axiom_kernel::layer::RuntimeTier;
use axiom_runtime::bus::{BusInterceptor, MessageBus};
use axiom_runtime::guardian::ArchitectureGuardian;
use axiom_runtime::interceptors::HopLimitInterceptor;
use axiom_runtime::mailbox::Mailbox;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn bench_guardian_intercept(c: &mut Criterion) {
    let guardian = ArchitectureGuardian::new();
    let env = make_signal("Bench", "src", "dst");

    c.bench_function("guardian_intercept_allow", |b| {
        b.iter(|| {
            let decision = guardian.intercept(black_box(&env));
            black_box(decision);
        });
    });
}

fn bench_guardian_intercept_reject(c: &mut Criterion) {
    let guardian = ArchitectureGuardian::new();
    let mut env = make_signal("Bench", "src", "dst");
    env.source_layer = RuntimeTier::Exec;
    env.target_layer = RuntimeTier::Agent;

    c.bench_function("guardian_intercept_reject", |b| {
        b.iter(|| {
            let decision = guardian.intercept(black_box(&env));
            black_box(decision);
        });
    });
}

fn bench_hop_limit_intercept(c: &mut Criterion) {
    let interceptor = HopLimitInterceptor::new(64);
    let env = make_signal("Bench", "src", "dst");

    c.bench_function("hop_limit_intercept", |b| {
        b.iter(|| {
            let decision = interceptor.intercept(black_box(&env));
            black_box(decision);
        });
    });
}

fn bench_bus_register_publish(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("bus_register_and_publish", |b| {
        b.iter(|| {
            let bus = MessageBus::new();
            let mailbox = Arc::new(Mailbox::new(1024));
            rt.block_on(async {
                bus.register_cell(
                    &axiom_kernel::id::CellId::new("dst"),
                    mailbox,
                    RuntimeTier::Exec,
                )
                .await;
                let env = make_signal("Bench", "src", "dst");
                let _ = bus.publish(env).await;
            });
            black_box(bus);
        });
    });
}

fn bench_bus_publish_only(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let bus = MessageBus::new();
    let mailbox = Arc::new(Mailbox::new(1024));
    rt.block_on(async {
        bus.register_cell(&axiom_kernel::id::CellId::new("dst"), mailbox, RuntimeTier::Exec).await;
    });

    c.bench_function("bus_publish_only", |b| {
        b.iter(|| {
            let env = make_signal("Bench", "src", "dst");
            rt.block_on(async {
                let _ = bus.publish(env).await;
            });
            black_box(());
        });
    });
}

/// Measure sustained MessageBus throughput: publish 100 messages to a
/// registered cell and drain its mailbox, capturing end-to-end dispatch cost.
fn bench_bus_throughput_100(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let bus = MessageBus::new();
    let mailbox = Arc::new(Mailbox::new(2048));
    rt.block_on(async {
        bus.register_cell(
            &axiom_kernel::id::CellId::new("dst"),
            mailbox.clone(),
            RuntimeTier::Exec,
        )
        .await;
    });

    c.bench_function("bus_throughput_100", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..100 {
                    let env = make_signal("Bench", "src", "dst");
                    let _ = bus.publish(env).await;
                }
                // drain so the mailbox does not fill up across iterations
                let _ = mailbox.drain().await;
            });
            black_box(());
        });
    });
}

criterion_group!(
    benches,
    bench_guardian_intercept,
    bench_guardian_intercept_reject,
    bench_hop_limit_intercept,
    bench_bus_register_publish,
    bench_bus_publish_only,
    bench_bus_throughput_100
);
criterion_main!(benches);
