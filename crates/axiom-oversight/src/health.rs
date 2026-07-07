//! HealthCollector - aggregates system-wide health state.

use crate::entropy_governor::{EntropyGovernorCell, EntropyLevel, EntropySnapshot};
use crate::resource_manager::ResourceStats;
use axiom_kernel::id::CellId;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellHealth {
    pub id: String,
    pub layer: String,
    pub state: String,
    pub processed_messages: u64,
    pub failed_messages: u64,
    pub restart_count: u32,
    pub last_message_ns: Option<u64>,
    pub mailbox_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStats {
    pub total_delivered: u64,
    pub total_rejected: u64,
    pub dlq_size: usize,
    pub active_cells: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OversightHealth {
    pub interceptor_count: usize,
    pub unresponsive_cells: Vec<String>,
    pub compliance_violations: HashMap<String, u64>,
    pub guardian_stats: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub status: HealthStatus,
    pub uptime_secs: u64,
    pub version: String,
    pub cells: Vec<CellHealth>,
    pub entropy: Option<EntropySnapshot>,
    pub messages: MessageStats,
    pub resources: Option<ResourceStats>,
    pub oversight: OversightHealth,
    pub started_at: u64,
}

pub struct HealthCollectorCell {
    id: CellId,
    started_at: Instant,
    started_at_unix: u64,
    cells: Arc<Mutex<HashMap<String, CellHealth>>>,
    message_stats: Arc<Mutex<MessageStats>>,
    entropy: Arc<Mutex<Option<Arc<EntropyGovernorCell>>>>,
    resources: Arc<Mutex<Option<Arc<crate::resource_manager::ResourceManagerCell>>>>,
    oversight_health: Arc<Mutex<OversightHealth>>,
}

impl HealthCollectorCell {
    pub fn new() -> Self {
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            id: CellId::new("oversight:health-collector"),
            started_at: Instant::now(),
            started_at_unix: now_unix,
            cells: Arc::new(Mutex::new(HashMap::new())),
            message_stats: Arc::new(Mutex::new(MessageStats {
                total_delivered: 0,
                total_rejected: 0,
                dlq_size: 0,
                active_cells: 0,
            })),
            entropy: Arc::new(Mutex::new(None)),
            resources: Arc::new(Mutex::new(None)),
            oversight_health: Arc::new(Mutex::new(OversightHealth {
                interceptor_count: 0,
                unresponsive_cells: Vec::new(),
                compliance_violations: HashMap::new(),
                guardian_stats: HashMap::new(),
            })),
        }
    }

    pub fn id(&self) -> &CellId {
        &self.id
    }

    pub fn update_cell(&self, health: CellHealth) {
        self.cells.lock().insert(health.id.clone(), health);
    }

    pub fn set_message_stats(
        &self,
        delivered: u64,
        rejected: u64,
        dlq: usize,
        active_cells: usize,
    ) {
        let mut m = self.message_stats.lock();
        m.total_delivered = delivered;
        m.total_rejected = rejected;
        m.dlq_size = dlq;
        m.active_cells = active_cells;
    }

    pub fn bind_entropy(&self, g: Arc<EntropyGovernorCell>) {
        *self.entropy.lock() = Some(g);
    }

    pub fn bind_resources(&self, r: Arc<crate::resource_manager::ResourceManagerCell>) {
        *self.resources.lock() = Some(r);
    }

    pub fn update_oversight(&self, h: OversightHealth) {
        *self.oversight_health.lock() = h;
    }

    pub fn collect(&self) -> SystemHealth {
        let uptime = self.started_at.elapsed().as_secs();
        let cells: Vec<CellHealth> = self.cells.lock().values().cloned().collect();
        let messages = self.message_stats.lock().clone();
        let entropy = self.entropy.lock().as_ref().map(|g| g.snapshot());
        let resources = self.resources.lock().as_ref().map(|r| r.stats());
        let oversight = self.oversight_health.lock().clone();

        let mut status = HealthStatus::Healthy;

        if let Some(ref s) = entropy {
            match s.level {
                EntropyLevel::Critical => status = HealthStatus::Critical,
                EntropyLevel::Red => {
                    if status == HealthStatus::Healthy {
                        status = HealthStatus::Degraded;
                    }
                }
                EntropyLevel::Yellow => {
                    if status == HealthStatus::Healthy {
                        status = HealthStatus::Degraded;
                    }
                }
                EntropyLevel::Green => {}
            }
        }

        if !oversight.unresponsive_cells.is_empty() {
            status = HealthStatus::Critical;
        }

        let any_stopped = cells
            .iter()
            .any(|c| c.state == "Stopped" || c.state == "Crashed");
        if any_stopped {
            status = HealthStatus::Critical;
        }

        let any_restarting = cells
            .iter()
            .any(|c| c.state == "Restarting" || c.state == "CircuitOpen");
        if any_restarting && status == HealthStatus::Healthy {
            status = HealthStatus::Degraded;
        }

        SystemHealth {
            status,
            uptime_secs: uptime,
            version: env!("CARGO_PKG_VERSION").to_string(),
            cells,
            entropy,
            messages,
            resources,
            oversight,
            started_at: self.started_at_unix,
        }
    }
}

impl Default for HealthCollectorCell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healthy_by_default() {
        let h = HealthCollectorCell::new();
        let s = h.collect();
        assert_eq!(s.status, HealthStatus::Healthy);
        assert_eq!(s.cells.len(), 0);
    }

    #[test]
    fn test_crashed_cell_critical() {
        let h = HealthCollectorCell::new();
        h.update_cell(CellHealth {
            id: "c1".into(),
            layer: "Exec".into(),
            state: "Crashed".into(),
            processed_messages: 0,
            failed_messages: 0,
            restart_count: 0,
            last_message_ns: None,
            mailbox_depth: 0,
        });
        let s = h.collect();
        assert_eq!(s.status, HealthStatus::Critical);
    }

    #[test]
    fn test_degraded_on_restarting() {
        let h = HealthCollectorCell::new();
        h.update_cell(CellHealth {
            id: "c1".into(),
            layer: "Exec".into(),
            state: "Restarting".into(),
            processed_messages: 10,
            failed_messages: 2,
            restart_count: 1,
            last_message_ns: None,
            mailbox_depth: 0,
        });
        let s = h.collect();
        assert_eq!(s.status, HealthStatus::Degraded);
    }
}
