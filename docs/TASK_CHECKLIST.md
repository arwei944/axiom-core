# Architecture Upgrade Task Checklist

> Target: axiom-core-project  
> Baseline: v0.4.0  
> Goal: v0.5.0 production readiness  
> **Status: COMPLETE — open = 0（2026-07-19）**

完成说明（生产接线、测试、偏差）：见 **[ENGINEERING_HARDENING_v050.md](./ENGINEERING_HARDENING_v050.md)**。

## Legend
- `[ ]` Pending
- `[~]` In progress
- `[x]` Done

---

## Phase 1: Request-Reply Protocol
- [x] Add `reply_to` and `request_id` to `SignalEnvelope`
- [x] Add `CellContext::reply()` helper
- [x] Add `AxiomRuntime::pending_responses`
- [x] Add `AxiomRuntime::submit_query()` API
- [x] Update dispatch loop to route replies via `reply_to`
- [x] Add roundtrip test `test_submit_query_reply_roundtrip`
- [x] Fix all `SignalEnvelope` struct literal initializations
- [x] Verify `cargo check --package axiom-runtime`
- [x] Verify `cargo test --package axiom-runtime`

---

## Phase 2: Unify CapabilityDescriptor Definitions
- [x] Rename plugin ABI type to `PluginCapabilityDescriptor`
- [x] Keep `registry::CapabilityDescriptor` unchanged
- [x] Update `AxiomPlugin::capabilities()` return type
- [x] Update re-exports in `plugin/mod.rs`
- [x] Update all plugin implementations and tests
- [x] Verify `cargo check --workspace`
- [x] Verify `cargo test --package axiom-kernel`
- [x] Verify `cargo test --package axiom-runtime`

---

## Phase 3: Cell Self-Description + Code Generator
- [x] Add `DynCell::supported_signals()` with default
- [x] Add `RuntimeCellHandle::supported_signals()`
- [x] Add `CellRegistration::supported_signals`
- [x] Add `RegisteredCell::supported_signals`
- [x] Plumb `supported_signals` through runtime registration
- [x] Update dispatch tuple to carry supported signals
- [x] Update test fixtures to include `supported_signals`
- [x] Add `tools/cell_codegen` binary prototype
- [x] Verify generator output compiles semantically
- [x] Verify `cargo test --package axiom-runtime`

---

## Phase 4: PluginRegistry Kind Detection + Signal Cache
- [x] Add `AxiomPlugin::kind()` default method
- [x] Update `PluginRegistry::register()` to use `plugin.kind()`
- [x] Add `SignalKernel::cache` field
- [x] Add cache hit short-circuit in `SignalKernel::send()`
- [x] Verify `cargo check --package axiom-kernel`
- [x] Verify `cargo test --package axiom-kernel`
- [x] Verify `cargo test --package axiom-runtime`

---

## v0.5.0 Upgrade Tasks

### P0-1: Fix Witness Hash Chain
- [x] Define `WitnessHash` computation path using Blake3/SHA-256 fallback
- [x] Update `WitnessBuilder::emit` to set non-zero hash
- [x] Update `verify_chain_integrity` to enforce hash linkage
- [x] Add unit test: tampered witness fails verification
- [x] Add property test: chain length N preserves monotonic hash

### P0-2: Unify Layer Validation
- [x] Make `SignalEnvelope::validate_layer_transition` delegate to `Layer::can_send_to`
- [x] Remove hardcoded match arms
- [x] Add conformance test covering all legal/illegal transitions

### P0-3: CellKernel O(1) Lookup
- [x] Replace `RwLock<Vec<(CellHandle, CellState)>>` with `DashMap<CellId, CellState>`
- [x] Update `create/send/receive/list/status` to use new map
- [x] Benchmark 1000 cells send/receive latency

### P0-4: Per-Type SchemaVersion
- [x] Redesign `SchemaVersion` to per-type mapping
- [x] Update `Signal` trait and serialization path
- [x] Update migration registry verification per type
- [x] Add compatibility test for mixed schema versions

### P0-5: Decouple CircuitBreaker
- [x] Extract `CircuitBreaker` from `SupervisionStrategy`
- [x] Apply default circuit break policy to all cells
- [x] Persist restart/circuit state to `EventStore`
- [x] Add restart-after-crash recovery test

---

### P1-1: DLQ Persistence
- [x] Define `DeadLetterStore` trait
- [x] Implement memory + sqlite store adapters
- [x] Add consumer semantics: `peek/ack/retry`
- [x] Add crash recovery test for DLQ

### P1-2: Backoff Jitter
- [x] Implement full jitter backoff
- [x] Expose base/cap/multiplier in `RuntimeConfig`
- [x] Add restart dispersion test under 100 concurrent failures

### P1-3: Fix ReplayEngine Snapshot Lookup
- [x] Pass `aggregate_id` through replay paths
- [x] Remove hardcoded empty aggregate_id
- [x] Add snapshot acceleration benchmark

### P1-4: Guard Layer Binding
- [x] Change `Guard::layer` from `Option<Layer>` to `Layer`
- [x] Add registration-time validation
- [x] Add compile-time conformance test

### P1-5: WASM ABI Versioning
- [x] Define exported `axiom_abi_version` symbol
- [x] Add loader version check
- [x] Add mismatch error test

---

### P2-1: Lock-Free Mailbox
- [x] Replace `tokio::Mutex<VecDeque>` with lock-free queue
- [x] Benchmark throughput/latency before and after

### P2-2: SignalKernel LRU Cache
- [x] Add bounded LRU cache with TTL
- [x] Add memory growth regression test

### P2-3: Metrics Default Enable
- [x] Flip `metrics` feature to default
- [x] Add global `MetricsRegistry`
- [x] Add endpoint availability test

### P2-4: Health Liveness Probe
- [x] Add dispatch loop heartbeat timestamp
- [x] Add degraded health detection
- [x] Add stuck dispatch loop test

### P2-5: Composer Wiring
- [x] Implement `ConnectionSpec` wiring
- [x] Add TOML-driven topology integration test

---

### P3-1: Macro Error UX
- [x] Centralize macro error types
- [x] Improve compile-fail test stderr readability

### P3-2: Plugin Hot Reload
- [x] Add versioned plugin instances
- [x] Add upgrade API with refcount semantics

### P3-3: Native ABI Versioning
- [x] Add `abi_version` to `PluginPackage`
- [x] Add loader compatibility check

### P3-4: CI Coverage Gate
- [x] Add `cargo llvm-cov` workflow
- [x] Set 80% threshold for core crates

### P3-5: Dynamic Crate Version
- [x] Inject `CARGO_PKG_VERSION` into `Version::CURRENT`
- [x] Add build-time verification test
