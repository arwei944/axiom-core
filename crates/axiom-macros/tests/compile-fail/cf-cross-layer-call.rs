// Cross-layer calls should fail at compile time due to CanSendTo constraint
use axiom_core::cell::Cell;
use axiom_core::context::{CellContext, LayeredCellContext};
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::schema::ValidationResult;
use axiom_core::sealed::{AgentLayer, ExecLayer};
use axiom_core::signal::{now_ns, Signal, SignalKind, VectorClock};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestSignal {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
}

impl Signal for TestSignal {
    fn signal_type(&self) -> &'static str { "TestSignal" }
    fn msg_id(&self) -> &MsgId { &self.msg_id }
    fn correlation_id(&self) -> &CorrelationId { &self.correlation_id }
    fn vector_clock(&self) -> &VectorClock { &self.vector_clock }
    fn timestamp_ns(&self) -> u64 { now_ns() }
    fn kind(&self) -> SignalKind { SignalKind::Command }
    fn layer(&self) -> Layer { Layer::Exec }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn clone_signal(&self) -> Box<dyn Signal> { Box::new(self.clone()) }
    fn validate(&self) -> ValidationResult { ValidationResult::ok() }
    fn serialize_to_json(&self) -> ::axiom_core::Result<serde_json::Value> {
        serde_json::to_value(self).map_err(|e| ::axiom_core::AxiomError::SignalSerialization {
            signal_type: "TestCmd".into(),
            message: e.to_string(),
        })
    }
}

struct ExecCell {
    id: CellId,
}

impl Cell for ExecCell {
    type Message = TestSignal;
    type Layer = ExecLayer;
    fn id(&self) -> &CellId { &self.id }
    fn handle<'a>(&'a mut self, signal: TestSignal, mut ctx: LayeredCellContext<'a, Self::Layer>) -> impl std::future::Future<Output = (axiom_core::Result<()>, Vec<axiom_core::context::OutgoingEnvelope>, Vec<axiom_core::context::OutgoingWitness>)> + Send + 'a {
        async move {
            // ERROR: ExecLayer cannot send to AgentLayer - Should fail at compile time
            ctx.send_to::<AgentLayer, _>(signal, "agent-cell").unwrap();
            let (outgoing, witnesses) = ctx.end_processing();
            (Ok(()), outgoing, witnesses)
        }
    }
}

fn main() {}
