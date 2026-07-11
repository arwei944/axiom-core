# API Gateway Implementation Plan

> Target: axiom-core-project
> Baseline: v0.4.0
> Source: Architecture Review (2026-07-12)
> Goal: Create unified frontend-facing API layer (`axiom-api`)

## Legend
- `[ ]` Pending
- `[~]` In progress
- `[x]` Done

---

## Overview

Current architecture has HTTP endpoints scattered across:
- `axiom-runtime/src/server.rs` (port 9090): `/metrics`, `/health`
- `axiom-runtime/src/dashboard.rs` (port 9091): `/dashboard/health`, `/dashboard/cells`, `/dashboard/heatmap`, `/dashboard/ws`

This violates separation of concerns and makes frontend integration complex.

**Solution**: Create `axiom-api` crate at Layer 1 to serve as unified API gateway.

---

## Architecture Design

### Crate Layer Position
```toml
# architecture.toml update
[crate-layers]
axiom-api = 1  # Unified API gateway (Layer 1: Visualization/API)
```

### Dependency Rules
| Dependency | Direction | Reason |
|------------|-----------|--------|
| `axiom-api` → `axiom-viz` | ✓ | Data models for visualization |
| `axiom-api` → `axiom-runtime` | ✓ | Runtime data via trait interface |
| `axiom-api` → `axiom-oversight` | ✓ | Health and governance data |
| `axiom-api` → `axiom-kernel` | ✓ | Core types and errors |
| `axiom-runtime` → `axiom-api` | ✗ | Runtime must not depend on API |

### API Contract Versioning
- `v1`: Initial version, backward compatible with existing endpoints
- Future versions: `/api/v2/*` with breaking changes

---

## Phase 1: Interface Abstraction

### P1-1: Define RuntimeDataSource Trait
- [ ] Create `axiom-runtime/src/api/data_source.rs` with `RuntimeDataSource` trait
- [ ] Define async methods: `get_health()`, `get_cells()`, `get_heatmap()`, `subscribe_signals()`
- [ ] Define `DataSourceError` type with proper error handling
- [ ] Implement `RuntimeDataSource` for `AxiomRuntime`
- [ ] Add unit test: trait interface compiles and works
- [ ] Verify `cargo check --package axiom-runtime`

### P1-2: Define OversightDataSource Trait
- [ ] Create `axiom-oversight/src/api/data_source.rs` with `OversightDataSource` trait
- [ ] Define methods: `get_system_health()`, `get_entropy_status()`, `get_compliance_report()`
- [ ] Implement `OversightDataSource` for `OversightKernelAdapter`
- [ ] Add unit test: trait interface compiles and works
- [ ] Verify `cargo check --package axiom-oversight`

### P1-3: Expose Trait Re-exports
- [ ] Re-export `RuntimeDataSource` from `axiom-runtime/src/lib.rs`
- [ ] Re-export `OversightDataSource` from `axiom-oversight/src/lib.rs`
- [ ] Add documentation for each trait method
- [ ] Verify `cargo doc --package axiom-runtime --open` builds successfully

**Phase 1 Acceptance Criteria**:
- [ ] `RuntimeDataSource` trait defined and implemented
- [ ] `OversightDataSource` trait defined and implemented
- [ ] All trait methods are async-compatible with axum extractors
- [ ] `cargo check --package axiom-runtime --package axiom-oversight` passes
- [ ] `cargo test --package axiom-runtime --package axiom-oversight` passes

---

## Phase 2: Create axiom-api Crate

### P2-1: Create axiom-api Crate Structure
- [ ] Create `crates/axiom-api/Cargo.toml` with dependencies: axum, serde, tokio, hyper-tls
- [ ] Create `crates/axiom-api/src/lib.rs` with module structure
- [ ] Add `axiom-api` to workspace `Cargo.toml`
- [ ] Add `axiom-api = 1` to `.axiom/architecture.toml`
- [ ] Verify `cargo check --package axiom-api`

