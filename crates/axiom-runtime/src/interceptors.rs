//! Built-in BusInterceptors for runtime enforcement (hop limit, idempotency, schema, loop detection, capability version, guard).

use crate::bus::{BusInterceptor, InterceptDecision};
use crate::constraint_validator::{ConstraintValidator, ValidationContext};
use crate::constants::{IDEMPOTENCY_CLEANUP_THRESHOLD, IDEMPOTENCY_SET_CAPACITY, MAX_HOPS};
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

pub struct GuardInterceptor;

impl GuardInterceptor {
    pub fn new() -> Self {
        Self
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
        let allowed = env.signal_type != "ForbiddenSignal";
        if allowed {
            InterceptDecision::Allow
        } else {
            InterceptDecision::Reject {
                reason: format!("guard blocked signal: {}", env.signal_type),
            }
        }
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
        let i = GuardInterceptor;
        let mut e = make_env(0, "g");
        e.signal_type = "ForbiddenSignal".into();
        assert!(matches!(i.intercept(&e), InterceptDecision::Reject { .. }));
    }

    #[test]
    fn guard_allows_normal_signal() {
        let i = GuardInterceptor;
        let e = make_env(0, "g");
        assert!(matches!(i.intercept(&e), InterceptDecision::Allow));
    }
}
