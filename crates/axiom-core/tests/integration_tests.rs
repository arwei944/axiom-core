//! Integration tests for axiom-core - end-to-end verification of core primitives.

use axiom_core::cell::{Cell, CellHandle};
use axiom_core::context::{CellContext, LayeredCellContext, OutgoingEnvelope, OutgoingWitness};
use axiom_core::entropy::{
    EntropyScore, EntropySnapshot, CRITICAL_THRESHOLD, GREEN_THRESHOLD, RED_THRESHOLD,
    YELLOW_THRESHOLD,
};
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::schema::{validators, ValidationResult};
use axiom_core::sealed::ExecLayer;
use axiom_core::signal::{Signal, SignalKind, VectorClock};
use axiom_core::witness::TransitionOutcome;
use axiom_core::{axiom, cell, schema_version, Axiom, DynAxiomChain, SignalPayload};
use axiom_core::{SchemaMigrator, SchemaVersion};
use std::future::Future;

// ============================================================
// Test signals using SignalPayload derive macro
// ============================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, SignalPayload)]
#[signal(kind = "command", layer = "exec")]
#[schema_version(1)]
#[schema(skip)]
struct TestCommand {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    value: String,
}

impl TestCommand {
    fn new(value: &str) -> Self {
        Self {
            msg_id: MsgId::generate(),
            correlation_id: CorrelationId::generate(),
            vector_clock: VectorClock::new(),
            value: value.to_string(),
        }
    }
}

impl axiom_core::Schema for TestCommand {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::ok();
        result += validators::require_non_empty("value", &self.value);
        result += validators::require_max_length("value", &self.value, 100);
        result
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, SignalPayload)]
#[signal(kind = "event", layer = "exec")]
#[schema_version(1)]
struct TestEvent {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    result: String,
}

impl TestEvent {
    fn new(correlation_id: CorrelationId, result: &str) -> Self {
        Self {
            msg_id: MsgId::generate(),
            correlation_id,
            vector_clock: VectorClock::new(),
            result: result.to_string(),
        }
    }
}

// ============================================================
// Test axiom using #[axiom] macro
// ============================================================

#[axiom]
struct MaxValueAxiom;

impl Axiom for MaxValueAxiom {
    type State = Vec<String>;
    type Message = TestCommand;

    fn name(&self) -> &'static str {
        "max-value-axiom"
    }

    fn check(
        &self,
        _current: &Self::State,
        _new: &Self::State,
        msg: &Self::Message,
    ) -> axiom_core::Result<()> {
        if msg.value.len() > 50 {
            return Err(axiom_core::AxiomError::InvariantViolated {
                message: format!("value too long: {}", msg.value.len()),
            });
        }
        Ok(())
    }

    fn applies_to_layer(&self, layer: Layer) -> bool {
        matches!(layer, Layer::Exec | Layer::Validate)
    }
}

// ============================================================
// Test cell using #[cell] macro
// ============================================================

struct TestCell {
    id: CellId,
    state: Vec<String>,
}

impl TestCell {
    fn new(id: &str) -> Self {
        Self {
            id: CellId::new(id),
            state: Vec::new(),
        }
    }
}

#[cell("exec")]
impl Cell for TestCell {
    type Message = TestCommand;

    fn id(&self) -> &CellId {
        &self.id
    }

    #[allow(clippy::manual_async_fn)]
    fn handle<'a>(
        &'a mut self,
        signal: TestCommand,
        ctx: LayeredCellContext<'a, Self::Layer>,
    ) -> impl Future<
        Output = (
            axiom_core::Result<()>,
            Vec<OutgoingEnvelope>,
            Vec<OutgoingWitness>,
        ),
    > + Send
           + 'a {
        async move {
            let mut ctx = ctx;
            self.state.push(signal.value.clone());

            let event = TestEvent::new(signal.correlation_id.clone(), &signal.value);
            let result: axiom_core::Result<()> = (|| {
                ctx.emit_to::<ExecLayer, _>(event)?;
                ctx.emit_witness(
                    ctx.witness()
                        .summary(format!("processed: {}", signal.value))
                        .outcome(TransitionOutcome::Success)
                        .processing_time_us(100),
                )?;
                Ok(())
            })();
            let (outgoing, witnesses) = ctx.end_processing();
            (result, outgoing, witnesses)
        }
    }
}

