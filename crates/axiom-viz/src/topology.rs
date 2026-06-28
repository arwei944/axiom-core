//! System topology data for architecture diagrams.

use serde::Serialize;
use axiom_core::layer::Layer;

#[derive(Debug, Serialize)]
pub struct CellNode {
    pub id: String,
    pub name: String,
    pub layer: Layer,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct TopologyGraph {
    pub cells: Vec<CellNode>,
    pub edges: Vec<(String, String)>,
}
