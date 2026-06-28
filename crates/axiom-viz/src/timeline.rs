//! Witness timeline data for time-travel debugging.

use serde::Serialize;
use axiom_core::layer::Layer;

#[derive(Debug, Serialize)]
pub struct TimelineEntry {
    pub cell_id: String,
    pub layer: Layer,
    pub timestamp_ns: u64,
    pub summary: String,
    pub outcome: String,
}

#[derive(Debug, Serialize)]
pub struct Timeline {
    pub entries: Vec<TimelineEntry>,
}
