//! EntropyGovernorCell - quantifies disorder and prescribes governance actions.
//!
//! Defined in axiom-runtime to avoid circular dependencies (oversight→runtime→oversight).
//! axiom-oversight re-exports these types for oversight-level consumers.
//! See [layer-leakage-exemptions] in architecture.toml for formal exemption.

use crate::constants::{DEFAULT_THROTTLE_FACTOR, ENTROPY_COOLDOWN_NS};
use axiom_kernel::clock::global_clock;
use axiom_kernel::entropy::{
    EntropyLevel, EntropyScore, CRITICAL_THRESHOLD, GREEN_THRESHOLD, RED_THRESHOLD,
    YELLOW_THRESHOLD,
};
use axiom_kernel::id::{CellId, WitnessId};
use axiom_kernel::witness::{Witness as KernelWitness, WitnessHash, WitnessMetrics};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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
    AxiomViolation { cell_id: String },
    DroppedMessage { cell_id: String },
    RejectedByGuardian { cell_id: String },
    CellRestart { cell_id: String },
    CircuitBreak { cell_id: String },
    Timeout { cell_id: String },
    DuplicateMessage { cell_id: String },
    StaleStateViolation { cell_id: String },
    Custom { cell_id: String, weight: f64 },
}

pub struct EntropyGovernorCell {
    id: CellId,
    global: Arc<Mutex<EntropyScore>>,
    per_cell: Arc<Mutex<HashMap<String, EntropyScore>>>,
    last_action_ns: Arc<Mutex<u64>>,
    last_action: Arc<Mutex<Option<GovernanceAction>>>,
    cooldown_ns: u64,
    witness_kernel: Option<Arc<axiom_kernel::WitnessKernel>>,
}

impl EntropyGovernorCell {
    pub fn new(green: f64, yellow: f64, red: f64, critical: f64) -> Self {
        Self {
            id: CellId::new("oversight:entropy-governor"),
            global: Arc::new(Mutex::new(
                EntropyScore::new().with_thresholds(green, yellow, red, critical),
            )),
            per_cell: Arc::new(Mutex::new(HashMap::new())),
            last_action_ns: Arc::new(Mutex::new(0)),
            last_action: Arc::new(Mutex::new(None)),
            cooldown_ns: ENTROPY_COOLDOWN_NS,
            witness_kernel: None,
        }
    }

    pub fn with_witness_kernel(
        green: f64,
        yellow: f64,
        red: f64,
        critical: f64,
        witness_kernel: Arc<axiom_kernel::WitnessKernel>,
    ) -> Self {
        Self {
            id: CellId::new("oversight:entropy-governor"),
            global: Arc::new(Mutex::new(
                EntropyScore::new().with_thresholds(green, yellow, red, critical),
            )),
            per_cell: Arc::new(Mutex::new(HashMap::new())),
            last_action_ns: Arc::new(Mutex::new(0)),
            last_action: Arc::new(Mutex::new(None)),
            cooldown_ns: ENTROPY_COOLDOWN_NS,
            witness_kernel: Some(witness_kernel),
        }
    }

    pub fn set_witness_kernel(&mut self, kernel: Arc<axiom_kernel::WitnessKernel>) {
        self.witness_kernel = Some(kernel);
    }

    pub async fn record_witness(&self, ev: &EntropyEvent) {
        if let Some(kernel) = &self.witness_kernel {
            let now = global_clock().now_ns();
            let _cell_id = match ev {
                EntropyEvent::AxiomViolation { cell_id } => cell_id.clone(),
                EntropyEvent::DroppedMessage { cell_id } => cell_id.clone(),
                EntropyEvent::RejectedByGuardian { cell_id } => cell_id.clone(),
                EntropyEvent::CellRestart { cell_id } => cell_id.clone(),
                EntropyEvent::CircuitBreak { cell_id } => cell_id.clone(),
                EntropyEvent::Timeout { cell_id } => cell_id.clone(),
                EntropyEvent::DuplicateMessage { cell_id } => cell_id.clone(),
                EntropyEvent::StaleStateViolation { cell_id } => cell_id.clone(),
                EntropyEvent::Custom { cell_id, .. } => cell_id.clone(),
            };
            let witness = KernelWitness {
                witness_id: WitnessId::new(format!("entropy-{}", now)),
                schema_version: axiom_kernel::version::SchemaVersion::new(1),
                cell_id: self.id.as_str().to_string(),
                correlation_id: axiom_kernel::id::CorrelationId::new("none"),
                trace_id: None,
                triggering_msg_id: None,
                vector_clock: axiom_kernel::signal::VectorClock::new(),
                timestamp_ns: now,
                prev_hash: Some(WitnessHash::zero()),
                state_before_hash: Some(WitnessHash::zero()),
                state_after_hash: Some(WitnessHash::zero()),
                hash: WitnessHash::zero(),
                summary: format!("entropy event for cell {}", self.id.as_str()),
                outcome: axiom_kernel::witness::TransitionOutcome::Success,
                metrics: WitnessMetrics::default(),
                version_info: axiom_kernel::version::VersionInfo::current(),
                signal_fingerprint: [0u8; 32],
                payload_size_bytes: 0,
                kind: axiom_kernel::witness::WitnessKind::StateTransition,
            };
            kernel.record(witness).await;
        }
    }

