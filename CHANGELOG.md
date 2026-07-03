# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — Auto-Injection Mechanism
- **`#[signal]` macro**: Auto-generates `msg_id`, `correlation_id`, `vector_clock` fields + `Signal` trait implementation
- **`#[cell]` macro**: Auto-implements `LayerOf`, layer marker traits, and `WitnessGenerator`
- **`#[tool]` macro**: Auto-wraps `execute()` method with Witness recording and permission checks
- **`#[guard]` macro**: Auto-wraps `check()` method with Witness recording and outcome tracking
- **`#[capability]` macro**: NEW auto-registration for capability version management across 5 dimensions
- **`WITNESS_REGISTRY`**: Global witness registry for auto-injected audit records
- **Compile-time constraint injection**: No manual API calls required — constraints are baked in

### Added — Capability Version Management
- **`CapabilityDimension`**: 5 dimensions (Witness/Schema/Layer/Tool/Guard) with independent versioning
- **`CapabilityDescriptor`**: Version + compatibility + layer + migration chain association
- **`CAPABILITY_REGISTRY`**: linkme distributed slice for auto-discovery across crates
- **`CapabilityVersionRegistry`**: Auto-compatibility checking and latest version resolution
- **`Compatibility::SemVer`**: New default compatibility strategy following SemVer rules

### Changed
- `Witness` struct now includes `kind` field for categorization (StateTransition/ToolInvocation/GuardCheck)
- `AxiomError` now includes `CellPanic` variant for panic recovery tracking
- `#[signal]` macro generates complete `new()` constructor with user-defined fields

## [0.1.0] - 2026-07-03

### Added — Core Architecture
- **Cell** primitive: isolated state unit with `handle_dyn` dispatch interface
- **Signal** system: type-safe messages with `SignalEnvelope` type-erased wrapper
- **Axiom** constraint framework: compile-time + runtime constraint enforcement
- **Witness** audit chain: SHA-256 hash-linked immutable audit records
- **Layer** enforcement: 4-layer architecture (Oversight → Agent → Validate → Exec)
  - Compile-time: `CanSendTo` trait + `LayeredCellContext`
  - Runtime: `ArchitectureGuardian` bus interceptor
- **Schema versioning**: `SchemaVersion`, `VersionInfo`, migration support
- **Entropy** system: system disorder measurement and governance

### Added — Runtime
- `AxiomRuntime`: runtime orchestrator with Cell registration and dispatch
- `MessageBus`: async message bus with interceptor chain
- `Mailbox`: per-Cell bounded async queue with backpressure
- `Supervisor`: crash recovery with Restart/Stop/Escalate/CircuitBreaker strategies
- `DeadLetterQueue`: captures undeliverable messages
- Bus interceptors: HopLimit, Idempotency, SchemaVersion, ArchitectureGuardian
- Governance interceptors: Throttle, Emergency (entropy-based)
- `LoopDetector`: prevents infinite message cycles
- `submit_signal()`: external signal entry point

### Added — Persistence
- `EventStore`: append-only event log with sequence numbering
- `MemoryStore`: in-memory event store implementation
- `SnapshotStore`: state snapshots for crash recovery
- `ReplayEngine`: event replay by correlation_id, cell_id, time range

### Added — Agent Toolchain
- `axiom-llm`: LLM client abstraction with Mock, retry, token budget
- `axiom-tool`: type-safe Tool trait with permission control
- `axiom-memory`: WorkingMemory with auto-summarization and token budget
- `axiom-planner`: ReAct and Plan-and-Execute planning strategies
- `axiom-prompt`: type-safe prompt templates with composition and versioning
- `axiom-identity`: AgentIdentity, AgentPersona, Skill system with progressive disclosure
- `axiom-agent`: AgentCell facade, AgentBuilder chain, unified re-exports

### Added — CLI
- `axm new`: project scaffolding
- `axm run`: runtime launcher
- `axm top`: real-time TUI monitoring
- `axm trace`: correlation chain tracing
- `axm why`: root cause analysis
- `axm witness`: witness chain inspection
- `axm cell`: cell management (list/restart/stop)
- `axm entropy`: entropy level monitoring
- `axm init`: project initialization
- `axm verify`: architecture constraint verification

### Added — MCP Protocol
- MCP client and server implementations
- Tool bridge: MCP Tool ↔ axiom Tool mapping
- Security layer: Permission → Rules → Axiom → Human-in-the-loop

### Added — Oversight
- `ArchitectureGuardian`: layer violation detection
- `ComplianceGuard`: PII redaction and policy enforcement
- `IntentAuditor`: agent intent drift detection
- `EntropyGovernor`: system disorder monitoring with 5-level governance
- `ResourceManager`: resource quota management
- `MetaOversight`: oversight-of-oversight
- `HealthMonitor`: system health tracking

### Added — Visualization
- `axiom-viz`: timeline and topology visualization
- Entropy visualization dashboard

### Added — Benchmarks
- `axiom-bench`: criterion benchmarks for message passing, witness chain, mailbox, bus dispatch
- Stress test binary: long-running stability validation

### Added — Developer Tooling
- `axiom-macros`: `#[cell]`, `#[signal]`, `#[axiom]`, `#[schema_version]` procedural macros
- Compile-fail tests: verify architectural constraints are enforced at compile time
- CI/CD: GitHub Actions with fmt → clippy → build → test → bench → release pipeline

### Statistics
- 16 crates in workspace
- 391 tests (all passing)
- 0 clippy warnings
- 0 unwrap/expect in non-test code (safe error handling)

### Architecture Constraints
- **Layer enforcement**: Only downward or same-layer calls allowed
- **Compile-time safety**: `LayeredCellContext` prevents illegal cross-layer calls
- **Runtime safety**: `ArchitectureGuardian` rejects violations at bus level
- **Audit completeness**: Every state transition produces a Witness
- **Hash chain integrity**: Witness chain tampering is detectable
- **Constraint self-application**: Architecture components are themselves constrained
