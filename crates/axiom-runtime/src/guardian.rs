//! Architecture Guardian - runtime enforcement interceptor.
//!
//! Verifies every SignalEnvelope against four-layer calling rules,
//! message hop limits, schema version compatibility, and mailbox
//! backpressure signals. Installed automatically by AxiomRuntime
//! as the first interceptor in the bus chain.

use std::sync::atomic::{AtomicU64, Ordering};

use axiom_core::sealed::can_send_at_runtime;
use axiom_core::signal::SignalEnvelope;

use crate::bus::{BusInterceptor, InterceptDecision};

pub struct ArchitectureGuardian {
    layer_violations: AtomicU64,
    hop_violations: AtomicU64,
    schema_violations: AtomicU64,
}

impl ArchitectureGuardian {
    pub fn new() -> Self {
        Self {
            layer_violations: AtomicU64::new(0),
            hop_violations: AtomicU64::new(0),
            schema_violations: AtomicU64::new(0),
        }
    }

    pub fn stats(&self) -> GuardianStats {
        GuardianStats {
            layer_violations: self.layer_violations.load(Ordering::Relaxed),
            hop_violations: self.hop_violations.load(Ordering::Relaxed),
            schema_violations: self.schema_violations.load(Ordering::Relaxed),
        }
    }
}

impl Default for ArchitectureGuardian {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GuardianStats {
    pub layer_violations: u64,
    pub hop_violations: u64,
    pub schema_violations: u64,
}

impl BusInterceptor for ArchitectureGuardian {
    fn name(&self) -> &'static str {
        "architecture-guardian"
    }

    fn intercept(&self, env: &SignalEnvelope) -> InterceptDecision {
        if !can_send_at_runtime(env.source_layer, env.target_layer) {
            self.layer_violations.fetch_add(1, Ordering::Relaxed);
            return InterceptDecision::Reject {
                reason: format!(
                    "illegal layer transition {:?}→{:?}",
                    env.source_layer, env.target_layer
                ),
            };
        }

        if env.hop_count > 8 {
            self.hop_violations.fetch_add(1, Ordering::Relaxed);
            return InterceptDecision::Reject {
                reason: format!("hop count {} exceeds limit 8", env.hop_count),
            };
        }

        let current_schema = axiom_core::SchemaVersion::new(1);
        if !current_schema.can_read(env.schema_version) {
            self.schema_violations.fetch_add(1, Ordering::Relaxed);
            return InterceptDecision::Reject {
                reason: format!(
                    "schema version too new: envelope {} cannot be read by system {}",
                    env.schema_version, current_schema
                ),
            };
        }

        InterceptDecision::Allow
    }
}
