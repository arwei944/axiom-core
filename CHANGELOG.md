# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Future work items

---

## [0.3.0] - 2026-07-04

### Added
- **Phase 4: Runtime Robustness**
  - `SignalCodec` trait with `JsonCodec` and `BincodeCodec` implementations
  - `MessageBus::with_codec()` for pluggable serialization
  - Enhanced `RuntimeHealth` with metrics/telemetry/store connectivity
  - Bincode serialization support via `bincode-codec` feature
- **Phase 5: Technical Debt Repayment**
  - `runtime.rs` split into `runtime/` (9 files) and `dispatch/` (3 files) modules
  - `macros/lib.rs` split into 10 files with shared `utils.rs`
  - Removed unnecessary `#[allow(dead_code)]` and unused imports
  - Unified error handling across core crates using `thiserror`
- **Phase 6: Production Documentation**
  - `docs/PRODUCTION.md` - Deployment, configuration, backup/restore
  - `docs/PERFORMANCE.md` - Benchmarks, tuning decision trees, config recommendations
  - `docs/MIGRATION.md` - Migration guides from LangChain, CrewAI, custom frameworks
  - `docs/OPERATIONS.md` - Alert handling, log interpretation, witness troubleshooting

### Changed
- SQLite is now the default persistence backend
- `SnapshotPolicy` refactored to enum with configurable retention
- `tokio::select!` stop signal fixed in dispatch loop
- `Layer::can_send_to()` documented with layer transition rules

### Fixed
- SQLite `event_id` NOT NULL constraint violation
- sqlx 0.7 compatibility issues in `SqliteStore`
- Type conversion issues (`u64`, `u16`) in SQLite queries
- `oneshot::Receiver` usage in dispatch loop

### Security
- Added `bincode`, `prometheus`, `axum` to audited dependencies
- Capability version interceptor for runtime permission checks
- Architecture governance enforcement via `archcheck` and `xtask gatecheck`

---

## [Unreleased]

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

[Unreleased]: https://github.com/axiom-framework/axiom/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/axiom-framework/axiom/releases/tag/v0.3.0
[0.2.0]: https://github.com/axiom-framework/axiom/releases/tag/v0.2.0
[0.1.0]: https://github.com/axiom-framework/axiom/releases/tag/v0.1.0
