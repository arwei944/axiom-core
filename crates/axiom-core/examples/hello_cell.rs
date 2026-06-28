//! Hello Cell example - minimal working example.

use axiom_core::cell::{Cell, CellId};
use axiom_core::signal::{Signal, SignalKind, VectorClock};
use axiom_core::Result;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct HelloSignal {
    msg_id: String,
    correlation_id: String,
    vector_clock: VectorClock,
    timestamp_ns: u64,
    message: String,
}

impl Signal for HelloSignal {
    fn signal_type(&self) -> &'static str { "HelloSignal" }
    fn msg_id(&self) -> &str { &self.msg_id }
    fn correlation_id(&self) -> &str { &self.correlation_id }
    fn vector_clock(&self) -> &VectorClock { &self.vector_clock }
    fn timestamp_ns(&self) -> u64 { self.timestamp_ns }
    fn kind(&self) -> SignalKind { SignalKind::Command }
}

struct HelloCell {
    id: CellId,
    greetings: Vec<String>,
}

impl HelloCell {
    fn new() -> Self {
        Self {
            id: CellId("hello-cell".to_string()),
            greetings: Vec::new(),
        }
    }
}

#[async_trait::async_trait]
impl Cell for HelloCell {
    type Message = HelloSignal;

    fn id(&self) -> &CellId { &self.id }

    async fn handle(&mut self, signal: HelloSignal) -> Result<()> {
        println!("Received: {}", signal.message);
        self.greetings.push(signal.message);
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Axiom Core - Hello Cell example");

    let mut cell = HelloCell::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let signal = HelloSignal {
        msg_id: "msg-001".to_string(),
        correlation_id: "corr-001".to_string(),
        vector_clock: VectorClock::new(),
        timestamp_ns: now,
        message: "Hello, Axiom!".to_string(),
    };

    cell.handle(signal).await.unwrap();
    println!("Greetings received: {:?}", cell.greetings);
}
