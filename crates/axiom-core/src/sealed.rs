//! Sealed trait pattern + CanSendTo direction matrix for compile-time enforcement.
//!
//! The Sealed trait prevents downstream crates from implementing layer marker
//! traits, ensuring the direction matrix is exhaustive and cannot be extended.
//!
//! CanSendTo<Source, Target> is implemented only for legal transitions:
//! Oversight can send to all layers; Agent→Agent|Validate; Validate→Validate|Exec; Exec→Exec.

use crate::layer::Layer;

mod private {
    pub trait Sealed {}
    impl Sealed for super::OversightLayer {}
    impl Sealed for super::AgentLayer {}
    impl Sealed for super::ValidateLayer {}
    impl Sealed for super::ExecLayer {}
}

pub trait LayerMarker: private::Sealed + Send + Sync + 'static {
    const LAYER: Layer;
}

pub struct OversightLayer;
pub struct AgentLayer;
pub struct ValidateLayer;
pub struct ExecLayer;

impl LayerMarker for OversightLayer {
    const LAYER: Layer = Layer::Oversight;
}
impl LayerMarker for AgentLayer {
    const LAYER: Layer = Layer::Agent;
}
impl LayerMarker for ValidateLayer {
    const LAYER: Layer = Layer::Validate;
}
impl LayerMarker for ExecLayer {
    const LAYER: Layer = Layer::Exec;
}

pub trait CanSendTo<T: LayerMarker>: LayerMarker {}

impl CanSendTo<OversightLayer> for OversightLayer {}
impl CanSendTo<AgentLayer> for OversightLayer {}
impl CanSendTo<ValidateLayer> for OversightLayer {}
impl CanSendTo<ExecLayer> for OversightLayer {}

impl CanSendTo<AgentLayer> for AgentLayer {}
impl CanSendTo<ValidateLayer> for AgentLayer {}

impl CanSendTo<ValidateLayer> for ValidateLayer {}
impl CanSendTo<ExecLayer> for ValidateLayer {}

impl CanSendTo<ExecLayer> for ExecLayer {}

pub fn can_send_at_runtime(from: Layer, to: Layer) -> bool {
    from.can_send_to(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<S: CanSendTo<T>, T: LayerMarker>() {}

    #[test]
    fn test_legal_directions_compile() {
        assert_send::<OversightLayer, OversightLayer>();
        assert_send::<OversightLayer, AgentLayer>();
        assert_send::<OversightLayer, ValidateLayer>();
        assert_send::<OversightLayer, ExecLayer>();
        assert_send::<AgentLayer, AgentLayer>();
        assert_send::<AgentLayer, ValidateLayer>();
        assert_send::<ValidateLayer, ValidateLayer>();
        assert_send::<ValidateLayer, ExecLayer>();
        assert_send::<ExecLayer, ExecLayer>();
    }

    #[test]
    fn test_runtime_direction_check() {
        assert!(can_send_at_runtime(Layer::Oversight, Layer::Exec));
        assert!(can_send_at_runtime(Layer::Agent, Layer::Validate));
        assert!(!can_send_at_runtime(Layer::Exec, Layer::Agent));
        assert!(!can_send_at_runtime(Layer::Agent, Layer::Oversight));
        assert!(!can_send_at_runtime(Layer::Exec, Layer::Oversight));
    }
}
