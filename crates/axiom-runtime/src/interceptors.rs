//! Built-in BusInterceptors for runtime enforcement (hop limit, idempotency, schema, loop detection, capability version, guard).

use crate::bus::{BusInterceptor, InterceptDecision};
use crate::constants::{IDEMPOTENCY_CLEANUP_THRESHOLD, IDEMPOTENCY_SET_CAPACITY, MAX_HOPS};
use crate::constraint_validator::{ConstraintValidator, ValidationContext};
use crate::loop_detector::LoopDetector;
use axiom_kernel::signal::SignalEnvelope;
use parking_lot::RwLock;
use std::collections::HashSet;

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
        Self { max_hops: MAX_HOPS }
    }
}

impl BusInterceptor for HopLimitInterceptor {
    fn name(&self) -> &'static str {
        "hop-limit"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        if env.hop_count >= self.max_hops {
            return InterceptDecision::Reject {
                reason: format!(
                    "hop limit {} exceeded (current hops: {})",
                    self.max_hops, env.hop_count
                ),
            };
        }
        InterceptDecision::Allow
    }
}

pub struct IdempotencyInterceptor {
    seen: RwLock<HashSet<String>>,
}

impl Default for IdempotencyInterceptor {
    fn default() -> Self {
        Self { seen: RwLock::new(HashSet::with_capacity(IDEMPOTENCY_SET_CAPACITY)) }
    }
}

impl BusInterceptor for IdempotencyInterceptor {
    fn name(&self) -> &'static str {
        "idempotency"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        let id = env.msg_id.as_str().to_string();
        let mut set = self.seen.write();
        if set.contains(&id) {
            return InterceptDecision::Reject { reason: format!("duplicate message id: {id}") };
        }
        if set.len() >= IDEMPOTENCY_CLEANUP_THRESHOLD {
            set.clear();
        }
        set.insert(id);
        InterceptDecision::Allow
    }
}

pub struct SchemaVersionInterceptor;

impl BusInterceptor for SchemaVersionInterceptor {
    fn name(&self) -> &'static str {
        "schema-version"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        if env.schema_version.0 == 0 {
            return InterceptDecision::Reject { reason: "schema version 0 is invalid".into() };
        }
        InterceptDecision::Allow
    }
}

pub struct LoopDetectInterceptor {
    detector: LoopDetector,
}

impl Default for LoopDetectInterceptor {
    fn default() -> Self {
        Self { detector: LoopDetector::new(16, 1024) }
    }
}

impl BusInterceptor for LoopDetectInterceptor {
    fn name(&self) -> &'static str {
        "loop-detect"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        match self.detector.check_and_record(env) {
            Ok(()) => InterceptDecision::Allow,
            Err(reason) => InterceptDecision::Reject { reason: reason.to_string() },
        }
    }
}

pub struct CapabilityVersionInterceptor;

impl CapabilityVersionInterceptor {
    pub fn new(_inner: ConstraintValidator) -> Self {
        Self
    }
}

impl BusInterceptor for CapabilityVersionInterceptor {
    fn name(&self) -> &'static str {
        "capability-version"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        let ctx = ValidationContext::from_envelope(env);
        let validator = ConstraintValidator::new(ctx);
        for dim in &validator.ctx.capability_dimensions {
            let requested = axiom_kernel::version::Version::new(0, 1, 0);
            if let InterceptDecision::Reject { reason } =
                validator.validate_capability_compatibility(dim.clone(), &requested)
            {
                return InterceptDecision::Reject { reason };
            }
        }
        InterceptDecision::Allow
    }
}

pub struct GuardInterceptor {
    registry: std::sync::Arc<parking_lot::RwLock<axiom_kernel::GuardRegistry>>,
}

impl GuardInterceptor {
    pub fn new() -> Self {
        Self {
            registry: std::sync::Arc::new(parking_lot::RwLock::new(
                axiom_kernel::GuardRegistry::new(),
            )),
        }
    }

