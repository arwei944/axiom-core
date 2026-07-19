//! Shared run summaries for the single U4 observation surface.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub kind: String,
    pub ok: bool,
    pub label: String,
    pub governor_level: String,
    pub governor_score: f64,
    pub witness_count: usize,
    pub error: Option<String>,
}

pub type SharedRunLog = Arc<Mutex<Vec<RunSummary>>>;

pub fn new_run_log() -> SharedRunLog {
    Arc::new(Mutex::new(Vec::new()))
}

pub fn push_run(log: &SharedRunLog, summary: RunSummary) {
    if let Ok(mut g) = log.lock() {
        g.push(summary);
        // keep last 32
        let len = g.len();
        if len > 32 {
            g.drain(0..len - 32);
        }
    }
}

pub fn snapshot_runs(log: &SharedRunLog) -> Vec<RunSummary> {
    log.lock().map(|g| g.clone()).unwrap_or_default()
}
