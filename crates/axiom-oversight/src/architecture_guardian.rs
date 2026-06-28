//! ArchitectureGuardian - enforces architectural self-constraints.
//!
//! Checks that cross-layer calls respect the call-direction rule:
//! Oversight → Agent → Validate → Exec (no reverse, no skip).

use axiom_core::layer::Layer;

pub struct ArchitectureGuardian;

impl ArchitectureGuardian {
    pub fn check_cross_layer_signal(source: Layer, target: Layer) -> Result<(), String> {
        if source.can_send_to(target) {
            Ok(())
        } else {
            Err(format!(
                "Illegal cross-layer signal: {} → {} (violates architecture call-direction rule)",
                source, target
            ))
        }
    }
}
