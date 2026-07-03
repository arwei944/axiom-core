use axiom_core::{
    CapabilityDimension, CapabilityVersionRegistry, CAPABILITY_VERSION_REGISTRY, Version,
};

#[axiom_core::capability(dim = "witness", version = "1.0.0")]
struct WitnessCapability;

#[axiom_core::capability(dim = "schema", version = "2.1.0")]
struct SchemaCapability;

#[axiom_core::capability(dim = "layer", version = "1.0.0", layer = "exec")]
struct ExecLayerCapability;

#[axiom_core::capability(dim = "tool", version = "1.2.0")]
struct ToolCapability;

#[axiom_core::capability(dim = "guard", version = "1.0.0", layer = "all")]
struct GuardCapability;

#[axiom_core::capability(dim = "identity", version = "1.0.0")]
struct IdentityCapability;

#[axiom_core::capability(dim = "entropy", version = "1.0.0")]
struct EntropyCapability;

#[axiom_core::capability(dim = "runtime", version = "1.0.0")]
struct RuntimeCapability;

#[test]
fn test_capability_macro_auto_registration() {
    let caps = CapabilityVersionRegistry::registered_capabilities();
    assert!(caps.len() >= 8);

    let witness_caps = CapabilityVersionRegistry::capabilities_by_dimension(CapabilityDimension::Witness);
    assert!(witness_caps.iter().any(|c| c.name == "WitnessCapability"));

    let schema_caps = CapabilityVersionRegistry::capabilities_by_dimension(CapabilityDimension::Schema);
    assert!(schema_caps.iter().any(|c| c.name == "SchemaCapability"));

    let identity_caps = CapabilityVersionRegistry::capabilities_by_dimension(CapabilityDimension::Identity);
    assert!(identity_caps.iter().any(|c| c.name == "IdentityCapability"));

    let entropy_caps = CapabilityVersionRegistry::capabilities_by_dimension(CapabilityDimension::Entropy);
    assert!(entropy_caps.iter().any(|c| c.name == "EntropyCapability"));

    let runtime_caps = CapabilityVersionRegistry::capabilities_by_dimension(CapabilityDimension::Runtime);
    assert!(runtime_caps.iter().any(|c| c.name == "RuntimeCapability"));
}

#[test]
fn test_capability_version_parsing() {
    let caps = CapabilityVersionRegistry::registered_capabilities();
    let schema_cap = caps.iter().find(|c| c.name == "SchemaCapability").unwrap();

    assert_eq!(schema_cap.version.major, 2);
    assert_eq!(schema_cap.version.minor, 1);
    assert_eq!(schema_cap.version.patch, 0);
}

#[test]
fn test_capability_layer_association() {
    let caps = CapabilityVersionRegistry::registered_capabilities();
    let exec_layer_cap = caps.iter().find(|c| c.name == "ExecLayerCapability").unwrap();
    let guard_cap = caps.iter().find(|c| c.name == "GuardCapability").unwrap();

    assert_eq!(exec_layer_cap.applies_to_layer, Some(axiom_core::Layer::Exec));
    assert_eq!(guard_cap.applies_to_layer, None);
}

#[test]
fn test_latest_version_for_dimension() {
    let witness_version = CapabilityVersionRegistry::latest_version_for_dimension(CapabilityDimension::Witness);
    assert!(witness_version.is_some());
    assert_eq!(witness_version.unwrap(), Version::new(1, 0, 0));

    let schema_version = CapabilityVersionRegistry::latest_version_for_dimension(CapabilityDimension::Schema);
    assert!(schema_version.is_some());
    assert_eq!(schema_version.unwrap(), Version::new(2, 1, 0));
}

#[test]
fn test_auto_check_compatibility() {
    let result = CapabilityVersionRegistry::auto_check_compatibility();
    assert!(result.is_ok());
}

#[test]
fn test_count_by_dimension() {
    let witness_count = CapabilityVersionRegistry::count_by_dimension(CapabilityDimension::Witness);
    assert!(witness_count >= 1);

    let schema_count = CapabilityVersionRegistry::count_by_dimension(CapabilityDimension::Schema);
    assert!(schema_count >= 1);
}
