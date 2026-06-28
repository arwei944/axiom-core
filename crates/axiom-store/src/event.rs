//! Event - immutable fact stored in the event log.

use serde::{Deserialize, Serialize};
use axiom_core::signal::VectorClock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub aggregate_id: String,
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
    pub payload: serde_json::Value,
    pub event_type: String,
}
