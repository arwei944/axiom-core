//! Hello Cell example - minimal working example demonstrating Cell, Signal, Witness, Entropy.

use axiom_core::cell::{Cell, CellHandle, ExecCell};
use axiom_core::context::CellContext;
use axiom_core::entropy::EntropyScore;
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::schema::{validators, ValidationResult};
use axiom_core::signal::{Signal, SignalKind, VectorClock};
use axiom_core::witness::TransitionOutcome;
use axiom_core::Result;

#[derive(Debug, Clone)]
struct HelloSignal {
    message: String,
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
}

impl HelloSignal {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            msg_id: MsgId::generate(),
            correlation_id: CorrelationId::new("hello-correlation"),
            vector_clock: VectorClock::new(),
        }
    }
}

impl Signal for HelloSignal {
    fn signal_type(&self) -> &'static str {
        "HelloSignal"
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
        axiom_core::signal::now_ns()
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
        let mut result = ValidationResult::ok();
        result.merge(validators::require_non_empty("message", &self.message));
        result.merge(validators::require_max_length(
            "message",
            &self.message,
            1024,
        ));
        result
    }
    fn serialize_to_json(&self) -> serde_json::Value {
        serde_json::json!({"message": self.message})
    }
}

struct HelloCell {
    id: CellId,
    greetings: Vec<String>,
}

impl HelloCell {
    fn new() -> Self {
        Self {
            id: CellId::new("hello-cell"),
            greetings: Vec::new(),
        }
    }
}

impl Cell for HelloCell {
    type Message = HelloSignal;

    fn id(&self) -> &CellId {
        &self.id
    }
    fn layer() -> Layer {
        Layer::Exec
    }

    async fn handle(&mut self, signal: HelloSignal, ctx: &mut CellContext<'_>) -> Result<()> {
        println!("Received: {}", signal.message);
        self.greetings.push(signal.message.clone());
        ctx.witness()
            .summary(format!("processed greeting: {}", signal.message))
            .outcome(TransitionOutcome::Success)
            .processing_time_us(42)
            .emit(ctx);
        Ok(())
    }
}

impl ExecCell for HelloCell {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Axiom Core - Hello Cell example");

    let cell = HelloCell::new();
    let handle = CellHandle::new(cell);
    println!("Cell ID: {}", handle.id());
    println!("Cell Layer: {:?}", handle.layer());
    assert!(handle.downcast_ref::<HelloCell>().is_some());

    let mut cell = HelloCell::new();
    let cell_id = CellId::new("hello-cell");
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);

    let signal = HelloSignal::new("Hello, Axiom!");
    assert!(signal.validate().is_valid(), "signal should validate");
    assert_eq!(signal.signal_type(), "HelloSignal");
    assert_eq!(signal.layer(), Layer::Exec);

    cell.handle(signal, &mut ctx).await.unwrap();
    println!("Greetings received: {:?}", cell.greetings);
    assert_eq!(cell.greetings, vec!["Hello, Axiom!"]);

    let witnesses = ctx.take_witnesses();
    println!("Witnesses produced: {}", witnesses.len());
    for w in &witnesses {
        println!(
            "  - {} [{:?}] (schema v{}, took {}us)",
            w.0.summary, w.0.outcome, w.0.schema_version.0, w.0.metrics.processing_time_us
        );
        println!(
            "    version: crate={}, witness_schema=v{}",
            w.0.version_info.crate_version, w.0.version_info.witness_schema.0
        );
        println!(
            "    payload: {} bytes, fingerprint: {:02x}{:02x}{:02x}...",
            w.0.payload_size_bytes,
            w.0.signal_fingerprint[0],
            w.0.signal_fingerprint[1],
            w.0.signal_fingerprint[2]
        );
    }
    assert_eq!(witnesses.len(), 1);
    assert!(matches!(witnesses[0].0.outcome, TransitionOutcome::Success));

    let mut entropy = EntropyScore::new();
    assert!(entropy.is_green());
    println!(
        "Initial entropy: {:.3} [{:?}]",
        entropy.compute(),
        entropy.level()
    );

    for _ in 0..20 {
        entropy.record_axiom_violation();
        entropy.record_witness_anomaly();
        entropy.record_message_loop();
        entropy.record_intent_drift(0.3);
    }
    println!(
        "After multiple factors: {:.3} [{:?}]",
        entropy.compute(),
        entropy.level()
    );
    assert!(entropy.is_red());

    entropy.reset();
    println!(
        "After reset: {:.3} [{:?}]",
        entropy.compute(),
        entropy.level()
    );
    assert!(entropy.is_green());

    let snap = entropy.snapshot();
    println!("Snapshot level: {:?}, value: {:.3}", snap.level, snap.value);

    println!("\n=== All P1 core primitives verified successfully! ===");
}
