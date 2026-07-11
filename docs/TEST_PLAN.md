# Architecture Upgrade Test Document

> This document defines the verification strategy for the v0.5.0 architecture upgrade tasks.
> Each task maps to one or more executable tests.

---

## 1. Request-Reply Protocol
- **Unit**: `crates/axiom-runtime/src/runtime/tests.rs::test_submit_query_reply_roundtrip`
  - Verifies `submit_query` returns payload from `ctx.reply()`.
- **Integration**: publish a `Query` without reply -> verify `Timeout` error with structured fields.
- **Regression**: ensure existing `publish_command` and `submit_signal` still broadcast normally.

---

## 2. Unified CapabilityDescriptor
- **Compile**: `cargo check --workspace`
- **Test**: `cargo test --package axiom-kernel && cargo test --package axiom-runtime`
- **Refactor guard**: grep for old type name; zero matches required.

---

## 3. Cell Self-Description + Code Generator
- **Unit**: add test asserting `RuntimeCellHandle::supported_signals()` returns registration data.
- **CLI smoke**: run `cell_codegen QueryCell Foo Bar` and compile generated snippet.
- **Regression**: existing tests without `supported_signals` still pass with default empty list.

---

## 4. PluginRegistry Kind Detection + Signal Cache
- **Unit**: register plugin with custom `kind()` and assert `get_all_by_kind` returns it.
- **Unit**: call `SignalKernel::send` twice with same `msg_id` and assert cached path is hit.
- **Regression**: `cargo test --package axiom-kernel && cargo test --package axiom-runtime`

---

## 5. Witness Hash Chain (P0-1)
- **Unit**: create witness chain and assert every witness has non-zero hash.
- **Unit**: mutate a witness payload and assert `verify_chain_integrity` fails.
- **Property**: generate chain of length N and assert hash monotonic advancement.

---

## 6. Unified Layer Validation (P0-2)
- **Conformance**: matrix test over all `source_layer -> target_layer` pairs.
- **Assert**: compile-time `CanSendTo` and runtime `validate_layer_transition` results match.

---

## 7. CellKernel O(1) Lookup (P0-3)
- **Benchmark**: 1000 cells, measure `send`/`receive` p99 latency.
- **Assert**: average latency < 1µs and scales linearly.
- **Regression**: existing `CellKernel` tests still pass after map migration.

---

## 8. Per-Type SchemaVersion (P0-4)
- **Unit**: register two signal types with different versions and assert independent compatibility checks.
- **Migration**: verify migration chain completeness per type.
- **Regression**: existing signals without explicit type version default to version 1.

---

## 9. Decoupled CircuitBreaker (P0-5)
- **Unit**: restart-strategy cell exceeds failure threshold and triggers circuit break.
- **Unit**: supervisor state restart preserves counts via `EventStore`.
- **Chaos**: kill cell process and assert recovered state restores circuit/open counts.

---

## 10. DLQ Persistence (P1-1)
- **Crash test**: enqueue dead letters, restart runtime, assert messages retained.
- **Consumer test**: `peek/ack/retry` sequence completes without duplication.

---

## 11. Backoff Jitter (P1-2)
- **Statistical**: 100 failing cells restart within window; assert no more than 20 restart in same second.
- **Config**: override base/cap and assert reflected in computed delays.

---

## 12. ReplayEngine Snapshot Lookup (P1-3)
- **Benchmark**: replay with snapshot vs replay from event 0; assert >30% speedup.
- **Unit**: `replay_by_correlation` loads correct snapshot by `aggregate_id`.

---

## 13. Guard Layer Binding (P1-4)
- **Compile-fail**: trybuild case where Guard omits layer binding.
- **Runtime**: register mismatched layer and assert rejected.

---

## 14. WASM ABI Versioning (P1-5)
- **Load test**: plugin without version symbol returns `AbiMismatch`.
- **Compatibility**: load matching version succeeds.

---

## 15. Lock-Free Mailbox (P2-1)
- **Benchmark**: single-threaded push/pop throughput before and after.
- **Assert**: throughput improves by >= 20%, no message loss under contention.

---

## 16. SignalKernel LRU Cache (P2-2)
- **Memory test**: send 20k unique signals with cache size 10k and assert bounded memory growth.
- **TTL test**: advance time beyond TTL and assert cache miss.

---

## 17. Metrics Default Enable (P2-3)
- **Smoke**: default build exposes `/metrics` endpoint.
- **Registry**: two crates register same metric name and assert collision handling.

---

## 18. Health Liveness Probe (P2-4)
- **Fault inject**: freeze dispatch loop and assert health becomes degraded.
- **Recovery**: resume loop and assert health recovers within probe interval.

---

## 19. Composer Wiring (P2-5)
- **Integration**: TOML spec defines 2 cells + 1 plugin connection; assert messages route end-to-end.
- **Regression**: missing connection spec produces explicit configuration error.

---

## 20. Macro Error UX (P3-1)
- **Review**: compile-fail stderr diff before/after; assert human-readable location and hint.
- **Coverage**: every macro has at least one compile-fail test.

---

## 21. Plugin Hot Reload (P3-2)
- **Upgrade test**: load v1 plugin, upgrade to v2, assert old instance kept alive until dropped.
- **Refcount test**: verify in-flight requests complete before old plugin unload.

---

## 22. Native ABI Versioning (P3-3)
- **Load test**: old ABI package rejected with `AbiMismatch`.
- **Package test**: `PluginPackage` includes `abi_version` round-trips through pack/unpack.

---

## 23. CI Coverage Gate (P3-4)
- **Workflow**: GitHub Actions runs `cargo llvm-cov` on PR.
- **Gate**: core crates `axiom-kernel`, `axiom-runtime`, `axiom-store` coverage >= 80%.

---

## 24. Dynamic Crate Version (P3-5)
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
