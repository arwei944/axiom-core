//! Entropy metrics - quantifying system disorder.
//!
//! Entropy is not a metaphor; it is a measurable, monitorable property.
//! Axiom violations, witness anomalies, message loops, and intent drift
//! all contribute to real-time entropy scores.
//!
//! Entropy scores are time-decayed (half-life) and clamped to [0.0, 1.0].
//! - Green (<0.4): healthy
//! - Yellow (0.4-0.8): warning, monitoring required
//! - Red (>=0.8): circuit breakers may trigger

use serde::{Deserialize, Serialize};

pub const DEFAULT_GREEN_THRESHOLD: f64 = 0.4;
pub const DEFAULT_YELLOW_THRESHOLD: f64 = 0.8;
pub const DEFAULT_HALF_LIFE_NS: u64 = 60_000_000_000;
pub const WEIGHT_AXIOM_VIOLATION: f64 = 0.4;
pub const WEIGHT_WITNESS_ANOMALY: f64 = 0.2;
pub const WEIGHT_MESSAGE_LOOP: f64 = 0.2;
pub const WEIGHT_INTENT_DRIFT: f64 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntropyLevel {
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EntropyWeights {
    pub axiom_violation: f64,
    pub witness_anomaly: f64,
    pub message_loop: f64,
    pub intent_drift: f64,
}

impl Default for EntropyWeights {
    fn default() -> Self {
        Self {
            axiom_violation: WEIGHT_AXIOM_VIOLATION,
            witness_anomaly: WEIGHT_WITNESS_ANOMALY,
            message_loop: WEIGHT_MESSAGE_LOOP,
            intent_drift: WEIGHT_INTENT_DRIFT,
        }
    }
}

impl EntropyWeights {
    pub fn normalize(&self) -> Self {
        let total = self.axiom_violation + self.witness_anomaly + self.message_loop + self.intent_drift;
        if total <= 0.0 {
            return Self::default();
        }
        Self {
            axiom_violation: self.axiom_violation / total,
            witness_anomaly: self.witness_anomaly / total,
            message_loop: self.message_loop / total,
            intent_drift: self.intent_drift / total,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EntropyScore {
    pub value: f64,
    pub axiom_violation_rate: f64,
    pub witness_anomaly_rate: f64,
    pub message_loop_count: u64,
    pub intent_drift: f64,
    pub last_updated_ns: u64,
    pub green_threshold: f64,
    pub yellow_threshold: f64,
    pub half_life_ns: u64,
}

impl EntropyScore {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            axiom_violation_rate: 0.0,
            witness_anomaly_rate: 0.0,
            message_loop_count: 0,
            intent_drift: 0.0,
            last_updated_ns: 0,
            green_threshold: DEFAULT_GREEN_THRESHOLD,
            yellow_threshold: DEFAULT_YELLOW_THRESHOLD,
            half_life_ns: DEFAULT_HALF_LIFE_NS,
        }
    }

    pub fn with_thresholds(mut self, green: f64, yellow: f64) -> Self {
        self.green_threshold = green;
        self.yellow_threshold = yellow;
        self
    }

    pub fn with_half_life(mut self, half_life_ns: u64) -> Self {
        self.half_life_ns = half_life_ns;
        self
    }

    pub fn compute(&mut self) -> f64 {
        self.compute_with_weights(&EntropyWeights::default())
    }

    pub fn compute_with_weights(&mut self, weights: &EntropyWeights) -> f64 {
        let w = weights.normalize();
        let loop_component = self.message_loop_count.min(10) as f64 / 10.0;
        self.value = (self.axiom_violation_rate * w.axiom_violation
            + self.witness_anomaly_rate * w.witness_anomaly
            + loop_component * w.message_loop
            + self.intent_drift * w.intent_drift)
            .clamp(0.0, 1.0);
        self.last_updated_ns = crate::signal::now_ns();
        self.value
    }

    pub fn record_axiom_violation(&mut self) {
        self.axiom_violation_rate = (self.axiom_violation_rate + 0.1).min(1.0);
        self.compute();
    }

    pub fn record_witness_anomaly(&mut self) {
        self.witness_anomaly_rate = (self.witness_anomaly_rate + 0.05).min(1.0);
        self.compute();
    }

    pub fn record_message_loop(&mut self) {
        self.message_loop_count = self.message_loop_count.saturating_add(1);
        self.compute();
    }

    pub fn record_intent_drift(&mut self, amount: f64) {
        self.intent_drift = (self.intent_drift + amount).clamp(0.0, 1.0);
        self.compute();
    }

    pub fn decay(&mut self, now_ns: u64) {
        if self.half_life_ns == 0 || self.last_updated_ns == 0 {
            return;
        }
        let elapsed = now_ns.saturating_sub(self.last_updated_ns);
        if elapsed == 0 {
            return;
        }
        let half_lives = elapsed as f64 / self.half_life_ns as f64;
        let decay_factor = 0.5f64.powf(half_lives);
        self.axiom_violation_rate *= decay_factor;
        self.witness_anomaly_rate *= decay_factor;
        self.intent_drift *= decay_factor;
        self.message_loop_count = ((self.message_loop_count as f64) * decay_factor) as u64;
        self.compute();
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
        self.axiom_violation_rate = 0.0;
        self.witness_anomaly_rate = 0.0;
        self.message_loop_count = 0;
        self.intent_drift = 0.0;
        self.last_updated_ns = crate::signal::now_ns();
    }

    pub fn level(&self) -> EntropyLevel {
        if self.value >= self.yellow_threshold {
            EntropyLevel::Red
        } else if self.value >= self.green_threshold {
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

    pub fn snapshot(&self) -> EntropySnapshot {
        EntropySnapshot {
            value: self.value,
            level: self.level(),
            axiom_violation_rate: self.axiom_violation_rate,
            witness_anomaly_rate: self.witness_anomaly_rate,
            message_loop_count: self.message_loop_count,
            intent_drift: self.intent_drift,
            timestamp_ns: self.last_updated_ns,
        }
    }
}

impl Default for EntropyScore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropySnapshot {
    pub value: f64,
    pub level: EntropyLevel,
    pub axiom_violation_rate: f64,
    pub witness_anomaly_rate: f64,
    pub message_loop_count: u64,
    pub intent_drift: f64,
    pub timestamp_ns: u64,
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
    }

    #[test]
    fn test_axiom_violation_increases_entropy() {
        let mut score = EntropyScore::new();
        for _ in 0..20 {
            score.record_axiom_violation();
        }
        assert!(score.value > 0.0, "violations should increase entropy, got {}", score.value);
        assert!(score.is_yellow() || score.is_red(), "after many violations entropy should be at least yellow, got {}", score.value);
    }

    #[test]
    fn test_multiple_factors_reach_red() {
        let mut score = EntropyScore::new();
        for _ in 0..20 {
            score.record_axiom_violation();
            score.record_witness_anomaly();
            score.record_message_loop();
            score.record_intent_drift(0.3);
        }
        assert!(score.is_red(), "multiple factors should push to red, got {}", score.value);
    }

    #[test]
    fn test_reset_returns_to_green() {
        let mut score = EntropyScore::new();
        score.record_axiom_violation();
        score.record_axiom_violation();
        score.reset();
        assert!(score.is_green());
    }

    #[test]
    fn test_weights_normalize() {
        let w = EntropyWeights {
            axiom_violation: 0.8,
            witness_anomaly: 0.8,
            message_loop: 0.2,
            intent_drift: 0.2,
        }.normalize();
        let total = w.axiom_violation + w.witness_anomaly + w.message_loop + w.intent_drift;
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_entropy_snapshot() {
        let mut score = EntropyScore::new();
        score.record_axiom_violation();
        let snap = score.snapshot();
        assert_eq!(snap.level, EntropyLevel::Green);
        assert!(snap.value > 0.0);
    }
}