    pub fn id(&self) -> &CellId {
        &self.id
    }

    pub fn record(&self, ev: EntropyEvent) {
        let now = global_clock().now_ns();
        let cell_id = match &ev {
            EntropyEvent::AxiomViolation { cell_id } => cell_id.clone(),
            EntropyEvent::DroppedMessage { cell_id } => cell_id.clone(),
            EntropyEvent::RejectedByGuardian { cell_id } => cell_id.clone(),
            EntropyEvent::CellRestart { cell_id } => cell_id.clone(),
            EntropyEvent::CircuitBreak { cell_id } => cell_id.clone(),
            EntropyEvent::Timeout { cell_id } => cell_id.clone(),
            EntropyEvent::DuplicateMessage { cell_id } => cell_id.clone(),
            EntropyEvent::StaleStateViolation { cell_id } => cell_id.clone(),
            EntropyEvent::Custom { cell_id, .. } => cell_id.clone(),
        };

        let mut global = self.global.lock();
        match &ev {
            EntropyEvent::AxiomViolation { .. } => global.record_axiom_violation(),
            EntropyEvent::DroppedMessage { .. } => global.record_dropped_message(),
            EntropyEvent::RejectedByGuardian { .. } => global.record_rejected_by_guardian(),
            EntropyEvent::CellRestart { .. } => global.record_cell_restart(),
            EntropyEvent::CircuitBreak { .. } => global.record_circuit_break(),
            EntropyEvent::Timeout { .. } => global.record_timeout(),
            EntropyEvent::DuplicateMessage { .. } => global.record_duplicate_message(),
            EntropyEvent::StaleStateViolation { .. } => global.record_stale_state_violation(),
            EntropyEvent::Custom { weight, .. } => global.record_custom(*weight),
        }
        global.last_updated_ns = now;
        let gt = global.green_threshold;
        let yt = global.yellow_threshold;
        let rt = global.red_threshold;
        let ct = global.critical_threshold;
        drop(global);

        if !cell_id.is_empty() {
            let mut per_cell = self.per_cell.lock();
            let entry = per_cell
                .entry(cell_id.clone())
                .or_insert_with(|| EntropyScore::new().with_thresholds(gt, yt, rt, ct));
            match &ev {
                EntropyEvent::AxiomViolation { .. } => entry.record_axiom_violation(),
                EntropyEvent::DroppedMessage { .. } => entry.record_dropped_message(),
                EntropyEvent::RejectedByGuardian { .. } => entry.record_rejected_by_guardian(),
                EntropyEvent::CellRestart { .. } => entry.record_cell_restart(),
                EntropyEvent::CircuitBreak { .. } => entry.record_circuit_break(),
                EntropyEvent::Timeout { .. } => entry.record_timeout(),
                EntropyEvent::DuplicateMessage { .. } => entry.record_duplicate_message(),
                EntropyEvent::StaleStateViolation { .. } => entry.record_stale_state_violation(),
                EntropyEvent::Custom { weight, .. } => entry.record_custom(*weight),
            }
            entry.last_updated_ns = now;
        }
    }

    pub fn decay_tick(&self) {
        let now = global_clock().now_ns();
        let mut g = self.global.lock();
        g.decay(now);
        drop(g);
        let mut per_cell = self.per_cell.lock();
        for s in per_cell.values_mut() {
            s.decay(now);
        }
    }

    pub fn snapshot(&self) -> EntropySnapshot {
        let now = global_clock().now_ns();
        let g = *self.global.lock();
        let level = g.level();
        let per_cell = self.per_cell.lock().iter().map(|(k, v)| (k.clone(), v.value)).collect();
        let last_action_ns = *self.last_action_ns.lock();
        let cooldown_remaining = (last_action_ns + self.cooldown_ns).saturating_sub(now);
        EntropySnapshot {
            global: g,
            per_cell,
            level,
            last_action: self.last_action.lock().clone(),
            last_action_ns,
            cooldown_remaining_ns: cooldown_remaining,
        }
    }

