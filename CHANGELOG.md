# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Phase 4: Witness time travel and replay capabilities
  - `ReplayEngine::replay_at_timestamp` - Replay at specific timestamp
  - `ReplayEngine::replay_at_sequence` - Replay at specific sequence
  - `ReplayEngine::diff_between` - State diff between sequences
  - `WitnessReplay` - Witness replay executor with chain validation
  - `StateDiff` - State difference structure
- Phase 3: Unified constraint runtime
  - `ConstraintValidator` - Unified validation context
  - `ValidationContext` - Validation context from envelope
  - `CapabilityVersionInterceptor` - Capability version checking
  - `GuardInterceptor` - Guard permission checking
- Phase 2: Store persistence
  - `SqliteStore` - SQLite backend with migrations
  - `FileStore` - Append-only file log backend
  - `FileSnapshotStore` - File-based snapshot store with compression
  - `StoreConfig` / `StoreFactory` - Backend configuration
  - `verify_witness_chain` - Witness chain integrity verification
- Phase 1: Lens primitive
  - `Lens` trait - On-demand state projection
  - `Projectable` trait - Object-safe lens trait
  - `ProjectionCache` - Cache with TTL/LRU
  - `IncrementalProjectionCache` - Incremental cache
  - `LensRegistry` / `LENS_REGISTRY` - Lens registration
  - `#[lens]` macro - Automatic lens implementation

### Changed
- `AxiomRuntime` now auto-registers capability version and guard interceptors
- `ReplayEngine` supports multiple replay modes (aggregate, cell, correlation, time, sequence, diff)
- `Witness` chain verification integrated into store layer

### Fixed
- Witness hash chain validation in runtime persistence
- Snapshot retention enforcement in file backend

---

## [0.2.0] - 2026-07-04

### Added
- **Phase 1: Lens Primitive**
  - Complete Lens implementation with `Lens` and `Projectable` traits
  - `ProjectionCache` with in-memory and incremental implementations
  - `#[lens]` macro for automatic registration
  - 10 integration tests and 9 macro tests
- **Phase 2: Store Persistence**
  - SQLite backend with connection pooling and migrations
  - File-based append-only event log with rolling cleanup
  - File-based snapshot store with compression
  - `StoreConfig` and `StoreFactory` for backend selection
  - Witness auto-persistence with chain validation
  - 14 persistence tests including performance benchmarks
- **Phase 3: Constraint Runtime Unification**
  - `ConstraintValidator` for unified validation context
  - `CapabilityVersionInterceptor` for runtime version checking
  - `GuardInterceptor` for permission enforcement
  - 8 constraint tests covering interceptors and guard
- **Phase 4: Witness Time Travel**
  - `ReplayEngine` enhancements for timestamp/sequence replay
  - `StateDiff` for comparing state at different points
  - `WitnessReplay` executor with chain validation
  - 4 new replay/witness tests
- **Documentation**
  - `API_BOUNDARY.md` - Stable v1 API boundary definition
  - `VERSIONING.md` - Semantic versioning and deprecation policy
  - `CHANGELOG.md` - This file

### Changed
- All crates unified under v0.2.0 version
- `AxiomRuntime` auto-registers all built-in interceptors
- `ReplayEngine` supports 7 replay modes
- `Witness` includes `kind` field for different witness types

### Fixed
- Compilation errors in macro expansion
- Import cycles between core and store crates
- Unused variable warnings in test code

### Security
- Added `sqlx`, `snap`, `tempfile` to audited dependencies
- Witness hash chain validation after persistence
- Guard interceptor blocks forbidden signals at runtime

---

## [0.1.0] - 2025-01-15

### Added
- Initial release with 4 core primitives:
  - **Cell**: Isolated stateful unit with mailbox
  - **Signal**: Typed immutable messages with vector clocks
  - **Axiom**: Global invariant constraints
  - **Witness**: Immutable audit records with hash chains
- Four-layer architecture (Oversight/Agent/Validate/Exec)
- Compile-time layer enforcement with `CanSendTo`
- Runtime layer enforcement with `ArchitectureGuardian`
- Entropy governance system
- Version management with schema migrations
- Macro system (`#[cell]`, `#[signal]`, `#[axiom]`, etc.)
- In-memory event store with replay engine
- 391+ tests passing

### Documentation
- Architecture documentation
- Development plan v0.2.0
- Contributing guidelines

---

[Unreleased]: https://github.com/axiom-framework/axiom/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/axiom-framework/axiom/releases/tag/v0.2.0
[0.1.0]: https://github.com/axiom-framework/axiom/releases/tag/v0.1.0
