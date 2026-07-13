use crate::compliance_guard::{ComplianceResult, ComplianceViolation};
use crate::entropy_governor::EntropySnapshot;
use crate::health::SystemHealth;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OversightDataSourceError {
    #[error("data source not initialized")]
    NotInitialized,
    #[error("failed to get system health")]
    HealthFailed,
    #[error("failed to get entropy status")]
    EntropyFailed,
    #[error("failed to get compliance report")]
    ComplianceFailed,
}

pub trait OversightDataSource: Send + Sync {
    fn get_system_health(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<SystemHealth, OversightDataSourceError>>
                + Send
                + '_,
        >,
    >;
    fn get_entropy_status(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<EntropySnapshot, OversightDataSourceError>>
                + Send
                + '_,
        >,
    >;
    fn get_compliance_report(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<ComplianceReportData, OversightDataSourceError>>
                + Send
                + '_,
        >,
    >;
}

#[derive(Debug, Clone)]
pub struct ComplianceReportData {
    pub violations: Vec<ComplianceViolation>,
    pub checks: Vec<ComplianceResult>,
    pub total_violations: u64,
    pub passed_checks: usize,
    pub failed_checks: usize,
}
