//! IntentAuditor - detects agent intent drift.

use axiom_kernel::id::CellId;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentProfile {
    pub agent_id: String,
    pub declared_intent: String,
    pub expected_signal_types: HashSet<String>,
    pub expected_targets: HashSet<String>,
    pub forbidden_actions: HashSet<String>,
    pub confidence: f64,
}

#[derive(Debug, Default, Clone)]
pub struct BehaviorSample {
    pub signal_types: HashMap<String, u64>,
    pub targets: HashMap<String, u64>,
    pub errors: u64,
    pub total_messages: u64,
}

impl BehaviorSample {
    pub fn record_signal(&mut self, signal_type: &str, target: Option<&str>, is_error: bool) {
        *self
            .signal_types
            .entry(signal_type.to_string())
            .or_insert(0) += 1;
        if let Some(t) = target {
            *self.targets.entry(t.to_string()).or_insert(0) += 1;
        }
        self.total_messages += 1;
        if is_error {
            self.errors += 1;
        }
    }
    pub fn error_rate(&self) -> f64 {
        if self.total_messages == 0 {
            0.0
        } else {
            self.errors as f64 / self.total_messages as f64
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviationReport {
    pub agent_id: String,
    pub jaccard_signal: f64,
    pub jaccard_target: f64,
    pub error_rate: f64,
    pub unexpected_signal_types: Vec<String>,
    pub unexpected_targets: Vec<String>,
    pub confidence: f64,
    pub drifted: bool,
}

pub struct IntentAuditorCell {
    id: CellId,
    profiles: Arc<Mutex<HashMap<String, IntentProfile>>>,
    samples: Arc<Mutex<HashMap<String, BehaviorSample>>>,
    history_error_rates: Arc<Mutex<HashMap<String, Vec<f64>>>>,
    deviation_threshold: f64,
    error_spike_z: f64,
}

impl IntentAuditorCell {
    pub fn new() -> Self {
        Self {
            id: CellId::new("oversight:intent-auditor"),
            profiles: Arc::new(Mutex::new(HashMap::new())),
            samples: Arc::new(Mutex::new(HashMap::new())),
            history_error_rates: Arc::new(Mutex::new(HashMap::new())),
            deviation_threshold: 0.5,
            error_spike_z: 2.0,
        }
    }

    pub fn id(&self) -> &CellId {
        &self.id
    }

    pub fn register_intent(&self, profile: IntentProfile) {
        self.profiles
            .lock()
            .insert(profile.agent_id.clone(), profile);
    }

    pub fn record_behavior(
        &self,
        agent_id: &str,
        signal_type: &str,
        target: Option<&str>,
        is_error: bool,
    ) {
        self.samples
            .lock()
            .entry(agent_id.to_string())
            .or_default()
            .record_signal(signal_type, target, is_error);
    }

    fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        let inter = a.intersection(b).count() as f64;
        let union = a.union(b).count() as f64;
        if union == 0.0 {
            1.0
        } else {
            inter / union
        }
    }

    fn mean_std(values: &[f64]) -> (f64, f64) {
        if values.is_empty() {
            return (0.0, 0.0);
        }
        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let var = values
            .iter()
            .map(|v| {
                let d = v - mean;
                d * d
            })
            .sum::<f64>()
            / n;
        (mean, var.sqrt())
    }

    pub fn audit(&self, agent_id: &str) -> Option<DeviationReport> {
        let profiles = self.profiles.lock();
        let profile = profiles.get(agent_id)?;
        let samples = self.samples.lock();
        let sample = samples.get(agent_id)?;

        let observed_signals: HashSet<String> = sample.signal_types.keys().cloned().collect();
        let observed_targets: HashSet<String> = sample.targets.keys().cloned().collect();

        let j_sig = Self::jaccard(&profile.expected_signal_types, &observed_signals);
        let j_tgt = Self::jaccard(&profile.expected_targets, &observed_targets);

        let unexpected_signals: Vec<String> = observed_signals
            .difference(&profile.expected_signal_types)
            .cloned()
            .collect();
        let unexpected_targets: Vec<String> = observed_targets
            .difference(&profile.expected_targets)
            .cloned()
            .collect();

        let forbidden_hits: Vec<String> = observed_signals
            .intersection(&profile.forbidden_actions)
            .cloned()
            .collect();

        let mut history = self.history_error_rates.lock();
        let hist = history.entry(agent_id.to_string()).or_default();
        let (mean, std) = Self::mean_std(hist);
        let err = sample.error_rate();
        let error_spike = std > 0.0 && (err - mean) > self.error_spike_z * std;
        hist.push(err);
        while hist.len() > 100 {
            hist.remove(0);
        }
        drop(history);

        let confidence = (j_sig + j_tgt) / 2.0 * profile.confidence;
        let drifted = confidence < self.deviation_threshold
            || !unexpected_signals.is_empty()
            || !forbidden_hits.is_empty()
            || error_spike;

        Some(DeviationReport {
            agent_id: agent_id.to_string(),
            jaccard_signal: j_sig,
            jaccard_target: j_tgt,
            error_rate: err,
            unexpected_signal_types: unexpected_signals,
            unexpected_targets,
            confidence,
            drifted,
        })
    }
}

impl Default for IntentAuditorCell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_behavior_no_drift() {
        let a = IntentAuditorCell::new();
        let mut expected = HashSet::new();
        expected.insert("OkSignal".to_string());
        let mut targets = HashSet::new();
        targets.insert("exec:worker".to_string());
        a.register_intent(IntentProfile {
            agent_id: "agent1".into(),
            declared_intent: "do work".into(),
            expected_signal_types: expected,
            expected_targets: targets,
            forbidden_actions: HashSet::new(),
            confidence: 1.0,
        });
        for _ in 0..10 {
            a.record_behavior("agent1", "OkSignal", Some("exec:worker"), false);
        }
        let report = a.audit("agent1").unwrap();
        assert!(!report.drifted);
        assert!(report.confidence >= 0.9);
    }

    #[test]
    fn test_unexpected_signal_drifts() {
        let a = IntentAuditorCell::new();
        let mut expected = HashSet::new();
        expected.insert("OkSignal".to_string());
        a.register_intent(IntentProfile {
            agent_id: "agent1".into(),
            declared_intent: "do work".into(),
            expected_signal_types: expected,
            expected_targets: HashSet::new(),
            forbidden_actions: HashSet::new(),
            confidence: 1.0,
        });
        for _ in 0..5 {
            a.record_behavior("agent1", "OkSignal", None, false);
        }
        a.record_behavior("agent1", "DeleteDatabase", None, false);
        let report = a.audit("agent1").unwrap();
        assert!(report.drifted);
        assert!(report
            .unexpected_signal_types
            .contains(&"DeleteDatabase".to_string()));
    }

    #[test]
    fn test_jaccard_similarity() {
        let mut a = HashSet::new();
        a.insert("x".to_string());
        let mut b = HashSet::new();
        b.insert("x".to_string());
        b.insert("y".to_string());
        let j = IntentAuditorCell::jaccard(&a, &b);
        assert!((j - 0.5).abs() < 0.001);
    }
}