### P2-2: Define API Data Types
- [ ] Create `crates/axiom-api/src/types/mod.rs` with versioned API types
- [ ] Define `ApiHealth`, `ApiCell`, `ApiHeatmap`, `ApiSignalEvent` types
- [ ] Add `ApiError` type with proper HTTP status codes
- [ ] Add serde serialization/deserialization for all types
- [ ] Add unit test: serialization roundtrip for all types

### P2-3: Implement Versioned API Router
- [ ] Create `crates/axiom-api/src/router/v1.rs` with v1 API routes
- [ ] Implement `/api/v1/health` (GET) - aggregates runtime + oversight health
- [ ] Implement `/api/v1/cells` (GET) - returns cell list
- [ ] Implement `/api/v1/heatmap` (GET) - returns signal heatmap
- [ ] Implement `/api/v1/entropy` (GET) - returns entropy status
- [ ] Implement `/api/v1/ws` (WebSocket) - real-time signal stream
- [ ] Add CORS middleware with configurable origins
- [ ] Add request logging middleware
- [ ] Add error handling middleware (converts `ApiError` to HTTP response)

### P2-4: Implement Data Aggregation Layer
- [ ] Create `crates/axiom-api/src/aggregator/mod.rs`
- [ ] Implement `HealthAggregator` that combines runtime and oversight health
- [ ] Implement `SignalAggregator` that subscribes to runtime signals
- [ ] Add caching for expensive queries (heatmap, entropy)
- [ ] Add unit test: aggregation logic works correctly

**Phase 2 Acceptance Criteria**:
- [ ] `axiom-api` crate compiles successfully
- [ ] All v1 API endpoints defined and implemented
- [ ] CORS, logging, error handling middleware in place
- [ ] `cargo check --package axiom-api` passes
- [ ] Unit tests for API types and aggregators pass
- [ ] Architecture check passes: `axm verify`

---

## Phase 3: Integration with Runtime

### P3-1: Wire API Gateway to Runtime
- [ ] Update `AxiomRuntime::new()` to create `axiom-api` server instance
- [ ] Pass `RuntimeDataSource` and `OversightDataSource` to API server
- [ ] Configure API server port (default: 9092)
- [ ] Start API server alongside existing MetricsServer/DashboardServer
- [ ] Add integration test: API server starts and responds

### P3-2: Migrate Existing Endpoints
- [ ] Add `/api/v1/metrics` endpoint (mirrors `/metrics` on port 9090)
- [ ] Add `/api/v1/dashboard/*` endpoints (mirrors port 9091 endpoints)
- [ ] Mark old endpoints as deprecated in documentation
- [ ] Add deprecation warning headers to old endpoints
- [ ] Add integration test: deprecated endpoints still work

### P3-3: Add Authentication Middleware
- [ ] Add API key authentication to `/api/v1/*` endpoints
- [ ] Add JWT authentication support (optional feature)
- [ ] Add permission scopes for different endpoint groups
- [ ] Add integration test: unauthorized request returns 401
- [ ] Add integration test: authorized request succeeds

**Phase 3 Acceptance Criteria**:
- [ ] API server integrates with `AxiomRuntime` without breaking changes
- [ ] All old endpoints mirrored in v1 API
- [ ] Old endpoints return deprecation warnings
- [ ] Authentication middleware works correctly
- [ ] Integration tests for API + Runtime pass
- [ ] `cargo test --package axiom-runtime` passes

---

## Phase 4: Deprecate and Remove Old Endpoints

### P4-1: Remove DashboardServer from Runtime
- [ ] Remove `DashboardServer` struct and implementation from `axiom-runtime/src/dashboard.rs`
- [ ] Remove `/dashboard/*` endpoints from port 9091
- [ ] Update runtime tests to use API endpoints
- [ ] Verify `cargo test --package axiom-runtime`

### P4-2: Consolidate MetricsServer
- [ ] Move `/metrics` endpoint to `axiom-api`
- [ ] Remove `MetricsServer` struct from `axiom-runtime/src/server.rs`
- [ ] Update health endpoint in runtime to use API
- [ ] Verify `cargo test --package axiom-runtime`

