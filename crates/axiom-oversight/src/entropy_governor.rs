//! EntropyGovernorCell - quantifies disorder and prescribes governance actions.

use axiom_core::entropy::EntropyScore;
use axiom_core::id::CellId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntropyLevel {
    Green,
    Yellow,
    Red,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GovernanceAction {
    None,
    Warn { message: String },
    Throttle { target_cell: Option<String>, factor: f64 },
    Emergency { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropySnapshot {
    pub global: EntropyScore,
    pub per_cell: HashMap<String, f64>,
    pub level: EntropyLevel,
    pub last_action: Option<GovernanceAction>,
    pub last_action_ns: u64,
    pub cooldown_remaining_ns: u64,
}

#[derive(Debug, Clone)]
pub enum EntropyEvent {
    AxiomViolation { cell_id: String, severity: f64 },
    WitnessAnomaly { cell_id: String },
    MessageLoop { correlation_id: String },
    IntentDrift { agent_id: String, amount: f64 },
    ResourceExhausted { resource: String },
    CellCrashed { cell_id: String },
    CircuitBreakerOpened { cell_id: String },
    Custom { cell_id: String, weight: f64 },
}

pub struct EntropyGovernorCell {
    id: CellId,
    global: Arc<Mutex<EntropyScore>>,
    per_cell: Arc<Mutex<HashMap<String, EntropyScore>>>,
    last_action_ns: Arc<Mutex<u64>>,
    last_action: Arc<Mutex<Option<GovernanceAction>>>,
    cooldown_ns: u64,
    critical_threshold: f64,
}

impl EntropyGovernorCell {
    pub fn new(green: f64, yellow: f64, critical: f64) -> Self {
        Self {
            id: CellId::new("oversight:entropy-governor"),
            global: Arc::new(Mutex::new(
                EntropyScore::new().with_thresholds(green, yellow),
            )),
            per_cell: Arc::new(Mutex::new(HashMap::new())),
            last_action_ns: Arc::new(Mutex::new(0)),
            last_action: Arc::new(Mutex::new(None)),
            cooldown_ns: 30_000_000_000,
            critical_threshold: critical,
        }
    }

    pub fn id(&self) -> &CellId {
        &self.id
    }

    pub fn record(&self, ev: EntropyEvent) {
        let now = now_ns();
        let cell_id = match &ev {
            EntropyEvent::AxiomViolation { cell_id, .. } => cell_id.clone(),
            EntropyEvent::WitnessAnomaly { cell_id } => cell_id.clone(),
            EntropyEvent::MessageLoop { .. } => String::new(),
            EntropyEvent::IntentDrift { agent_id, .. } => agent_id.clone(),
            EntropyEvent::ResourceExhausted { .. } => String::new(),
            EntropyEvent::CellCrashed { cell_id } => cell_id.clone(),
            EntropyEvent::CircuitBreakerOpened { cell_id } => cell_id.clone(),
            EntropyEvent::Custom { cell_id, .. } => cell_id.clone(),
        };

        let mut global = self.global.lock().unwrap();
        global.record_axiom_violation();
        global.record_witness_anomaly();
        if matches!(ev, EntropyEvent::MessageLoop { .. }) {
            global.record_message_loop();
        }
        if let EntropyEvent::IntentDrift { amount, .. } = &ev {
            global.record_intent_drift(*amount);
        }
        global.last_updated_ns = now;
        drop(global);

        if !cell_id.is_empty() {
            let mut per_cell = self.per_cell.lock().unwrap();
            let entry = per_cell
                .entry(cell_id)
                .or_insert_with(|| EntropyScore::new().with_thresholds(0.3, 0.6));
            entry.record_axiom_violation();
            entry.last_updated_ns = now;
        }
    }

    pub fn decay_tick(&self) {
        let now = now_ns();
        let mut g = self.global.lock().unwrap();
        g.decay(now);
        drop(g);
        let mut per_cell = self.per_cell.lock().unwrap();
        for s in per_cell.values_mut() {
            s.decay(now);
        }
    }

    pub fn snapshot(&self) -> EntropySnapshot {
        let now = now_ns();
        let g = *self.global.lock().unwrap();
        let level = if g.value >= self.critical_threshold {
            EntropyLevel::Critical
        } else if g.value >= g.yellow_threshold {
            EntropyLevel::Red
        } else if g.value >= g.green_threshold {
            EntropyLevel::Yellow
        } else {
            EntropyLevel::Green
        };
        let per_cell = self
            .per_cell
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.value))
            .collect();
        let last_action_ns = *self.last_action_ns.lock().unwrap();
        let cooldown_remaining = (last_action_ns + self.cooldown_ns).saturating_sub(now);
        EntropySnapshot {
            global: g,
            per_cell,
            level,
            last_action: self.last_action.lock().unwrap().clone(),
            last_action_ns,
            cooldown_remaining_ns: cooldown_remaining,
        }
    }

    pub fn take_action(&self) -> GovernanceAction {
        let snap = self.snapshot();
        let now = now_ns();
        let mut last_action_ns = self.last_action_ns.lock().unwrap();
        if *last_action_ns + self.cooldown_ns > now {
            return GovernanceAction::None;
        }

        let action = match snap.level {
            EntropyLevel::Green => GovernanceAction::None,
            EntropyLevel::Yellow => GovernanceAction::Warn {
                message: format!("entropy yellow: {:.3}", snap.global.value),
            },
            EntropyLevel::Red => {
                let hottest = snap
                    .per_cell
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(k, _)| k.clone());
                GovernanceAction::Throttle {
                    target_cell: hottest,
                    factor: 0.5,
                }
            }
            EntropyLevel::Critical => GovernanceAction::Emergency {
                reason: format!("entropy critical: {:.3}", snap.global.value),
            },
        };

        if !matches!(action, GovernanceAction::None) {
            *last_action_ns = now;
            *self.last_action.lock().unwrap() = Some(action.clone());
        }
        action
    }

    pub fn reset(&self) {
        let now = now_ns();
        let g = self.global.lock().unwrap();
        let gt = g.green_threshold;
        let yt = g.yellow_threshold;
        drop(g);
        let mut g = self.global.lock().unwrap();
        *g = EntropyScore::new().with_thresholds(gt, yt);
        g.last_updated_ns = now;
        self.per_cell.lock().unwrap().clear();
    }
}

