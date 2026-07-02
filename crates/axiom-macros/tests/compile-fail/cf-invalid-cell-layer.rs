// Invalid layer name in cell macro should cause compile error
#[axiom_macros::cell("invalid_layer")]
impl axiom_core::cell::Cell for BogusCell {
    type Message = BogusSignal;
    fn id(&self) -> &axiom_core::id::CellId { loop {} }
    fn layer() -> axiom_core::layer::Layer { axiom_core::layer::Layer::Exec }
    fn handle<'a>(&'a mut self, _: BogusSignal, _: &'a mut axiom_core::context::CellContext<'a>) -> impl std::future::Future<Output = axiom_core::Result<()>> + Send + 'a { async { Ok(()) } }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct BogusSignal {
    msg_id: axiom_core::id::MsgId,
    correlation_id: axiom_core::id::CorrelationId,
    vector_clock: axiom_core::signal::VectorClock,
}
impl axiom_core::signal::Signal for BogusSignal {
    fn signal_type(&self) -> &'static str { "Bogus" }
    fn msg_id(&self) -> &axiom_core::id::MsgId { &self.msg_id }
    fn correlation_id(&self) -> &axiom_core::id::CorrelationId { &self.correlation_id }
    fn vector_clock(&self) -> &axiom_core::signal::VectorClock { &self.vector_clock }
    fn timestamp_ns(&self) -> u64 { 0 }
    fn kind(&self) -> axiom_core::signal::SignalKind { axiom_core::signal::SignalKind::Command }
    fn layer(&self) -> axiom_core::layer::Layer { axiom_core::layer::Layer::Exec }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn clone_signal(&self) -> Box<dyn axiom_core::signal::Signal> { Box::new(self.clone()) }
    fn validate(&self) -> axiom_core::schema::ValidationResult { axiom_core::schema::ValidationResult::ok() }
    fn serialize_to_json(&self) -> ::axiom_core::Result<serde_json::Value> { serde_json::to_value(self).map_err(|e| ::axiom_core::AxiomError::SignalSerialization { signal_type: "TestCmd".into(), message: e.to_string() }) }
}
struct BogusCell;

fn main() {}
