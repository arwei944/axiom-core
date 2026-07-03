//! Core alert types.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::threshold::{Threshold, Window};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warn,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertStatus {
    Pending,
    Firing,
    Resolved,
    Silenced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub severity: Severity,
    pub status: AlertStatus,
    pub message: String,
    pub labels: Vec<(String, String)>,
    pub starts_at_ns: u64,
    pub ends_at_ns: Option<u64>,
    pub generator_url: Option<String>,
}

impl Alert {
    pub fn new(rule_id: &str, severity: Severity, message: impl Into<String>) -> Self {
        let id = format!("alert-{}", Uuid::new_v4());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        Self {
            id,
            rule_id: rule_id.to_string(),
            severity,
            status: AlertStatus::Pending,
            message: message.into(),
            labels: Vec::new(),
            starts_at_ns: now,
            ends_at_ns: None,
            generator_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub expression: String,
    pub severity: Severity,
    pub window: Window,
    pub threshold: Threshold,
    pub labels: Vec<(String, String)>,
    pub enabled: bool,
}

impl AlertRule {
    pub fn new(id: &str, expression: &str, severity: Severity) -> Self {
        Self {
            id: id.to_string(),
            expression: expression.to_string(),
            severity,
            window: Window::default(),
            threshold: Threshold::default(),
            labels: Vec::new(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSink {
    Log,
    Runtime,
}
