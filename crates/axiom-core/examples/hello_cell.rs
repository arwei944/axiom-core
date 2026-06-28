//! Hello Cell example - minimal working example demonstrating Cell, Signal, Witness.

use axiom_core::cell::{Cell, ExecCell};
use axiom_core::context::CellContext;
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::schema::{Schema, ValidationResult};
use axiom_core::signal::{Signal, SignalKind, VectorClock, now_ns};
use axiom_core::witness::TransitionOutcome;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HelloSignal {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    message: String,
}

impl Signal for HelloSignal {
    fn signal_type(&self) -> &'static str { "HelloSignal" }
    fn msg_id(&self) -> &MsgId { &self.msg_id }
    fn correlation_id(&self) -> &CorrelationId { &self.correlation_id }
    fn vector_clock(&self) -> &VectorClock { &self.vector_clock }
    fn timestamp_ns(&self) -> u64 { now_ns() }
    fn kind(&self) -> SignalKind { SignalKind::Command }
    fn layer(&self) -> Layer { Layer::Exec }
}

impl Schema for HelloSignal {
    fn validate(&self) -> ValidationResult { ValidationResult::ok() }
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

    fn id(&self) -> &CellId { &self.id }
    fn layer() -> Layer { Layer::Exec }

    async fn handle(&mut self, signal: HelloSignal, ctx: &mut CellContext<'_>) -> axiom_core::Result<()> {
        println!("Received: {}", signal.message);
        self.greetings.push(signal.message.clone());
        ctx.witness()
            .summary(format!("processed greeting: {}", signal.message))
            .outcome(TransitionOutcome::Success)
            .emit(ctx);
        Ok(())
    }
}

impl ExecCell for HelloCell {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Axiom Core - Hello Cell example");

    let mut cell = HelloCell::new();
    let cell_id = CellId::new("hello-cell");
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);

    let signal = HelloSignal {
        msg_id: MsgId::new("msg-001"),
        correlation_id: CorrelationId::new("corr-001"),
        vector_clock: VectorClock::new(),
        message: "Hello, Axiom!".to_string(),
    };

    cell.handle(signal, &mut ctx).await.unwrap();
    println!("Greetings received: {:?}", cell.greetings);

    let witnesses = ctx.take_witnesses();
    println!("Witnesses produced: {}", witnesses.len());
    for w in &witnesses {
        println!("  - {} [{:?}]", w.0.summary, w.0.outcome);
    }
}
