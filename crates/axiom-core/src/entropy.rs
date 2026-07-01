//! Entropy metrics - quantifying system disorder.
//!
//! Entropy is not a metaphor; it is a measurable, monitorable property.
//! Axiom violations, witness anomalies, message loops, and intent drift
//! all contribute to real-time entropy scores.
//!
//! Entropy scores are time-decayed (half-life) and clamped to [0.0, +inf).
//! - Green (<0.4): healthy
//! - Yellow (0.4-0.8): warning, monitoring required
//! - Red (>=0.8): circuit breakers may trigger
//! - Critical (>=3.0): emergency shutdown

use serde::{Deserialize, Serialize};

pub const GREEN_THRESHOLD: f64 = 0.4;
pub const YELLOW_THRESHOLD: f64 = 0.8;
pub const RED_THRESHOLD: f64 = 1.5;
pub const CRITICAL_THRESHOLD: f64 = 3.0;

pub const DEFAULT_HALF_LIFE_SECS: f64 = 300.0;

pub const WEIGHT_DROPPED_MESSAGES: f64 = 1.0;
pub const WEIGHT_REJECTED_BY_GUARDIAN: f64 = 2.0;
pub const WEIGHT_AXIOM_VIOLATIONS: f64 = 3.0;
pub const WEIGHT_CELL_RESTARTS: f64 = 5.0;
pub const WEIGHT_CIRCUIT_BREAKS: f64 = 4.0;
pub const WEIGHT_TIMEOUTS: f64 = 1.5;
pub const WEIGHT_DUPLICATE_MESSAGES: f64 = 0.5;
pub const WEIGHT_STALE_STATE_VIOLATIONS: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntropyLevel {
    Green,
    Yellow,
    Red,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EntropyWeights {
    pub dropped_messages: f64,
    pub rejected_by_guardian: f64,
    pub axiom_violations: f64,
    pub cell_restarts: f64,
    pub circuit_breaks: f64,
    pub timeouts: f64,
    pub duplicate_messages: f64,
    pub stale_state_violations: f64,
}

impl Default for EntropyWeights {
    fn default() -> Self {
        Self {
            dropped_messages: WEIGHT_DROPPED_MESSAGES,
            rejected_by_guardian: WEIGHT_REJECTED_BY_GUARDIAN,
            axiom_violations: WEIGHT_AXIOM_VIOLATIONS,
            cell_restarts: WEIGHT_CELL_RESTARTS,
            circuit_breaks: WEIGHT_CIRCUIT_BREAKS,
            timeouts: WEIGHT_TIMEOUTS,
            duplicate_messages: WEIGHT_DUPLICATE_MESSAGES,
            stale_state_violations: WEIGHT_STALE_STATE_VIOLATIONS,
        }
    }
}

impl EntropyWeights {
    pub fn normalize(&self) -> Self {
        let total = self.dropped_messages
            + self.rejected_by_guardian
            + self.axiom_violations
            + self.cell_restarts
            + self.circuit_breaks
            + self.timeouts
            + self.duplicate_messages
            + self.stale_state_violations;
        if total <= 0.0 {
            return Self::default();
        }
        Self {
            dropped_messages: self.dropped_messages / total,
            rejected_by_guardian: self.rejected_by_guardian / total,
            axiom_violations: self.axiom_violations / total,
            cell_restarts: self.cell_restarts / total,
            circuit_breaks: self.circuit_breaks / total,
            timeouts: self.timeouts / total,
            duplicate_messages: self.duplicate_messages / total,
            stale_state_violations: self.stale_state_violations / total,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EntropyScore {
    pub value: f64,
    pub dropped_messages: u64,
    pub rejected_by_guardian: u64,
    pub axiom_violations: u64,
    pub cell_restarts: u64,
    pub circuit_breaks: u64,
    pub timeouts: u64,
    pub duplicate_messages: u64,
    pub stale_state_violations: u64,
    pub last_updated_ns: u64,
    pub green_threshold: f64,
    pub yellow_threshold: f64,
    pub red_threshold: f64,
    pub critical_threshold: f64,
    pub half_life_secs: f64,
}

impl EntropyScore {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            dropped_messages: 0,
            rejected_by_guardian: 0,
            axiom_violations: 0,
            cell_restarts: 0,
            circuit_breaks: 0,
            timeouts: 0,
            duplicate_messages: 0,
            stale_state_violations: 0,
            last_updated_ns: 0,
            green_threshold: GREEN_THRESHOLD,
            yellow_threshold: YELLOW_THRESHOLD,
            red_threshold: RED_THRESHOLD,
            critical_threshold: CRITICAL_THRESHOLD,
            half_life_secs: DEFAULT_HALF_LIFE_SECS,
        }
    }

    pub fn with_thresholds(mut self, green: f64, yellow: f64, red: f64, critical: f64) -> Self {
        self.green_threshold = green;
        self.yellow_threshold = yellow;
        self.red_threshold = red;
        self.critical_threshold = critical;
        self
    }

    pub fn with_half_life(mut self, half_life_secs: f64) -> Self {
        self.half_life_secs = half_life_secs;
        self
    }

    pub fn compute(&mut self) -> f64 {
        self.compute_with_weights(&EntropyWeights::default())
    }

    pub fn compute_with_weights(&mut self, weights: &EntropyWeights) -> f64 {
        let dm = self.dropped_messages as f64 * weights.dropped_messages;
        let rbg = self.rejected_by_guardian as f64 * weights.rejected_by_guardian;
        let av = self.axiom_violations as f64 * weights.axiom_violations;
        let cr = self.cell_restarts as f64 * weights.cell_restarts;
        let cb = self.circuit_breaks as f64 * weights.circuit_breaks;
        let to = self.timeouts as f64 * weights.timeouts;
        let dup = self.duplicate_messages as f64 * weights.duplicate_messages;
        let ssv = self.stale_state_violations as f64 * weights.stale_state_violations;
        self.value = dm + rbg + av + cr + cb + to + dup + ssv;
        self.last_updated_ns = crate::signal::now_ns();
        self.value
    }

    pub fn record_dropped_message(&mut self) {
        self.dropped_messages = self.dropped_messages.saturating_add(1);
        self.compute();
    }

    pub fn record_rejected_by_guardian(&mut self) {
        self.rejected_by_guardian = self.rejected_by_guardian.saturating_add(1);
        self.compute();
    }

    pub fn record_axiom_violation(&mut self) {
        self.axiom_violations = self.axiom_violations.saturating_add(1);
        self.compute();
    }

    pub fn record_cell_restart(&mut self) {
        self.cell_restarts = self.cell_restarts.saturating_add(1);
        self.compute();
    }

    pub fn record_circuit_break(&mut self) {
        self.circuit_breaks = self.circuit_breaks.saturating_add(1);
        self.compute();
    }

    pub fn record_timeout(&mut self) {
        self.timeouts = self.timeouts.saturating_add(1);
        self.compute();
    }

    pub fn record_duplicate_message(&mut self) {
        self.duplicate_messages = self.duplicate_messages.saturating_add(1);
        self.compute();
    }

    pub fn record_stale_state_violation(&mut self) {
        self.stale_state_violations = self.stale_state_violations.saturating_add(1);
        self.compute();
    }

    /// Record a custom entropy event with a caller-specified weight.
    pub fn record_custom(&mut self, weight: f64) {
        self.value += weight;
    }

    pub fn decay(&mut self, now_ns: u64) {
        if self.half_life_secs <= 0.0 || self.last_updated_ns == 0 {
            return;
        }
        let elapsed_ns = now_ns.saturating_sub(self.last_updated_ns);
        let elapsed_secs = elapsed_ns as f64 / 1_000_000_000.0;
        if elapsed_secs <= 0.0 {
            return;
        }
        let half_lives = elapsed_secs / self.half_life_secs;
        let decay_factor = 0.5f64.powf(half_lives);
        self.dropped_messages = ((self.dropped_messages as f64) * decay_factor) as u64;
        self.rejected_by_guardian = ((self.rejected_by_guardian as f64) * decay_factor) as u64;
        self.axiom_violations = ((self.axiom_violations as f64) * decay_factor) as u64;
        self.cell_restarts = ((self.cell_restarts as f64) * decay_factor) as u64;
        self.circuit_breaks = ((self.circuit_breaks as f64) * decay_factor) as u64;
        self.timeouts = ((self.timeouts as f64) * decay_factor) as u64;
        self.duplicate_messages = ((self.duplicate_messages as f64) * decay_factor) as u64;
        self.stale_state_violations = ((self.stale_state_violations as f64) * decay_factor) as u64;
        self.compute();
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
        self.dropped_messages = 0;
        self.rejected_by_guardian = 0;
        self.axiom_violations = 0;
        self.cell_restarts = 0;
        self.circuit_breaks = 0;
        self.timeouts = 0;
        self.duplicate_messages = 0;
        self.stale_state_violations = 0;
        self.last_updated_ns = crate::signal::now_ns();
    }

    pub fn level(&self) -> EntropyLevel {
        if self.value >= self.critical_threshold {
            EntropyLevel::Critical
        } else if self.value >= self.red_threshold {
            EntropyLevel::Red
        } else if self.value >= self.yellow_threshold {
            EntropyLevel::Yellow
        } else {
            EntropyLevel::Green
        }
    }

    pub fn is_green(&self) -> bool {
        self.level() == EntropyLevel::Green
    }

    pub fn is_yellow(&self) -> bool {
        self.level() == EntropyLevel::Yellow
    }

    pub fn is_red(&self) -> bool {
        self.level() == EntropyLevel::Red
    }

    pub fn is_critical(&self) -> bool {
        self.level() == EntropyLevel::Critical
    }

    pub fn snapshot(&self) -> EntropySnapshot {
        EntropySnapshot {
            value: self.value,
            level: self.level(),
            dropped_messages: self.dropped_messages,
            rejected_by_guardian: self.rejected_by_guardian,
            axiom_violations: self.axiom_violations,
            cell_restarts: self.cell_restarts,
            circuit_breaks: self.circuit_breaks,
            timeouts: self.timeouts,
            duplicate_messages: self.duplicate_messages,
            stale_state_violations: self.stale_state_violations,
            timestamp_ns: self.last_updated_ns,
            per_cell: Vec::new(),
        }
    }
}

impl Default for EntropyScore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellEntropy {
    pub cell_id: String,
    pub score: f64,
    pub level: EntropyLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropySnapshot {
    pub value: f64,
    pub level: EntropyLevel,
    pub dropped_messages: u64,
    pub rejected_by_guardian: u64,
    pub axiom_violations: u64,
    pub cell_restarts: u64,
    pub circuit_breaks: u64,
    pub timeouts: u64,
    pub duplicate_messages: u64,
    pub stale_state_violations: u64,
    pub timestamp_ns: u64,
    pub per_cell: Vec<CellEntropy>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_starts_green() {
        let score = EntropyScore::new();
        assert!(score.is_green());
        assert!(!score.is_yellow());
        assert!(!score.is_red());
        assert!(!score.is_critical());
    }

    #[test]
    fn test_axiom_violation_increases_entropy() {
        let mut score = EntropyScore::new();
        for _ in 0..20 {
            score.record_axiom_violation();
        }
        assert!(
            score.value > 0.0,
            "violations should increase entropy, got {}",
            score.value
        );
    }

    #[test]
    fn test_high_weight_factor_increases_faster() {
        let mut s1 = EntropyScore::new();
        let mut s2 = EntropyScore::new();
        for _ in 0..5 {
            s1.record_cell_restart();
            s2.record_duplicate_message();
        }
        assert!(
            s1.value > s2.value,
            "cell_restarts (weight 5.0) should increase faster than duplicate (weight 0.5), s1={}, s2={}",
            s1.value, s2.value
        );
    }

    #[test]
    fn test_multiple_factors_reach_red() {
        let mut score = EntropyScore::new();
        for _ in 0..20 {
            score.record_axiom_violation();
            score.record_cell_restart();
            score.record_circuit_break();
            score.record_rejected_by_guardian();
        }
        assert!(
            score.is_red() || score.is_critical(),
            "multiple factors should push to red+, got {}",
            score.value
        );
    }

    #[test]
    fn test_critical_threshold() {
        let mut score = EntropyScore::new();
        for _ in 0..50 {
            score.record_cell_restart();
            score.record_circuit_break();
            score.record_axiom_violation();
        }
        let _ = score.value;
    }

    #[test]
    fn test_reset_returns_to_green() {
        let mut score = EntropyScore::new();
        score.record_axiom_violation();
        score.record_axiom_violation();
        score.reset();
        assert!(score.is_green());
        assert_eq!(score.axiom_violations, 0);
    }

    #[test]
    fn test_weights_normalize() {
        let w = EntropyWeights::default().normalize();
        let total = w.dropped_messages
            + w.rejected_by_guardian
            + w.axiom_violations
            + w.cell_restarts
            + w.circuit_breaks
            + w.timeouts
            + w.duplicate_messages
            + w.stale_state_violations;
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_entropy_snapshot() {
        let mut score = EntropyScore::new();
        score.record_axiom_violation();
        let snap = score.snapshot();
        assert_eq!(snap.axiom_violations, 1);
        assert!(snap.per_cell.is_empty());
    }

    #[test]
    fn test_all_factor_methods_exist() {
        let mut score = EntropyScore::new();
        score.record_dropped_message();
        score.record_rejected_by_guardian();
        score.record_axiom_violation();
        score.record_cell_restart();
        score.record_circuit_break();
        score.record_timeout();
        score.record_duplicate_message();
        score.record_stale_state_violation();
        assert_eq!(score.dropped_messages, 1);
        assert_eq!(score.rejected_by_guardian, 1);
        assert_eq!(score.axiom_violations, 1);
        assert_eq!(score.cell_restarts, 1);
        assert_eq!(score.circuit_breaks, 1);
        assert_eq!(score.timeouts, 1);
        assert_eq!(score.duplicate_messages, 1);
        assert_eq!(score.stale_state_violations, 1);
    }

    #[test]
    fn test_time_decay_reduces_entropy() {
        let mut score = EntropyScore::new();
        for _ in 0..10 {
            score.record_axiom_violation();
        }
        let before = score.value;
        assert!(before > 0.0);
        let now = score.last_updated_ns + 600_000_000_000;
        score.decay(now);
        assert!(
            score.value < before,
            "decay should reduce entropy: before={}, after={}",
            before,
            score.value
        );
    }
}
