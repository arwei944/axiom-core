//! MetaOversight - supervises the supervisors. Monitors Oversight cells' health.

use axiom_kernel::id::CellId;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CellHeartbeat {
    pub last_seen_ns: u64,
    pub consecutive_misses: u32,
    pub responsive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OversightIntegrityReport {
    pub expected_cells: Vec<String>,
    pub unresponsive_cells: Vec<String>,
    pub witness_chain_ok: bool,
    pub interceptor_count: usize,
    pub healthy: bool,
}

pub struct MetaOversightCell {
    id: CellId,
    expected_cells: Arc<Mutex<Vec<String>>>,
    heartbeats: Arc<Mutex<HashMap<String, CellHeartbeat>>>,
    witness_chain_ok: Arc<Mutex<bool>>,
    interceptor_count: Arc<Mutex<usize>>,
    max_missed_pings: u32,
    last_ping: Arc<Mutex<Instant>>,
}

impl MetaOversightCell {
    pub fn new() -> Self {
        Self {
            id: CellId::new("oversight:meta-oversight"),
            expected_cells: Arc::new(Mutex::new(Vec::new())),
            heartbeats: Arc::new(Mutex::new(HashMap::new())),
            witness_chain_ok: Arc::new(Mutex::new(true)),
            interceptor_count: Arc::new(Mutex::new(0)),
            max_missed_pings: 3,
            last_ping: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn id(&self) -> &CellId {
        &self.id
    }

    pub fn register_expected_cell(&self, cell_id: &str) {
        self.expected_cells.lock().push(cell_id.to_string());
        self.heartbeats.lock().entry(cell_id.to_string()).or_default();
    }

    pub fn set_interceptor_count(&self, n: usize) {
        *self.interceptor_count.lock() = n;
    }

    pub fn set_witness_chain_ok(&self, ok: bool) {
        *self.witness_chain_ok.lock() = ok;
    }

    pub fn record_pong(&self, cell_id: &str) {
        let mut hb = self.heartbeats.lock();
        let entry = hb.entry(cell_id.to_string()).or_default();
        entry.last_seen_ns = axiom_kernel::clock::global_clock().now_ns();
        entry.consecutive_misses = 0;
        entry.responsive = true;
    }

    pub fn tick_ping(&self) -> Vec<String> {
        *self.last_ping.lock() = Instant::now();
        let expected = self.expected_cells.lock().clone();
        let mut hb = self.heartbeats.lock();
        let mut unresponsive = Vec::new();
        for cid in &expected {
            let entry = hb.entry(cid.clone()).or_default();
            entry.consecutive_misses += 1;
            if entry.consecutive_misses > self.max_missed_pings {
                entry.responsive = false;
                unresponsive.push(cid.clone());
            }
        }
        unresponsive
    }

    pub fn integrity_report(&self) -> OversightIntegrityReport {
        let expected = self.expected_cells.lock().clone();
        let hb = self.heartbeats.lock();
        let unresponsive: Vec<String> = expected
            .iter()
            .filter(|cid| !hb.get(*cid).map(|h| h.responsive).unwrap_or(false))
            .cloned()
            .collect();
        let witness_ok = *self.witness_chain_ok.lock();
        let interceptor_count = *self.interceptor_count.lock();
        let healthy = unresponsive.is_empty() && witness_ok && interceptor_count > 0;
        OversightIntegrityReport {
            expected_cells: expected,
            unresponsive_cells: unresponsive,
            witness_chain_ok: witness_ok,
            interceptor_count,
            healthy,
        }
    }
}

impl Default for MetaOversightCell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unresponsive_after_misses() {
        let m = MetaOversightCell::new();
        m.register_expected_cell("g1");
        m.register_expected_cell("g2");
        m.set_interceptor_count(3);
        m.record_pong("g1");
        m.record_pong("g2");

        assert!(m.integrity_report().healthy);

        for _ in 0..5 {
            m.tick_ping();
        }
        let r = m.integrity_report();
        assert!(!r.healthy);
        assert_eq!(r.unresponsive_cells.len(), 2);
    }

    #[test]
    fn test_pong_resets_misses() {
        let m = MetaOversightCell::new();
        m.register_expected_cell("g1");
        m.set_interceptor_count(1);
        for _ in 0..4 {
            m.tick_ping();
        }
        m.record_pong("g1");
        let r = m.integrity_report();
        assert!(r.healthy);
    }

    #[test]
    fn test_witness_chain_failure() {
        let m = MetaOversightCell::new();
        m.register_expected_cell("g1");
        m.record_pong("g1");
        m.set_interceptor_count(1);
        assert!(m.integrity_report().healthy);
        m.set_witness_chain_ok(false);
        assert!(!m.integrity_report().healthy);
    }
}
