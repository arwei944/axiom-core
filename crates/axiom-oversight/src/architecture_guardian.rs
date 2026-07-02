//! ArchitectureGuardian - enforces architectural self-constraints with violation stats.
//!
//! As an Oversight component, it tracks violations and produces audit records.

use axiom_core::id::CellId;
use axiom_core::layer::Layer;
use axiom_core::signal::SignalEnvelope;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GuardianStats {
    pub layer_violations: HashMap<String, u64>,
    pub hop_limit_exceeded: u64,
    pub schema_version_mismatch: u64,
    pub unknown_signal_type: u64,
    pub total_intercepted: u64,
    pub total_allowed: u64,
    pub total_rejected: u64,
}

#[derive(Debug, Clone)]
pub struct ViolationEvent {
    pub kind: String,
    pub from: Option<Layer>,
    pub to: Option<Layer>,
    pub signal_type: String,
    pub reason: String,
    pub timestamp_ns: u64,
}

pub struct ArchitectureGuardianCell {
    id: CellId,
    stats: Arc<Mutex<GuardianStats>>,
    recent_violations: Arc<Mutex<Vec<ViolationEvent>>>,
    max_recent: usize,
}

impl ArchitectureGuardianCell {
    pub fn new() -> Self {
        Self {
            id: CellId::new("oversight:architecture-guardian"),
            stats: Arc::new(Mutex::new(GuardianStats::default())),
            recent_violations: Arc::new(Mutex::new(Vec::new())),
            max_recent: 128,
        }
    }

    pub fn id(&self) -> &CellId {
        &self.id
    }

    pub fn check_envelope(&self, env: &SignalEnvelope) -> Result<(), ViolationEvent> {
        let mut stats = self.stats.lock();
        stats.total_intercepted += 1;

        if !env.source_layer.can_send_to(env.target_layer) {
            stats.total_rejected += 1;
            let key = format!("{:?}->{:?}", env.source_layer, env.target_layer);
            *stats.layer_violations.entry(key).or_insert(0) += 1;
            let ev = ViolationEvent {
                kind: "layer_violation".into(),
                from: Some(env.source_layer),
                to: Some(env.target_layer),
                signal_type: env.signal_type.clone(),
                reason: format!(
                    "illegal cross-layer {} -> {}",
                    env.source_layer, env.target_layer
                ),
                timestamp_ns: env.timestamp_ns,
            };
            drop(stats);
            self.record_violation(&ev);
            return Err(ev);
        }

        if env.hop_count > 8 {
            stats.total_rejected += 1;
            stats.hop_limit_exceeded += 1;
            let ev = ViolationEvent {
                kind: "hop_limit".into(),
                from: Some(env.source_layer),
                to: Some(env.target_layer),
                signal_type: env.signal_type.clone(),
                reason: format!("hop count {} exceeded limit 8", env.hop_count),
                timestamp_ns: env.timestamp_ns,
            };
            drop(stats);
            self.record_violation(&ev);
            return Err(ev);
        }

        if env.schema_version.0 == 0 {
            stats.total_rejected += 1;
            stats.schema_version_mismatch += 1;
            let ev = ViolationEvent {
                kind: "schema_version".into(),
                from: Some(env.source_layer),
                to: Some(env.target_layer),
                signal_type: env.signal_type.clone(),
                reason: "schema version 0 is reserved".into(),
                timestamp_ns: env.timestamp_ns,
            };
            drop(stats);
            self.record_violation(&ev);
            return Err(ev);
        }

        stats.total_allowed += 1;
        Ok(())
    }

    pub fn report_violation(&self, kind: &str, reason: String, signal_type: String) {
        let mut stats = self.stats.lock();
        stats.total_rejected += 1;
        *stats.layer_violations.entry(kind.to_string()).or_insert(0) += 1;
        drop(stats);
        let ev = ViolationEvent {
            kind: kind.into(),
            from: None,
            to: None,
            signal_type,
            reason,
            timestamp_ns: axiom_core::signal::now_ns(),
        };
        self.record_violation(&ev);
    }

    fn record_violation(&self, ev: &ViolationEvent) {
        let mut recent = self.recent_violations.lock();
        recent.push(ev.clone());
        while recent.len() > self.max_recent {
            recent.remove(0);
        }
    }

    pub fn stats(&self) -> GuardianStats {
        self.stats.lock().clone()
    }

    pub fn recent_violations(&self, n: usize) -> Vec<ViolationEvent> {
        let recent = self.recent_violations.lock();
        recent.iter().rev().take(n).cloned().collect()
    }
}

impl Default for ArchitectureGuardianCell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_core::id::{CorrelationId, MsgId};
    use axiom_core::signal::{SignalKind, VectorClock};

    fn env(from: Layer, to: Layer, hop_count: u32) -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new("m"),
            correlation_id: CorrelationId::new("c"),
            trace_id: None,
            signal_type: "T".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            kind: SignalKind::Command,
            source_layer: from,
            target_layer: to,
            source_cell: None,
            target_cell: Some("c1".into()),
            payload: serde_json::Value::Null,
            schema_version: axiom_core::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count,
        }
    }

    #[test]
    fn test_legal_direction_allowed() {
        let c = ArchitectureGuardianCell::new();
        assert!(c
            .check_envelope(&env(Layer::Oversight, Layer::Agent, 0))
            .is_ok());
        assert!(c
            .check_envelope(&env(Layer::Agent, Layer::Validate, 0))
            .is_ok());
        assert!(c
            .check_envelope(&env(Layer::Validate, Layer::Exec, 0))
            .is_ok());
        assert_eq!(c.stats().total_allowed, 3);
        assert_eq!(c.stats().total_rejected, 0);
    }

    #[test]
    fn test_illegal_direction_rejected() {
        let c = ArchitectureGuardianCell::new();
        assert!(c
            .check_envelope(&env(Layer::Exec, Layer::Agent, 0))
            .is_err());
        assert!(c
            .check_envelope(&env(Layer::Exec, Layer::Oversight, 0))
            .is_err());
        assert_eq!(c.stats().total_rejected, 2);
        assert_eq!(c.stats().layer_violations.len(), 2);
    }

    #[test]
    fn test_hop_limit_rejected() {
        let c = ArchitectureGuardianCell::new();
        assert!(c.check_envelope(&env(Layer::Exec, Layer::Exec, 9)).is_err());
        assert_eq!(c.stats().hop_limit_exceeded, 1);
    }

    #[test]
    fn test_schema_version_zero_rejected() {
        let c = ArchitectureGuardianCell::new();
        let mut e = env(Layer::Exec, Layer::Exec, 0);
        e.schema_version = axiom_core::SchemaVersion::new(0);
        assert!(c.check_envelope(&e).is_err());
        assert_eq!(c.stats().schema_version_mismatch, 1);
    }

    #[test]
    fn test_recent_violations_bounded() {
        let c = ArchitectureGuardianCell::new();
        for _ in 0..200 {
            let _ = c.check_envelope(&env(Layer::Exec, Layer::Agent, 0));
        }
        assert!(c.recent_violations(1000).len() <= 128);
    }
}
