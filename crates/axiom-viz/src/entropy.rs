//! Entropy dashboard data.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EntropyData {
    pub system_entropy: f64,
    pub cell_entropies: Vec<(String, f64)>,
    pub status: String,
}
