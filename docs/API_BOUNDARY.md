# API Boundary - v1 Stable API

> **Version**: v0.2.0  
> **Status**: Stable  
> **Last Updated**: 2026-07-04

This document defines the stable v1 API boundary for the Axiom framework. All public APIs listed here are guaranteed to maintain backward compatibility within the same major version.

---

## 1. Stability Levels

| Level | Meaning | Guarantee |
|-------|---------|-----------|
| **Stable** | Production-ready, backward compatible | Semantic versioning, no breaking changes within v1 |
| **Unstable** | Experimental, subject to change | May change without notice, gated behind `unstable` feature |
| **Internal** | Private implementation detail | Not exported, may change anytime |

---

## 2. Stable API - axiom-core

### 2.1 Primitives

#### Cell
- `CellId` - Cell identifier
- `CellContext` - Cell execution context
- `LayeredCellContext` - Layer-aware cell context
- `OutgoingEnvelope` - Outgoing signal envelope
- `OutgoingWitness` - Outgoing witness record

#### Signal
- `Signal` - Signal trait
- `SignalEnvelope` - Signal envelope with metadata
- `SignalKind` - Signal type enumeration
- `VectorClock` - Causal ordering vector clock

#### Lens
- `Lens` - Lens trait for state projection
- `Projectable` - Object-safe lens trait
- `Projection` - Projection result
- `ProjectionCache` - Cache trait for projections
- `InMemoryProjectionCache` - In-memory cache implementation
- `IncrementalProjectionCache` - Incremental cache implementation
- `LensRegistry` - Lens registry management
- `LENS_REGISTRY` - Global lens registry
- `LensAccessor` - Type-safe lens accessor
- `LensEvent` - Lens event type
- `CacheMetrics` - Cache metrics

#### Axiom
- `Axiom` - Axiom trait for invariants
- `DynAxiom` - Object-safe axiom trait
- `DynAxiomChain` - Dynamic axiom chain
- `Guard` - Guard trait for permissions
- `ViolationAction` - Violation action enumeration
- `AxiomViolation` - Axiom violation record

#### Witness
- `Witness` - Immutable audit record
- `WitnessBuilder` - Witness builder
- `WitnessBatch` - Batch of witnesses
- `WitnessHash` - Witness hash
- `WitnessKind` - Witness kind enumeration
- `WitnessEvent` - Witness event enumeration
- `WitnessMetrics` - Witness metrics
- `TransitionOutcome` - Transition outcome enumeration
- `WitnessGenerator` - Witness generator trait

### 2.2 Infrastructure

#### Identity
- `AxiomId` - Base identifier
- `CorrelationId` - Correlation identifier
- `MsgId` - Message identifier
- `TraceId` - Trace identifier
- `WitnessId` - Witness identifier

#### Versioning
- `Version` - Semantic version
- `SchemaVersion` - Schema version
- `ProtocolVersion` - Protocol version
- `VersionInfo` - Version information
- `Compatibility` - Compatibility rules
- `Versioned` - Versioned trait
- `SchemaMigrator` - Schema migration

#### Capability
- `CapabilityDescriptor` - Capability descriptor
- `CapabilityDimension` - Capability dimension
- `CapabilityVersionRegistry` - Version registry
- `CAPABILITY_REGISTRY` - Global registry
- `CAPABILITY_VERSION_REGISTRY` - Global version registry

#### Layer
- `Layer` - Layer enumeration
- `CanSendTo` - Layer communication trait
- `LayerMarker` - Layer marker trait
- `OversightLayer` - Oversight layer
- `AgentLayer` - Agent layer
- `ValidateLayer` - Validate layer
- `ExecLayer` - Exec layer

#### Entropy
- `CellEntropy` - Cell entropy
- `EntropyLevel` - Entropy level
- `EntropyScore` - Entropy score
- `EntropySnapshot` - Entropy snapshot
- `EntropyWeights` - Entropy weights

#### Error
- `AxiomError` - Error enumeration
- `Result` - Result type alias

#### Schema
- `Schema` - Schema trait
- `ValidationResult` - Validation result

#### Sealed
- `can_send_at_runtime` - Runtime layer check

### 2.3 Macros (axiom-macros)

- `#[axiom]` - Axiom macro
- `#[capability]` - Capability macro
- `#[cell]` - Cell macro
- `#[guard]` - Guard macro
- `#[lens]` - Lens macro
- `#[migration]` - Migration macro
- `#[schema_version]` - Schema version macro
- `#[signal]` - Signal macro
- `SignalPayload` - Signal payload macro
- `#[tool]` - Tool macro

---

## 3. Unstable API (gated behind `unstable` feature)

### 3.1 Cell Handles

```rust
#[cfg(feature = "unstable")]
pub use cell::{
    BoxHandleFuture,
    CellHandle,
    CellHealth,
    CellMeta,
    DynCell,
    DynHandleCell,
    ExecCell,
    LayerOf,
    OversightCell,
    AgentCell,
    ValidateCell,
    SupervisionStrategy,
};
```

