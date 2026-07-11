//! Sealed trait pattern + CanSendTo direction matrix for compile-time enforcement.
//!
//! The Sealed trait prevents downstream crates from implementing runtime tier marker
//! traits, ensuring the direction matrix is exhaustive and cannot be extended.
//!
//! CanSendTo<Source, Target> is implemented only for legal transitions:
//! Oversight can send to all tiers; Agent->Agent|Validate; Validate->Validate|Exec; Exec->Exec.

use crate::layer::RuntimeTier;

mod private {
    pub trait Sealed {}
    impl Sealed for super::OversightTier {}
    impl Sealed for super::AgentTier {}
    impl Sealed for super::ValidateTier {}
    impl Sealed for super::ExecTier {}
}

pub trait RuntimeTierMarker: private::Sealed + Send + Sync + 'static {
    const TIER: RuntimeTier;
}

pub struct OversightTier;
pub struct AgentTier;
pub struct ValidateTier;
pub struct ExecTier;

impl OversightTier {
    pub const LAYER: RuntimeTier = RuntimeTier::Oversight;
}
impl AgentTier {
    pub const LAYER: RuntimeTier = RuntimeTier::Agent;
}
impl ValidateTier {
    pub const LAYER: RuntimeTier = RuntimeTier::Validate;
}
impl ExecTier {
    pub const LAYER: RuntimeTier = RuntimeTier::Exec;
}

impl RuntimeTierMarker for OversightTier {
    const TIER: RuntimeTier = RuntimeTier::Oversight;
}
impl RuntimeTierMarker for AgentTier {
    const TIER: RuntimeTier = RuntimeTier::Agent;
}
impl RuntimeTierMarker for ValidateTier {
    const TIER: RuntimeTier = RuntimeTier::Validate;
}
impl RuntimeTierMarker for ExecTier {
    const TIER: RuntimeTier = RuntimeTier::Exec;
}

pub trait CanSendTo<T: RuntimeTierMarker>: RuntimeTierMarker {}

impl CanSendTo<OversightTier> for OversightTier {}
impl CanSendTo<AgentTier> for OversightTier {}
impl CanSendTo<ValidateTier> for OversightTier {}
impl CanSendTo<ExecTier> for OversightTier {}

impl CanSendTo<AgentTier> for AgentTier {}
impl CanSendTo<ValidateTier> for AgentTier {}

impl CanSendTo<ValidateTier> for ValidateTier {}
impl CanSendTo<ExecTier> for ValidateTier {}
impl CanSendTo<AgentTier> for ValidateTier {}

impl CanSendTo<ExecTier> for ExecTier {}

pub fn can_send_at_runtime(from: RuntimeTier, to: RuntimeTier) -> bool {
    from.can_send_to(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<S: CanSendTo<T>, T: RuntimeTierMarker>() {}

    #[test]
    fn test_legal_directions_compile() {
        assert_send::<OversightTier, OversightTier>();
        assert_send::<OversightTier, AgentTier>();
        assert_send::<OversightTier, ValidateTier>();
        assert_send::<OversightTier, ExecTier>();
        assert_send::<AgentTier, AgentTier>();
        assert_send::<AgentTier, ValidateTier>();
        assert_send::<ValidateTier, ValidateTier>();
        assert_send::<ValidateTier, ExecTier>();
        assert_send::<ValidateTier, AgentTier>();
        assert_send::<ExecTier, ExecTier>();
    }

    #[test]
    fn test_runtime_direction_check() {
        assert!(can_send_at_runtime(RuntimeTier::Oversight, RuntimeTier::Exec));
        assert!(can_send_at_runtime(RuntimeTier::Agent, RuntimeTier::Validate));
        assert!(!can_send_at_runtime(RuntimeTier::Exec, RuntimeTier::Agent));
        assert!(!can_send_at_runtime(RuntimeTier::Agent, RuntimeTier::Oversight));
        assert!(!can_send_at_runtime(RuntimeTier::Exec, RuntimeTier::Oversight));
    }
}
