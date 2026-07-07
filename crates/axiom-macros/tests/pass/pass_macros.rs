use axiom_kernel::axiom::Axiom;
use axiom_kernel::id::{CellId, CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::signal::{Signal, SignalKind, VectorClock};
use axiom_kernel::version::{Migration, SchemaVersion, Versioned};
use axiom_kernel::{KernelError, KernelResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PassCmd {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
}

impl Signal for PassCmd {
    fn signal_type(&self) -> &'static str { "PassCmd" }
    fn msg_id(&self) -> &MsgId { &self.msg_id }
    fn correlation_id(&self) -> &CorrelationId { &self.correlation_id }
    fn vector_clock(&self) -> &VectorClock { &self.vector_clock }
    fn timestamp_ns(&self) -> u64 { 0 }
    fn kind(&self) -> SignalKind { SignalKind::Command }
    fn layer(&self) -> Layer { Layer::Exec }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn clone_signal(&self) -> Box<dyn Signal> { Box::new(self.clone()) }
    fn validate(&self) -> axiom_kernel::axiom::ValidationResult { axiom_kernel::axiom::ValidationResult::ok() }
    fn serialize_to_json(&self) -> KernelResult<serde_json::Value> { serde_json::to_value(self).map_err(|e| KernelError::SerializationError(e.to_string())) }
}

#[axiom_macros::cell("validate")]
struct PassCell {
    id: CellId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[axiom_macros::schema_version(3)]
struct V3Signal {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
}

impl Signal for V3Signal {
    fn signal_type(&self) -> &'static str { "V3Signal" }
    fn msg_id(&self) -> &MsgId { &self.msg_id }
    fn correlation_id(&self) -> &CorrelationId { &self.correlation_id }
    fn vector_clock(&self) -> &VectorClock { &self.vector_clock }
    fn timestamp_ns(&self) -> u64 { 0 }
    fn kind(&self) -> SignalKind { SignalKind::Event }
    fn layer(&self) -> Layer { Layer::Exec }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn clone_signal(&self) -> Box<dyn Signal> { Box::new(self.clone()) }
    fn validate(&self) -> axiom_kernel::axiom::ValidationResult { axiom_kernel::axiom::ValidationResult::ok() }
    fn serialize_to_json(&self) -> KernelResult<serde_json::Value> { serde_json::to_value(self).map_err(|e| KernelError::SerializationError(e.to_string())) }
}

#[derive(Debug)]
struct PassMigration;

#[axiom_macros::migration(from = 2)]
impl Migration for PassMigration {
    fn migrate(&self, input: Value) -> KernelResult<Value> { Ok(input) }
}

#[axiom_macros::axiom]
#[derive(Default)]
struct PassAxiom;

impl Axiom for PassAxiom {
    type State = i32;
    type Message = PassCmd;
    fn name(&self) -> &'static str { "PassAxiom" }
    fn check(&self, _current: &i32, new: &i32, _msg: &PassCmd) -> KernelResult<()> {
        if *new < 0 {
            Err(KernelError::InvariantViolated { message: "negative".into() })
        } else {
            Ok(())
        }
    }
}

fn main() {
    assert_eq!(<V3Signal as Versioned>::schema_version(), SchemaVersion::new(3));
    let _a = PassAxiom::default();
}