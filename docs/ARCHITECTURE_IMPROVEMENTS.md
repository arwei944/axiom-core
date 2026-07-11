# Architecture Improvement Task Checklist

> Target: axiom-core-project
> Baseline: v0.4.0
> Source: Architecture Review (2026-07-11)

## Legend
- `[ ]` Pending
- `[~]` In progress
- `[x]` Done

---

## Critical - Security & Reliability

### C-1: Fix MCP Security Layer Fail-Open
- [ ] Change `check_permission()` in `axiom-mcp/src/security.rs` to return `Err(PermissionDenied)` when tool permission not found
- [ ] Add unit test: unregistered tool returns permission denied
- [ ] Add integration test: MCP call with unregistered permission fails
- [ ] Verify `cargo test --package axiom-mcp`

### C-2: Add SQLite Transaction Protection
- [ ] Wrap `append_batch()` and `append()` operations in `BEGIN/COMMIT` transactions in `axiom-store/src/sqlite/store.rs`
- [ ] Enable WAL mode in SQLite connection config
- [ ] Add crash recovery test: restart after partial write
- [ ] Verify `cargo test --package axiom-store --features sqlite`

### C-3: Add LlmClient Global Timeout
- [ ] Add `request_timeout_ms` to `RetryConfig` in `axiom-llm/src/types.rs`
- [ ] Implement timeout wrapping in `LlmClient::with_retry()` in `axiom-llm/src/client.rs`
- [ ] Add test: request exceeding timeout returns `LlmError::Timeout`
- [ ] Verify `cargo test --package axiom-llm`

---

## High Priority - Maintainability

### H-1: Refactor Dispatch Loop Monolith
- [ ] Extract witness persistence logic into `dispatch/witness_persistence.rs`
- [ ] Extract snapshot management into `dispatch/snapshot_manager.rs`
- [ ] Extract supervision decision handling into `dispatch/supervision.rs`
- [ ] Extract entropy governance into `dispatch/entropy.rs`
- [ ] Reduce main loop function to < 100 lines
- [ ] Add unit tests for each extracted component
- [ ] Verify `cargo test --package axiom-runtime`

### H-2: Fix EntropyGovernorCell Layer Leakage
- [ ] Move `EntropyGovernorCell` implementation from `axiom-runtime` to `axiom-oversight`
- [ ] Add formal exemption in `architecture.toml` if keeping current arrangement
- [ ] Update re-exports in `axiom-oversight/src/entropy_governor.rs`
- [ ] Verify `cargo check --workspace`
- [ ] Verify `cargo test --package axiom-oversight`

### H-3: Replace Magic Numbers with Constants
- [ ] Extract snapshot interval (100 events) as `SNAPSHOT_INTERVAL` in `axiom-runtime/src/dispatch/loop.rs`
- [ ] Extract entropy cooldown (30s) as `ENTROPY_COOLDOWN_NS` in `axiom-runtime/src/entropy_gov.rs`
- [ ] Extract DLQ capacity default as `DEFAULT_DLQ_CAPACITY` in `axiom-runtime/src/runtime/config.rs`
- [ ] Add all constants to `axiom-kernel/src/lib.rs` re-exports for visibility

---

## Medium Priority - Scalability

### M-1: Complete Distributed Event Sync
- [ ] Implement gossip protocol in `axiom-distributed/src/sync.rs`
- [ ] Add actual event retrieval in `EventSync::sync()`
- [ ] Implement conflict resolution for concurrent writes
- [ ] Add cluster integration test: 3-node event propagation
- [ ] Verify `cargo test --package axiom-distributed`

### M-2: Add Deployment Artifacts
- [ ] Create `Dockerfile` for runtime binary
- [ ] Create `docker-compose.yml` with runtime + SQLite
- [ ] Add health check endpoint to `MetricsServer`
- [ ] Add deployment documentation in `docs/PRODUCTION.md`

### M-3: Add Prometheus Metrics Export
- [ ] Enable `metrics` feature by default in `axiom-runtime/Cargo.toml`
- [ ] Add global `MetricsRegistry` in `axiom-runtime/src/telemetry.rs`
- [ ] Export entropy score, cell restart count, circuit breaker state
- [ ] Add metrics endpoint to `MetricsServer`
- [ ] Add Grafana dashboard template

---

## Low Priority - Polish

### L-1: Add TLS Support for MCP
- [ ] Add TLS configuration to `axiom-mcp/src/server.rs`
- [ ] Add certificate management to `SecurityContext`
- [ ] Add integration test: encrypted MCP connection

### L-2: Implement Gray Release
- [ ] Add versioned cell instances in runtime
- [ ] Add traffic routing based on cell version
- [ ] Add gradual rollout API
- [ ] Add canary deployment test

### L-3: Add Kubernetes Operator
- [ ] Create `deploy/k8s/` directory with CRD definitions
- [ ] Implement basic operator for cell deployment
- [ ] Add rolling update support
- [ ] Add integration test with minikube

---

## Risk Mitigation Matrix

| Risk | Task | Priority | Owner | Estimate |
|------|------|----------|-------|----------|
| MCP Security Fail-Open | C-1 | Critical | Security | 1d |
| SQLite No Transaction | C-2 | Critical | Storage | 1d |
| LLM Client No Timeout | C-3 | Critical | LLM | 0.5d |
| Dispatch Loop Monolith | H-1 | High | Runtime | 3d |
| EntropyGovernor Leakage | H-2 | High | Oversight | 0.5d |
| Magic Numbers | H-3 | High | Kernel | 0.5d |
| Distributed Sync Empty | M-1 | Medium | Distributed | 5d |
| No Deployment Artifacts | M-2 | Medium | DevOps | 2d |
| No Metrics Export | M-3 | Medium | Telemetry | 2d |

---

## Completion Criteria

- [ ] All Critical tasks completed
- [ ] All High priority tasks completed
- [ ] CI pipeline passes: `cargo test --workspace`
- [ ] Coverage threshold met: `cargo llvm-cov --workspace`
- [ ] Architecture verification passes: `axm verify`
- [ ] Security audit passes: `foxguard .`