    pub fn take_action(&self) -> GovernanceAction {
        let snap = self.snapshot();
        let now = global_clock().now_ns();
        let mut last_action_ns = self.last_action_ns.lock();
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
                GovernanceAction::Throttle { target_cell: hottest, factor: DEFAULT_THROTTLE_FACTOR }
            }
            EntropyLevel::Critical => GovernanceAction::Emergency {
                reason: format!("entropy critical: {:.3}", snap.global.value),
            },
        };

        if !matches!(action, GovernanceAction::None) {
            *last_action_ns = now;
            *self.last_action.lock() = Some(action.clone());
        }
        action
    }

    /// Returns true when entropy reaches Red or Critical level and the cooldown
    /// has elapsed. On returning true, the cooldown timer is updated.
    pub fn should_reduce(&self, _cooldown_ms: u64) -> bool {
        let g = *self.global.lock();
        let hot = g.is_red() || g.is_critical();
        if !hot {
            return false;
        }
        let now = global_clock().now_ns();
        let mut last_action_ns = self.last_action_ns.lock();
        if *last_action_ns + self.cooldown_ns > now {
            return false;
        }
        *last_action_ns = now;
        true
    }

    pub fn reset(&self) {
        let now = global_clock().now_ns();
        let mut g = self.global.lock();
        let gt = g.green_threshold;
        let yt = g.yellow_threshold;
        let rt = g.red_threshold;
        let ct = g.critical_threshold;
        *g = EntropyScore::new().with_thresholds(gt, yt, rt, ct);
        g.last_updated_ns = now;
        drop(g);
        self.per_cell.lock().clear();
    }
}

impl Default for EntropyGovernorCell {
    fn default() -> Self {
        Self::new(GREEN_THRESHOLD, YELLOW_THRESHOLD, RED_THRESHOLD, CRITICAL_THRESHOLD)
    }
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
        let g = EntropyGovernorCell::new(1.0, 5.0, 10.0, 20.0);
        for _ in 0..10 {
            g.record(EntropyEvent::AxiomViolation { cell_id: "c1".into() });
            g.record(EntropyEvent::CellRestart { cell_id: "c1".into() });
            g.record(EntropyEvent::CircuitBreak { cell_id: "c1".into() });
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
        let g = EntropyGovernorCell::new(0.0, 0.0, 0.0, 0.0);
        for _ in 0..50 {
            g.record(EntropyEvent::AxiomViolation { cell_id: "c1".into() });
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
            g.record(EntropyEvent::AxiomViolation { cell_id: "c1".into() });
        }
        g.reset();
        assert_eq!(g.snapshot().level, EntropyLevel::Green);
    }

    #[test]
    fn test_per_cell_tracking() {
        let g = EntropyGovernorCell::default();
        for _ in 0..5 {
            g.record(EntropyEvent::AxiomViolation { cell_id: "hot".into() });
        }
        g.record(EntropyEvent::AxiomViolation { cell_id: "cold".into() });
        let s = g.snapshot();
        assert!(s.per_cell.get("hot").unwrap() > s.per_cell.get("cold").unwrap());
    }

    #[test]
    fn test_all_entropy_event_types() {
        let g = EntropyGovernorCell::default();
        g.record(EntropyEvent::AxiomViolation { cell_id: "c".into() });
        g.record(EntropyEvent::DroppedMessage { cell_id: "c".into() });
        g.record(EntropyEvent::RejectedByGuardian { cell_id: "c".into() });
        g.record(EntropyEvent::CellRestart { cell_id: "c".into() });
        g.record(EntropyEvent::CircuitBreak { cell_id: "c".into() });
        g.record(EntropyEvent::Timeout { cell_id: "c".into() });
        g.record(EntropyEvent::DuplicateMessage { cell_id: "c".into() });
        g.record(EntropyEvent::StaleStateViolation { cell_id: "c".into() });
        let s = g.snapshot();
        assert!(s.global.value > 0.0);
    }

    #[test]
    fn test_entropy_accumulates() {
        let g = EntropyGovernorCell::new(100.0, 100.0, 100.0, 100.0);
        g.record(EntropyEvent::DroppedMessage { cell_id: "c".into() });
        g.record(EntropyEvent::DroppedMessage { cell_id: "c".into() });
        g.record(EntropyEvent::RejectedByGuardian { cell_id: "c".into() });
        let s = g.snapshot();
        assert_eq!(s.global.dropped_messages, 2);
        assert_eq!(s.global.rejected_by_guardian, 1);
        assert!(s.global.value > 0.0);
    }

    #[test]
    fn test_entropy_reset() {
        let g = EntropyGovernorCell::new(100.0, 100.0, 100.0, 100.0);
        g.record(EntropyEvent::DroppedMessage { cell_id: "c".into() });
        g.record(EntropyEvent::CircuitBreak { cell_id: "c".into() });
        g.reset();
        let s = g.snapshot();
        assert_eq!(s.global.value, 0.0);
    }

    #[test]
    fn test_entropy_uses_core_weights() {
        let g = EntropyGovernorCell::new(100.0, 100.0, 100.0, 100.0);
        g.record(EntropyEvent::CellRestart { cell_id: "c".into() });
        let s1 = g.snapshot().global.value;
        g.reset();
        g.record(EntropyEvent::DuplicateMessage { cell_id: "c".into() });
        let s2 = g.snapshot().global.value;
        assert!(
            s1 > s2,
            "cell_restart (weight 5.0) should score higher than duplicate (weight 0.5): {s1} vs {s2}"
        );
    }
}