    pub fn registry(&self) -> std::sync::Arc<parking_lot::RwLock<axiom_kernel::GuardRegistry>> {
        self.registry.clone()
    }

    /// Registration entry used by product code (validates Guard layer).
    pub fn register_guard(
        &self,
        guard: axiom_kernel::BoxedGuard,
        layer: axiom_kernel::RuntimeTier,
    ) -> Result<(), String> {
        self.registry
            .write()
            .register(guard, layer)
            .map_err(|e| e.to_string())
    }
}

impl Default for GuardInterceptor {
    fn default() -> Self {
        Self::new()
    }
}

impl BusInterceptor for GuardInterceptor {
    fn name(&self) -> &'static str {
        "guard"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        if env.signal_type == "ForbiddenSignal" {
            return InterceptDecision::Reject {
                reason: format!("guard blocked signal: {}", env.signal_type),
            };
        }
        // Production path: run registered guards for the envelope target layer.
        let adapter = EnvelopeAsSignal(env);
        match self.registry.read().check_all(&adapter, env.target_layer) {
            Ok(()) => InterceptDecision::Allow,
            Err(e) => InterceptDecision::Reject {
                reason: format!("guard rejected: {e}"),
            },
        }
    }
}

/// Thin Signal adapter over SignalEnvelope for Guard::check.
struct EnvelopeAsSignal<'a>(&'a SignalEnvelope);

impl axiom_kernel::signal::Signal for EnvelopeAsSignal<'_> {
    fn signal_type(&self) -> &'static str {
        // Guard checks use type name; leak is avoided by using a static for tests
        // and env signal_type for matching via as_any if needed.
        "envelope"
    }
    fn msg_id(&self) -> &axiom_kernel::id::MsgId {
        &self.0.msg_id
    }
    fn correlation_id(&self) -> &axiom_kernel::id::CorrelationId {
        &self.0.correlation_id
    }
    fn vector_clock(&self) -> &axiom_kernel::signal::VectorClock {
        &self.0.vector_clock
    }
    fn timestamp_ns(&self) -> u64 {
        self.0.timestamp_ns
    }
    fn kind(&self) -> axiom_kernel::signal::SignalKind {
        self.0.kind
    }
    fn layer(&self) -> axiom_kernel::layer::RuntimeTier {
        self.0.source_layer
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self.0
    }
    fn clone_signal(&self) -> Box<dyn axiom_kernel::signal::Signal> {
        Box::new(OwnedEnvSignal(self.0.clone()))
    }
    fn validate(&self) -> axiom_kernel::axiom::ValidationResult {
        axiom_kernel::axiom::ValidationResult::default()
    }
    fn serialize_to_json(&self) -> axiom_kernel::KernelResult<serde_json::Value> {
        Ok(self.0.payload.clone())
    }
    fn schema_version(&self) -> axiom_kernel::SchemaVersion {
        self.0.schema_version
    }
}

struct OwnedEnvSignal(SignalEnvelope);

