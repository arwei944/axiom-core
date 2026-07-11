# Architecture Upgrade Plan and Test Specification

> Target: axiom-core-project
> Baseline: v0.4.0
> Goal: v0.5.0 production readiness

## Legend
- `[ ]` Pending
- `[~]` In progress
- `[x]` Done

---

## Completed Work

### Phase 1: Request-Reply Protocol
- [x] Add `reply_to` and `request_id` to `SignalEnvelope`
- [x] Add `CellContext::reply()` helper
- [x] Add `AxiomRuntime::pending_responses`
- [x] Add `AxiomRuntime::submit_query()` API
- [x] Update dispatch loop to route replies via `reply_to`
- [x] Add roundtrip test `test_submit_query_reply_roundtrip`
- [x] Fix all `SignalEnvelope` struct literal initializations
- [x] Verify `cargo check --package axiom-runtime`
- [x] Verify `cargo test --package axiom-runtime`

### Phase 2: Unify CapabilityDescriptor Definitions
- [x] Rename plugin ABI type to `PluginCapabilityDescriptor`
- [x] Keep `registry::CapabilityDescriptor` unchanged
- [x] Update `AxiomPlugin::capabilities()` return type
- [x] Update re-exports in `plugin/mod.rs`
- [x] Update all plugin implementations and tests
- [x] Verify `cargo check --workspace`
- [x] Verify `cargo test --package axiom-kernel`
- [x] Verify `cargo test --package axiom-runtime`

### Phase 3: Cell Self-Description + Code Generator
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

### Phase 4: PluginRegistry Kind Detection + Signal Cache
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

**Tests**
- **Unit**: create witness chain and assert every witness has non-zero hash.
- **Unit**: mutate a witness payload and assert `verify_chain_integrity` fails.
- **Property**: generate chain of length N and assert hash monotonic advancement.

---

### P0-2: Unify Layer Validation
- [ ] Make `SignalEnvelope::validate_layer_transition` delegate to `Layer::can_send_to`
- [ ] Remove hardcoded match arms
- [ ] Add conformance test covering all legal/illegal transitions

**Tests**
- **Conformance**: matrix test over all `source_layer -> target_layer` pairs.
- **Assert**: compile-time `CanSendTo` and runtime `validate_layer_transition` results match.

---

### P0-3: CellKernel O(1) Lookup
- [ ] Replace `RwLock<Vec<(CellHandle, CellState)>>` with `DashMap<CellId, CellState>`
- [ ] Update `create/send/receive/list/status` to use new map
- [ ] Benchmark 1000 cells send/receive latency

**Tests**
- **Benchmark**: 1000 cells, measure `send`/`receive` p99 latency.
- **Assert**: average latency < 1µs and scales linearly.
- **Regression**: existing `CellKernel` tests still pass after map migration.

---

### P0-4: Per-Type SchemaVersion
- [ ] Redesign `SchemaVersion` to per-type mapping
- [ ] Update `Signal` trait and serialization path
- [ ] Update migration registry verification per type
- [ ] Add compatibility test for mixed schema versions

**Tests**
- **Unit**: register two signal types with different versions and assert independent compatibility checks.
- **Migration**: verify migration chain completeness per type.
- **Regression**: existing signals without explicit type version default to version 1.

---

### P0-5: Decouple CircuitBreaker
- [ ] Extract `CircuitBreaker` from `SupervisionStrategy`
- [ ] Apply default circuit break policy to all cells
- [ ] Persist restart/circuit state to `EventStore`
- [ ] Add restart-after-crash recovery test

**Tests**
- **Unit**: restart-strategy cell exceeds failure threshold and triggers circuit break.
- **Unit**: supervisor state restart preserves counts via `EventStore`.
- **Chaos**: kill cell process and assert recovered state restores circuit/open counts.

---

### P1-1: DLQ Persistence
- [ ] Define `DeadLetterStore` trait
- [ ] Implement memory + sqlite store adapters
- [ ] Add consumer semantics: `peek/ack/retry`
- [ ] Add crash recovery test for DLQ

**Tests**
- **Crash test**: enqueue dead letters, restart runtime, assert messages retained.
- **Consumer test**: `peek/ack/retry` sequence completes without duplication.

---

### P1-2: Backoff Jitter
- [ ] Implement full jitter backoff
- [ ] Expose base/cap/multiplier in `RuntimeConfig`
- [ ] Add restart dispersion test under 100 concurrent failures

**Tests**
- **Statistical**: 100 failing cells restart within window; assert no more than 20 restart in same second.
- **Config**: override base/cap and assert reflected in computed delays.

---