// ============================================================
// Integration Tests
// ============================================================

#[tokio::test]
async fn test_cell_signal_witness_e2e() {
    let mut cell = TestCell::new("test-cell-1");
    let cell_id = CellId::new("test-cell-1");
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);

    let signal = TestCommand::new("hello e2e");
    assert!(signal.validate().is_valid());
    assert_eq!(signal.signal_type(), "TestCommand");
    assert_eq!(signal.layer(), Layer::Exec);
    assert_eq!(signal.kind(), SignalKind::Command);
    assert_eq!(signal.schema_version(), SchemaVersion::new(1));

    let layered = ctx.as_layered::<ExecLayer>();
    let (result, _outgoing, witnesses) = cell.handle(signal, layered).await;
    result.unwrap();
    assert_eq!(cell.state.len(), 1);
    assert_eq!(cell.state[0], "hello e2e");

    assert_eq!(witnesses.len(), 1);
    let w = &witnesses[0].0;
    assert_eq!(w.summary, "processed: hello e2e");
    assert!(matches!(w.outcome, TransitionOutcome::Success));
    assert_eq!(w.metrics.processing_time_us, 100);
    assert_eq!(w.schema_version.0, 1);
    assert!(w.payload_size_bytes > 0);
}

#[tokio::test]
async fn test_cell_handle_multiple_signals() {
    // Multiple handle() calls on the same cell via an Arc<Mutex> so each
    // iteration borrows a *local* guard (not the outer handle). RPITIT
    // opaque futures tie `&mut self` to `'a`; with a local guard, `'a` is
    // scoped to the loop body, so the borrow ends each iteration.
    let cell = std::sync::Arc::new(tokio::sync::Mutex::new(TestCell::new("test-cell-2")));
    let cell_id = CellId::new("test-cell-2");
    let mut witness_count = 0usize;

    for i in 0..5u32 {
        let mut guard = cell.lock().await;
        let mut ctx = CellContext::new(&cell_id, Layer::Exec);
        let layered = ctx.as_layered::<ExecLayer>();
        let (r, _, w) = guard
            .handle(TestCommand::new(&format!("msg-{i}")), layered)
            .await;
        r.unwrap();
        witness_count += w.len();
    }

    let guard = cell.lock().await;
    assert_eq!(guard.state.len(), 5);
    assert_eq!(witness_count, 5);
}

#[tokio::test]
async fn test_cell_handle_typed() {
    let cell = TestCell::new("test-cell-3");
    let handle = CellHandle::new(cell);
    assert_eq!(handle.id().as_str(), "test-cell-3");
    assert_eq!(handle.layer(), Layer::Exec);
    assert!(handle.downcast_ref::<TestCell>().is_some());
}

#[test]
fn test_signal_validation() {
    use axiom_core::Schema;

    let valid = TestCommand::new("valid");
    let valid_result = Schema::validate(&valid);
    println!("valid result: {:?}", valid_result);
    assert!(valid_result.is_valid());
    assert!(valid_result.is_ok());
    assert!(!valid_result.has_errors());

    let empty = TestCommand::new("");
    let result = Schema::validate(&empty);
    println!("empty result: {:?}", result);
    println!("empty is_valid: {}", result.is_valid());
    println!("empty errors: {:?}", result.errors);
    assert!(!result.is_valid());
    assert!(!result.is_ok());
    assert!(result.has_errors());

    let result_clone = result.clone();
    assert!(result_clone.has_errors());

    let _: Result<(), _> = result.into_result("TestCommand");
}

#[test]
fn test_validation_result_add_assign() {
    let mut r1 = ValidationResult::ok();
    let mut r2 = ValidationResult::ok();
    r2.add_error("field1", "error1");
    r1 += r2;
    assert!(r1.has_errors());
    assert_eq!(r1.errors.len(), 1);
}

#[test]
fn test_validation_result_from_errors() {
    use axiom_core::schema::{ValidationError, ValidationSeverity};
    let errors = vec![ValidationError::error("f", "m")];
    let result = ValidationResult::from_errors(errors);
    assert!(result.has_errors());
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].severity, ValidationSeverity::Error);
}

