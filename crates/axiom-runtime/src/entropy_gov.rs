//! Entropy Governor - runtime entropy tracking backed by `axiom_core::entropy::EntropyScore`.
//!
//! Unlike the previous standalone implementation, this delegates to the canonical
//! 8-factor `EntropyScore` from axiom-core, ensuring weights and thresholds are
//! consistent across the runtime and oversight layers.

use axiom_core::entropy::{
    EntropyLevel, EntropyScore, CRITICAL_THRESHOLD, GREEN_THRESHOLD, RED_THRESHOLD,
    YELLOW_THRESHOLD,
};
use std::sync::Mutex;
use std::time::Instant;

/// Runtime entropy governor wrapping `axiom_core::entropy::EntropyScore`.
///
/// Uses the canonical 8-factor model with weights from `EntropyWeights::default()`,
/// so runtime entropy scoring is consistent with the oversight `EntropyGovernorCell`.
pub struct EntropyGovernor {
    score: Mutex<EntropyScore>,
    last_reduction: Mutex<Option<Instant>>,
}

/// Lightweight snapshot of runtime entropy for health reporting.
#[derive(Debug, Clone, Copy)]
pub struct EntropySnapshot {
    pub score: f64,
    pub level: EntropyLevel,
    pub dropped_messages: u64,
    pub rejected_by_guardian: u64,
    pub axiom_violations: u64,
    pub cell_restarts: u64,
    pub circuit_breaks: u64,
    pub timeouts: u64,
    pub duplicate_messages: u64,
    pub stale_state_violations: u64,
}

impl EntropySnapshot {
    fn from_score(s: &EntropyScore) -> Self {
        Self {
            score: s.value,
            level: s.level(),
            dropped_messages: s.dropped_messages,
            rejected_by_guardian: s.rejected_by_guardian,
            axiom_violations: s.axiom_violations,
            cell_restarts: s.cell_restarts,
            circuit_breaks: s.circuit_breaks,
            timeouts: s.timeouts,
            duplicate_messages: s.duplicate_messages,
            stale_state_violations: s.stale_state_violations,
        }
    }
}

impl EntropyGovernor {
    /// Create a new governor. `critical_threshold` maps to the critical level;
    /// red/yellow/green are derived proportionally.
    pub fn new(critical_threshold: f64) -> Self {
        let critical = critical_threshold.max(CRITICAL_THRESHOLD);
        let red = critical * (RED_THRESHOLD / CRITICAL_THRESHOLD);
        let yellow = critical * (YELLOW_THRESHOLD / CRITICAL_THRESHOLD);
        let green = critical * (GREEN_THRESHOLD / CRITICAL_THRESHOLD);
        Self {
            score: Mutex::new(
                EntropyScore::new().with_thresholds(green, yellow, red, critical),
            ),
            last_reduction: Mutex::new(None),
        }
    }

    pub fn record_dropped_message(&self) {
        self.score.lock().unwrap().record_dropped_message();
    }

    pub fn record_rejected_by_guardian(&self) {
        self.score.lock().unwrap().record_rejected_by_guardian();
    }

    pub fn record_axiom_violation(&self) {
        self.score.lock().unwrap().record_axiom_violation();
    }

    pub fn record_cell_restart(&self) {
        self.score.lock().unwrap().record_cell_restart();
    }

    pub fn record_circuit_break(&self) {
        self.score.lock().unwrap().record_circuit_break();
    }

    pub fn record_timeout(&self) {
        self.score.lock().unwrap().record_timeout();
    }

    pub fn record_duplicate_message(&self) {
        self.score.lock().unwrap().record_duplicate_message();
    }

    pub fn record_stale_state_violation(&self) {
        self.score.lock().unwrap().record_stale_state_violation();
    }

    pub fn snapshot(&self) -> EntropySnapshot {
        EntropySnapshot::from_score(&self.score.lock().unwrap())
    }

    pub fn reset(&self) {
        self.score.lock().unwrap().reset();
    }

    /// Returns true when entropy reaches Red or Critical level and the cooldown
    /// has elapsed. On returning true, the cooldown timer is updated.
    pub fn should_reduce(&self, cooldown_ms: u64) -> bool {
        let s = self.score.lock().unwrap();
        let hot = s.is_red() || s.is_critical();
        drop(s);
        if !hot {
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
        true
    }
}

impl Default for EntropyGovernor {
    fn default() -> Self {
        Self::new(CRITICAL_THRESHOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_accumulates() {
        let g = EntropyGovernor::new(10.0);
        g.record_dropped_message();
        g.record_dropped_message();
        g.record_rejected_by_guardian();
        let s = g.snapshot();
        assert_eq!(s.dropped_messages, 2);
        assert_eq!(s.rejected_by_guardian, 1);
        assert!(s.score > 0.0);
    }

    #[test]
    fn test_entropy_reset() {
        let g = EntropyGovernor::new(100.0);
        g.record_dropped_message();
        g.record_circuit_break();
        g.reset();
        let s = g.snapshot();
        assert_eq!(s.score, 0.0);
    }

    #[test]
    fn test_entropy_uses_core_weights() {
        // Verify runtime governor uses core's canonical weights:
        // cell_restart weight = 5.0, duplicate_message weight = 0.5
        let g = EntropyGovernor::new(100.0);
        g.record_cell_restart();
        let s1 = g.snapshot().score;
        g.reset();
        g.record_duplicate_message();
        let s2 = g.snapshot().score;
        assert!(
            s1 > s2,
            "cell_restart (weight 5.0) should score higher than duplicate (weight 0.5): {s1} vs {s2}"
        );
    }
}
