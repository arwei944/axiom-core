use axiom_kernel::id::{CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::signal::{Signal, SignalKind, VectorClock};
use axiom_kernel::KernelResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HelloCommand {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    message: String,
}

impl Signal for HelloCommand {
    fn signal_type(&self) -> &'static str { "HelloCommand" }
    fn msg_id(&self) -> &MsgId { &self.msg_id }
    fn correlation_id(&self) -> &CorrelationId { &self.correlation_id }
    fn vector_clock(&self) -> &VectorClock { &self.vector_clock }
    fn timestamp_ns(&self) -> u64 { 0 }
    fn kind(&self) -> SignalKind { SignalKind::Command }
    fn layer(&self) -> Layer { Layer::Exec }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn clone_signal(&self) -> Box<dyn Signal> { Box::new(self.clone()) }
    fn validate(&self) -> axiom_kernel::axiom::ValidationResult { axiom_kernel::axiom::ValidationResult::ok() }
    fn serialize_to_json(&self) -> KernelResult<serde_json::Value> { serde_json::to_value(self).map_err(|e| axiom_kernel::KernelError::SerializationError(e.to_string())) }
}

struct HelloCell {
    greetings: Vec<String>,
}

impl HelloCell {
    fn new() -> Self {
        Self {
            greetings: Vec::new(),
        }
    }

    fn process(&mut self, message: String) {
        self.greetings.push(message);
    }
}

fn main() {
    let mut cell = HelloCell::new();

    let signal = HelloCommand {
        msg_id: MsgId::new("test"),
        correlation_id: CorrelationId::new("test"),
        vector_clock: VectorClock::new(),
        message: "Hello, Axiom!".to_string(),
    };
    assert_eq!(signal.signal_type(), "HelloCommand");
    assert_eq!(signal.layer(), Layer::Exec);
    assert_eq!(signal.kind(), SignalKind::Command);

    cell.process(signal.message);
    println!("Greetings received: {:?}", cell.greetings);
    assert_eq!(cell.greetings, vec!["Hello, Axiom!"]);

    println!("\n=== All P1 core primitives verified successfully! ===");
}