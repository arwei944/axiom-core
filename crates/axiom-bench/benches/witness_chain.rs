//! Benchmark: Witness chain construction and verification.

use axiom_core::id::{CorrelationId, MsgId, WitnessId};
use axiom_core::version::{SchemaVersion, VersionInfo};
use axiom_core::witness::{TransitionOutcome, Witness, WitnessHash, WitnessMetrics};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn make_witness(seq: u64, prev_hash: Option<WitnessHash>) -> Witness {
    Witness {
        witness_id: WitnessId::new(&format!("wit-{seq}")),
        schema_version: SchemaVersion::new(1),
        cell_id: "bench-cell".to_string(),
        correlation_id: CorrelationId::new("bench-corr"),
        trace_id: None,
        triggering_msg_id: Some(MsgId::new(&format!("msg-{seq}"))),
        vector_clock: axiom_core::signal::VectorClock::new(),
        timestamp_ns: seq * 1000,
        prev_hash,
        state_before_hash: None,
        state_after_hash: None,
        hash: WitnessHash([seq as u8; 32]),
        summary: format!("witness-{seq}"),
        outcome: TransitionOutcome::Success,
        metrics: WitnessMetrics::default(),
        version_info: VersionInfo::current(),
        signal_fingerprint: [0u8; 32],
        payload_size_bytes: 0,
    }
}

fn bench_witness_creation(c: &mut Criterion) {
    c.bench_function("witness_creation", |b| {
        let mut seq = 0u64;
        b.iter(|| {
            seq += 1;
            let w = make_witness(seq, None);
            black_box(w);
        });
    });
}

fn bench_witness_serialization(c: &mut Criterion) {
    let w = make_witness(1, None);
    c.bench_function("witness_serialize_json", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&w)).unwrap();
            black_box(json);
        });
    });
}

fn bench_witness_chain_verify_100(c: &mut Criterion) {
    let mut chain = Vec::with_capacity(100);
    let mut prev: Option<WitnessHash> = None;
    for i in 1..=100 {
        let w = make_witness(i, prev.clone());
        prev = Some(w.hash.clone());
        chain.push(w);
    }

    c.bench_function("witness_chain_verify_100", |b| {
        b.iter(|| {
            let result = Witness::verify_chain_integrity(black_box(&chain));
            black_box(result);
        });
    });
}

fn bench_witness_chain_verify_1000(c: &mut Criterion) {
    let mut chain = Vec::with_capacity(1000);
    let mut prev: Option<WitnessHash> = None;
    for i in 1..=1000 {
        let w = make_witness(i, prev.clone());
        prev = Some(w.hash.clone());
        chain.push(w);
    }

    c.bench_function("witness_chain_verify_1000", |b| {
        b.iter(|| {
            let result = Witness::verify_chain_integrity(black_box(&chain));
            black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_witness_creation,
    bench_witness_serialization,
    bench_witness_chain_verify_100,
    bench_witness_chain_verify_1000
);
criterion_main!(benches);