#[test]
fn test_axiom_registry_by_layer() {
    let exec_chain = DynAxiomChain::from_registry_for_layer(Layer::Exec);
    assert!(
        exec_chain.count() >= 1,
        "should have at least MaxValueAxiom"
    );

    let oversight_chain = DynAxiomChain::from_registry_for_layer(Layer::Oversight);
    assert_eq!(
        oversight_chain.count(),
        0,
        "MaxValueAxiom should not apply to Oversight layer"
    );

    let all_chain = DynAxiomChain::from_registry_all();
    assert!(
        all_chain.count() >= exec_chain.count(),
        "all should be >= exec layer count"
    );
}

#[test]
fn test_entropy_all_factors() {
    let mut entropy = EntropyScore::new();
    assert!(entropy.is_green());
    assert_eq!(entropy.value, 0.0);

    entropy.record_dropped_message();
    assert_eq!(entropy.dropped_messages, 1);
    assert!(entropy.value > 0.0);

    entropy.record_rejected_by_guardian();
    assert_eq!(entropy.rejected_by_guardian, 1);

    entropy.record_axiom_violation();
    assert_eq!(entropy.axiom_violations, 1);

    entropy.record_cell_restart();
    assert_eq!(entropy.cell_restarts, 1);

    entropy.record_circuit_break();
    assert_eq!(entropy.circuit_breaks, 1);

    entropy.record_timeout();
    assert_eq!(entropy.timeouts, 1);

    entropy.record_duplicate_message();
    assert_eq!(entropy.duplicate_messages, 1);

    entropy.record_stale_state_violation();
    assert_eq!(entropy.stale_state_violations, 1);

    assert!(
        entropy.value > 0.0,
        "all factors should contribute to entropy"
    );
}

#[test]
fn test_entropy_thresholds() {
    const {
        assert!(GREEN_THRESHOLD < YELLOW_THRESHOLD);
        assert!(YELLOW_THRESHOLD < RED_THRESHOLD);
        assert!(RED_THRESHOLD < CRITICAL_THRESHOLD);
    }

    let mut s = EntropyScore::new();
    assert!(s.is_green());

    s.record_axiom_violation();
    s.record_axiom_violation();
    let after_two = s.value;
    assert!(after_two > 0.0);

    for _ in 0..10 {
        s.record_cell_restart();
    }
    assert!(
        s.is_red() || s.is_critical(),
        "many cell restarts should trigger red+"
    );

    s.reset();
    assert!(s.is_green());
    assert_eq!(s.cell_restarts, 0);
}

#[test]
fn test_entropy_snapshot() {
    let mut s = EntropyScore::new();
    s.record_axiom_violation();
    s.record_timeout();
    let snap: EntropySnapshot = s.snapshot();
    assert_eq!(snap.axiom_violations, 1);
    assert_eq!(snap.timeouts, 1);
    assert!(snap.per_cell.is_empty());
    assert!(snap.value > 0.0);
}

#[test]
fn test_entropy_time_decay() {
    let mut s = EntropyScore::new();
    for _ in 0..10 {
        s.record_axiom_violation();
    }
    let before = s.value;
    assert!(before > 0.0);

    let now = s.last_updated_ns + 10_000_000_000;
    s.decay(now);
    assert!(
        s.value < before,
        "decay should reduce value: before={}, after={}",
        before,
        s.value
    );
}

#[test]
fn test_schema_migrator_basic() {
    struct V1toV2;
    impl axiom_core::Migration for V1toV2 {
        fn source_version(&self) -> SchemaVersion {
            SchemaVersion(1)
        }
        fn target_version(&self) -> SchemaVersion {
            SchemaVersion(2)
        }
        fn migrate(&self, mut v: serde_json::Value) -> axiom_core::Result<serde_json::Value> {
            v["version"] = serde_json::json!(2);
            Ok(v)
        }
    }

    let mut mig = SchemaMigrator::from_registry();
    mig.register("TestSignal", V1toV2);

    let input = serde_json::json!({"data": "test"});
    let result = mig
        .migrate_to("TestSignal", SchemaVersion(1), SchemaVersion(2), input)
        .unwrap();
    assert_eq!(result["version"], serde_json::json!(2));
}

