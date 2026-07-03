use axiom_core::Guard;
use axiom_core::id::{CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::Signal;
use axiom_core::witness::WitnessKind;

#[axiom_core::signal]
struct TestSignal {
    payload: String,
}

#[test]
fn test_signal_macro_auto_fields() {
    let signal = TestSignal::new(MsgId::new("test"), CorrelationId::new("test"), "hello".to_string());
    assert_eq!(signal.signal_type(), "TestSignal");
    assert_eq!(signal.payload, "hello");
}

#[axiom_core::guard(layer = "exec")]
struct AutoGuard;

#[test]
fn test_guard_macro_auto_witness() {
    let guard = AutoGuard;
    assert_eq!(guard.name(), "AutoGuard");
    assert_eq!(guard.layer(), Some(Layer::Exec));
}

#[test]
fn test_witness_registry_auto_injection() {
    let initial_len = axiom_core::registry::WITNESS_REGISTRY.len();

    let guard = AutoGuard;
    let signal = TestSignal::new(MsgId::new("test"), CorrelationId::new("test"), "test payload".to_string());
    let _ = guard.check(&signal);

    let after_len = axiom_core::registry::WITNESS_REGISTRY.len();
    assert!(after_len > initial_len);

    let witnesses = axiom_core::registry::WITNESS_REGISTRY.get_recent(1);
    assert!(!witnesses.is_empty());
    assert_eq!(witnesses[0].kind, WitnessKind::GuardCheck);
}
