//! Single entropy authority (constitution: one Governor).

use crate::error::{IsaError, IsaResult};
use crate::journal::WitnessJournal;
use crate::primitives::StepKind;
use axiom_kernel::entropy::{EntropyLevel, EntropyScore, RED_THRESHOLD};

/// Decision emitted by the sole Governor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Reject { reason: String },
}

#[derive(Debug, Clone)]
pub struct GovernorConfig {
    /// Reject when entropy level is Red or Critical.
    pub reject_from: EntropyLevel,
    /// Optional hard open (manual trip).
    pub force_open: bool,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            reject_from: EntropyLevel::Red,
            force_open: false,
        }
    }
}

/// Unique entropy / admit decision surface for ULE.
///
/// Collectors may feed metrics; only Governor decides admit/reject.
pub struct Governor {
    pub entropy: EntropyScore,
    pub config: GovernorConfig,
}

impl Governor {
    pub fn new() -> Self {
        Self {
            entropy: EntropyScore::new(),
            config: GovernorConfig::default(),
        }
    }

    pub fn with_config(config: GovernorConfig) -> Self {
        Self {
            entropy: EntropyScore::new(),
            config,
        }
    }

    /// Lower thresholds so demos can trip Red quickly.
    pub fn for_demo() -> Self {
        let mut g = Self::new();
        // green < 0.5, yellow < 1.0, red >= 1.0, critical >= 3.0
        g.entropy = g.entropy.with_thresholds(0.5, 1.0, 1.0, 3.0);
        g
    }

    pub fn level(&self) -> EntropyLevel {
        self.entropy.level()
    }

    pub fn score(&self) -> f64 {
        self.entropy.value
    }

    pub fn decide(&self) -> Decision {
        if self.config.force_open {
            return Decision::Reject {
                reason: "governor force_open".into(),
            };
        }
        let level = self.entropy.level();
        let reject = match (self.config.reject_from, level) {
            (EntropyLevel::Yellow, EntropyLevel::Yellow)
            | (EntropyLevel::Yellow, EntropyLevel::Red)
            | (EntropyLevel::Yellow, EntropyLevel::Critical) => true,
            (EntropyLevel::Red, EntropyLevel::Red)
            | (EntropyLevel::Red, EntropyLevel::Critical) => true,
            (EntropyLevel::Critical, EntropyLevel::Critical) => true,
            _ => false,
        };
        if reject {
            Decision::Reject {
                reason: format!(
                    "entropy {:?} (value={:.2}) exceeds admit policy",
                    level, self.entropy.value
                ),
            }
        } else {
            Decision::Allow
        }
    }

    /// Admit gate used at Cell entry before Composer runs.
    pub fn admit(&self, journal: &mut WitnessJournal<'_>) -> IsaResult<()> {
        match self.decide() {
            Decision::Allow => {
                journal.record_ok(
                    StepKind::Governor,
                    "governor",
                    &format!("admit level={:?}", self.level()),
                )?;
                Ok(())
            }
            Decision::Reject { reason } => {
                let _ = journal.record_rejected(&reason);
                Err(IsaError::rejected(reason))
            }
        }
    }

    pub fn record_port_failure(&mut self) {
        self.entropy.record_timeout();
    }

    pub fn record_circuit_break(&mut self) {
        self.entropy.record_circuit_break();
    }

    pub fn record_axiom_violation(&mut self) {
        self.entropy.record_axiom_violation();
    }

    pub fn record_rejected(&mut self) {
        self.entropy.record_rejected_by_guardian();
    }

    pub fn trip(&mut self) {
        self.config.force_open = true;
    }

    pub fn reset(&mut self) {
        self.config.force_open = false;
        self.entropy.reset();
    }

    /// Convenience: how many circuit_break weight units to reach Red with default weights.
    pub fn red_threshold_hint() -> f64 {
        RED_THRESHOLD
    }
}

impl Default for Governor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_kernel::context::CellContext;
    use axiom_kernel::id::CellId;
    use axiom_kernel::RuntimeTier;

    #[test]
    fn admits_when_green() {
        let cell_id = CellId::new("g");
        let mut ctx = CellContext::new(&cell_id, RuntimeTier::Exec);
        let mut journal = WitnessJournal::new(&mut ctx);
        let g = Governor::new();
        assert!(g.admit(&mut journal).is_ok());
    }

    #[test]
    fn rejects_when_tripped() {
        let cell_id = CellId::new("g");
        let mut ctx = CellContext::new(&cell_id, RuntimeTier::Exec);
        let mut journal = WitnessJournal::new(&mut ctx);
        let mut g = Governor::new();
        g.trip();
        assert!(matches!(g.admit(&mut journal), Err(IsaError::Rejected { .. })));
    }

    #[test]
    fn circuit_breaks_raise_entropy() {
        let mut g = Governor::for_demo();
        assert_eq!(g.level(), EntropyLevel::Green);
        // WEIGHT_CIRCUIT_BREAKS = 4.0 → one break is enough for red threshold 1.0
        g.record_circuit_break();
        assert!(matches!(g.level(), EntropyLevel::Red | EntropyLevel::Critical));
        assert!(matches!(g.decide(), Decision::Reject { .. }));
    }
}