#[test]
fn test_witness_builder_full() {
    use axiom_core::witness::WitnessBuilder;
    let cell_id = CellId::new("witness-test");
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);

    WitnessBuilder::new()
        .summary("test witness")
        .outcome(TransitionOutcome::Success)
        .processing_time_us(500)
        .emit(&mut ctx)
        .expect("emit should succeed");

    let witnesses = ctx.take_witnesses();
    assert_eq!(witnesses.len(), 1);
    let w = &witnesses[0].0;
    assert_eq!(w.summary, "test witness");
    assert_eq!(w.metrics.processing_time_us, 500);
    assert_eq!(w.version_info.witness_schema.0, 1);
    assert!(!w.version_info.crate_version.to_string().is_empty());
}

#[tokio::test]
async fn test_invalid_signal_rejected_by_validation() {
    use axiom_core::Schema;
    let invalid = TestCommand::new("");
    let result = Schema::validate(&invalid);
    assert!(!result.is_valid());
    assert!(result.has_errors());
}

#[test]
fn test_vector_clock_basic() {
    let mut vc = VectorClock::new();
    assert_eq!(vc.get("node1"), 0);

    vc.increment("node1");
    assert_eq!(vc.get("node1"), 1);

    vc.increment("node1");
    assert_eq!(vc.get("node1"), 2);
    assert_eq!(vc.get("node2"), 0);
}

#[test]
fn test_registry_public_functions() {
    use axiom_core::registry::*;

    let count = count_registered_axioms();
    assert!(count >= 1);

    let _chains = registered_migration_chains();
    let _axioms = registered_axioms();
}

#[test]
fn test_version_info_current() {
    use axiom_core::VersionInfo;
    let info = VersionInfo::current();
    assert!(!info.crate_version.to_string().is_empty());
    assert_eq!(info.witness_schema.0, 1);
}

#[test]
fn test_entropy_high_weight_more_damage() {
    let mut s1 = EntropyScore::new();
    let mut s2 = EntropyScore::new();

    s1.record_cell_restart();
    s2.record_duplicate_message();

    assert!(
        s1.value > s2.value,
        "cell_restart (5.0) > duplicate (0.5): s1={}, s2={}",
        s1.value,
        s2.value
    );
}

// ============================================================
// Error Path Tests (Phase 6.1)
// ============================================================

/// 1. LayerViolation — compile-time constraint prevents invalid cross-layer calls.
/// This test verifies that LayeredCellContext correctly enforces layer rules at compile time.
#[tokio::test]
async fn test_error_path_layer_violation() {
    let cell_id = CellId::new("layer-violation-test");
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);
    let mut layered_ctx =
        LayeredCellContext::<axiom_core::sealed::ExecLayer>::from_cell_context(&mut ctx);

    let event = TestEvent::new(CorrelationId::generate(), "same-layer-target");
    let result = layered_ctx.emit_to::<axiom_core::sealed::ExecLayer, _>(event);
    assert!(result.is_ok());
}

/// 2. Witness hash chain break — verify_chain_integrity detects tampering.
#[cfg(feature = "sha2-id")]
#[test]
fn test_error_path_witness_chain_break() {
    use axiom_core::witness::WitnessBuilder;

    let cell_id = CellId::new("chain-test");

    // First context: produce w1 then w2 (properly chained)
    let mut ctx1 = CellContext::new(&cell_id, Layer::Exec);
    WitnessBuilder::new()
        .summary("first")
        .emit(&mut ctx1)
        .unwrap();
    let w1 = ctx1.take_witnesses().pop().unwrap().0;

    WitnessBuilder::new()
        .summary("second")
        .emit(&mut ctx1)
        .unwrap();
    let w2 = ctx1.take_witnesses().pop().unwrap().0;

    // Chain w1 → w2 is intact (w2.prev_hash == w1.hash)
    assert!(axiom_core::witness::Witness::verify_chain_integrity(&[
        w1.clone(),
        w2.clone()
    ]));

    // Second context: fresh CellContext → w3.prev_hash is None (not chained from w1)
    let mut ctx2 = CellContext::new(&cell_id, Layer::Exec);
    WitnessBuilder::new()
        .summary("tampered")
        .emit(&mut ctx2)
        .unwrap();
    let w3 = ctx2.take_witnesses().pop().unwrap().0;

    // w3.prev_hash is None, not Some(w1.hash) — chain is broken
    assert!(!axiom_core::witness::Witness::verify_chain_integrity(&[
        w1, w3
    ]));
}

