//! Entropy governance interceptors for Throttle and Emergency actions.
//!
//! These interceptors implement the actual enforcement of governance decisions
//! made by the EntropyGovernor. When entropy reaches critical levels:
//! - ThrottleInterceptor: Reduces message flow to specific cells based on factor
//! - EmergencyInterceptor: Blocks all non-Oversight messages
//!
//! Both interceptors receive their state updates from the dispatch loop
//! via shared Arc<RwLock> references, enabling runtime dynamic adjustment.

use std::collections::HashMap;
use std::sync::Arc;

use axiom_core::layer::Layer;
use axiom_core::signal::SignalEnvelope;
use parking_lot::{Mutex, RwLock};

use crate::bus::{BusInterceptor, InterceptDecision};

pub struct ThrottleInterceptor {
    factors: Arc<RwLock<HashMap<String, f64>>>,
    counters: Mutex<HashMap<String, u64>>,
}

impl ThrottleInterceptor {
    pub fn new(factors: Arc<RwLock<HashMap<String, f64>>>) -> Self {
        Self {
            factors,
            counters: Mutex::new(HashMap::new()),
        }
    }

    pub fn factor_for(&self, cell_id: &str) -> Option<f64> {
        self.factors.read().get(cell_id).copied()
    }

    pub fn set_factor(&self, cell_id: &str, factor: f64) {
        self.factors
            .write()
            .insert(cell_id.to_string(), factor.clamp(0.0, 1.0));
    }

    pub fn clear_factor(&self, cell_id: &str) {
        self.factors.write().remove(cell_id);
    }

    pub fn clear_all(&self) {
        self.factors.write().clear();
        self.counters.lock().clear();
    }
}

impl BusInterceptor for ThrottleInterceptor {
    fn name(&self) -> &'static str {
        "entropy-throttle"
    }

    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        let target = match &env.target_cell {
            Some(tc) => tc.clone(),
            None => return InterceptDecision::Allow,
        };

        let factor = {
            let factors = self.factors.read();
            match factors.get(&target) {
                Some(&f) => f.clamp(0.0, 1.0),
                None => return InterceptDecision::Allow,
            }
        };

        if factor >= 1.0 {
            return InterceptDecision::Allow;
        }

        let mut counters = self.counters.lock();
        let counter = counters.entry(target.clone()).or_insert(0);
        *counter += 1;

        let threshold = (1.0 / factor.max(0.01)) as u64;
        if *counter >= threshold {
            *counter = 0;
            return InterceptDecision::Reject {
                reason: format!("throttled (factor={:.2})", factor),
            };
        }

        InterceptDecision::Allow
    }
}

pub struct EmergencyInterceptor {
    enabled: Arc<RwLock<bool>>,
}

impl EmergencyInterceptor {
    pub fn new(enabled: Arc<RwLock<bool>>) -> Self {
        Self { enabled }
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write() = enabled;
    }

    pub fn toggle(&self) -> bool {
        let mut guard = self.enabled.write();
        *guard = !*guard;
        *guard
    }
}

