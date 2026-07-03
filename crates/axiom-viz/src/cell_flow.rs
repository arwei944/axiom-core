//! Cell message flow data.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CellFlowRecord {
    pub cell_id: String,
    pub message_id: String,
    pub kind: String,
    pub from_layer: String,
    pub to_layer: String,
    pub timestamp_ns: u64,
    pub status: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CellFlowSnapshot {
    pub records: Vec<CellFlowRecord>,
}
