//! System topology data for architecture diagrams.

use axiom_kernel::layer::RuntimeTier;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CellNode {
    pub id: String,
    pub name: String,
    pub layer: RuntimeTier,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopologyGraph {
    pub cells: Vec<CellNode>,
    pub edges: Vec<(String, String)>,
}
