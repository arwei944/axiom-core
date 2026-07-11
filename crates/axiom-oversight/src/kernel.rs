//! Kernel integration for `axiom-oversight`.
//!
//! Provides adapters so oversight governance can consume kernel witness
//! chains and entropy state.

use crate::api::{ComplianceReportData, OversightDataSource, OversightDataSourceError};
use crate::compliance_guard::ComplianceGuardCell;
use crate::entropy_governor::{EntropyGovernorCell, EntropySnapshot, GovernanceAction};
use crate::health::{HealthCollectorCell, SystemHealth};
use std::sync::Arc;

pub struct OversightKernelAdapter {
    governor: Arc<EntropyGovernorCell>,
    health_collector: Arc<HealthCollectorCell>,
    #[allow(dead_code)]
    compliance_guard: Arc<ComplianceGuardCell>,
}

impl OversightKernelAdapter {
    pub fn new(
        governor: Arc<EntropyGovernorCell>,
        health_collector: Arc<HealthCollectorCell>,
        compliance_guard: Arc<ComplianceGuardCell>,
    ) -> Self {
        Self {
            governor,
            health_collector,
            compliance_guard,
        }
    }

    pub fn governor(&self) -> &EntropyGovernorCell {
        &self.governor
    }

    pub fn evaluate(&self) -> Vec<GovernanceAction> {
        let action = self.governor.take_action();
        vec![action]
    }
}

impl OversightDataSource for OversightKernelAdapter {
    fn get_system_health(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SystemHealth, OversightDataSourceError>> + Send + '_>> {
        Box::pin(async {
            Ok(self.health_collector.collect())
        })
    }

    fn get_entropy_status(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<EntropySnapshot, OversightDataSourceError>> + Send + '_>> {
        Box::pin(async {
            Ok(self.governor.snapshot())
        })
    }

    fn get_compliance_report(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ComplianceReportData, OversightDataSourceError>> + Send + '_>> {
        Box::pin(async {
            Ok(ComplianceReportData {
                violations: Vec::new(),
                checks: Vec::new(),
                total_violations: 0,
                passed_checks: 0,
                failed_checks: 0,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_wraps_governor() {
        let governor = Arc::new(EntropyGovernorCell::default());
        let health_collector = Arc::new(HealthCollectorCell::new());
        let compliance_guard = Arc::new(ComplianceGuardCell::new());
        let adapter = OversightKernelAdapter::new(governor, health_collector, compliance_guard);
        let actions = adapter.evaluate();
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], GovernanceAction::None));
    }
}
