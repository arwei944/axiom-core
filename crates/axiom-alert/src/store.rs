//! In-memory alert store.

use serde::{Deserialize, Serialize};

use crate::alert::{Alert, AlertStatus};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryAlertStore {
    alerts: Vec<Alert>,
}

impl MemoryAlertStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, alert: Alert) {
        self.alerts.push(alert);
    }

    pub fn update_status(&mut self, id: &str, status: AlertStatus) -> bool {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.id == id) {
            alert.status = status;
            true
        } else {
            false
        }
    }

    pub fn query(&self, status: Option<AlertStatus>) -> Vec<&Alert> {
        self.alerts.iter().filter(|a| status.map(|s| s == a.status).unwrap_or(true)).collect()
    }
}