**Note**: These APIs are experimental and may change in future releases. Enable with `--features unstable`.

---

## 4. Stable API - axiom-runtime

### 4.1 Core Runtime

- `AxiomRuntime` - Main runtime
- `RuntimeBuilder` - Runtime builder
- `RuntimeConfig` - Runtime configuration
- `RuntimeHealth` - Runtime health status
- `CellRegistration` - Cell registration

### 4.2 Bus

- `MessageBus` - Message bus
- `BusInterceptor` - Bus interceptor trait
- `InterceptDecision` - Intercept decision enumeration

### 4.3 Interceptors

- `HopLimitInterceptor` - Hop limit interceptor
- `IdempotencyInterceptor` - Idempotency interceptor
- `SchemaVersionInterceptor` - Schema version interceptor
- `LoopDetectInterceptor` - Loop detection interceptor
- `CapabilityVersionInterceptor` - Capability version interceptor
- `GuardInterceptor` - Guard interceptor
- `EmergencyInterceptor` - Emergency interceptor
- `ThrottleInterceptor` - Throttle interceptor

### 4.4 Governance

- `EntropyGovernorCell` - Entropy governor
- `EntropyEvent` - Entropy event
- `EntropySnapshot` - Entropy snapshot
- `GovernanceAction` - Governance action

### 4.5 Supervisor

- `Supervisor` - Cell supervisor
- `SupervisionDecision` - Supervision decision

### 4.6 Other

- `Mailbox` - Cell mailbox
- `DeadLetterQueue` - Dead letter queue
- `DeadLetter` - Dead letter record
- `LoopDetector` - Loop detector
- `ArchitectureGuardian` - Architecture guardian
- `ConstraintValidator` - Constraint validator
- `ValidationContext` - Validation context

---

## 5. Stable API - axiom-store

### 5.1 Event Store

- `EventStore` - Event store trait
- `MemoryStore` - In-memory store
- `SqliteStore` - SQLite store (feature `sqlite`)
- `FileStore` - File-based store
- `StoreConfig` - Store configuration
- `StoreFactory` - Store factory
- `StoreError` - Store error enumeration

### 5.2 Snapshots

- `SnapshotStore` - Snapshot store trait
- `MemorySnapshotStore` - In-memory snapshot store
- `FileSnapshotStore` - File-based snapshot store
- `Snapshot` - Snapshot record
- `SnapshotPolicy` - Snapshot policy

### 5.3 Replay

- `ReplayEngine` - Replay engine
- `ReplayableState` - Replayable state trait
- `ReplayResult` - Replay result
- `StateDiff` - State difference
- `WitnessReplay` - Witness replay executor
- `WitnessReplayResult` - Witness replay result

### 5.4 Metrics

- `MeteredStore` - Metered store wrapper
- `StoreMetrics` - Store metrics
- `StoreHealth` - Store health

### 5.5 Other

- `Event` - Event record
- `EventBuilder` - Event builder
- `EventMetadata` - Event metadata
- `EventOutcome` - Event outcome
- `WitnessHashData` - Witness hash data
- `verify_witness_chain` - Witness chain verification

---

## 6. Versioning Policy

### 6.1 Semantic Versioning

Axiom follows [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR**: Breaking API changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

### 6.2 Stability Guarantees

| API Level | Guarantee | Deprecation Policy |
|-----------|-----------|-------------------|
| Stable | No breaking changes within v1 | 6-month deprecation notice |
| Unstable | No guarantees | May change anytime |
| Internal | No guarantees | No notice |

### 6.3 Deprecation Process

1. **Mark**: Add `#[deprecated(since = "X.Y.Z", note = "use Z instead")]`
2. **Document**: Update `CHANGELOG.md` and migration guide
3. **Notify**: Release notes highlight deprecated items
4. **Remove**: After 2 minor versions or 6 months, whichever is longer

### 6.4 Breaking Change Notification

Breaking changes require:
1. Major version bump
2. `MIGRATION.md` guide with step-by-step instructions
3. At least 2 minor versions of deprecation warnings
4. Announcement in release notes

---

## 7. Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `unstable` | No | Enable unstable APIs |
| `sqlite` | No | Enable SQLite store backend |
| `sha2-id` | No | Enable SHA-2 witness hashing |

---

## 8. Crate Dependencies

```
axiom-core (level 7)
    ↑
axiom-macros (level 6)
    ↑
axiom-store (level 5)
axiom-runtime (level 4)
    ↑
axiom-oversight (level 3)
axiom-agent (level 2)
    ↑
axiom-viz (level 1)
axiom-cli (level 0)
```

**Rule**: A crate may only depend on crates at the same level or lower (higher index).

---

## 9. Quality Gates

All APIs must pass:
- `cargo clippy --workspace -D warnings`
- `cargo test --workspace`
- `cargo doc --workspace --no-deps`
- `cargo fmt --all --check`

---

## 10. Contact

For API questions or breaking change proposals:
- Open an issue with `api-change` label
- Discuss in GitHub Discussions
- See `CONTRIBUTING.md` for PR process
