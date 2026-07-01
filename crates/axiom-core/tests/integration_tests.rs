//! Integration tests for axiom-core - end-to-end verification of core primitives.

use axiom_core::cell::{Cell, CellHandle};
use axiom_core::context::{CellContext, OutgoingEnvelope, OutgoingWitness};
use axiom_core::entropy::{
    EntropyScore, EntropySnapshot, CRITICAL_THRESHOLD, GREEN_THRESHOLD, RED_THRESHOLD,
    YELLOW_THRESHOLD,
};
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::schema::{validators, ValidationResult};
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

    fn layer() -> Layer
    where
        Self: Sized,
    {
        Layer::Exec
    }

    fn handle<'a>(
        &'a mut self,
        signal: TestCommand,
        ctx: &'a mut CellContext<'a>,
    ) -> impl Future<
        Output = (
            axiom_core::Result<()>,
            Vec<OutgoingEnvelope>,
            Vec<OutgoingWitness>,
        ),
    > + Send
           + 'a {
        async move {
            self.state.push(signal.value.clone());

            let event = TestEvent::new(signal.correlation_id.clone(), &signal.value);
            let result: axiom_core::Result<()> = (|| {
                ctx.emit_event(event, Layer::Exec)?;
                ctx.witness()
                    .summary(format!("processed: {}", signal.value))
                    .outcome(TransitionOutcome::Success)
                    .processing_time_us(100)
                    .emit(ctx)?;
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

    let (result, _outgoing, witnesses) = cell.handle(signal, &mut ctx).await;
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
        let (r, _, w) = guard
            .handle(TestCommand::new(&format!("msg-{i}")), &mut ctx)
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

    let _: Result<(), _> = result.into_result();
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
