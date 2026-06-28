//! Entropy dashboard data.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct EntropyData {
    pub system_entropy: f64,
    pub cell_entropies: Vec<(String, f64)>,
    pub status: String,
}