### P4-3: Clean Up Runtime API Dependencies
- [ ] Remove axum dependency from `axiom-runtime/Cargo.toml`
- [ ] Remove reqwest dependency from `axiom-runtime/Cargo.toml` (if not needed)
- [ ] Remove `metrics` feature flag from runtime (move to API)
- [ ] Verify `cargo check --package axiom-runtime`

**Phase 4 Acceptance Criteria**:
- [ ] `axiom-runtime` no longer contains HTTP server code
- [ ] All API endpoints served by `axiom-api` only
- [ ] `axiom-runtime/Cargo.toml` has no axum/reqwest dependencies
- [ ] `cargo test --workspace` passes
- [ ] Architecture check passes: `axm verify`

---

## Risk Mitigation Matrix

| Risk | Task | Priority | Owner | Estimate | Mitigation |
|------|------|----------|-------|----------|------------|
| Trait design too broad | P1-1, P1-2 | High | Runtime | 1d | Start with minimal interface, expand as needed |
| Breaking existing frontend | P3-2 | High | API | 1d | Mirror old endpoints, add deprecation warnings |
| Runtime compilation failures | P4-3 | Medium | Runtime | 0.5d | Keep old endpoints until new ones verified |
| Authentication complexity | P3-3 | Medium | Security | 1d | Start with API key auth, add JWT later |
| Performance regression | P2-4 | Medium | API | 0.5d | Add caching, use async streaming |
| CORS configuration issues | P2-3 | Low | API | 0.5d | Make origins configurable via config |

---

## Completion Criteria

### Phase 1
- [ ] `RuntimeDataSource` trait defined in `axiom-runtime`
- [ ] `OversightDataSource` trait defined in `axiom-oversight`
- [ ] Both traits implemented and tested
- [ ] `cargo check --package axiom-runtime --package axiom-oversight` passes

### Phase 2
- [ ] `axiom-api` crate created and compiles
- [ ] All v1 API endpoints implemented
- [ ] CORS, logging, error handling middleware in place
- [ ] Unit tests for API types and aggregators pass
- [ ] `axm verify` passes

### Phase 3
- [ ] API server integrates with `AxiomRuntime`
- [ ] Old endpoints mirrored in v1 API with deprecation warnings
- [ ] Authentication middleware implemented
- [ ] Integration tests pass
- [ ] `cargo test --package axiom-runtime` passes

### Phase 4
- [ ] `axiom-runtime` contains no HTTP server code
- [ ] All API endpoints served by `axiom-api` only
- [ ] Runtime dependencies cleaned up
- [ ] `cargo test --workspace` passes
- [ ] `axm verify` passes

---

## Definition of Done (DoD)

For each task, "Done" means:
1. **Code**: Implementation complete with proper error handling
2. **Tests**: Unit tests written and passing
3. **Docs**: Documentation updated (README, code comments)
4. **Build**: `cargo check` and `cargo test` pass
5. **Architecture**: `axm verify` passes (no layer violations)

---

## Estimated Timeline

| Phase | Duration | Total |
|-------|----------|-------|
| Phase 1: Interface Abstraction | 1.5 days | 1.5d |
| Phase 2: Create axiom-api Crate | 2 days | 3.5d |
| Phase 3: Integration with Runtime | 2 days | 5.5d |
| Phase 4: Deprecate Old Endpoints | 1 day | 6.5d |

**Total Estimate**: ~6-7 days

---

## Success Metrics

| Metric | Target |
|--------|--------|
| API endpoints consolidated | 1 port (9092) instead of 2 ports |
| API versioning | v1 API with backward compatibility |
| Runtime HTTP dependencies | 0 (removed axum/reqwest) |
| Test coverage | >= 80% for axiom-api |
| Response time | < 100ms for health/cells endpoints |
| WebSocket latency | < 50ms for signal events |
