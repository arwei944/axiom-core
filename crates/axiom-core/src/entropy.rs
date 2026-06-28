//! Entropy metrics - quantifying system disorder.
//!
//! Entropy is not a metaphor; it is a measurable, monitorable property.
//! Axiom violations, witness anomalies, message loops, and intent drift
//! all contribute to real-time entropy scores.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EntropyScore {
    pub value: f64,
    pub axiom_violation_rate: f64,
    pub witness_anomaly_rate: f64,
    pub message_loop_count: u64,
    pub intent_drift: f64,
}

impl EntropyScore {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            axiom_violation_rate: 0.0,
            witness_anomaly_rate: 0.0,
            message_loop_count: 0,
            intent_drift: 0.0,
        }
    }

    pub fn compute(&mut self) -> f64 {
        self.value = (self.axiom_violation_rate * 0.4
            + self.witness_anomaly_rate * 0.2
            + (self.message_loop_count.min(10) as f64 / 10.0) * 0.2
            + self.intent_drift * 0.2)
            .clamp(0.0, 1.0);
        self.value
    }

    pub fn is_green(&self) -> bool {
        self.value < 0.4
    }

    pub fn is_yellow(&self) -> bool {
        self.value >= 0.4 && self.value < 0.8
    }

    pub fn is_red(&self) -> bool {
        self.value >= 0.8
    }
}

impl Default for EntropyScore {
    fn default() -> Self {
        Self::new()
    }
}
