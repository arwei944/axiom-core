//! Kernel integration for `axiom-alert`.
//!
//! Provides adapters so alerts can be emitted through the kernel runtime
//! and recorded as witnesses.

use crate::alert::Alert;

/// Adapter that exposes alert emission through the kernel runtime.
pub struct AlertKernelAdapter {
    alerts: Vec<Alert>,
}

impl Default for AlertKernelAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AlertKernelAdapter {
    pub fn new() -> Self {
        Self { alerts: Vec::new() }
    }

    pub fn push(&mut self, alert: Alert) {
        self.alerts.push(alert);
    }

    pub fn alerts(&self) -> &[Alert] {
        &self.alerts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::Severity;

    #[test]
    fn adapter_records_alerts() {
        let mut adapter = AlertKernelAdapter::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        adapter.push(Alert {
            id: "a1".into(),
            rule_id: "rule-1".into(),
            severity: Severity::Info,
            status: crate::AlertStatus::Pending,
            message: "test".into(),
            labels: Vec::new(),
            starts_at_ns: now,
            ends_at_ns: None,
            generator_url: None,
        });
        assert_eq!(adapter.alerts().len(), 1);
    }
}
