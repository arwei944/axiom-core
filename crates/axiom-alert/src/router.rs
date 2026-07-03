//! Alert routing and escalation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::alert::{Alert, Severity};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlertRouter {
    routes: HashMap<String, Vec<Severity>>,
}

impl AlertRouter {
    pub fn add_route(&mut self, tag: &str, severities: Vec<Severity>) {
        self.routes.insert(tag.to_string(), severities);
    }

    pub fn route(&self, alert: &Alert) -> Vec<Severity> {
        self.routes
            .get(&alert.rule_id)
            .cloned()
            .unwrap_or_else(|| vec![alert.severity])
    }
}