### P1-3: Fix ReplayEngine Snapshot Lookup
- [ ] Pass `aggregate_id` through replay paths
- [ ] Remove hardcoded empty aggregate_id
- [ ] Add snapshot acceleration benchmark

**Tests**
- **Benchmark**: replay with snapshot vs replay from event 0; assert >30% speedup.
- **Unit**: `replay_by_correlation` loads correct snapshot by `aggregate_id`.

---

### P1-4: Guard Layer Binding
- [ ] Change `Guard::layer` from `Option<Layer>` to `Layer`
- [ ] Add registration-time validation
- [ ] Add compile-time conformance test

**Tests**
- **Compile-fail**: trybuild case where Guard omits layer binding.
- **Runtime**: register mismatched layer and assert rejected.

---

### P1-5: WASM ABI Versioning
- [ ] Define exported `axiom_abi_version` symbol
- [ ] Add loader version check
- [ ] Add mismatch error test

**Tests**
- **Load test**: plugin without version symbol returns `AbiMismatch`.
- **Compatibility**: load matching version succeeds.

---

### P2-1: Lock-Free Mailbox
- [ ] Replace `tokio::Mutex<VecDeque>` with lock-free queue
- [ ] Benchmark throughput/latency before and after

**Tests**
- **Benchmark**: single-threaded push/pop throughput before and after.
- **Assert**: throughput improves by >= 20%, no message loss under contention.

---

### P2-2: SignalKernel LRU Cache
- [ ] Add bounded LRU cache with TTL
- [ ] Add memory growth regression test

**Tests**
- **Memory test**: send 20k unique signals with cache size 10k and assert bounded memory growth.
- **TTL test**: advance time beyond TTL and assert cache miss.

---

### P2-3: Metrics Default Enable
- [ ] Flip `metrics` feature to default
- [ ] Add global `MetricsRegistry`
- [ ] Add endpoint availability test

**Tests**
- **Smoke**: default build exposes `/metrics` endpoint.
- **Registry**: two crates register same metric name and assert collision handling.

---

### P2-4: Health Liveness Probe
- [ ] Add dispatch loop heartbeat timestamp
- [ ] Add degraded health detection
- [ ] Add stuck dispatch loop test

**Tests**
- **Fault inject**: freeze dispatch loop and assert health becomes degraded.
- **Recovery**: resume loop and assert health recovers within probe interval.

---

### P2-5: Composer Wiring
- [ ] Implement `ConnectionSpec` wiring
- [ ] Add TOML-driven topology integration test

**Tests**
- **Integration**: TOML spec defines 2 cells + 1 plugin connection; assert messages route end-to-end.
- **Regression**: missing connection spec produces explicit configuration error.

---

### P3-1: Macro Error UX
- [ ] Centralize macro error types
- [ ] Improve compile-fail test stderr readability

**Tests**
- **Review**: compile-fail stderr diff before/after; assert human-readable location and hint.
- **Coverage**: every macro has at least one compile-fail test.

---

### P3-2: Plugin Hot Reload
- [ ] Add versioned plugin instances
- [ ] Add upgrade API with refcount semantics

**Tests**
- **Upgrade test**: load v1 plugin, upgrade to v2, assert old instance kept alive until dropped.
- **Refcount test**: verify in-flight requests complete before old plugin unload.

---

### P3-3: Native ABI Versioning
- [ ] Add `abi_version` to `PluginPackage`
- [ ] Add loader compatibility check

**Tests**
- **Load test**: old ABI package rejected with `AbiMismatch`.
- **Package test**: `PluginPackage` includes `abi_version` round-trips through pack/unpack.

---

### P3-4: CI Coverage Gate
- [ ] Add `cargo llvm-cov` workflow
- [ ] Set 80% threshold for core crates

**Tests**
- **Workflow**: GitHub Actions runs `cargo llvm-cov` on PR.
- **Gate**: core crates `axiom-kernel`, `axiom-runtime`, `axiom-store` coverage >= 80%.

---

### P3-5: Dynamic Crate Version
- [ ] Inject `CARGO_PKG_VERSION` into `Version::CURRENT`
- [ ] Add build-time verification test

**Tests**
- **Build test**: `Version::CURRENT` equals `CARGO_PKG_VERSION` from Cargo.toml.
- **Witness test**: emitted witness records actual crate version, not hardcoded `0.1.0`.

---

## Test Execution Commands

```bash
# Fast checks
cargo check --workspace
cargo test --package axiom-kernel
cargo test --package axiom-runtime

# Full matrix
cargo test --workspace
cargo test --all-features --workspace

# Coverage
cargo llvm-cov --workspace --html
```

## Coverage Requirement
- Core crates: >= 80%
- Plugin crates: >= 60%
- Macro crate: compile-fail coverage for all documented failure modes
