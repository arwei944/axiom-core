//! EntropyGovernor - monitors and manages system entropy.

use axiom_core::entropy::EntropyScore;

#[allow(dead_code)]
pub struct EntropyGovernor {
    system_entropy: EntropyScore,
    yellow_threshold: f64,
    red_threshold: f64,
}

impl EntropyGovernor {
    pub fn new() -> Self {
        Self {
            system_entropy: EntropyScore::new(),
            yellow_threshold: 0.4,
            red_threshold: 0.8,
        }
    }

    pub fn update(&mut self, score: EntropyScore) {
        self.system_entropy = score;
        let value = self.system_entropy.compute();

        if self.system_entropy.is_red() {
            tracing::error!(entropy = value, "SYSTEM ENTROPY RED - triggering de-entropy");
        } else if self.system_entropy.is_yellow() {
            tracing::warn!(entropy = value, "System entropy elevated (yellow)");
        }
    }

    pub fn current_entropy(&self) -> f64 {
        self.system_entropy.value
    }
}

impl Default for EntropyGovernor {
    fn default() -> Self {
        Self::new()
    }
}
