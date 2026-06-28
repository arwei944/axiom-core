//! Entropy Governor - tracks system entropy and triggers auto-reduction.
//!
//! Accumulates entropy metrics: message drops, rejected dispatches,
//! restart counts, circuit breaks, slow handoffs. When entropy exceeds
//! thresholds, emits EntropyReduction signals that the Oversight layer
//! consumes to perform cleanup (consolidation, GC, circuit reset).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub struct EntropyGovernor {
    message_drops: AtomicU64,
    rejected_dispatches: AtomicU64,
    cell_restarts: AtomicU64,
    circuit_breaks: AtomicU64,
    slow_handoffs: AtomicU64,
    threshold: f64,
    last_reduction: std::sync::Mutex<Option<Instant>>,
    reduction_count: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct EntropySnapshot {
    pub message_drops: u64,
    pub rejected_dispatches: u64,
    pub cell_restarts: u64,
    pub circuit_breaks: u64,
    pub slow_handoffs: u64,
    pub score: f64,
}

impl EntropyGovernor {
    pub fn new(threshold: f64) -> Self {
        Self {
            message_drops: AtomicU64::new(0),
            rejected_dispatches: AtomicU64::new(0),
            cell_restarts: AtomicU64::new(0),
            circuit_breaks: AtomicU64::new(0),
            slow_handoffs: AtomicU64::new(0),
            threshold,
            last_reduction: std::sync::Mutex::new(None),
            reduction_count: AtomicU64::new(0),
        }
    }

    pub fn record_drop(&self) {
        self.message_drops.fetch_add(1, Ordering::Relaxed);
    }
    pub fn record_rejection(&self) {
        self.rejected_dispatches.fetch_add(1, Ordering::Relaxed);
    }
    pub fn record_restart(&self) {
        self.cell_restarts.fetch_add(1, Ordering::Relaxed);
    }
    pub fn record_circuit_break(&self) {
        self.circuit_breaks.fetch_add(1, Ordering::Relaxed);
    }
    pub fn record_slow_handoff(&self) {
        self.slow_handoffs.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> EntropySnapshot {
        let drops = self.message_drops.load(Ordering::Relaxed);
        let rejects = self.rejected_dispatches.load(Ordering::Relaxed);
        let restarts = self.cell_restarts.load(Ordering::Relaxed);
        let breaks = self.circuit_breaks.load(Ordering::Relaxed);
        let slows = self.slow_handoffs.load(Ordering::Relaxed);

        let score = (drops as f64) * 0.5
            + (rejects as f64) * 1.0
            + (restarts as f64) * 2.0
            + (breaks as f64) * 3.0
            + (slows as f64) * 0.3;

        EntropySnapshot {
            message_drops: drops,
            rejected_dispatches: rejects,
            cell_restarts: restarts,
            circuit_breaks: breaks,
            slow_handoffs: slows,
            score,
        }
    }

    pub fn should_reduce(&self, cooldown_ms: u64) -> bool {
        let snap = self.snapshot();
        if snap.score < self.threshold {
            return false;
        }
        let mut last = self.last_reduction.lock().unwrap();
        let now = Instant::now();
        if let Some(t) = *last {
            if now.duration_since(t).as_millis() < cooldown_ms as u128 {
                return false;
            }
        }
        *last = Some(now);
        self.reduction_count.fetch_add(1, Ordering::Relaxed);
        true
    }

    pub fn reset(&self) {
        self.message_drops.store(0, Ordering::Relaxed);
        self.rejected_dispatches.store(0, Ordering::Relaxed);
        self.cell_restarts.store(0, Ordering::Relaxed);
        self.circuit_breaks.store(0, Ordering::Relaxed);
        self.slow_handoffs.store(0, Ordering::Relaxed);
    }

    pub fn reduction_count(&self) -> u64 {
        self.reduction_count.load(Ordering::Relaxed)
    }
}

impl Default for EntropyGovernor {
    fn default() -> Self {
        Self::new(100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_accumulates() {
        let g = EntropyGovernor::new(10.0);
        g.record_drop();
        g.record_drop();
        g.record_rejection();
        let s = g.snapshot();
        assert_eq!(s.message_drops, 2);
        assert_eq!(s.rejected_dispatches, 1);
        assert!(s.score > 0.0);
    }

    #[test]
    fn test_entropy_cooldown_prevents_rapid_reductions() {
        let g = EntropyGovernor::new(1.0);
        g.record_restart();
        assert!(g.should_reduce(60_000), "first call triggers reduction");
        g.record_restart();
        assert!(
            !g.should_reduce(60_000),
            "second immediate call blocked by cooldown"
        );
    }

    #[test]
    fn test_entropy_reset() {
        let g = EntropyGovernor::new(100.0);
        g.record_drop();
        g.record_circuit_break();
        g.reset();
        let s = g.snapshot();
        assert_eq!(s.score, 0.0);
    }
}
