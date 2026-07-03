//! Governance integration: alert -> oversight action.

use crate::alert::{Alert, AlertStatus};

pub struct GovernanceMapper;

impl GovernanceMapper {
    pub fn map(alert: &Alert, _governor: &axiom_oversight::entropy_governor::EntropyGovernorCell) -> Option<axiom_oversight::entropy_governor::GovernanceAction> {
        match alert.status {
            AlertStatus::Firing => match alert.severity {
                crate::Severity::Critical => {
                    let reason = format!("alert critical: {}", alert.message);
                    Some(axiom_oversight::entropy_governor::GovernanceAction::Emergency { reason })
                }
                crate::Severity::Warn => Some(axiom_oversight::entropy_governor::GovernanceAction::Warn {
                    message: alert.message.clone(),
                }),
                crate::Severity::Info => None,
            },
            AlertStatus::Resolved => Some(axiom_oversight::entropy_governor::GovernanceAction::None),
            AlertStatus::Silenced | AlertStatus::Pending => None,
        }
    }
}
