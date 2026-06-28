//! Built-in BusInterceptors beyond ArchitectureGuardian.

use crate::bus::{BusInterceptor, InterceptDecision};
use crate::loop_detector::LoopDetector;
use axiom_core::signal::SignalEnvelope;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::RwLock;

pub struct HopLimitInterceptor {
    max_hops: u32,
}

impl HopLimitInterceptor {
    pub fn new(max_hops: u32) -> Self {
        Self { max_hops }
    }
}

impl Default for HopLimitInterceptor {
    fn default() -> Self {
        Self::new(8)
    }
}

impl BusInterceptor for HopLimitInterceptor {
    fn name(&self) -> &'static str {
        "hop-limit"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        if env.hop_count > self.max_hops {
            InterceptDecision::Reject {
                reason: format!("hop limit {} exceeded (got {})", self.max_hops, env.hop_count),
            }
        } else {
            InterceptDecision::Allow
        }
    }
}

pub struct IdempotencyInterceptor {
    seen: RwLock<HashSet<String>>,
    capacity: usize,
}

impl IdempotencyInterceptor {
    pub fn new(capacity: usize) -> Self {
        Self {
            seen: RwLock::new(HashSet::with_capacity(capacity)),
            capacity,
        }
    }
}

impl Default for IdempotencyInterceptor {
    fn default() -> Self {
        Self::new(4096)
    }
}

impl BusInterceptor for IdempotencyInterceptor {
    fn name(&self) -> &'static str {
        "idempotency"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        let mut seen = self.seen.write().unwrap();
        let msg_id = env.msg_id.as_str().to_string();
        if seen.contains(&msg_id) {
            InterceptDecision::Reject {
                reason: format!("duplicate message {}", msg_id),
            }
        } else {
            if seen.len() >= self.capacity {
                seen.clear();
            }
            seen.insert(msg_id);
            InterceptDecision::Allow
        }
    }
}

pub struct LoopDetectInterceptor {
    detector: LoopDetector,
}

impl LoopDetectInterceptor {
    pub fn new(max_cells: usize, max_tracked: usize) -> Self {
        Self {
            detector: LoopDetector::new(max_cells, max_tracked),
        }
    }
}

impl Default for LoopDetectInterceptor {
    fn default() -> Self {
        Self::new(10, 1024)
    }
}

impl BusInterceptor for LoopDetectInterceptor {
    fn name(&self) -> &'static str {
        "loop-detect"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        match self.detector.check_and_record(env) {
            Ok(()) => InterceptDecision::Allow,
            Err(reason) => InterceptDecision::Reject { reason },
        }
    }
}

pub struct SchemaVersionInterceptor;

impl BusInterceptor for SchemaVersionInterceptor {
    fn name(&self) -> &'static str {
        "schema-version"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        if env.schema_version.0 == 0 {
            InterceptDecision::Reject {
                reason: "schema version 0 is reserved".into(),
            }
        } else if env.schema_version.0 > 1000 {
            InterceptDecision::Reject {
                reason: format!(
                    "schema version {} unreasonably high",
                    env.schema_version.0
                ),
            }
        } else {
            InterceptDecision::Allow
        }
    }
}

pub struct ComplianceInterceptor {
    guard: Arc<axiom_oversight::ComplianceGuardCell>,
}

impl ComplianceInterceptor {
    pub fn new(guard: Arc<axiom_oversight::ComplianceGuardCell>) -> Self {
        Self { guard }
    }
}

impl BusInterceptor for ComplianceInterceptor {
    fn name(&self) -> &'static str {
        "compliance-guard"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        let text = match serde_json::to_string(&env.payload) {
            Ok(s) => s,
            Err(_) => return InterceptDecision::Allow,
        };
        let result = self.guard.check_text(&text);
        if result.rejected {
            let reasons: Vec<String> = result
                .violations
                .iter()
                .map(|v| format!("[{}] {:?}", v.pattern, v.severity))
                .collect();
            return InterceptDecision::Reject {
                reason: format!("compliance reject: {}", reasons.join(",")),
            };
        }
        InterceptDecision::Allow
    }
}

pub struct ResourceInterceptor {
    manager: Arc<axiom_oversight::ResourceManagerCell>,
}

impl ResourceInterceptor {
    pub fn new(manager: Arc<axiom_oversight::ResourceManagerCell>) -> Self {
        Self { manager }
    }
}

impl BusInterceptor for ResourceInterceptor {
    fn name(&self) -> &'static str {
        "resource-manager"
    }
    fn intercept(&self, _env: &SignalEnvelope) -> InterceptDecision {
        if self.manager.global_tokens().try_acquire(1) {
            InterceptDecision::Allow
        } else {
            InterceptDecision::Reject {
                reason: "global token bucket exhausted; throttling".into(),
            }
        }
    }
}

pub struct OversightReportInterceptor {
    arch: Arc<axiom_oversight::ArchitectureGuardianCell>,
    entropy: Arc<axiom_oversight::EntropyGovernorCell>,
}

impl OversightReportInterceptor {
    pub fn new(
        arch: Arc<axiom_oversight::ArchitectureGuardianCell>,
        entropy: Arc<axiom_oversight::EntropyGovernorCell>,
    ) -> Self {
        Self { arch, entropy }
    }
}

impl BusInterceptor for OversightReportInterceptor {
    fn name(&self) -> &'static str {
        "oversight-report"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        if let Err(_violation) = self.arch.check_envelope(env) {
            let cell_id = env
                .target_cell
                .clone()
                .unwrap_or_else(|| format!("{:?}", env.target_layer));
            self.entropy.record(axiom_oversight::EntropyEvent::AxiomViolation {
                cell_id,
                severity: 5.0,
            });
        }
        InterceptDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_core::id::{CorrelationId, MsgId};
    use axiom_core::layer::Layer;
    use axiom_core::signal::{SignalKind, VectorClock};

    fn make_env(hops: u32, msg_id: &str) -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new(msg_id),
            correlation_id: CorrelationId::new("c"),
            trace_id: None,
            signal_type: "T".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            kind: SignalKind::Command,
            source_layer: Layer::Exec,
            target_layer: Layer::Exec,
            source_cell: None,
            target_cell: Some("c1".to_string()),
            payload: serde_json::Value::Null,
            schema_version: axiom_core::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: hops,
        }
    }

    #[test]
    fn test_hop_limit_interceptor() {
        let i = HopLimitInterceptor::new(3);
        assert!(matches!(i.intercept(&make_env(2, "m1")), InterceptDecision::Allow));
        assert!(matches!(
            i.intercept(&make_env(4, "m2")),
            InterceptDecision::Reject { .. }
        ));
    }

    #[test]
    fn test_idempotency_interceptor() {
        let i = IdempotencyInterceptor::default();
        assert!(matches!(i.intercept(&make_env(0, "unique1")), InterceptDecision::Allow));
        assert!(matches!(
            i.intercept(&make_env(0, "unique1")),
            InterceptDecision::Reject { .. }
        ));
        assert!(matches!(i.intercept(&make_env(0, "unique2")), InterceptDecision::Allow));
    }

    #[test]
    fn test_schema_version_rejects_zero() {
        let i = SchemaVersionInterceptor;
        let mut env = make_env(0, "m1");
        assert!(matches!(i.intercept(&env), InterceptDecision::Allow));
        env.schema_version = axiom_core::SchemaVersion::new(0);
        assert!(matches!(
            i.intercept(&env),
            InterceptDecision::Reject { .. }
        ));
    }
}
