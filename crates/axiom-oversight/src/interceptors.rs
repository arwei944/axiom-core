//! L2 Oversight interceptors that plug into axiom-runtime's MessageBus.
//!
//! These interceptors bridge the Oversight Cells with the runtime's
//! interceptor chain, providing architecture compliance, entropy
//! reporting, resource throttling, PII redaction/rejection, and
//! meta-oversight (heartbeat) monitoring.

use std::sync::Arc;

use axiom_runtime::bus::{BusInterceptor, InterceptDecision};
use axiom_runtime::MessageBus;

use crate::ArchitectureGuardianCell;
use crate::ComplianceGuardCell;
use crate::EntropyEvent;
use crate::EntropyGovernorCell;
use crate::HealthCollectorCell;
use crate::MetaOversightCell;
use crate::OversightSupervisor;
use crate::ResourceManagerCell;

pub struct ComplianceInterceptor {
    guard: Arc<ComplianceGuardCell>,
}

impl ComplianceInterceptor {
    pub fn new(guard: Arc<ComplianceGuardCell>) -> Self {
        Self { guard }
    }
}

impl BusInterceptor for ComplianceInterceptor {
    fn name(&self) -> &'static str {
        "compliance-guard"
    }
    fn intercept(&self, env: &axiom_kernel::signal::SignalEnvelope) -> InterceptDecision {
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
    manager: Arc<ResourceManagerCell>,
}

impl ResourceInterceptor {
    pub fn new(manager: Arc<ResourceManagerCell>) -> Self {
        Self { manager }
    }
}

impl BusInterceptor for ResourceInterceptor {
    fn name(&self) -> &'static str {
        "resource-manager"
    }
    fn intercept(&self, _env: &axiom_kernel::signal::SignalEnvelope) -> InterceptDecision {
        if self.manager.global_tokens().try_acquire(1) {
            InterceptDecision::Allow
        } else {
            InterceptDecision::Reject { reason: "global token bucket exhausted; throttling".into() }
        }
    }
}

pub struct OversightReportInterceptor {
    arch: Arc<ArchitectureGuardianCell>,
    entropy: Arc<EntropyGovernorCell>,
}

impl OversightReportInterceptor {
    pub fn new(arch: Arc<ArchitectureGuardianCell>, entropy: Arc<EntropyGovernorCell>) -> Self {
        Self { arch, entropy }
    }
}

impl BusInterceptor for OversightReportInterceptor {
    fn name(&self) -> &'static str {
        "oversight-report"
    }
    fn intercept(&self, env: &axiom_kernel::signal::SignalEnvelope) -> InterceptDecision {
        if let Err(_violation) = self.arch.check_envelope(env) {
            let cell_id =
                env.target_cell.clone().unwrap_or_else(|| format!("{:?}", env.target_layer));
            self.entropy.record(EntropyEvent::AxiomViolation { cell_id });
        }
        InterceptDecision::Allow
    }
}

pub struct MetaOversightInterceptor {
    meta: Arc<MetaOversightCell>,
    health: Arc<HealthCollectorCell>,
}

impl MetaOversightInterceptor {
    pub fn new(meta: Arc<MetaOversightCell>, health: Arc<HealthCollectorCell>) -> Self {
        Self { meta, health }
    }
}

impl BusInterceptor for MetaOversightInterceptor {
    fn name(&self) -> &'static str {
        "meta-oversight"
    }
    fn intercept(&self, _env: &axiom_kernel::signal::SignalEnvelope) -> InterceptDecision {
        let snapshot = self.health.collect();
        if snapshot.status == crate::HealthStatus::Critical {
            return InterceptDecision::Reject {
                reason: "system health is Critical; circuit breaker open".into(),
            };
        }
        let _ = self.meta.tick_ping();
        InterceptDecision::Allow
    }
}

pub async fn wire_oversight_interceptors(bus: &MessageBus, supervisor: &OversightSupervisor) {
    bus.register_interceptor(Arc::new(OversightReportInterceptor::new(
        supervisor.architecture_guardian(),
        supervisor.entropy_governor(),
    )))
    .await;
    bus.register_interceptor(Arc::new(ResourceInterceptor::new(supervisor.resource_manager())))
        .await;
    bus.register_interceptor(Arc::new(ComplianceInterceptor::new(supervisor.compliance_guard())))
        .await;
    bus.register_interceptor(Arc::new(MetaOversightInterceptor::new(
        supervisor.meta_oversight(),
        supervisor.health_collector(),
    )))
    .await;
}

pub async fn wire_oversight_default(bus: &MessageBus) -> Arc<OversightSupervisor> {
    let supervisor = OversightSupervisor::new();
    wire_oversight_interceptors(bus, &supervisor).await;
    supervisor
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_kernel::id::{CorrelationId, MsgId};
    use axiom_kernel::layer::RuntimeTier;
    use axiom_kernel::signal::{SignalEnvelope, SignalKind, VectorClock};
    use std::time::Duration;

    fn make_env(from: RuntimeTier, to: RuntimeTier, payload: serde_json::Value) -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new("t"),
            correlation_id: CorrelationId::new("c"),
            trace_id: None,
            signal_type: "T".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 0,
            kind: SignalKind::Command,
            source_layer: from,
            target_layer: to,
            source_cell: None,
            target_cell: Some("c".into()),
            payload,
            schema_version: axiom_kernel::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        }
    }

    #[test]
    fn compliance_rejects_github_token() {
        let guard = Arc::new(ComplianceGuardCell::new());
        let i = ComplianceInterceptor::new(guard);
        let env = make_env(
            RuntimeTier::Exec,
            RuntimeTier::Exec,
            serde_json::json!({"token":"ghp_abcdefghijklmnopqrstuvwxyz0123456789ABCD"}),
        );
        assert!(matches!(i.intercept(&env), InterceptDecision::Reject { .. }));
    }

    #[test]
    fn compliance_allows_clean_payload() {
        let guard = Arc::new(ComplianceGuardCell::new());
        let i = ComplianceInterceptor::new(guard);
        let env = make_env(RuntimeTier::Exec, RuntimeTier::Exec, serde_json::json!({"msg":"hello"}));
        assert!(matches!(i.intercept(&env), InterceptDecision::Allow));
    }

    #[tokio::test]
    async fn resource_throttles_when_exhausted() {
        let mgr = Arc::new(ResourceManagerCell::new(1, 100.0, 4));
        let i = ResourceInterceptor::new(mgr.clone());
        let env = make_env(RuntimeTier::Exec, RuntimeTier::Exec, serde_json::Value::Null);
        assert!(matches!(i.intercept(&env), InterceptDecision::Allow));
        assert!(matches!(i.intercept(&env), InterceptDecision::Reject { .. }));
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