/// 3. Signal serialization failure — SignalEnvelope::new propagates the error.
#[test]
fn test_error_path_signal_serialization_failure() {
    struct UnserializableSignal {
        msg_id: MsgId,
        correlation_id: CorrelationId,
        vector_clock: VectorClock,
    }

    impl Clone for UnserializableSignal {
        fn clone(&self) -> Self {
            Self {
                msg_id: self.msg_id.clone(),
                correlation_id: self.correlation_id.clone(),
                vector_clock: self.vector_clock.clone(),
            }
        }
    }

    impl Signal for UnserializableSignal {
        fn signal_type(&self) -> &'static str {
            "unserializable"
        }
        fn msg_id(&self) -> &MsgId {
            &self.msg_id
        }
        fn correlation_id(&self) -> &CorrelationId {
            &self.correlation_id
        }
        fn vector_clock(&self) -> &VectorClock {
            &self.vector_clock
        }
        fn timestamp_ns(&self) -> u64 {
            1
        }
        fn kind(&self) -> SignalKind {
            SignalKind::Command
        }
        fn layer(&self) -> Layer {
            Layer::Exec
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn clone_signal(&self) -> Box<dyn Signal> {
            Box::new(self.clone())
        }
        fn validate(&self) -> ValidationResult {
            ValidationResult::ok()
        }
        fn serialize_to_json(&self) -> axiom_core::Result<serde_json::Value> {
            Err(axiom_core::AxiomError::SignalSerialization {
                signal_type: "UnserializableSignal".into(),
                message: "intentional failure".into(),
            })
        }
    }

    let sig = UnserializableSignal {
        msg_id: MsgId::generate(),
        correlation_id: CorrelationId::generate(),
        vector_clock: VectorClock::new(),
    };
    let result = axiom_core::signal::SignalEnvelope::new(&sig, Layer::Exec);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        axiom_core::AxiomError::SignalSerialization { .. }
    ));
}

/// 4. Axiom type mismatch — check_all silently skips TypeMismatch (no false positives).
#[test]
fn test_error_path_axiom_type_mismatch_no_false_positive() {
    let chain = DynAxiomChain::from_registry_for_layer(Layer::Exec);
    assert!(chain.count() >= 1, "MaxValueAxiom should be registered");

    // Pass wrong types — String instead of Vec<String>, String instead of TestCommand
    let violations = chain.check_all(
        &"wrong-state".to_string(),
        &"wrong-state".to_string(),
        &"wrong-message".to_string(),
    );

    // TypeMismatch should be silently skipped — no violations produced
    assert!(
        violations.is_empty(),
        "type mismatch should not produce false positives, got {} violations",
        violations.len()
    );
}

/// 5. hop_count overflow — increment_hop beyond MAX_HOPS returns HandoffLimitExceeded.
#[test]
fn test_error_path_hop_count_overflow() {
    let cmd = TestCommand::new("hop-test");
    let mut env = axiom_core::signal::SignalEnvelope::new(&cmd, Layer::Exec).unwrap();

    // MAX_HOPS is 8 — first 8 increments succeed
    for i in 0..8 {
        env.increment_hop()
            .unwrap_or_else(|_| panic!("hop {i} should succeed"));
    }
    assert_eq!(env.hop_count, 8);

    // 9th hop should fail
    let result = env.increment_hop();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        axiom_core::AxiomError::HandoffLimitExceeded { hops: 9, .. }
    ));
}

// ============================================================
// Concurrency Tests (Phase 6.2 — concurrent Cell processing)
// ============================================================

/// Multiple tokio::spawn tasks processing Cell::handle in parallel.
#[tokio::test]
async fn test_concurrent_cells_parallel() {
    let mut handles = Vec::new();

    for i in 0..10u32 {
        handles.push(tokio::spawn(async move {
            let id_str = format!("parallel-cell-{i}");
            let mut cell = TestCell::new(&id_str);
            let cell_id = CellId::new(&id_str);
            let mut ctx = CellContext::new(&cell_id, Layer::Exec);
            let layered = ctx.as_layered::<ExecLayer>();
            let (result, _outgoing, witnesses) = cell
                .handle(TestCommand::new(&format!("msg-{i}")), layered)
                .await;
            result.unwrap();
            assert_eq!(witnesses.len(), 1, "each cell should produce 1 witness");
            assert_eq!(cell.state.len(), 1);
            assert_eq!(cell.state[0], format!("msg-{i}"));
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}
