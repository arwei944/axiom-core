//! Event - immutable fact stored in the event log.

use axiom_core::id::{CorrelationId, MsgId};
use axiom_core::signal::VectorClock;
use axiom_core::version::{EventSchema, SchemaVersion, Versioned};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub aggregate_id: String,
    pub cell_id: String,
    pub correlation_id: CorrelationId,
    pub triggering_msg_id: Option<MsgId>,
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
    pub payload: serde_json::Value,
    pub event_type: String,
    pub schema_version: SchemaVersion,
}

impl Event {
    pub fn new(aggregate_id: &str, event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            event_id: format!("evt-{}", axiom_core::signal::now_ns()),
            aggregate_id: aggregate_id.to_string(),
            cell_id: String::new(),
            correlation_id: CorrelationId::new("system"),
            triggering_msg_id: None,
            vector_clock: VectorClock::new(),
            timestamp_ns: axiom_core::signal::now_ns(),
            payload,
            event_type: event_type.to_string(),
            schema_version: EventSchema::schema_version(),
        }
    }
}
