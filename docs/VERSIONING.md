# Versioning Strategy

> **Status**: Draft
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

### 1.3 Non-breaking Change Criteria

A change is **non-breaking** if it:

- Adds new public API (functions, types, traits)
- Adds new fields to existing structs (with default values)
- Adds new variants to existing enums
- Improves documentation
- Fixes bugs without changing API behavior
- Adds new features that are opt-in (e.g., feature flags)

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

## 7. Feature Flags

### 7.1 Stable vs Unstable

- **Stable**: Enabled by default, guaranteed backwards-compatible
- **Unstable**: Disabled by default, may change without warning

### 7.2 Feature Flag List

| Feature | Stability | Description |
|---------|-----------|-------------|
| `default` | Stable | Core functionality |
| `sha2-id` | Stable | SHA2-based ID generation |
| `uuid` | Stable | UUID-based ID generation |
| `unstable` | Unstable | Experimental APIs |

### 7.3 Unstable API Guidelines

- Unstable APIs are prefixed with `#[cfg(feature = "unstable")]`
- Users must explicitly opt-in with `features = ["unstable"]`
- No guarantees of backwards compatibility
- Breaking changes may occur in any release

---

## 8. Release Process

### 8.1 Pre-release Checklist

Before any release:

- [ ] All tests pass (`cargo test --workspace`)
- [ ] Clippy warnings resolved (`cargo clippy --workspace`)
- [ ] Documentation builds (`cargo doc --no-deps`)
- [ ] CHANGELOG updated with breaking changes
- [ ] Migration guides written (if needed)
- [ ] Version bumped in `Cargo.toml`

### 8.2 Release Branching

- `main`: Development branch
- `vX.Y`: Release branch for major/minor versions
- Hotfixes applied to release branches and merged back to main

---

## 9. Backwards Compatibility Pledge

### 9.1 Commitment

We pledge to maintain backwards compatibility for:

- **Patch releases**: 100% compatible
- **Minor releases**: 99% compatible (deprecations allowed)
- **Major releases**: Breaking changes allowed with migration guide

### 9.2 Exceptions

Backwards compatibility does not apply to:

- Private APIs (not exported from crate root)
- Unstable features (behind `unstable` feature flag)
- Internal implementation details
- Test-only code
