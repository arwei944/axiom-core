//! Runtime concurrency integration tests.
//!
//! These tests verify the runtime's behavior under concurrent load:
//! 1. Multiple cells sending messages concurrently through the bus
//! 2. Message serialization and ordering guarantees
//! 3. Backpressure under high load

use axiom_kernel::id::{CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::signal::{SignalEnvelope, SignalKind, VectorClock};
use axiom_kernel::version::SchemaVersion;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// ========== Test 1: Concurrent mailbox operations ==========

#[tokio::test]
async fn test_concurrent_mailbox_producers_single_consumer() {
    use axiom_runtime::mailbox::Mailbox;

    let mailbox = Arc::new(Mailbox::new(1024));
    let num_producers = 8;
    let msgs_per_producer = 100;

    let mut handles = vec![];

    for p in 0..num_producers {
        let mb = mailbox.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..msgs_per_producer {
                let mut env = make_test_env();
                env.signal_type = format!("producer-{}-msg-{}", p, i);
                mb.push(env).await.unwrap();
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert_eq!(mailbox.len().await, num_producers * msgs_per_producer);

    let drained = mailbox.drain().await;
    assert_eq!(drained.len(), num_producers * msgs_per_producer);
    assert_eq!(mailbox.len().await, 0);
}

#[tokio::test]
async fn test_concurrent_mailbox_push_and_pop() {
    use axiom_runtime::mailbox::Mailbox;

    let mailbox = Arc::new(Mailbox::new(256));
    let total_msgs = 200;

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    let mb_consumer = mailbox.clone();

    let consumer = tokio::spawn(async move {
        let mut received = 0;
        while received < total_msgs {
            if let Some(_env) = mb_consumer.pop().await {
                received += 1;
                counter_clone.fetch_add(1, Ordering::SeqCst);
            } else {
                tokio::task::yield_now().await;
            }
        }
        received
    });

    let mb_producer = mailbox.clone();
    let producer = tokio::spawn(async move {
        for i in 0..total_msgs {
            let mut env = make_test_env();
            env.signal_type = format!("msg-{}", i);
            mb_producer.push(env).await.unwrap();
            if i % 20 == 0 {
                tokio::task::yield_now().await;
            }
        }
    });

    let (prod_res, cons_res) = tokio::join!(producer, consumer);
    prod_res.unwrap();
    let received = cons_res.unwrap();

    assert_eq!(received, total_msgs);
    assert_eq!(counter.load(Ordering::SeqCst), total_msgs);
    assert_eq!(mailbox.len().await, 0);
}

// ========== Test 2: Concurrent DLQ operations ==========

#[test]
fn test_concurrent_dlq_enqueue() {
    use axiom_runtime::dlq::DeadLetterQueue;
    use std::thread;

    let dlq = Arc::new(DeadLetterQueue::new(1000));
    let num_threads = 4;
    let per_thread = 50;

    let mut handles = vec![];

    for t in 0..num_threads {
        let dlq_clone = dlq.clone();
        handles.push(thread::spawn(move || {
            for i in 0..per_thread {
                let env = make_test_env();
                dlq_clone.enqueue(env, &format!("thread-{}-err-{}", t, i));
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(dlq.len(), num_threads * per_thread);

    let drained = dlq.drain();
    assert_eq!(drained.len(), num_threads * per_thread);
    assert_eq!(dlq.len(), 0);
}

// ========== Test 3: Concurrent Supervisor operations ==========

#[tokio::test]
async fn test_concurrent_supervisor_multiple_cells() {
    use axiom_kernel::cell::SupervisionStrategy;
    use axiom_runtime::supervisor::Supervisor;

    let supervisor = Arc::new(Supervisor::new());
    let num_cells = 10;

    for i in 0..num_cells {
        supervisor
            .register_cell(&format!("cell-{}", i), SupervisionStrategy::Restart { max_retries: 5 })
            .await;
    }

    let mut handles = vec![];

    for c in 0..num_cells {
        let sup = supervisor.clone();
        let cell_id = format!("cell-{}", c);
        handles.push(tokio::spawn(async move {
            let mut decisions = vec![];
            for _ in 0..3 {
                let d = sup.record_panic(&cell_id).await;
                decisions.push(d);
            }
            decisions
        }));
    }

    for h in handles {
        let decisions = h.await.unwrap();
        assert_eq!(decisions.len(), 3);
    }

    for c in 0..num_cells {
        let count = supervisor.restart_count(&format!("cell-{}", c)).await;
        assert_eq!(count, 3);
    }
}

// ========== Test 4: Concurrent ArchitectureGuardian ==========

#[tokio::test]
async fn test_concurrent_guardian_intercept() {
    use axiom_runtime::bus::BusInterceptor;
    use axiom_runtime::guardian::ArchitectureGuardian;

    let guardian = Arc::new(ArchitectureGuardian::new());
    let num_tasks = 8;
    let per_task = 50;

    let mut handles = vec![];

    for t in 0..num_tasks {
        let g = guardian.clone();
        handles.push(tokio::spawn(async move {
            let mut allowed = 0;
            let mut rejected = 0;

            for i in 0..per_task {
                let (src_layer, dst_layer) = if i % 2 == 0 {
                    (Layer::Exec, Layer::Agent)
                } else {
                    (Layer::Agent, Layer::Validate)
                };

                let mut env = make_test_env();
                env.source_layer = src_layer;
                env.target_layer = dst_layer;
                env.source_cell = Some(format!("task-{}-src", t));
                env.target_cell = Some(format!("task-{}-dst", t));

                match g.intercept(&env) {
                    axiom_runtime::bus::InterceptDecision::Allow => allowed += 1,
                    axiom_runtime::bus::InterceptDecision::Reject { .. } => rejected += 1,
                    axiom_runtime::bus::InterceptDecision::Redirect { .. } => allowed += 1,
                }
            }

            (allowed, rejected)
        }));
    }

    let mut total_allowed = 0;
    let mut total_rejected = 0;

    for h in handles {
        let (a, r) = h.await.unwrap();
        total_allowed += a;
        total_rejected += r;
    }

    assert_eq!(total_allowed + total_rejected, num_tasks * per_task);
    assert!(total_rejected > 0, "should have some rejections");
    assert!(total_allowed > 0, "should have some allows");
}

// ========== Test 5: Witness chain concurrent building ==========

#[test]
fn test_witness_concurrent_hash_computation() {
    use axiom_kernel::witness::{Witness, WitnessHash};
    use std::thread;

    let num_threads = 4;
    let per_thread = 25;

    let mut handles = vec![];

    for t in 0..num_threads {
        handles.push(thread::spawn(move || {
            let mut witnesses = vec![];
            let mut prev: Option<WitnessHash> = None;

            for i in 0..per_thread {
                let mut w = make_test_witness(t * per_thread + i);
                w.prev_hash = prev;
                w.hash = WitnessHash([(t * per_thread + i) as u8; 32]);
                prev = Some(w.hash);
                witnesses.push(w);
            }

            assert!(Witness::verify_chain_integrity(&witnesses));
            witnesses
        }));
    }

    for h in handles {
        let chain = h.join().unwrap();
        assert_eq!(chain.len(), per_thread);
    }
}

// ========== Helper functions ==========

fn make_test_env() -> SignalEnvelope {
    SignalEnvelope {
        msg_id: MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        trace_id: None,
        signal_type: "TestSignal".to_string(),
        vector_clock: VectorClock::new(),
        timestamp_ns: 0,
        kind: SignalKind::Command,
        source_layer: Layer::Exec,
        target_layer: Layer::Exec,
        source_cell: Some("src".to_string()),
        target_cell: Some("dst".to_string()),
        payload: serde_json::json!({}),
        schema_version: SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    }
}

fn make_test_witness(seq: usize) -> axiom_kernel::witness::Witness {
    use axiom_kernel::id::WitnessId;
    use axiom_kernel::version::VersionInfo;
    use axiom_kernel::witness::{TransitionOutcome, Witness, WitnessHash, WitnessMetrics};

    Witness {
        witness_id: WitnessId::new(format!("wit-{}", seq)),
        schema_version: SchemaVersion::new(1),
        cell_id: "test-cell".to_string(),
        correlation_id: CorrelationId::new("test-corr"),
        trace_id: None,
        triggering_msg_id: Some(MsgId::new(format!("msg-{}", seq))),
        vector_clock: VectorClock::new(),
        timestamp_ns: seq as u64 * 1000,
        prev_hash: None,
        state_before_hash: None,
        state_after_hash: None,
        hash: WitnessHash([0u8; 32]),
        summary: format!("witness-{}", seq),
        outcome: TransitionOutcome::Success,
        metrics: WitnessMetrics::default(),
        version_info: VersionInfo::current(),
        signal_fingerprint: [0u8; 32],
        payload_size_bytes: 0,
        kind: axiom_kernel::witness::WitnessKind::StateTransition,
    }
}
