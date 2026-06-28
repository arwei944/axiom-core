use axiom_core::cell::Cell;
use axiom_core::context::CellContext;
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::schema::ValidationResult;
use axiom_core::signal::{now_ns, Signal, SignalKind, VectorClock};
use axiom_core::version::{Migration, SchemaVersion, Versioned};
use axiom_core::{axiom::Axiom, AxiomError, Result};
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
    fn timestamp_ns(&self) -> u64 { now_ns() }
    fn kind(&self) -> SignalKind { SignalKind::Command }
    fn layer(&self) -> Layer { Layer::Exec }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn clone_signal(&self) -> Box<dyn Signal> { Box::new(self.clone()) }
    fn validate(&self) -> ValidationResult { ValidationResult::ok() }
    fn serialize_to_json(&self) -> serde_json::Value { serde_json::to_value(self).unwrap_or(serde_json::Value::Null) }
}

struct PassCell {
    id: CellId,
}

#[axiom_macros::cell("validate")]
impl Cell for PassCell {
    type Message = PassCmd;
    fn id(&self) -> &CellId { &self.id }
    fn layer() -> Layer { Layer::Validate }
    async fn handle(&mut self, _: PassCmd, _: &mut CellContext<'_>) -> axiom_core::Result<()> { Ok(()) }
}

fn _assert_validate_cell() {
    fn assert_validate<T: axiom_core::cell::ValidateCell>() {}
    assert_validate::<PassCell>();
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
    fn timestamp_ns(&self) -> u64 { now_ns() }
    fn kind(&self) -> SignalKind { SignalKind::Event }
    fn layer(&self) -> Layer { Layer::Exec }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn clone_signal(&self) -> Box<dyn Signal> { Box::new(self.clone()) }
    fn validate(&self) -> ValidationResult { ValidationResult::ok() }
    fn serialize_to_json(&self) -> serde_json::Value { serde_json::to_value(self).unwrap_or(serde_json::Value::Null) }
}

#[derive(Debug)]
struct PassMigration;

#[axiom_macros::migration(from = 2)]
impl Migration for PassMigration {
    fn migrate(&self, input: Value) -> Result<Value> { Ok(input) }
}

#[axiom_macros::axiom]
#[derive(Default)]
struct PassAxiom;

impl Axiom for PassAxiom {
    type State = i32;
    type Message = PassCmd;
    fn name(&self) -> &'static str { "PassAxiom" }
    fn check(&self, _current: &i32, new: &i32, _msg: &PassCmd) -> Result<()> {
        if *new < 0 {
            Err(AxiomError::InvariantViolated { message: "negative".into() })
        } else {
            Ok(())
        }
    }
}

fn main() {
    assert_eq!(<V3Signal as Versioned>::schema_version(), SchemaVersion::new(3));
    let _a = PassAxiom::default();
}