impl BusInterceptor for EmergencyInterceptor {
    fn name(&self) -> &'static str {
        "entropy-emergency"
    }

    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        if !*self.enabled.read() {
            return InterceptDecision::Allow;
        }

        if env.source_layer == Layer::Oversight {
            return InterceptDecision::Allow;
        }

        InterceptDecision::Reject {
            reason: "emergency mode active — only oversight messages accepted".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_core::id::{CorrelationId, MsgId};
    use axiom_core::layer::Layer;
    use axiom_core::signal::{SignalKind, VectorClock};

    fn make_env(target_cell: Option<&str>, source_layer: Layer) -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new("test"),
            correlation_id: CorrelationId::new("corr"),
            trace_id: None,
            signal_type: "TestSignal".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 0,
            kind: SignalKind::Command,
            source_layer,
            target_layer: Layer::Exec,
            source_cell: None,
            target_cell: target_cell.map(|s| s.to_string()),
            payload: serde_json::Value::Null,
            schema_version: axiom_core::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        }
    }

    #[test]
    fn throttle_allows_when_no_factor() {
        let factors = Arc::new(RwLock::new(HashMap::new()));
        let interceptor = ThrottleInterceptor::new(factors);

        let env = make_env(Some("target-cell"), Layer::Exec);
        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Allow
        ));
    }

    #[test]
    fn throttle_allows_when_factor_is_one() {
        let factors = Arc::new(RwLock::new(HashMap::new()));
        factors.write().insert("target-cell".to_string(), 1.0);
        let interceptor = ThrottleInterceptor::new(factors);

        let env = make_env(Some("target-cell"), Layer::Exec);
        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Allow
        ));
    }

    #[test]
    fn throttle_rejects_based_on_factor() {
        let factors = Arc::new(RwLock::new(HashMap::new()));
        factors.write().insert("target-cell".to_string(), 0.5);
        let interceptor = ThrottleInterceptor::new(factors);

        let env = make_env(Some("target-cell"), Layer::Exec);

        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Allow
        ));
        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Reject { .. }
        ));
        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Allow
        ));
    }

    #[test]
    fn throttle_allows_other_cells() {
        let factors = Arc::new(RwLock::new(HashMap::new()));
        factors.write().insert("hot-cell".to_string(), 0.1);
        let interceptor = ThrottleInterceptor::new(factors);

        let env = make_env(Some("other-cell"), Layer::Exec);
        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Allow
        ));
    }

    #[test]
    fn throttle_allows_without_target_cell() {
        let factors = Arc::new(RwLock::new(HashMap::new()));
        factors.write().insert("target-cell".to_string(), 0.1);
        let interceptor = ThrottleInterceptor::new(factors);

        let env = make_env(None, Layer::Exec);
        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Allow
        ));
    }

    #[test]
    fn throttle_factor_clamped() {
        let factors = Arc::new(RwLock::new(HashMap::new()));
        factors.write().insert("target-cell".to_string(), 1.5);
        let interceptor = ThrottleInterceptor::new(factors);

        let env = make_env(Some("target-cell"), Layer::Exec);
        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Allow
        ));

        let factors2 = Arc::new(RwLock::new(HashMap::new()));
        factors2.write().insert("target-cell".to_string(), -0.5);
        let interceptor2 = ThrottleInterceptor::new(factors2);

        let env2 = make_env(Some("target-cell"), Layer::Exec);
        let mut count = 0;
        while count < 99 && matches!(interceptor2.intercept(&env2), InterceptDecision::Allow) {
            count += 1;
        }
        assert_eq!(
            count, 99,
            "factor -0.5 clamped to 0.0, threshold=100, should allow 99 messages"
        );
        assert!(matches!(
            interceptor2.intercept(&env2),
            InterceptDecision::Reject { .. }
        ));
    }

    #[test]
    fn throttle_clear_factor() {
        let factors = Arc::new(RwLock::new(HashMap::new()));
        let interceptor = ThrottleInterceptor::new(factors.clone());

        interceptor.set_factor("target-cell", 0.5);
        assert_eq!(interceptor.factor_for("target-cell"), Some(0.5));

        interceptor.clear_factor("target-cell");
        assert_eq!(interceptor.factor_for("target-cell"), None);
    }

    #[test]
    fn throttle_clear_all() {
        let factors = Arc::new(RwLock::new(HashMap::new()));
        let interceptor = ThrottleInterceptor::new(factors.clone());

        interceptor.set_factor("cell-a", 0.5);
        interceptor.set_factor("cell-b", 0.3);

        interceptor.clear_all();
        assert!(factors.read().is_empty());
    }

    #[test]
    fn emergency_allows_when_disabled() {
        let enabled = Arc::new(RwLock::new(false));
        let interceptor = EmergencyInterceptor::new(enabled);

        let env = make_env(Some("target-cell"), Layer::Exec);
        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Allow
        ));
    }

    #[test]
    fn emergency_rejects_when_enabled() {
        let enabled = Arc::new(RwLock::new(true));
        let interceptor = EmergencyInterceptor::new(enabled);

        let env = make_env(Some("target-cell"), Layer::Exec);
        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Reject { .. }
        ));
    }

    #[test]
    fn emergency_allows_oversight() {
        let enabled = Arc::new(RwLock::new(true));
        let interceptor = EmergencyInterceptor::new(enabled);

        let env = make_env(Some("target-cell"), Layer::Oversight);
        assert!(matches!(
            interceptor.intercept(&env),
            InterceptDecision::Allow
        ));
    }

    #[test]
    fn emergency_toggle() {
        let enabled = Arc::new(RwLock::new(false));
        let interceptor = EmergencyInterceptor::new(enabled);

        assert!(!interceptor.is_enabled());
        assert!(interceptor.toggle());
        assert!(interceptor.is_enabled());
        assert!(!interceptor.toggle());
        assert!(!interceptor.is_enabled());
    }
}
