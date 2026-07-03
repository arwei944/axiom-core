# Versioning Policy

> **Version**: v0.2.0  
> **Status**: Stable  
> **Last Updated**: 2026-07-04

This document defines the versioning strategy for the Axiom framework.

---

## 1. Semantic Versioning

Axiom follows [Semantic Versioning 2.0.0](https://semver.org/):

```
MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]
```

### 1.1 Version Components

| Component | Increment When | Example |
|-----------|---------------|---------|
| **MAJOR** | Breaking API changes | `1.0.0` → `2.0.0` |
| **MINOR** | New features, backward compatible | `1.0.0` → `1.1.0` |
| **PATCH** | Bug fixes, backward compatible | `1.0.0` → `1.0.1` |
| **PRERELEASE** | Pre-release identifiers | `1.0.0-alpha.1` |
| **BUILD** | Build metadata | `1.0.0+20240101` |

### 1.2 Version Examples

- `1.0.0` - Stable release
- `1.0.1` - Bug fix release
- `1.1.0` - Feature release
- `2.0.0` - Breaking change release
- `1.0.0-alpha.1` - Alpha pre-release
- `1.0.0-beta.1` - Beta pre-release
- `1.0.0-rc.1` - Release candidate

---

## 2. API Stability Levels

### 2.1 Stable API

- Guaranteed backward compatibility within major version
- Only removed in major version bumps
- 6-month deprecation notice before removal
- Documented in `API_BOUNDARY.md`

### 2.2 Unstable API

- Gated behind feature flags (`--features unstable`)
- No compatibility guarantees
- May change without notice
- Not recommended for production

### 2.3 Internal API

- Not exported from crates
- No stability guarantees
- May change anytime

---

## 3. Deprecation Policy

### 3.1 Deprecation Timeline

```
v1.2.0: Deprecate API X
  ↓
v1.3.0: Emit deprecation warnings
  ↓
v1.4.0: Continue warnings
  ↓
v2.0.0: Remove API X (after 2 minor versions or 6 months)
```

### 3.2 Deprecation Requirements

1. **Mark**: Add `#[deprecated(since = "X.Y.Z", note = "use Z instead")]`
2. **Document**: Update `CHANGELOG.md` with migration instructions
3. **Notify**: Release notes must highlight deprecated items
4. **Warn**: Compiler warnings for all deprecated API usage

### 3.3 Deprecation Attributes

```rust
#[deprecated(
    since = "1.2.0",
    note = "Use `new_api` instead. See migration guide at https://..."
)]
pub fn old_api() { }
```

---

## 4. Breaking Changes

### 4.1 What Constitutes a Breaking Change

- Removing or renaming public API items
- Changing function signatures
- Changing trait implementations
- Changing behavior in incompatible ways
- Removing feature flags
- Changing default values

### 4.2 Breaking Change Process

1. **Proposal**: Open issue with `breaking-change` label
2. **Discussion**: Community feedback period (minimum 2 weeks)
3. **Deprecation**: Mark old API as deprecated
4. **Migration Guide**: Provide step-by-step migration instructions
5. **Release**: Bump major version, include migration guide in release notes

### 4.3 Breaking Change Checklist

- [ ] All deprecated APIs have migration paths
- [ ] `MIGRATION.md` updated with instructions
- [ ] `CHANGELOG.md` clearly documents changes
- [ ] Release notes highlight breaking changes
- [ ] Semantic version major bump

---

## 5. Pre-release Versions

### 5.1 Pre-release Identifiers

| Identifier | Meaning | Usage |
|------------|---------|-------|
| `alpha` | Early testing | Internal testing, API may change |
| `beta` | Feature complete | Public testing, API mostly stable |
| `rc` | Release candidate | Final testing before stable |

### 5.2 Pre-release Examples

- `1.0.0-alpha.1` - First alpha release
- `1.0.0-beta.1` - First beta release
- `1.0.0-rc.1` - First release candidate
- `1.0.0` - Stable release

### 5.3 Pre-release Rules

- Pre-releases are not guaranteed stable
- API may change between pre-releases
- No deprecation policy applies to pre-releases
- Stable release requires full test suite pass

---

## 6. Crate Versioning

### 6.1 Unified Versioning

All crates in the Axiom workspace share the same version number:

```
Cargo.toml: version = "1.2.0"
```

### 6.2 Independent Crates

External crates depending on specific Axiom crates can use:

```toml
axiom-core = "1.2"
axiom-runtime = "1.2"
```

### 6.3 Version Ranges

| Range | Meaning |
|-------|---------|
| `"1.2"` | Compatible with 1.2.x |
| `"1.2.3"` | Exactly 1.2.3 |
| `"~1.2.3"` | Compatible with 1.2.3 (>=1.2.3, <1.3.0) |
| `"^1.2.3"` | Compatible with 1.2.3 (>=1.2.3, <2.0.0) |

---

## 7. Compatibility Matrix

| Axiom Version | Rust Version | Features |
|--------------|--------------|----------|
| v1.x | 1.75+ | Stable |
| v0.2.x | 1.75+ | Stable |
| v0.1.x | 1.70+ | Legacy |

---

## 8. Migration Guides

### 8.1 Current Guides

- [Migrating from v0.1 to v0.2](MIGRATION_0.1_TO_0.2.md)
- [Migrating from v0.2 to v1.0](MIGRATION_0.2_TO_1.0.md)

### 8.2 Guide Structure

Each migration guide includes:
1. Overview of changes
2. Step-by-step migration instructions
3. Code examples (before/after)
4. Troubleshooting section
5. API reference for new features

---

## 9. Release Process

### 9.1 Release Checklist

1. Update version in all `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Run full test suite: `cargo test --workspace`
4. Run clippy: `cargo clippy --workspace -D warnings`
5. Run fmt: `cargo fmt --all --check`
6. Run dry-run: `cargo publish --dry-run`
7. Create git tag: `git tag vX.Y.Z`
8. Push tag: `git push origin vX.Y.Z`
9. Publish: `cargo publish --workspace`

### 9.2 Release Frequency

- **Patch**: As needed (bug fixes)
- **Minor**: Monthly (new features)
- **Major**: Annually or as needed (breaking changes)

---

## 10. Support Policy

### 10.1 Supported Versions

| Version | Supported | Security Fixes |
|---------|-----------|----------------|
| v1.x (current) | Yes | Yes |
| v0.2.x | No | No |
| v0.1.x | No | No |

### 10.2 Support Duration

- Current major version: Full support
- Previous major version: Security fixes only (6 months)
- Older versions: No support

---

## 11. References

- [Semantic Versioning 2.0.0](https://semver.org/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [API_BOUNDARY.md](./API_BOUNDARY.md)
- [CHANGELOG.md](./CHANGELOG.md)
