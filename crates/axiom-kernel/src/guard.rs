use crate::axiom::{KernelError, KernelResult};
use crate::signal::Signal;
use crate::RuntimeTier;

/// Guard must bind to a concrete layer at registration (P1-4).
pub trait Guard: Send + Sync {
    fn id(&self) -> &'static str;
    fn layer(&self) -> RuntimeTier;
    fn check(&self, signal: &dyn Signal) -> KernelResult<()>;
}

pub trait DynGuard: 'static {
    fn id(&self) -> &'static str;
    fn layer(&self) -> RuntimeTier;
    fn check(&self, signal: &dyn Signal) -> KernelResult<()>;
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: Guard + 'static> DynGuard for T {
    fn id(&self) -> &'static str {
        Guard::id(self)
    }
    fn layer(&self) -> RuntimeTier {
        Guard::layer(self)
    }
    fn check(&self, signal: &dyn Signal) -> KernelResult<()> {
        Guard::check(self, signal)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub type BoxedGuard = Box<dyn DynGuard + Send + Sync>;

/// Registration-time validation: guard layer must match declared cell/runtime tier.
pub fn validate_guard_registration(
    guard: &dyn DynGuard,
    expected: RuntimeTier,
) -> KernelResult<()> {
    let layer = guard.layer();
    if layer != expected {
        return Err(KernelError::LayerViolation {
            from: layer,
            to: expected,
            signal_type: format!("guard:{}", guard.id()),
            source_cell: "registration".into(),
        });
    }
    Ok(())
}

/// Runtime guard registry — only accepts layer-validated guards (P1-4 entry point).
pub struct GuardRegistry {
    guards: parking_lot::RwLock<Vec<(RuntimeTier, BoxedGuard)>>,
}

impl GuardRegistry {
    pub fn new() -> Self {
        Self {
            guards: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Real registration path: validates layer before insert.
    pub fn register(
        &self,
        guard: BoxedGuard,
        expected_layer: RuntimeTier,
    ) -> KernelResult<()> {
        validate_guard_registration(guard.as_ref(), expected_layer)?;
        self.guards.write().push((expected_layer, guard));
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.guards.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn check_all(&self, signal: &dyn Signal, layer: RuntimeTier) -> KernelResult<()> {
        let guards = self.guards.read();
        for (gl, g) in guards.iter() {
            if *gl == layer {
                g.check(signal)?;
            }
        }
        Ok(())
    }
}

impl Default for GuardRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axiom::ValidationResult;
    use crate::id::{CorrelationId, MsgId};
    use crate::signal::{Signal, SignalKind, VectorClock};
    use crate::version::SchemaVersion;

    struct DummySignal;
    impl Signal for DummySignal {
        fn signal_type(&self) -> &'static str {
            "Dummy"
        }
        fn msg_id(&self) -> &MsgId {
            use std::sync::LazyLock;
            static ID: LazyLock<MsgId> = LazyLock::new(|| MsgId::new("m"));
            &ID
        }
        fn correlation_id(&self) -> &CorrelationId {
            use std::sync::LazyLock;
            static ID: LazyLock<CorrelationId> = LazyLock::new(|| CorrelationId::new("c"));
            &ID
        }
        fn vector_clock(&self) -> &VectorClock {
            use std::sync::LazyLock;
            static VC: LazyLock<VectorClock> = LazyLock::new(VectorClock::new);
            &VC
        }
        fn timestamp_ns(&self) -> u64 {
            0
        }
        fn kind(&self) -> SignalKind {
            SignalKind::Event
        }
        fn layer(&self) -> RuntimeTier {
            RuntimeTier::Exec
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn clone_signal(&self) -> Box<dyn Signal> {
            Box::new(DummySignal)
        }
        fn validate(&self) -> ValidationResult {
            ValidationResult::default()
        }
        fn serialize_to_json(&self) -> KernelResult<serde_json::Value> {
            Ok(serde_json::json!({}))
        }
        fn schema_version(&self) -> SchemaVersion {
            SchemaVersion::new(1)
        }
    }

    struct ExecGuard;
    impl Guard for ExecGuard {
        fn id(&self) -> &'static str {
            "exec-g"
        }
        fn layer(&self) -> RuntimeTier {
            RuntimeTier::Exec
        }
        fn check(&self, _: &dyn Signal) -> KernelResult<()> {
            Ok(())
        }
    }

    #[test]
    fn registration_rejects_layer_mismatch() {
        let g = ExecGuard;
        assert!(validate_guard_registration(&g, RuntimeTier::Exec).is_ok());
        assert!(validate_guard_registration(&g, RuntimeTier::Agent).is_err());
    }

    #[test]
    fn registry_entry_point_validates() {
        let reg = GuardRegistry::new();
        assert!(reg
            .register(Box::new(ExecGuard), RuntimeTier::Exec)
            .is_ok());
        assert!(reg
            .register(Box::new(ExecGuard), RuntimeTier::Agent)
            .is_err());
        assert_eq!(reg.len(), 1);
        assert!(reg.check_all(&DummySignal, RuntimeTier::Exec).is_ok());
    }
}
