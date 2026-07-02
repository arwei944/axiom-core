//! Built-in BusInterceptors for runtime enforcement (hop limit, idempotency, schema, loop detection).

use crate::bus::{BusInterceptor, InterceptDecision};
use crate::loop_detector::LoopDetector;
use axiom_core::signal::SignalEnvelope;
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
        Self { max_hops: 8 }
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
        Self {
            seen: RwLock::new(HashSet::with_capacity(1024)),
        }
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
            return InterceptDecision::Reject {
                reason: format!("duplicate message id: {id}"),
            };
        }
        if set.len() >= 100_000 {
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
            return InterceptDecision::Reject {
                reason: "schema version 0 is invalid".into(),
            };
        }
        InterceptDecision::Allow
    }
}

pub struct LoopDetectInterceptor {
    detector: LoopDetector,
}

impl Default for LoopDetectInterceptor {
    fn default() -> Self {
        Self {
            detector: LoopDetector::new(16, 1024),
        }
    }
}

impl BusInterceptor for LoopDetectInterceptor {
    fn name(&self) -> &'static str {
        "loop-detect"
    }
    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        match self.detector.check_and_record(env) {
            Ok(()) => InterceptDecision::Allow,
            Err(reason) => InterceptDecision::Reject {
                reason: reason.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_core::id::{CorrelationId, MsgId};
    use axiom_core::layer::Layer;
    use axiom_core::signal::{SignalKind, VectorClock};

    fn make_env(hops: u32, id: &str) -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new(id),
            correlation_id: CorrelationId::new("c"),
            trace_id: None,
            signal_type: "Test".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 0,
            kind: SignalKind::Command,
            source_layer: Layer::Exec,
            target_layer: Layer::Exec,
            source_cell: None,
            target_cell: None,
            payload: serde_json::Value::Null,
            schema_version: axiom_core::SchemaVersion::new(1),
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
        e.schema_version = axiom_core::SchemaVersion::new(0);
        assert!(matches!(i.intercept(&e), InterceptDecision::Reject { .. }));
    }
}
