//! Unified runtime constraint validation layer.
//!
//! This module provides a shared validation context used by bus interceptors
//! to enforce capability version compatibility, guard permissions, and
//! other cross-cutting constraints at runtime.

use crate::bus::InterceptDecision;
use axiom_kernel::registry::CapabilityDimension;
use axiom_kernel::signal::SignalEnvelope;

#[derive(Debug, Clone, Default)]
pub struct ValidationContext {
    pub capability_dimensions: Vec<CapabilityDimension>,
    pub target_layer: Option<axiom_kernel::RuntimeTier>,
    pub source_cell: Option<String>,
    pub signal_type: Option<String>,
    pub schema_version: Option<axiom_kernel::version::SchemaVersion>,
}

impl ValidationContext {
    pub fn from_envelope(env: &SignalEnvelope) -> Self {
        Self {
            capability_dimensions: vec![
                CapabilityDimension::Schema,
                CapabilityDimension::Layer,
                CapabilityDimension::Runtime,
            ],
            target_layer: Some(env.target_layer),
            source_cell: env.source_cell.clone(),
            signal_type: Some(env.signal_type.clone()),
            schema_version: Some(env.schema_version),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConstraintValidator {
    pub ctx: ValidationContext,
}

impl ConstraintValidator {
    pub fn new(ctx: ValidationContext) -> Self {
        Self { ctx }
    }

    pub fn validate_capability_compatibility(
        &self,
        dimension: CapabilityDimension,
        requested_version: &axiom_kernel::version::Version,
    ) -> InterceptDecision {
        let registered =
            axiom_kernel::registry::CapabilityVersionRegistry::capabilities_by_dimension(
                &dimension,
            );
        if registered.is_empty() {
            return InterceptDecision::Allow;
        }
        let Some(latest) = registered.iter().max_by_key(|c| c.version) else {
            return InterceptDecision::Allow;
        };
        if !latest.is_compatible_with(&axiom_kernel::registry::CapabilityDescriptor {
            dimension: dimension.clone(),
            name: "requested",
            version: *requested_version,
            compatibility: axiom_kernel::version::Compatibility::Exact,
            applies_to_layer: None,
            migration_chain_start: None,
        }) {
            return InterceptDecision::Reject {
                reason: format!(
                    "capability {dimension:?} incompatible: requested {requested_version}, latest {latest:?}"
                ),
            };
        }

        InterceptDecision::Allow
    }

    pub fn validate_signal_allowed(
        &self,
        env: &SignalEnvelope,
        allowed_signals: &[&str],
    ) -> InterceptDecision {
        if allowed_signals.is_empty() {
            return InterceptDecision::Allow;
        }
        if !allowed_signals.contains(&env.signal_type.as_str()) {
            return InterceptDecision::Reject {
                reason: format!("signal '{}' is not allowed", env.signal_type),
            };
        }
        InterceptDecision::Allow
    }

    pub fn validate_layer_direction(&self, env: &SignalEnvelope) -> InterceptDecision {
        if env.source_layer.can_send_to(env.target_layer) {
            InterceptDecision::Allow
        } else {
            InterceptDecision::Reject {
                reason: format!(
                    "layer violation: {:?} -> {:?}",
                    env.source_layer, env.target_layer
                ),
            }
        }
    }
}