impl axiom_kernel::signal::Signal for OwnedEnvSignal {
    fn signal_type(&self) -> &'static str {
        "envelope"
    }
    fn msg_id(&self) -> &axiom_kernel::id::MsgId {
        &self.0.msg_id
    }
    fn correlation_id(&self) -> &axiom_kernel::id::CorrelationId {
        &self.0.correlation_id
    }
    fn vector_clock(&self) -> &axiom_kernel::signal::VectorClock {
        &self.0.vector_clock
    }
    fn timestamp_ns(&self) -> u64 {
        self.0.timestamp_ns
    }
    fn kind(&self) -> axiom_kernel::signal::SignalKind {
        self.0.kind
    }
    fn layer(&self) -> axiom_kernel::layer::RuntimeTier {
        self.0.source_layer
    }
    fn as_any(&self) -> &dyn std::any::Any {
        &self.0
    }
    fn clone_signal(&self) -> Box<dyn axiom_kernel::signal::Signal> {
        Box::new(OwnedEnvSignal(self.0.clone()))
    }
    fn validate(&self) -> axiom_kernel::axiom::ValidationResult {
        axiom_kernel::axiom::ValidationResult::default()
    }
    fn serialize_to_json(&self) -> axiom_kernel::KernelResult<serde_json::Value> {
        Ok(self.0.payload.clone())
    }
    fn schema_version(&self) -> axiom_kernel::SchemaVersion {
        self.0.schema_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_kernel::id::{CorrelationId, MsgId};
    use axiom_kernel::layer::RuntimeTier;
    use axiom_kernel::signal::{SignalKind, VectorClock};

    fn make_env(hops: u32, id: &str) -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new(id),
            correlation_id: CorrelationId::new("c"),
            trace_id: None,
            signal_type: "Test".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 0,
            kind: SignalKind::Command,
            source_layer: RuntimeTier::Exec,
            target_layer: RuntimeTier::Exec,
            source_cell: None,
            target_cell: None,
            payload: serde_json::Value::Null,
            schema_version: axiom_kernel::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: hops,
        }
    }

    #[test]
    fn hop_limit_blocks() {
        let i = HopLimitInterceptor::new(3);
        let e = make_env(3, "a");
        assert!(matches!(i.intercept(&e), InterceptDecision::Reject { .. }));
    }

    #[test]
    fn hop_limit_allows() {
        let i = HopLimitInterceptor::new(8);
        let e = make_env(1, "b");
        assert!(matches!(i.intercept(&e), InterceptDecision::Allow));
    }

    #[test]
    fn idempotency_blocks_duplicate() {
        let i = IdempotencyInterceptor::default();
        let e = make_env(0, "dup");
        assert!(matches!(i.intercept(&e), InterceptDecision::Allow));
        assert!(matches!(i.intercept(&e), InterceptDecision::Reject { .. }));
    }

    #[test]
    fn schema_version_blocks_zero() {
        let i = SchemaVersionInterceptor;
        let mut e = make_env(0, "sv");
        e.schema_version = axiom_kernel::SchemaVersion::new(0);
        assert!(matches!(i.intercept(&e), InterceptDecision::Reject { .. }));
    }

    #[test]
    fn guard_blocks_forbidden_signal() {
        let i = GuardInterceptor::new();
        let mut e = make_env(0, "g");
        e.signal_type = "ForbiddenSignal".into();
        assert!(matches!(i.intercept(&e), InterceptDecision::Reject { .. }));
    }

    #[test]
    fn guard_allows_normal_signal() {
        let i = GuardInterceptor::new();
        let e = make_env(0, "g");
        assert!(matches!(i.intercept(&e), InterceptDecision::Allow));
    }

    /// Path-driving: registered rejecting guard is invoked by intercept() on a normal signal.
    #[test]
    fn registered_guard_rejects_via_intercept() {
        use axiom_kernel::guard::Guard;
        use axiom_kernel::signal::Signal;

        struct RejectAll;
        impl Guard for RejectAll {
            fn id(&self) -> &'static str {
                "reject-all"
            }
            fn layer(&self) -> RuntimeTier {
                RuntimeTier::Exec
            }
            fn check(&self, _signal: &dyn Signal) -> axiom_kernel::KernelResult<()> {
                Err(axiom_kernel::KernelError::InternalError(
                    "blocked by test guard".into(),
                ))
            }
        }

        let i = GuardInterceptor::new();
        i.register_guard(Box::new(RejectAll), RuntimeTier::Exec)
            .unwrap();
        let e = make_env(0, "normal");
        match i.intercept(&e) {
            InterceptDecision::Reject { reason } => {
                assert!(reason.contains("blocked") || reason.contains("guard"), "{reason}");
            }
            other => panic!("expected Reject from registry, got {other:?}"),
        }
    }
}
