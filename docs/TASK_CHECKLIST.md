# Architecture Upgrade Task Checklist

> Target: axiom-core-project
> Baseline: v0.4.0
> Goal: v0.5.0 production readiness

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
- [ ] Define `WitnessHash` computation path using Blake3/SHA-256 fallback
- [ ] Update `WitnessBuilder::emit` to set non-zero hash
- [ ] Update `verify_chain_integrity` to enforce hash linkage
- [ ] Add unit test: tampered witness fails verification
- [ ] Add property test: chain length N preserves monotonic hash

### P0-2: Unify Layer Validation
- [ ] Make `SignalEnvelope::validate_layer_transition` delegate to `Layer::can_send_to`
- [ ] Remove hardcoded match arms
- [ ] Add conformance test covering all legal/illegal transitions

### P0-3: CellKernel O(1) Lookup
- [ ] Replace `RwLock<Vec<(CellHandle, CellState)>>` with `DashMap<CellId, CellState>`
- [ ] Update `create/send/receive/list/status` to use new map
- [ ] Benchmark 1000 cells send/receive latency

### P0-4: Per-Type SchemaVersion
- [ ] Redesign `SchemaVersion` to per-type mapping
- [ ] Update `Signal` trait and serialization path
- [ ] Update migration registry verification per type
- [ ] Add compatibility test for mixed schema versions

### P0-5: Decouple CircuitBreaker
- [ ] Extract `CircuitBreaker` from `SupervisionStrategy`
- [ ] Apply default circuit break policy to all cells
- [ ] Persist restart/circuit state to `EventStore`
- [ ] Add restart-after-crash recovery test

---

### P1-1: DLQ Persistence
- [ ] Define `DeadLetterStore` trait
- [ ] Implement memory + sqlite store adapters
- [ ] Add consumer semantics: `peek/ack/retry`
- [ ] Add crash recovery test for DLQ

### P1-2: Backoff Jitter
- [ ] Implement full jitter backoff
- [ ] Expose base/cap/multiplier in `RuntimeConfig`
- [ ] Add restart dispersion test under 100 concurrent failures

### P1-3: Fix ReplayEngine Snapshot Lookup
- [ ] Pass `aggregate_id` through replay paths
- [ ] Remove hardcoded empty aggregate_id
- [ ] Add snapshot acceleration benchmark

### P1-4: Guard Layer Binding
- [ ] Change `Guard::layer` from `Option<Layer>` to `Layer`
- [ ] Add registration-time validation
- [ ] Add compile-time conformance test

### P1-5: WASM ABI Versioning
- [ ] Define exported `axiom_abi_version` symbol
- [ ] Add loader version check
- [ ] Add mismatch error test

---

### P2-1: Lock-Free Mailbox
- [ ] Replace `tokio::Mutex<VecDeque>` with lock-free queue
- [ ] Benchmark throughput/latency before and after

### P2-2: SignalKernel LRU Cache
- [ ] Add bounded LRU cache with TTL
- [ ] Add memory growth regression test

### P2-3: Metrics Default Enable
- [ ] Flip `metrics` feature to default
- [ ] Add global `MetricsRegistry`
- [ ] Add endpoint availability test

### P2-4: Health Liveness Probe
- [ ] Add dispatch loop heartbeat timestamp
- [ ] Add degraded health detection
- [ ] Add stuck dispatch loop test

### P2-5: Composer Wiring
- [ ] Implement `ConnectionSpec` wiring
- [ ] Add TOML-driven topology integration test

---

### P3-1: Macro Error UX
- [ ] Centralize macro error types
- [ ] Improve compile-fail test stderr readability

### P3-2: Plugin Hot Reload
- [ ] Add versioned plugin instances
- [ ] Add upgrade API with refcount semantics

### P3-3: Native ABI Versioning
- [ ] Add `abi_version` to `PluginPackage`
- [ ] Add loader compatibility check

### P3-4: CI Coverage Gate
- [ ] Add `cargo llvm-cov` workflow
- [ ] Set 80% threshold for core crates

### P3-5: Dynamic Crate Version
- [ ] Inject `CARGO_PKG_VERSION` into `Version::CURRENT`
- [ ] Add build-time verification test
