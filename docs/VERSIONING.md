# Versioning Strategy

> **Status**: Active
> **Effective**: v0.1.0

---

## 1. Semantic Versioning

Axiom Core follows [Semantic Versioning 2.0.0](https://semver.org/):

- **Major (MAJOR)**: Breaking changes that require migration
- **Minor (MINOR)**: New features, backwards-compatible
- **Patch (PATCH)**: Bug fixes, no new features

### 1.1 Version Format

```
MAJOR.MINOR.PATCH[-PRE][+BUILD]
```

### 1.2 Breaking Change Criteria

A change is considered **breaking** if it:

- Removes or renames a public API (function, type, trait, method)
- Changes the signature of a public API
- Changes the behavior of a public API in a way that could break existing code
- Removes or changes the format of persisted data (Witness, Event)
- Changes the protocol format or version requirements
- Changes the capability dimension version in an incompatible way

### 1.3 Non-breaking Change Criteria

A change is **non-breaking** if it:

- Adds new public API (functions, types, traits)
- Adds new fields to existing structs (with default values)
- Adds new variants to existing enums
- Improves documentation
- Fixes bugs without changing API behavior
- Adds new features that are opt-in (e.g., feature flags)
- Adds new capability dimension registrations (compatible versions)

---

## 2. Deprecation Process

### 2.1 Deprecation Lifecycle

1. **Phase 1: Warning** (MINOR release)
   - Mark API with `#[deprecated]` attribute
   - Add clear deprecation message with migration guidance
   - Update documentation

2. **Phase 2: Grace Period** (1-2 MINOR releases)
   - Keep deprecated API functional
   - Continue warning users
   - Provide migration examples

3. **Phase 3: Removal** (MAJOR release)
   - Remove deprecated API
   - Update CHANGELOG with migration guide

### 2.2 Deprecation Message Format

```rust
#[deprecated = "Use `new_function()` instead. See migration guide at: https://axiom-core.dev/migration/v0.2"]
pub fn old_function() { ... }
```

---

## 3. Breaking Change Notification

### 3.1 CHANGELOG Requirements

Every breaking change must include:

- Clear description of what changed
- Impact assessment (which code will break)
- Migration steps with code examples
- Version number where the change was introduced

### 3.2 Release Notes

Major releases must include:

- Summary of breaking changes
- Migration guide
- Compatibility table
- Known issues

---

## 4. Schema Versioning

### 4.1 Schema Version Scope

Each Signal type has its own schema version:

- Schema version starts at `1` (never `0`)
- Increment on structural changes to the signal payload
- Use `#[schema_version(N)]` macro to specify

### 4.2 Migration Requirements

When a schema changes:

1. Create a migration function using `#[migration]` macro
2. Migration functions are registered in the global registry
3. Runtime automatically migrates signals on deserialization
4. Missing migrations cause `MigrationPathNotFound` error

### 4.3 Compatibility Rules

- Higher schema versions cannot be read by older runtimes (`SchemaVersionTooNew`)
- Lower schema versions without migration paths cannot be upgraded (`MigrationChainGap`)

---

## 5. Protocol Versioning

### 5.1 Protocol Version Scope

The inter-cell communication protocol has its own version:

- Protocol version is negotiated during connection
- Mismatched protocol versions cause `ProtocolMismatch` error
- Protocol version is separate from package version

### 5.2 Compatibility Matrix

| Runtime Version | Protocol Version |
|-----------------|------------------|
| v0.x            | v1               |

---

## 6. Witness Chain Compatibility

### 6.1 Witness Hash Chain

Every witness includes a hash of the previous witness, forming an immutable chain.

### 6.2 Compatibility Rules

- Witness format changes require schema version increment
- Hash algorithm changes require major version increment
- Old witnesses must remain verifiable by new versions

---

## 7. Capability Dimension Versioning

### 7.1 Overview

Axiom Core manages 8 independent capability dimensions, each with its own versioning:

| Dimension | Purpose | Description |
|-----------|---------|-------------|
| **Witness** | Audit chain version | State transition record format |
| **Schema** | Signal protocol version | Message serialization format |
| **Layer** | Architecture layer version | Inter-layer call rules |
| **Tool** | Tool interface version | Tool execution protocol |
| **Guard** | Constraint rule version | Permission check rules |
| **Identity** | Identity protocol version | Agent identity/permission set |
| **Entropy** | Entropy governance version | Threshold policies/governance actions |
| **Runtime** | Runtime protocol version | Supervision policies/mailbox configuration |

### 7.2 CapabilityDescriptor Structure

```rust
pub struct CapabilityDescriptor {
    pub dimension: CapabilityDimension,
    pub name: &'static str,
    pub version: Version,
    pub compatibility: Compatibility,
    pub applies_to_layer: Option<Layer>,
    pub migration_chain_start: Option<u16>,
}
```

### 7.3 Compatibility Strategies

| Strategy | Description | Example |
|----------|-------------|---------|
| **Exact** | Only exact version matches | Locked to specific implementation |
| **SemVer** | Semantic versioning | Compatible with minor/patch updates |
| **Forward** | Forward compatible only | Newer versions understand older |
| **Backward** | Backward compatible only | Older versions understand newer |

### 7.4 Registration

Use the `#[capability]` macro to register capability versions:

```rust
#[axiom_core::capability(dim = "witness", version = "1.0.0")]
struct WitnessV1;

#[axiom_core::capability(dim = "identity", version = "1.0.0")]
struct IdentityCapability;

#[axiom_core::capability(dim = "entropy", version = "1.0.0")]
struct EntropyCapability;

#[axiom_core::capability(dim = "runtime", version = "1.0.0")]
struct RuntimeCapability;
```

### 7.5 Automatic Compatibility Check

The `CapabilityVersionRegistry` automatically checks compatibility at runtime:

```rust
CapabilityVersionRegistry::auto_check_compatibility()?;
```

This ensures all registered capabilities are compatible with each other.

### 7.6 Version Query API

```rust
// Get all registered capabilities
let caps = CapabilityVersionRegistry::registered_capabilities();

// Get capabilities by dimension
let witness_caps = CapabilityVersionRegistry::capabilities_by_dimension(CapabilityDimension::Witness);

// Get latest version for a dimension
let latest = CapabilityVersionRegistry::latest_version_for_dimension(CapabilityDimension::Schema);

// Count capabilities by dimension
let count = CapabilityVersionRegistry::count_by_dimension(CapabilityDimension::Tool);
```

---

## 8. Feature Flags

### 8.1 Stable vs Unstable

- **Stable**: Enabled by default, guaranteed backwards-compatible
- **Unstable**: Disabled by default, may change without warning

### 8.2 Feature Flag List

| Feature | Stability | Description |
|---------|-----------|-------------|
| `default` | Stable | Core functionality |
| `sha2-id` | Stable | SHA2-based ID generation |
| `uuid` | Stable | UUID-based ID generation |
| `unstable` | Unstable | Experimental APIs |

### 8.3 Unstable API Guidelines

- Unstable APIs are prefixed with `#[cfg(feature = "unstable")]`
- Users must explicitly opt-in with `features = ["unstable"]`
- No guarantees of backwards compatibility
- Breaking changes may occur in any release

---

## 9. Release Process

### 9.1 Pre-release Checklist

Before any release:

- [ ] All tests pass (`cargo test --workspace`)
- [ ] Clippy warnings resolved (`cargo clippy --workspace`)
- [ ] Documentation builds (`cargo doc --no-deps`)
- [ ] CHANGELOG updated with breaking changes
- [ ] Migration guides written (if needed)
- [ ] Version bumped in `Cargo.toml`
- [ ] Capability versions verified (`CapabilityVersionRegistry::auto_check_compatibility()`)

### 9.2 Release Branching

- `main`: Development branch
- `vX.Y`: Release branch for major/minor versions
- Hotfixes applied to release branches and merged back to main

---

## 10. Backwards Compatibility Pledge

### 10.1 Commitment

We pledge to maintain backwards compatibility for:

- **Patch releases**: 100% compatible
- **Minor releases**: 99% compatible (deprecations allowed)
- **Major releases**: Breaking changes allowed with migration guide

### 10.2 Exceptions

Backwards compatibility does not apply to:

- Private APIs (not exported from crate root)
- Unstable features (behind `unstable` feature flag)
- Internal implementation details
- Test-only code

---

## 11. Capability Version Migration

### 11.1 Migration Chain

When a capability dimension version changes, a migration chain must be established:

```rust
#[axiom_core::capability(dim = "schema", version = "1.0.0")]
struct SchemaV1;

#[axiom_core::capability(dim = "schema", version = "2.0.0", migration_chain_start = 1)]
struct SchemaV2;
```

### 11.2 Migration Requirements

- Each version increment must have a corresponding migration function
- Migration functions must be registered in the migration registry
- Missing migrations cause `MigrationChainGap` error at startup

---

## 12. Version Compatibility Matrix

### 12.1 Runtime Compatibility

| Runtime | Compatible With |
|---------|----------------|
| v0.2.0 | v0.1.x (backwards compatible) |
| v0.1.x | v0.1.x only |

### 12.2 Capability Dimension Compatibility

| Dimension | v0.1.0 | v0.2.0 |
|-----------|--------|--------|
| Witness | 1.0.0 | 1.0.0+ |
| Schema | 1.0.0 | 1.0.0+ |
| Layer | 1.0.0 | 1.0.0+ |
| Tool | 1.0.0 | 1.0.0+ |
| Guard | 1.0.0 | 1.0.0+ |
| Identity | - | 1.0.0+ |
| Entropy | - | 1.0.0+ |
| Runtime | - | 1.0.0+ |