impl Default for EntropyGovernorCell {
    fn default() -> Self {
        Self::new(
            axiom_core::entropy::DEFAULT_GREEN_THRESHOLD,
            axiom_core::entropy::DEFAULT_YELLOW_THRESHOLD,
            0.95,
        )
    }
}

fn now_ns() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_green_by_default() {
        let g = EntropyGovernorCell::default();
        let s = g.snapshot();
        assert_eq!(s.level, EntropyLevel::Green);
        assert!(matches!(g.take_action(), GovernanceAction::None));
    }

    #[test]
    fn test_red_after_multiple_events() {
        let g = EntropyGovernorCell::new(0.1, 0.3, 0.5);
        for _ in 0..20 {
            g.record(EntropyEvent::AxiomViolation {
                cell_id: "c1".into(),
                severity: 1.0,
            });
            g.record(EntropyEvent::WitnessAnomaly {
                cell_id: "c1".into(),
            });
            g.record(EntropyEvent::MessageLoop {
                correlation_id: "x".into(),
            });
            g.record(EntropyEvent::IntentDrift {
                agent_id: "a1".into(),
                amount: 1.0,
            });
        }
        let s = g.snapshot();
        assert!(matches!(s.level, EntropyLevel::Red | EntropyLevel::Critical));
        let action = g.take_action();
        assert!(matches!(
            action,
            GovernanceAction::Throttle { .. } | GovernanceAction::Emergency { .. }
        ));
    }

    #[test]
    fn test_cooldown_prevents_spam() {
        let g = EntropyGovernorCell::new(0.0, 0.0, 0.0);
        for _ in 0..50 {
            g.record(EntropyEvent::AxiomViolation {
                cell_id: "c1".into(),
                severity: 2.0,
            });
        }
        let a1 = g.take_action();
        assert!(!matches!(a1, GovernanceAction::None));
        let a2 = g.take_action();
        assert!(matches!(a2, GovernanceAction::None));
    }

    #[test]
    fn test_reset_returns_green() {
        let g = EntropyGovernorCell::default();
        for _ in 0..20 {
            g.record(EntropyEvent::AxiomViolation {
                cell_id: "c1".into(),
                severity: 1.0,
            });
        }
        g.reset();
        assert_eq!(g.snapshot().level, EntropyLevel::Green);
    }

    #[test]
    fn test_per_cell_tracking() {
        let g = EntropyGovernorCell::default();
        for _ in 0..5 {
            g.record(EntropyEvent::AxiomViolation {
                cell_id: "hot".into(),
                severity: 1.0,
            });
        }
        g.record(EntropyEvent::AxiomViolation {
            cell_id: "cold".into(),
            severity: 1.0,
        });
        let s = g.snapshot();
        assert!(s.per_cell.get("hot").unwrap() > s.per_cell.get("cold").unwrap());
    }
}
