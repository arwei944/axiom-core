//! Hello Cell example - minimal working example demonstrating Cell, Signal, Witness, Entropy.

use axiom_core::cell::{Cell, CellHandle};
use axiom_core::context::{CellContext, LayeredCellContext, OutgoingEnvelope, OutgoingWitness};
use axiom_core::entropy::EntropyScore;
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::schema::{validators, ValidationResult};
use axiom_core::sealed::ExecLayer;
use axiom_core::signal::{Signal, SignalKind, VectorClock};
use axiom_core::witness::TransitionOutcome;
use axiom_core::Result;
use axiom_core::{axiom, cell, schema_version, Axiom, DynAxiomChain, SignalPayload};
use std::future::Future;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, SignalPayload)]
#[signal(kind = "command", layer = "exec")]
#[schema_version(1)]
#[schema(skip)]
struct HelloCommand {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    message: String,
}

impl HelloCommand {
    fn new(message: &str) -> Self {
        Self {
            msg_id: MsgId::generate(),
            correlation_id: CorrelationId::generate(),
            vector_clock: VectorClock::new(),
            message: message.to_string(),
        }
    }
}

impl axiom_core::Schema for HelloCommand {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::ok();
        result += validators::require_non_empty("message", &self.message);
        result += validators::require_max_length("message", &self.message, 1024);
        result
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, SignalPayload)]
#[signal(kind = "event", layer = "exec")]
#[schema_version(1)]
struct GreetedEvent {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    greeting: String,
}

impl GreetedEvent {
    fn new(correlation_id: CorrelationId, greeting: &str) -> Self {
        Self {
            msg_id: MsgId::generate(),
            correlation_id,
            vector_clock: VectorClock::new(),
            greeting: greeting.to_string(),
        }
    }
}

#[axiom]
struct NonEmptyGreetingAxiom;

impl Axiom for NonEmptyGreetingAxiom {
    type State = Vec<String>;
    type Message = HelloCommand;

    fn name(&self) -> &'static str {
        "non-empty-greeting"
    }

    fn check(&self, _current: &Self::State, new: &Self::State, _msg: &Self::Message) -> Result<()> {
        if new.iter().any(|g| g.is_empty()) {
            return Err(axiom_core::AxiomError::InvariantViolated {
                message: "greeting must not be empty".into(),
            });
        }
        Ok(())
    }

    fn applies_to_layer(&self, layer: Layer) -> bool {
        matches!(layer, Layer::Exec | Layer::Validate)
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

#[cell("exec")]
impl Cell for HelloCell {
    type Message = HelloCommand;

    fn id(&self) -> &CellId {
        &self.id
    }

    #[allow(clippy::manual_async_fn)]
    fn handle<'a>(
        &'a mut self,
        signal: HelloCommand,
        ctx: LayeredCellContext<'a, Self::Layer>,
    ) -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a
    {
        async move {
            let mut ctx = ctx;
            println!("Received: {}", signal.message);
            self.greetings.push(signal.message.clone());

            let event = GreetedEvent::new(signal.correlation_id.clone(), &signal.message);
            let result: Result<()> = (|| {
                ctx.emit_to::<ExecLayer, _>(event)?;
                ctx.emit_witness(
                    ctx.witness()
                        .summary(format!("processed greeting: {}", signal.message))
                        .outcome(TransitionOutcome::Success)
                        .processing_time_us(42)
                )?;
                Ok(())
            })();
            let (outgoing, witnesses) = ctx.end_processing();
            (result, outgoing, witnesses)
        }
    }
}

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

    let signal = HelloCommand::new("Hello, Axiom!");
    assert!(
        axiom_core::Schema::validate(&signal).is_valid(),
        "signal should validate"
    );
    assert_eq!(signal.signal_type(), "HelloCommand");
    assert_eq!(signal.layer(), Layer::Exec);
    assert_eq!(signal.schema_version().0, 1);
    assert_eq!(signal.kind(), SignalKind::Command);

    let layered = ctx.as_layered::<ExecLayer>();
    let (result, _outgoing, witnesses) = cell.handle(signal, layered).await;
    result.unwrap();
    println!("Greetings received: {:?}", cell.greetings);
    assert_eq!(cell.greetings, vec!["Hello, Axiom!"]);
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

    let chain = DynAxiomChain::from_registry_for_layer(Layer::Exec);
    println!("Registered axioms for Exec layer: {}", chain.count());

    let mut entropy = EntropyScore::new();
    assert!(entropy.is_green());
    println!(
        "Initial entropy: {:.3} [{:?}]",
        entropy.compute(),
        entropy.level()
    );

    for _ in 0..3 {
        entropy.record_axiom_violation();
        entropy.record_cell_restart();
        entropy.record_circuit_break();
    }
    println!(
        "After multiple factors: {:.3} [{:?}]",
        entropy.compute(),
        entropy.level()
    );
    assert!(
        entropy.is_red() || entropy.is_critical(),
        "expected red or critical, got {:.3} [{:?}]",
        entropy.value,
        entropy.level()
    );

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
