use axiom_kernel::layer::Layer;
use axiom_kernel::registry::{CapabilityDimension, CapabilityVersionRegistry, CAPABILITY_REGISTRY};

#[allow(dead_code)]
#[axiom_macros::capability(dim = "witness", version = "1.0.0")]
struct WitnessCapability;

#[allow(dead_code)]
#[axiom_macros::capability(dim = "schema", version = "2.1.0")]
struct SchemaCapability;

#[allow(dead_code)]
#[axiom_macros::capability(dim = "layer", version = "1.0.0", layer = "exec")]
struct ExecLayerCapability;

#[allow(dead_code)]
#[axiom_macros::capability(dim = "tool", version = "1.2.0")]
struct ToolCapability;

#[allow(dead_code)]
#[axiom_macros::capability(dim = "guard", version = "1.0.0", layer = "all")]
struct GuardCapability;

#[allow(dead_code)]
#[axiom_macros::capability(dim = "identity", version = "1.0.0")]
struct IdentityCapability;

#[allow(dead_code)]
#[axiom_macros::capability(dim = "entropy", version = "1.0.0")]
struct EntropyCapability;

#[allow(dead_code)]
#[axiom_macros::capability(dim = "runtime", version = "1.0.0")]
struct RuntimeCapability;

#[test]
fn test_capability_macro_auto_registration() {
    let caps = CAPABILITY_REGISTRY.iter().copied().collect::<Vec<_>>();
    assert!(caps.len() >= 8);

    let witness_caps: Vec<_> = caps
        .iter()
        .filter(|c| c.name == "WitnessCapability")
        .collect();
    assert!(!witness_caps.is_empty());
}

#[test]
fn test_capability_version_parsing() {
    let caps = CAPABILITY_REGISTRY.iter().copied().collect::<Vec<_>>();
    let schema_cap = caps.iter().find(|c| c.name == "SchemaCapability").unwrap();

    assert_eq!(schema_cap.version.major, 2);
    assert_eq!(schema_cap.version.minor, 1);
    assert_eq!(schema_cap.version.patch, 0);
}

#[test]
fn test_capability_layer_association() {
    let caps = CAPABILITY_REGISTRY.iter().copied().collect::<Vec<_>>();
    let exec_layer_cap = caps
        .iter()
        .find(|c| c.name == "ExecLayerCapability")
        .unwrap();
    let guard_cap = caps.iter().find(|c| c.name == "GuardCapability").unwrap();

    assert_eq!(exec_layer_cap.applies_to_layer, Some(Layer::Exec));
    assert_eq!(guard_cap.applies_to_layer, None);
}

#[test]
fn test_latest_version_for_dimension() {
    let witness_version =
        CapabilityVersionRegistry::latest_version_for_dimension(&CapabilityDimension::Schema);
    assert!(witness_version.is_some());
}

#[test]
fn test_auto_check_compatibility() {
    let result = CapabilityVersionRegistry::verify_all();
    assert!(result.is_ok());
}

#[test]
fn test_count_by_dimension() {
    let witness_count = CapabilityVersionRegistry::count_by_dimension(&CapabilityDimension::Schema);
    assert!(witness_count >= 1);
}
