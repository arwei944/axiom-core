# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.4.0] - 2026-07-07

### Added
- **Phase 1: Witness System Migration**
  - `WitnessKind`, `WitnessHash`, `WitnessMetrics`, `TransitionOutcome`, `WitnessEvent`, `WitnessGenerator` types migrated to `axiom-kernel`
  - `WitnessKernel` runtime structure with `record`, `verify_chain`, `get_recent`, `len`, `is_empty` methods
  - Full `Witness` struct (17+ fields) with serialization support
- **Phase 2: Rich Trait Implementation**
  - `Signal` trait with 14 methods: `msg_id`, `correlation_id`, `vector_clock`, `timestamp_ns`, `kind`, `layer`, `sender`, `trace_id`, `as_any`, `clone_signal`, `validate`, `serialize_to_json`, `schema_version`, `signal_type`
  - `Guard` trait with `id`, `layer`, `check`, `as_any`, `clone_guard` methods
  - `Axiom` trait with `name`, `check`, `violation_action`, `as_any`, `clone_axiom`, `applies_to_layer` methods
  - `DynAxiom`, `DynLens`, `DynCell`, `DynHandleCell` dynamic trait implementations
- **Phase 3: Error System Expansion**
  - `KernelError` extended to cover all `AxiomError` variants (30+ variants)
  - `ValidationSeverity`, `ValidationError`, `ValidationResult` types for structured validation
  - `AxiomViolation`, `DynAxiomChain` types for axiom violation handling
- **Phase 4: Clock & Registry Infrastructure**
  - `Clock` trait with `SystemClock` and `MockClock` implementations
  - `global_clock()` / `set_global_clock()` functions for testability
  - Distributed registry system (`CAPABILITY_REGISTRY`, `AXIOM_REGISTRY`, `WITNESS_REGISTRY`, `MIGRATION_REGISTRY`, `LENS_REGISTRY`) using `linkme` compile-time registration
- **Phase 5: Infrastructure Migration**
  - `gate` module (architecture governance) migrated from `axiom-core` to `axiom-kernel`
  - `VersionInfo`, `ProtocolVersion`, `IdentityVersion`, `SchemaVersion`, `SignalSchema`, `EventSchema`, `WitnessSchema`, `Migration`, `Versioned` types migrated to `axiom-kernel`
- **Phase 6: Macro Comprehensive Switch**
  - All macros (`#[axiom]`, `#[cell]`, `#[guard]`, `#[tool]`, `#[signal]`, `#[lens]`, `#[schema_version]`, `#[migration]`, `#[capability]`) now generate `::axiom_kernel::...` code only
  - Removed all `::axiom_core::` hardcoded paths from macro source
- **Phase 7: Runtime/CLI/Application Layer Full Switch**
  - `axiom-runtime`: bus, dispatch, commands, entropy, interceptors, dlq all switched to `axiom-kernel`
  - `axiom-cli`: commands and template generation switched to `axiom-kernel`
  - `axiom-oversight`: architecture guardian, interceptors switched to `axiom-kernel`
  - `axiom-store`: Witness, Event, storage types switched to `axiom-kernel`
  - `axiom-agent`: `pub use axiom_core` replaced with `pub use axiom_kernel`
  - All other crates (distributed, identity, prompt, llm, mcp, memory, planner, tool, viz, alert) switched to `axiom-kernel`
- **Phase 8: Old Layer Exit & Full Verification**
  - `axiom-core` reduced to pure compatibility layer (only re-exports from `axiom-kernel`)
  - All old runtime module files deleted from `axiom-core`
  - `axiom-core/tests/` migrated to `axiom-kernel/tests/`
  - `axiom-core/examples/` migrated to `axiom-kernel/examples/`
  - All `Cargo.toml` files updated to remove `axiom-core` dependencies
  - `bincode-codec` and `sha2-id` features added to `axiom-kernel`

### Changed
- **100% Native Migration**: `axiom-kernel` now fully replaces `axiom-core` as the runtime layer
- **Version Unification**: All crates upgraded to v0.4.0
- **Trait Renaming**:
  - `Cell::id()` → `Cell::cell_id()`
  - `Axiom::check()` → `Axiom::check()` (kept, but `DynAxiom` uses `check_dyn()`)
  - `Guard::name()` → `Guard::id()`
- **Macro Input Format**: `#[cell]` macro now accepts struct definitions instead of impl blocks
- **Code Organization**: `axiom-kernel` now contains all core runtime modules (cell, signal, axiom, witness, guard, lens, tool, clock, registry, gate, plugin)

### Deprecated
- `axiom-core` crate is deprecated - users should migrate to `axiom-kernel`
- Old `axiom-core` traits (`Cell`, `Signal`, `Lens`, `Guard`, `Axiom`, `DynAxiom`) marked as deprecated in favor of `axiom-kernel` equivalents
- `AxiomError` renamed to `KernelError` in public API

### Removed
- `axiom-core/src/bridge/` - bridge layer completely removed
- `axiom-core/src/cell.rs`, `signal.rs`, `axiom.rs`, `lens/`, `witness/`, `clock.rs`, `registry.rs`, `error.rs`, `context.rs`, `entropy.rs`, `capability.rs`, `codec.rs`, `layer.rs`, `id.rs`, `schema.rs`, `sealed.rs`, `version.rs` - all runtime modules removed
- `axiom-core/tests/` - all tests moved to `axiom-kernel/tests/`
- `axiom-core/examples/` - all examples moved to `axiom-kernel/examples/`

### Fixed
- `CAPABILITY_REGISTRY` duplicate registration issue using `linkme` distributed slice iteration
- `DynAxiom::check()` renamed to `check_dyn()` to avoid method name conflicts
- `cell_id()` return type mismatch (`&CellId` → `CellId`)
- Compile-fail test updates for new macro format
- Constraint test error type expectation (`LayerViolation` → `SignalValidationFailed`)
- `axiom-cli`, `axiom-macros`, `axiom-runtime` Cargo.toml `axiom-core` dependencies removed
- Documentation files updated to use `axiom-kernel` instead of `axiom-core`

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

## [0.2.0]

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

[Unreleased]: https://github.com/axiom-framework/axiom/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/axiom-framework/axiom/releases/tag/v0.4.0
[0.3.0]: https://github.com/axiom-framework/axiom/releases/tag/v0.3.0
[0.2.0]: https://github.com/axiom-framework/axiom/releases/tag/v0.2.0
[0.1.0]: https://github.com/axiom-framework/axiom/releases/tag/v0.1.0