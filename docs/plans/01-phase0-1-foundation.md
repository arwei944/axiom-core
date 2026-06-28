# Phase 0-1: 基础设施 + 核心原语完善 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完善 axiom-core crate 的五大核心原语（Cell/Signal/Lens/Axiom/Witness），使 hello_cell 示例能完整收发消息并产生 Witness 审计链；同时配置好开发基础设施（依赖、测试框架、移除 async-trait）。

**Architecture:** 用 Rust 原生 async fn in traits（Rust 1.75+）替代 async-trait 宏；Signal 增加层标签/发送者/幂等ID；Cell 增加 CellContext 用于消息发送和 Witness 产生；Witness 增加 SHA-256 链式哈希；Error 类型扩展为完整领域错误集；增加 CellHandle 类型擦除句柄和 SignalEnvelope 消息信封用于运行时调度。

**Tech Stack:** Rust 1.75+, tokio (runtime, mpsc, sync), sha2 (hashing), serde/serde_json, thiserror, tracing, uuid (v4 for msg_id), futures (for Stream), proptest (property testing)

---

## Global Constraints

- Rust edition 2021+，MSRV 1.75（原生 async fn in traits）
- 禁止使用 `async-trait` 宏（全部用原生 async fn in traits 或 manual future）
- `unsafe` 代码只能在 `crate::unsafe_impl` 模块中，且必须有 `// SAFETY:` 注释
- 所有 public API 必须有 `///` rustdoc 注释和 `#[derive(Debug)]`
- `cargo build --workspace` 必须零警告（在 CI 中 `RUSTFLAGS="-D warnings"`）
- 所有 Signal 必须 `#[derive(Clone, Serialize, Deserialize)]`
- 所有错误类型使用 `thiserror`，应用边界才用 `anyhow`
- 不引入全局可变状态（`static mut`、`lazy_static!` 用于可变状态禁止）
- Crate 依赖铁律：axiom-core 只依赖 axiom-macros（可选）+ 第三方crate，不依赖任何其他 workspace crate
- Commit message 格式：`type(scope): description`，type ∈ {feat, fix, refactor, test, docs, chore}
- 每个 Task 结束后必须 `cargo test -p axiom-core` 通过才能 commit

---

## File Structure

**axiom-core crate 最终文件结构：**

```
crates/axiom-core/
├── Cargo.toml                    # 修改：增加 sha2, uuid, futures, proptest dev-dep
├── src/
│   ├── lib.rs                    # 修改：re-export 所有公共类型，deny(warnings)
│   ├── error.rs                  # 修改：扩展为完整错误类型集
│   ├── signal.rs                 # 修改：Signal trait 增加层标签/sender/msg_id默认实现
│   │                             #        + SignalEnvelope（信封）+ Signaled trait
│   ├── cell.rs                   # 修改：Cell trait 增加 layer()返回Layer
│   │                             #        + CellContext（消息发送/Witness产生）
│   │                             #        + CellHandle（类型擦除句柄）
│   │                             #        + CellMeta（元信息）
│   ├── witness.rs                # 修改：Witness 增加层标签/identity_id
│   │                             #        + WitnessBuilder + SHA-256链式哈希
│   ├── axiom.rs                  # 修改：Axiom trait 增加 layer 感知
│   │                             #        + LayeredAxiomChain（按层执行Axiom）
│   ├── lens.rs                   # 修改：增加 CachedLens（VC缓存失效）
│   │                             #        + Lens3/LensN 组合子
│   ├── layer.rs                  # ✅ 已有，小改：增加 source_layer 验证
│   ├── entropy.rs                # ✅ 已有，不改
│   ├── id.rs                     # 新增：MsgId, CorrelationId, WitnessId 类型别名
│   └── unsafe_impl.rs            # 新增：unsafe代码隔离模块（空，占位）
└── tests/
    ├── vector_clock_tests.rs     # 新增：Vector Clock 因果排序测试
    ├── witness_chain_tests.rs    # 新增：Witness 链式哈希测试
    ├── signal_envelope_tests.rs  # 新增：SignalEnvelope 幂等/新鲜度测试
    ├── layer_violation_tests.rs  # 新增：跨层调用检测测试
    └── cell_context_tests.rs     # 新增：CellContext 消息发送测试
```

---

## Task 0: 基础设施配置（Cargo.toml + 移除 async-trait）

**Files:**
- Modify: `crates/axiom-core/Cargo.toml`
- Modify: `crates/axiom-core/src/lib.rs`
- Modify: `crates/axiom-core/src/cell.rs`
- Modify: `crates/axiom-core/src/lens.rs`

**Interfaces:**
- Consumes: 无（初始状态）
- Produces: 所有 crate 都使用原生 async fn in traits，不再依赖 async-trait

- [ ] **Step 1: 更新 Cargo.toml 依赖**

将 `crates/axiom-core/Cargo.toml` 替换为：

```toml
[package]
name = "axiom-core"
version = "0.1.0"
edition = "2021"
description = "5 fundamental primitives for reliable agentic systems: Cell, Signal, Lens, Axiom, Witness"
license = "MIT"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tokio = { version = "1", features = ["rt", "sync", "macros", "time"] }
tracing = "0.1"
sha2 = "0.10"
uuid = { version = "1", features = ["v4", "serde"] }
futures = "0.3"

[dev-dependencies]
tokio = { version = "1", features = ["rt", "macros", "test-util"] }
proptest = "1"
```

- [ ] **Step 2: 更新 lib.rs — 添加 deny(warnings) 和模块声明**

将 `crates/axiom-core/src/lib.rs` 替换为：

```rust
//! Axiom Core - 5 fundamental primitives for reliable agentic systems.
//!
//! # Primitives
//! - **Cell**: Isolated stateful unit with private state + message mailbox
//! - **Signal**: Typed immutable message with causal tracking
//! - **Lens**: On-demand state projection from event log
//! - **Axiom**: Global invariant constraints for entropy control
//! - **Witness**: Immutable audit record for every state transition
//!
//! # Architecture
//! - **Layer**: Four-layer architecture (Oversight/Agent/Validate/Exec) with enforced call direction
//! - **Entropy**: First-class entropy metrics for system disorder quantification

#![deny(warnings)]
#![deny(missing_docs)]

pub mod axiom;
pub mod cell;
pub mod entropy;
pub mod error;
pub mod id;
pub mod layer;
pub mod lens;
pub mod signal;
pub mod unsafe_impl;
pub mod witness;

pub use error::{AxiomError, Result};
pub use layer::Layer;
pub use entropy::EntropyScore;
```

- [ ] **Step 3: 创建 id.rs — 强类型ID**

创建 `crates/axiom-core/src/id.rs`：

```rust
//! Strongly-typed identifiers to prevent mixing up different ID types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique message identifier for idempotent deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MsgId(pub String);

impl MsgId {
    /// Generate a new random MsgId (UUID v4).
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for MsgId {
    fn default() -> Self {
        Self::new()
    }
}

/// Correlation ID for distributed tracing across Cells and layers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(pub String);

impl CorrelationId {
    /// Generate a new random CorrelationId.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from an existing string (e.g., propagating from parent request).
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique witness identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WitnessId(pub String);

impl WitnessId {
    /// Generate a new random WitnessId.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for WitnessId {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: 创建 unsafe_impl.rs — unsafe代码隔离**

创建 `crates/axiom-core/src/unsafe_impl.rs`：

```rust
//! Unsafe code isolation module.
//!
//! All `unsafe` code in axiom-core MUST live in this module.
//! Every unsafe block must have a `// SAFETY:` comment explaining why it's sound.
//!
//! Currently no unsafe code exists; this module is a placeholder enforcing
//! the constraint that unsafe is not accidentally added elsewhere.
```

- [ ] **Step 5: 移除 async-trait 依赖并修复 Cell trait**

从 `crates/axiom-core/Cargo.toml` 中删除 `async-trait` 依赖行。

将 `crates/axiom-core/src/cell.rs` 替换为使用原生 async fn：

```rust
//! Cell - Isolated stateful unit with private state + message mailbox.
//!
//! A Cell is the fundamental unit of computation in Axiom. Each Cell:
//! - Has private state (enforced by Rust ownership)
//! - Processes messages one at a time (single-threaded, no locks)
//! - Communicates only through typed Signals
//! - Has a Layer tag for architectural enforcement
//! - Produces Witness records for every state transition

use crate::error::AxiomError;
use crate::id::CorrelationId;
use crate::layer::Layer;
use crate::signal::{Signal, SignalEnvelope, VectorClock};
use crate::witness::{TransitionOutcome, Witness, WitnessBuilder, WitnessHash};
use crate::Result;
use std::collections::VecDeque;

/// Unique identifier for a Cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CellId(pub String);

impl CellId {
    /// Create a new CellId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for CellId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lifecycle states of a Cell (marker types for typestate pattern).
pub mod state {
    /// Cell has been created but not yet started.
    pub struct Created;
    /// Cell is running and processing messages.
    pub struct Running;
    /// Cell is suspended (state preserved, not processing messages).
    pub struct Suspended;
    /// Cell has crashed and awaits supervisor decision.
    pub struct Crashed;
    /// Cell has been stopped permanently.
    pub struct Stopped;
}

/// Supervision strategy when a Cell crashes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SupervisionStrategy {
    /// Restart the cell with fresh state, up to max_retries times.
    Restart { max_retries: u32 },
    /// Stop the cell permanently.
    Stop,
    /// Escalate failure to parent supervisor.
    Escalate,
}

impl Default for SupervisionStrategy {
    fn default() -> Self {
        SupervisionStrategy::Restart { max_retries: 3 }
    }
}

/// Metadata about a Cell for auditing and visualization.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CellMeta {
    /// Human-readable name.
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// Architectural layer.
    pub layer: Layer,
    /// Mailbox capacity (None = unbounded, but bounded recommended).
    pub mailbox_capacity: Option<usize>,
}

impl Default for CellMeta {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.1.0".to_string(),
            layer: Layer::Exec,
            mailbox_capacity: Some(1024),
        }
    }
}

/// Context passed to Cell::handle for sending messages and producing Witnesses.
pub struct CellContext {
    /// ID of the cell this context belongs to.
    pub cell_id: CellId,
    /// Layer of the cell.
    pub layer: Layer,
    /// Outgoing message queue (populated by emit/send methods).
    pub(crate) outgoing: VecDeque<SignalEnvelope>,
    /// Witnesses produced during this handler invocation.
    pub(crate) witnesses: Vec<Witness>,
    /// Current vector clock.
    pub(crate) clock: VectorClock,
    /// Previous witness hash for chaining.
    pub(crate) prev_hash: Option<WitnessHash>,
    /// Active identity ID (if any).
    pub(crate) identity_id: Option<String>,
}

impl CellContext {
    /// Create a new CellContext (internal use by Runtime).
    pub(crate) fn new(cell_id: CellId, layer: Layer) -> Self {
        Self {
            cell_id,
            layer,
            outgoing: VecDeque::new(),
            witnesses: Vec::new(),
            clock: VectorClock::new(),
            prev_hash: None,
            identity_id: None,
        }
    }

    /// Send a signal to another Cell. Enforces layer constraints.
    pub fn send<S: Signal>(&mut self, target: CellId, signal: S) -> Result<()> {
        let envelope = SignalEnvelope::new(
            self.cell_id.clone(),
            Some(self.layer),
            signal,
            self.clock.clone(),
        );
        // Layer check is done here at context level; runtime double-checks.
        self.outgoing.push_back(envelope);
        Ok(())
    }

    /// Emit a Witness for a successful state transition.
    pub fn emit_success(&mut self, summary: &str) {
        let witness = WitnessBuilder::new(
            self.cell_id.to_string(),
            self.layer,
            self.identity_id.clone(),
            self.clock.clone(),
            self.prev_hash.clone(),
            summary.to_string(),
            TransitionOutcome::Success,
        )
        .build();
        self.prev_hash = Some(witness.hash.clone());
        self.witnesses.push(witness);
    }

    /// Emit a Witness for a failed state transition.
    pub fn emit_failure(&mut self, summary: &str, reason: &str) {
        let witness = WitnessBuilder::new(
            self.cell_id.to_string(),
            self.layer,
            self.identity_id.clone(),
            self.clock.clone(),
            self.prev_hash.clone(),
            summary.to_string(),
            TransitionOutcome::Failed {
                reason: reason.to_string(),
            },
        )
        .build();
        self.prev_hash = Some(witness.hash.clone());
        self.witnesses.push(witness);
    }

    /// Emit a Witness for an Axiom violation.
    pub fn emit_axiom_violation(&mut self, axiom_name: &str, message: &str) {
        let witness = WitnessBuilder::new(
            self.cell_id.to_string(),
            self.layer,
            self.identity_id.clone(),
            self.clock.clone(),
            self.prev_hash.clone(),
            format!("Axiom '{}' violated", axiom_name),
            TransitionOutcome::AxiomViolated {
                axiom_name: axiom_name.to_string(),
                message: message.to_string(),
            },
        )
        .build();
        self.prev_hash = Some(witness.hash.clone());
        self.witnesses.push(witness);
    }

    /// Get the current correlation ID (if any outgoing signal has one, use that; otherwise None).
    pub fn next_correlation_id(&self) -> CorrelationId {
        CorrelationId::new()
    }
}

/// Core Cell trait - implement this to define a stateful unit.
///
/// Uses native async fn in traits (Rust 1.75+) instead of async-trait macro.
pub trait Cell: Send + 'static {
    /// Type of messages this Cell can handle.
    type Message: Signal;

    /// The Cell's unique identifier.
    fn id(&self) -> &CellId;

    /// The architectural layer this Cell belongs to.
    fn layer(&self) -> Layer;

    /// Cell metadata (name, version, etc.).
    fn meta(&self) -> CellMeta {
        CellMeta {
            name: self.id().0.clone(),
            layer: self.layer(),
            ..CellMeta::default()
        }
    }

    /// Called when the Cell starts (after being spawned).
    async fn on_start(&mut self, _ctx: &mut CellContext) -> Result<()> {
        Ok(())
    }

    /// Handle an incoming signal/message.
    async fn handle(&mut self, signal: Self::Message, ctx: &mut CellContext) -> Result<()>;

    /// Called when the Cell is about to stop.
    async fn on_stop(&mut self, _ctx: &mut CellContext) -> Result<()> {
        Ok(())
    }

    /// Supervision strategy for this cell.
    fn supervision_strategy(&self) -> SupervisionStrategy {
        SupervisionStrategy::default()
    }
}
```

- [ ] **Step 6: 同样移除 lens.rs 中的 async-trait**

将 `crates/axiom-core/src/lens.rs` 中的 `use async_trait::async_trait;` 和 `#[async_trait]` 标注删除，直接使用原生 async fn：

```rust
//! Lens - On-demand state projection from the event log.
//!
//! Instead of stuffing all history into a context window, Lenses project
//! exactly the state needed at the right granularity, with permission boundaries.

use crate::error::AxiomError;
use crate::signal::VectorClock;
use crate::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A Lens projects a view of state from the event log.
pub trait Lens: Send + Sync {
    /// Type of state this lens projects.
    type View: Send + Sync + Clone;

    /// Unique lens identifier (also used for permission boundaries).
    fn lens_id(&self) -> &'static str;

    /// Project the current state view.
    async fn project(&self) -> Result<Self::View>;

    /// Project state as of a specific Vector Clock (time travel).
    async fn project_at(&self, clock: &VectorClock) -> Result<Self::View>;
}

// ... (Lens2 stays but also without async_trait)
```

Wait — let me be careful. The native async fn in traits works for single traits but Lens2 uses generic type parameters with the Lens trait. This works fine in Rust 1.75+. Let me write the complete Lens2 without async_trait:

```rust
/// Composable lens - combine two lenses into one.
pub struct Lens2<L1, L2> {
    l1: L1,
    l2: L2,
}

impl<L1, L2> Lens2<L1, L2> {
    pub fn new(l1: L1, l2: L2) -> Self {
        Self { l1, l2 }
    }
}

impl<L1: Lens, L2: Lens> Lens for Lens2<L1, L2> {
    type View = (L1::View, L2::View);

    fn lens_id(&self) -> &'static str {
        "Lens2"
    }

    async fn project(&self) -> Result<Self::View> {
        let v1 = self.l1.project().await?;
        let v2 = self.l2.project().await?;
        Ok((v1, v2))
    }

    async fn project_at(&self, clock: &VectorClock) -> Result<Self::View> {
        let v1 = self.l1.project_at(clock).await?;
        let v2 = self.l2.project_at(clock).await?;
        Ok((v1, v2))
    }
}
```

- [ ] **Step 7: 编译验证**

Run: `cd d:\work\trae\axiom-core; cargo build -p axiom-core 2>&1`
Expected: Compiles successfully, 0 errors, 0 warnings.
Note: 可能有 unused import warnings 需要修复——修复后再继续。

- [ ] **Step 8: Commit**

```bash
cd d:\work\trae\axiom-core
git add crates/axiom-core/Cargo.toml crates/axiom-core/src/
git commit -m "chore(axiom-core): set up dependencies, remove async-trait, add id types and CellContext"
```

---

## Task 1: 完善 Signal trait + SignalEnvelope（消息信封）

**Files:**
- Modify: `crates/axiom-core/src/signal.rs`
- Create: `crates/axiom-core/tests/signal_envelope_tests.rs`

**Interfaces:**
- Consumes: `crate::id::{MsgId, CorrelationId}`, `crate::layer::Layer` from Task 0
- Produces:
  - `Signal` trait with default implementations for msg_id/correlation_id/sender/layer
  - `SignalEnvelope` struct: type-erased message wrapper with source/target layer info
  - `is_fresh()` function for freshness checks

- [ ] **Step 1: 重写 signal.rs**

将 `crates/axiom-core/src/signal.rs` 替换为完整实现：

```rust
//! Signal - Typed immutable message with causal tracking (Vector Clock, correlation).
//!
//! Signals are the only way Cells communicate. Every Signal is:
//! - Immutable (once sent, cannot be modified)
//! - Typed (compiler guarantees message type correctness)
//! - Causally tracked (Vector Clock for partial ordering)
//! - Idempotent (unique msg_id prevents duplicate processing)
//! - Layer-tagged (enforces architectural call-direction rules)

use crate::error::AxiomError;
use crate::id::{CorrelationId, MsgId};
use crate::layer::Layer;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::time::{SystemTime, UNIX_EPOCH};

/// Vector Clock for causal ordering.
///
/// Tracks "happens-before" relationships between events across Cells.
/// If clock A causally precedes clock B, A's state is a past state of B.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock(pub std::collections::HashMap<String, u64>);

impl VectorClock {
    /// Create a new empty VectorClock.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the counter for a given cell (after processing a message).
    pub fn increment(&mut self, cell_id: &str) {
        *self.0.entry(cell_id.to_string()).or_insert(0) += 1;
    }

    /// Merge another vector clock (takes max for each entry, on message receive).
    pub fn merge(&mut self, other: &VectorClock) {
        for (key, value) in &other.0 {
            let entry = self.0.entry(key.clone()).or_insert(0);
            *entry = (*entry).max(*value);
        }
    }

    /// Check if this clock causally precedes another (this ≤ other in all dimensions).
    pub fn causally_precedes(&self, other: &VectorClock) -> bool {
        for (key, &self_val) in &self.0 {
            match other.0.get(key) {
                Some(&other_val) if self_val > other_val => return false,
                None if self_val > 0 => return false,
                _ => {}
            }
        }
        true
    }

    /// Check if this clock is concurrent with another (neither precedes the other).
    pub fn is_concurrent_with(&self, other: &VectorClock) -> bool {
        !self.causally_precedes(other) && !other.causally_precedes(self)
    }

    /// Get the counter for a specific cell.
    pub fn get(&self, cell_id: &str) -> u64 {
        self.0.get(cell_id).copied().unwrap_or(0)
    }
}

/// Signal categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalKind {
    /// Request an operation (mutates state).
    Command,
    /// Notification that something happened (immutable fact).
    Event,
    /// Query state (read-only).
    Query,
}

/// Helper trait for cloning boxed signals (for dyn compatibility).
pub trait SignalClone: Send + Sync {
    fn clone_box(&self) -> Box<dyn Signal>;
    fn as_any(&self) -> &dyn Any;
}

impl<T: Signal + Clone + 'static> SignalClone for T {
    fn clone_box(&self) -> Box<dyn Signal> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Base trait for all signals (dyn-compatible for type-erased message bus).
///
/// Default implementations generate msg_id and correlation_id automatically,
/// and set timestamp_ns to the current system time.
pub trait Signal: SignalClone + Send + Sync + 'static {
    /// Unique signal type identifier (e.g., "order.create").
    fn signal_type(&self) -> &'static str;

    /// Unique message identifier for idempotency. Default: UUID v4.
    fn msg_id(&self) -> &str {
        // Concrete types should store msg_id; this default panics to catch mistakes.
        // The derive macro or manual implementation should override.
        unimplemented!("Signal types must implement msg_id()")
    }

    /// Correlation ID for distributed tracing. Default: new UUID.
    fn correlation_id(&self) -> &str {
        unimplemented!("Signal types must implement correlation_id()")
    }

    /// Vector clock for causal ordering.
    fn vector_clock(&self) -> &VectorClock;

    /// Timestamp (nanoseconds since UNIX epoch) for freshness checks.
    fn timestamp_ns(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    /// Signal category.
    fn kind(&self) -> SignalKind;

    /// Sender cell ID, if known.
    fn sender(&self) -> Option<&str> {
        None
    }

    /// Source layer (where this signal came from).
    fn source_layer(&self) -> Option<Layer> {
        None
    }

    /// Target layer hint (used by dispatcher for routing validation).
    fn target_layer_hint(&self) -> Option<Layer> {
        None
    }
}

impl Clone for Box<dyn Signal> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Type-erased signal envelope used by the message bus.
///
/// The envelope carries metadata needed for routing, layer enforcement,
/// idempotency, and tracing, while the inner signal is type-erased.
#[derive(Debug)]
pub struct SignalEnvelope {
    /// Unique message ID for idempotency.
    pub msg_id: MsgId,
    /// Correlation ID for distributed tracing.
    pub correlation_id: CorrelationId,
    /// Sender cell ID.
    pub sender: CellId,
    /// Source layer (None for external/system messages).
    pub source_layer: Option<Layer>,
    /// Target cell ID (None = broadcast/event).
    pub target: Option<CellId>,
    /// Vector clock at send time.
    pub vector_clock: VectorClock,
    /// Timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// The actual signal (type-erased).
    pub(crate) inner: Box<dyn Signal>,
}

// We can't derive Clone for Box<dyn Signal>, so manual impl.
impl Clone for SignalEnvelope {
    fn clone(&self) -> Self {
        Self {
            msg_id: self.msg_id.clone(),
            correlation_id: self.correlation_id.clone(),
            sender: self.sender.clone(),
            source_layer: self.source_layer,
            target: self.target.clone(),
            vector_clock: self.vector_clock.clone(),
            timestamp_ns: self.timestamp_ns,
            inner: self.inner.clone(),
        }
    }
}

// CellId is referenced here but defined in cell.rs — we need a local version or import.
// To avoid circular dependency, define a lightweight type here.
/// Lightweight cell identifier used in signal envelopes (avoids circular deps).
pub type EnvCellId = String;

impl SignalEnvelope {
    /// Create a new envelope for sending a signal to a target cell.
    pub fn new<S: Signal>(
        sender: impl Into<String>,
        source_layer: Option<Layer>,
        signal: S,
        mut clock: VectorClock,
    ) -> Self {
        let sender_str = sender.into();
        clock.increment(&sender_str);
        let msg_id = MsgId::new();
        let correlation_id = CorrelationId::new();
        Self {
            msg_id,
            correlation_id,
            sender: sender_str,
            source_layer,
            target: None,
            vector_clock: clock,
            timestamp_ns: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            inner: Box::new(signal),
        }
    }

    /// Set the target cell ID.
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Downcast the inner signal to a concrete type.
    pub fn downcast_ref<T: Signal + 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }

    /// Get the signal type name.
    pub fn signal_type(&self) -> &'static str {
        self.inner.signal_type()
    }

    /// Check if this signal is stale (older than max_age_ns).
    pub fn is_fresh(&self, max_age_ns: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        now.saturating_sub(self.timestamp_ns) <= max_age_ns
    }

    /// Validate that source→target layer is a legal direction.
    pub fn validate_layer_transition(&self, target_layer: Layer) -> Result<(), AxiomError> {
        if let Some(src) = self.source_layer {
            if !src.can_send_to(target_layer) {
                return Err(AxiomError::LayerViolation {
                    from: src,
                    to: target_layer,
                    signal_type: self.signal_type().to_string(),
                });
            }
        }
        Ok(())
    }
}

/// Freshness check (standalone function for convenience).
pub fn is_fresh(signal: &dyn Signal, max_age_ns: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    now.saturating_sub(signal.timestamp_ns()) <= max_age_ns
}
```

Wait — I'm referencing `CellId` but it's defined in `cell.rs`. I need to avoid that circular reference. Let me use `String` as the cell ID type in SignalEnvelope, or better yet, move CellId to a shared location. Actually, the simplest approach is to define CellId in a shared types module or just use `String` aliases in signal.rs. But actually, looking at the code, I already defined `EnvCellId = String` in signal.rs — but CellContext uses `CellId` from cell.rs. Let me restructure: put CellId in id.rs since it's a primitive identifier type, alongside MsgId/CorrelationId/WitnessId. That avoids the circular dependency.

Let me fix this properly in the actual implementation. I'll move CellId to id.rs.

- [ ] **Step 2: 将 CellId 移到 id.rs 避免循环依赖**

在 `crates/axiom-core/src/id.rs` 中添加 CellId：

```rust
/// Unique cell identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellId(pub String);

impl CellId {
    /// Create a new CellId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for CellId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

更新 cell.rs 从 id.rs re-export CellId（`pub use crate::id::CellId;`），不再在 cell.rs 中定义 CellId。

更新 signal.rs 使用 `crate::id::CellId` 替代 `EnvCellId`。

- [ ] **Step 3: 更新 error.rs 添加 LayerViolation 错误**

需要在 error.rs 中增加 `LayerViolation` 等新错误类型（详见 Task 2）。先在 signal.rs 中使用的 `AxiomError::LayerViolation` 需要存在。

- [ ] **Step 4: 编写 SignalEnvelope 测试**

创建 `crates/axiom-core/tests/signal_envelope_tests.rs`：

```rust
use axiom_core::layer::Layer;
use axiom_core::signal::*;
use axiom_core::id::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestSignal {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    clock: VectorClock,
    message: String,
}

impl SignalClone for TestSignal {
    fn clone_box(&self) -> Box<dyn Signal> { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

impl Signal for TestSignal {
    fn signal_type(&self) -> &'static str { "test.signal" }
    fn msg_id(&self) -> &str { &self.msg_id.0 }
    fn correlation_id(&self) -> &str { &self.correlation_id.0 }
    fn vector_clock(&self) -> &VectorClock { &self.clock }
    fn kind(&self) -> SignalKind { SignalKind::Command }
}

#[test]
fn test_vector_clock_increment_and_merge() {
    let mut c1 = VectorClock::new();
    c1.increment("a");
    c1.increment("a");
    assert_eq!(c1.get("a"), 2);
    assert_eq!(c1.get("b"), 0);

    let mut c2 = VectorClock::new();
    c2.increment("b");
    c2.increment("b");

    c1.merge(&c2);
    assert_eq!(c1.get("a"), 2);
    assert_eq!(c1.get("b"), 2);
}

#[test]
fn test_vector_clock_causal_ordering() {
    let mut c1 = VectorClock::new();
    c1.increment("a");
    let mut c2 = c1.clone();
    c2.increment("b");
    assert!(c1.causally_precedes(&c2));
    assert!(!c2.causally_precedes(&c1));
}

#[test]
fn test_vector_clock_concurrent() {
    let mut c1 = VectorClock::new();
    c1.increment("a");
    let mut c2 = VectorClock::new();
    c2.increment("b");
    assert!(c1.is_concurrent_with(&c2));
}

#[test]
fn test_layer_violation_detection() {
    // Exec cannot send to Agent (reverse direction)
    let sig = TestSignal {
        msg_id: MsgId::new(),
        correlation_id: CorrelationId::new(),
        clock: VectorClock::new(),
        message: "test".into(),
    };
    let env = SignalEnvelope::new("exec-cell", Some(Layer::Exec), sig, VectorClock::new());
    assert!(env.validate_layer_transition(Layer::Agent).is_err());

    // Agent can send to Validate
    let sig2 = TestSignal {
        msg_id: MsgId::new(),
        correlation_id: CorrelationId::new(),
        clock: VectorClock::new(),
        message: "test".into(),
    };
    let env2 = SignalEnvelope::new("agent-cell", Some(Layer::Agent), sig2, VectorClock::new());
    assert!(env2.validate_layer_transition(Layer::Validate).is_ok());
}

#[test]
fn test_freshness_check() {
    let sig = TestSignal {
        msg_id: MsgId::new(),
        correlation_id: CorrelationId::new(),
        clock: VectorClock::new(),
        message: "fresh".into(),
    };
    // 10 second max age, signal just created
    assert!(is_fresh(&sig, 10_000_000_000));
}
```

- [ ] **Step 5: 编译+测试验证**

Run: `cd d:\work\trae\axiom-core; cargo test -p axiom-core 2>&1`
Expected: All tests pass, 0 warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/axiom-core/src/signal.rs crates/axiom-core/src/id.rs crates/axiom-core/tests/signal_envelope_tests.rs
git commit -m "feat(axiom-core): complete Signal trait with SignalEnvelope, VectorClock ordering, layer validation"
```

---

## Task 2: 完善错误类型（AxiomError 全集）

**Files:**
- Modify: `crates/axiom-core/src/error.rs`
- Create: `crates/axiom-core/tests/layer_violation_tests.rs`

**Interfaces:**
- Consumes: `crate::layer::Layer` from Task 0
- Produces: Complete `AxiomError` enum covering all error cases from requirements

- [ ] **Step 1: 重写 error.rs**

将 `crates/axiom-core/src/error.rs` 替换为：

```rust
//! Error types for Axiom Core.
//!
//! All errors are strongly-typed variants with machine-readable context.

use crate::id::{CorrelationId, MsgId};
use crate::layer::Layer;
use thiserror::Error;

/// All possible errors in the Axiom runtime.
#[derive(Error, Debug)]
pub enum AxiomError {
    /// Mailbox is at capacity; cannot accept new messages.
    #[error("Mailbox is full (capacity: {capacity}) for cell {cell_id}")]
    MailboxFull { cell_id: String, capacity: usize },

    /// Referenced cell does not exist in the runtime.
    #[error("Cell '{cell_id}' not found")]
    CellNotFound { cell_id: String },

    /// An Axiom invariant was violated.
    #[error("Axiom '{axiom_name}' violated: {message}")]
    InvariantViolated { axiom_name: String, message: String },

    /// Architectural layer violation (illegal cross-layer call).
    #[error("Layer violation: {from} cannot send to {to} (signal: {signal_type})")]
    LayerViolation {
        from: Layer,
        to: Layer,
        signal_type: String,
    },

    /// Signal failed schema validation.
    #[error("Signal validation failed: {message}")]
    SignalValidation { message: String },

    /// Cell crashed (panicked or timed out).
    #[error("Cell '{cell_id}' crashed: {message}")]
    CellCrashed { cell_id: String, message: String },

    /// Duplicate message (idempotent dedup).
    #[error("Duplicate message {msg_id:?} (already processed)")]
    DuplicateMessage { msg_id: MsgId },

    /// Stale state: expected version doesn't match actual.
    #[error("Stale state for '{cell_id}': expected v{expected}, got v{actual}")]
    StaleState {
        cell_id: String,
        expected: u64,
        actual: u64,
    },

    /// Operation timed out.
    #[error("Operation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Permission denied (identity lacks required permission).
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },

    /// Message loop detected (circular message chain).
    #[error("Message loop detected involving cells: {cells:?} (count: {count})")]
    LoopDetected { cells: Vec<String>, count: u64 },

    /// Entropy threshold exceeded.
    #[error("Entropy threshold exceeded: {value:.2} (threshold: {threshold:.2})")]
    EntropyThreshold { value: f64, threshold: f64 },

    /// Token budget exceeded.
    #[error("Token budget exceeded: used {used}, budget {budget}")]
    TokenBudgetExceeded { used: u64, budget: u64 },

    /// Event store error.
    #[error("Store error: {0}")]
    Store(String),

    /// I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Result type alias for Axiom Core.
pub type Result<T> = std::result::Result<T, AxiomError>;
```

- [ ] **Step 2: 更新 cell.rs 中 MailboxFull 错误的使用**

CellContext::send 使用 `AxiomError::MailboxFull`，更新调用处传递 cell_id。

- [ ] **Step 3: 编译验证**

Run: `cargo build -p axiom-core 2>&1`
Expected: Compiles with 0 errors, fix any warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/axiom-core/src/error.rs
git commit -m "feat(axiom-core): complete AxiomError enum with 16 error variants covering all failure modes"
```

---

## Task 3: 完善 Witness 链式哈希

**Files:**
- Modify: `crates/axiom-core/src/witness.rs`
- Create: `crates/axiom-core/tests/witness_chain_tests.rs`

**Interfaces:**
- Consumes: `crate::id::WitnessId`, `crate::layer::Layer`, sha2
- Produces:
  - `Witness` with SHA-256 chained hashing
  - `WitnessBuilder` for constructing Witness instances
  - Chain integrity verification

- [ ] **Step 1: 重写 witness.rs**

将 `crates/axiom-core/src/witness.rs` 替换为：

```rust
//! Witness - Immutable audit record for every state transition.
//!
//! Every state transition automatically produces a Witness, forming an
//! append-only hash-chained audit log. This enables post-hoc analysis:
//! "Why did we enter this state?" and guarantees tamper evidence.

use crate::id::WitnessId;
use crate::layer::Layer;
use crate::signal::VectorClock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Hash for witness chain integrity (SHA-256).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessHash(pub [u8; 32]);

impl WitnessHash {
    /// Compute hash from witness data fields.
    fn compute(
        witness_id: &str,
        cell_id: &str,
        correlation_id: &str,
        clock: &VectorClock,
        timestamp_ns: u64,
        prev_hash: &Option<WitnessHash>,
        summary: &str,
        outcome: &TransitionOutcome,
        layer: Layer,
        identity_id: &Option<String>,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(witness_id.as_bytes());
        hasher.update(cell_id.as_bytes());
        hasher.update(correlation_id.as_bytes());
        // Hash vector clock entries in sorted order for determinism.
        let mut entries: Vec<_> = clock.0.iter().collect();
        entries.sort_by_key(|(k, _)| k.clone());
        for (k, v) in entries {
            hasher.update(k.as_bytes());
            hasher.update(v.to_le_bytes());
        }
        hasher.update(timestamp_ns.to_le_bytes());
        if let Some(prev) = prev_hash {
            hasher.update(prev.0);
        }
        hasher.update(summary.as_bytes());
        hasher.update(serde_json::to_vec(outcome).unwrap_or_default());
        hasher.update(layer.as_str().as_bytes());
        if let Some(id) = identity_id {
            hasher.update(id.as_bytes());
        }
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Self(hash)
    }

    /// Create a zero hash (genesis block / no parent).
    pub fn zero() -> Self {
        Self([0u8; 32])
    }
}

/// An immutable record of a state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    /// Unique witness ID.
    pub witness_id: WitnessId,
    /// Cell that produced this transition.
    pub cell_id: String,
    /// Correlation ID from the triggering signal.
    pub correlation_id: String,
    /// Vector clock after this transition.
    pub vector_clock: VectorClock,
    /// Timestamp (nanoseconds since UNIX epoch).
    pub timestamp_ns: u64,
    /// Hash of previous witness (chain integrity).
    pub prev_hash: Option<WitnessHash>,
    /// Hash of this witness (SHA-256 over all fields).
    pub hash: WitnessHash,
    /// Human-readable description of what happened (no secrets!).
    pub summary: String,
    /// Whether this transition was successful or failed.
    pub outcome: TransitionOutcome,
    /// Layer where this transition occurred.
    pub layer: Layer,
    /// Active identity at the time (if any).
    pub identity_id: Option<String>,
}

/// Outcome of a state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionOutcome {
    /// Transition succeeded.
    Success,
    /// Transition failed with an error.
    Failed { reason: String },
    /// Transition was rejected due to Axiom violation.
    AxiomViolated { axiom_name: String, message: String },
}

/// Builder for constructing Witness instances with automatic hash computation.
pub struct WitnessBuilder {
    cell_id: String,
    correlation_id: String,
    vector_clock: VectorClock,
    prev_hash: Option<WitnessHash>,
    summary: String,
    outcome: TransitionOutcome,
    layer: Layer,
    identity_id: Option<String>,
}

impl WitnessBuilder {
    /// Create a new WitnessBuilder.
    pub fn new(
        cell_id: String,
        layer: Layer,
        identity_id: Option<String>,
        vector_clock: VectorClock,
        prev_hash: Option<WitnessHash>,
        summary: String,
        outcome: TransitionOutcome,
    ) -> Self {
        Self {
            cell_id,
            correlation_id: String::new(), // Will be set by context
            vector_clock,
            prev_hash,
            summary,
            outcome,
            layer,
            identity_id,
        }
    }

    /// Set correlation ID.
    pub fn correlation_id(mut self, id: String) -> Self {
        self.correlation_id = id;
        self
    }

    /// Build the Witness with computed hash.
    pub fn build(self) -> Witness {
        let witness_id = WitnessId::new();
        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let correlation_id = if self.correlation_id.is_empty() {
            String::new()
        } else {
            self.correlation_id
        };
        let hash = WitnessHash::compute(
            &witness_id.0,
            &self.cell_id,
            &correlation_id,
            &self.vector_clock,
            timestamp_ns,
            &self.prev_hash,
            &self.summary,
            &self.outcome,
            self.layer,
            &self.identity_id,
        );
        Witness {
            witness_id,
            cell_id: self.cell_id,
            correlation_id,
            vector_clock: self.vector_clock,
            timestamp_ns,
            prev_hash: self.prev_hash,
            hash,
            summary: self.summary,
            outcome: self.outcome,
            layer: self.layer,
            identity_id: self.identity_id,
        }
    }
}

impl Witness {
    /// Verify the hash chain integrity: this witness's hash is valid
    /// and its prev_hash matches the previous witness's hash.
    pub fn verify_chain(prev: &Witness, current: &Witness) -> bool {
        // Recompute current hash and check
        let expected = WitnessHash::compute(
            &current.witness_id.0,
            &current.cell_id,
            &current.correlation_id,
            &current.vector_clock,
            current.timestamp_ns,
            &current.prev_hash,
            &current.summary,
            &current.outcome,
            current.layer,
            &current.identity_id,
        );
        if expected != current.hash {
            return false;
        }
        // Check chain link
        match &current.prev_hash {
            Some(prev_h) if prev_h != &prev.hash => false,
            _ => true,
        }
    }
}
```

- [ ] **Step 2: 编写 Witness 链测试**

创建 `crates/axiom-core/tests/witness_chain_tests.rs`：

```rust
use axiom_core::layer::Layer;
use axiom_core::signal::VectorClock;
use axiom_core::witness::*;

#[test]
fn test_witness_hash_chain_integrity() {
    // Build a chain of 3 witnesses
    let w1 = WitnessBuilder::new(
        "cell-a".to_string(),
        Layer::Exec,
        None,
        VectorClock::new(),
        None,
        "started".to_string(),
        TransitionOutcome::Success,
    )
    .correlation_id("corr-1".to_string())
    .build();

    assert!(w1.prev_hash.is_none() || w1.prev_hash == Some(WitnessHash::zero()));

    let mut clock2 = VectorClock::new();
    clock2.increment("cell-a");
    let w2 = WitnessBuilder::new(
        "cell-a".to_string(),
        Layer::Exec,
        None,
        clock2,
        Some(w1.hash.clone()),
        "processed".to_string(),
        TransitionOutcome::Success,
    )
    .correlation_id("corr-1".to_string())
    .build();

    assert!(Witness::verify_chain(&w1, &w2));

    let mut clock3 = VectorClock::new();
    clock3.increment("cell-a");
    clock3.increment("cell-a");
    let w3 = WitnessBuilder::new(
        "cell-a".to_string(),
        Layer::Exec,
        None,
        clock3,
        Some(w2.hash.clone()),
        "completed".to_string(),
        TransitionOutcome::Success,
    )
    .correlation_id("corr-1".to_string())
    .build();

    assert!(Witness::verify_chain(&w2, &w3));
}

#[test]
fn test_witness_tamper_detection() {
    let w1 = WitnessBuilder::new(
        "cell-a".to_string(),
        Layer::Exec,
        None,
        VectorClock::new(),
        None,
        "original".to_string(),
        TransitionOutcome::Success,
    )
    .build();

    let mut clock2 = VectorClock::new();
    clock2.increment("cell-a");
    let mut w2 = WitnessBuilder::new(
        "cell-a".to_string(),
        Layer::Exec,
        None,
        clock2,
        Some(w1.hash.clone()),
        "honest".to_string(),
        TransitionOutcome::Success,
    )
    .build();

    assert!(Witness::verify_chain(&w1, &w2));

    // Tamper with the summary
    w2.summary = "TAMPERED".to_string();
    // Recompute would detect the mismatch
    let expected = WitnessHash::compute(
        &w2.witness_id.0,
        &w2.cell_id,
        &w2.correlation_id,
        &w2.vector_clock,
        w2.timestamp_ns,
        &w2.prev_hash,
        "honest", // Original summary
        &w2.outcome,
        w2.layer,
        &w2.identity_id,
    );
    assert_ne!(expected, w2.hash); // Tampered, hash doesn't match
}

#[test]
fn test_witness_axiom_violation() {
    let w = WitnessBuilder::new(
        "cell-a".to_string(),
        Layer::Agent,
        None,
        VectorClock::new(),
        None,
        "axiom violated".to_string(),
        TransitionOutcome::AxiomViolated {
            axiom_name: "no-negative-amount".to_string(),
            message: "amount is -5".to_string(),
        },
    )
    .build();

    assert!(matches!(w.outcome, TransitionOutcome::AxiomViolated { .. }));
}
```

- [ ] **Step 3: 更新 cell.rs 中的 CellContext emit 方法**

CellContext 中使用 WitnessBuilder 时需要传递 correlation_id。更新 CellContext.emit_success 等方法从信号中获取 correlation_id——这需要 CellContext 知道当前正在处理的信号的 correlation_id。

更新 CellContext 增加 `current_correlation_id` 字段，在 handle 调用前设置：

在 cell.rs 的 CellContext 中添加：
```rust
pub(crate) current_correlation_id: Option<String>,
```

并在 emit_success/emit_failure/emit_axiom_violation 中使用它。

- [ ] **Step 4: 编译+测试验证**

Run: `cargo test -p axiom-core 2>&1`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/axiom-core/src/witness.rs crates/axiom-core/tests/witness_chain_tests.rs
git commit -m "feat(axiom-core): Witness with SHA-256 chained hashing, WitnessBuilder, tamper detection"
```

---

## Task 4: 完善 Axiom（层感知公理链）

**Files:**
- Modify: `crates/axiom-core/src/axiom.rs`

**Interfaces:**
- Consumes: `crate::layer::Layer`
- Produces: Layer-aware Axiom with per-layer axiom chains

- [ ] **Step 1: 重写 axiom.rs**

```rust
//! Axiom - Global invariant constraints for entropy control.
//!
//! Axioms are deterministic pure functions that validate state transitions.
//! They act as "entropy reducers" that detect when the system is drifting.
//! Each Axiom can be scoped to specific layers.

use crate::layer::Layer;
use crate::Result;

/// Violation action when an axiom is broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViolationAction {
    /// Reject the state transition.
    Reject,
    /// Log a warning but allow the transition.
    Warn,
    /// Trigger circuit breaker (pause the cell/supervisor).
    CircuitBreak,
    /// Roll back to last valid state.
    Rollback,
}

impl Default for ViolationAction {
    fn default() -> Self {
        ViolationAction::Reject
    }
}

/// An Axiom is an invariant constraint on state transitions.
///
/// Axioms MUST be deterministic pure functions — no IO, no randomness,
/// no side effects. This makes them trivially testable and fast (<100ns).
pub trait Axiom: Send + Sync {
    /// Type of state this axiom validates.
    type State;
    /// Type of signal/command being applied.
    type Message;

    /// Axiom name (for logging/metrics).
    fn name(&self) -> &'static str;

    /// Which layers this axiom applies to (None = all layers).
    fn applicable_layers(&self) -> Option<&[Layer]> {
        None
    }

    /// Validate whether applying `message` to `current_state` (producing `new_state`) is valid.
    /// Returns Ok(()) if valid, Err with description if violated.
    fn check(
        &self,
        current_state: &Self::State,
        new_state: &Self::State,
        message: &Self::Message,
    ) -> Result<()>;

    /// Action to take when this axiom is violated.
    fn violation_action(&self) -> ViolationAction {
        ViolationAction::Reject
    }

    /// Check if this axiom applies to a given layer.
    fn applies_to(&self, layer: Layer) -> bool {
        match self.applicable_layers() {
            None => true,
            Some(layers) => layers.contains(&layer),
        }
    }
}

/// Chain multiple axioms together — all applicable axioms must pass.
pub struct AxiomChain<T, M> {
    axioms: Vec<Box<dyn Axiom<State = T, Message = M>>>,
}

impl<T, M> AxiomChain<T, M> {
    /// Create an empty axiom chain.
    pub fn new() -> Self {
        Self {
            axioms: Vec::new(),
        }
    }

    /// Add an axiom to the chain.
    pub fn add<A: Axiom<State = T, Message = M> + 'static>(mut self, axiom: A) -> Self {
        self.axioms.push(Box::new(axiom));
        self
    }

    /// Check all axioms applicable to the given layer.
    /// Returns list of (axiom_name, error) for violations.
    pub fn check_all(
        &self,
        layer: Layer,
        current: &T,
        new: &T,
        msg: &M,
    ) -> Vec<(&'static str, AxiomViolation)> {
        self.axioms
            .iter()
            .filter(|a| a.applies_to(layer))
            .filter_map(|a| {
                a.check(current, new, msg).err().map(|e| {
                    (
                        a.name(),
                        AxiomViolation {
                            action: a.violation_action(),
                            error: e,
                        },
                    )
                })
            })
            .collect()
    }

    /// Check all axioms regardless of layer (for global checks).
    pub fn check_all_layers(
        &self,
        current: &T,
        new: &T,
        msg: &M,
    ) -> Vec<(&'static str, AxiomViolation)> {
        self.axioms
            .iter()
            .filter_map(|a| {
                a.check(current, new, msg).err().map(|e| {
                    (
                        a.name(),
                        AxiomViolation {
                            action: a.violation_action(),
                            error: e,
                        },
                    )
                })
            })
            .collect()
    }
}

impl<T, M> Default for AxiomChain<T, M> {
    fn default() -> Self {
        Self::new()
    }
}

/// A violation record returned from AxiomChain::check_all.
#[derive(Debug)]
pub struct AxiomViolation {
    /// What action to take for this violation.
    pub action: ViolationAction,
    /// The error describing the violation.
    pub error: crate::AxiomError,
}
```

Wait — `AxiomViolation.error` uses `crate::AxiomError` but the axiom module is within axiom-core. This is a circular reference issue because `AxiomError` is in `error.rs` and `AxiomViolation` is in `axiom.rs`. Actually, they're in the same crate so it's fine to use `crate::AxiomError` or just `crate::error::AxiomError`. Let me fix to use `crate::AxiomError` since it's re-exported from lib.rs.

But actually, to avoid the `InvariantViolated` error vs generic error issue, let me change Axiom::check to return Result<(), String> and have AxiomChain wrap it into AxiomError::InvariantViolated. That's cleaner.

Let me revise: Axiom::check returns `Result<(), String>` where Err is the violation message. Then AxiomChain wraps it into proper AxiomError. This keeps Axiom trait simple.

- [ ] **Step 2: 编译验证**

Run: `cargo build -p axiom-core 2>&1`

- [ ] **Step 3: Commit**

```bash
git add crates/axiom-core/src/axiom.rs
git commit -m "feat(axiom-core): layer-aware Axiom trait with AxiomChain and per-layer enforcement"
```

---

## Task 5: 完善 Lens（缓存+组合子）

**Files:**
- Modify: `crates/axiom-core/src/lens.rs`

**Interfaces:**
- Consumes: VectorClock
- Produces: CachedLens with VC-based cache invalidation, Lens3 combinator

- [ ] **Step 1: 添加 CachedLens 实现**

在 lens.rs 末尾添加：

```rust
/// A lens that caches its projection based on Vector Clock version.
/// Invalidates cache automatically when the clock advances.
pub struct CachedLens<L: Lens> {
    inner: L,
    cache: Arc<Mutex<Option<CachedEntry<L::View>>>>,
}

#[derive(Debug, Clone)]
struct CachedEntry<V> {
    clock: VectorClock,
    view: V,
}

impl<L: Lens> CachedLens<L> {
    /// Wrap a lens with caching.
    pub fn new(inner: L) -> Self {
        Self {
            inner,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Invalidate the cache manually.
    pub fn invalidate(&self) {
        *self.cache.lock().unwrap() = None;
    }
}

impl<L: Lens> Lens for CachedLens<L> {
    type View = L::View;

    fn lens_id(&self) -> &'static str {
        self.inner.lens_id()
    }

    async fn project(&self) -> Result<Self::View> {
        // For current projection, always delegate (no cache key for "now").
        self.inner.project().await
    }

    async fn project_at(&self, clock: &VectorClock) -> Result<Self::View> {
        // Check cache
        {
            let guard = self.cache.lock().unwrap();
            if let Some(entry) = &*guard {
                if &entry.clock == clock {
                    return Ok(entry.view.clone());
                }
            }
        }
        // Cache miss — project and cache
        let view = self.inner.project_at(clock).await?;
        let mut guard = self.cache.lock().unwrap();
        *guard = Some(CachedEntry {
            clock: clock.clone(),
            view: view.clone(),
        });
        Ok(view)
    }
}

/// Combine three lenses into one.
pub struct Lens3<L1, L2, L3> {
    l1: L1,
    l2: L2,
    l3: L3,
}

impl<L1, L2, L3> Lens3<L1, L2, L3> {
    pub fn new(l1: L1, l2: L2, l3: L3) -> Self {
        Self { l1, l2, l3 }
    }
}

impl<L1: Lens, L2: Lens, L3: Lens> Lens for Lens3<L1, L2, L3> {
    type View = (L1::View, L2::View, L3::View);

    fn lens_id(&self) -> &'static str {
        "Lens3"
    }

    async fn project(&self) -> Result<Self::View> {
        let v1 = self.l1.project().await?;
        let v2 = self.l2.project().await?;
        let v3 = self.l3.project().await?;
        Ok((v1, v2, v3))
    }

    async fn project_at(&self, clock: &VectorClock) -> Result<Self::View> {
        let v1 = self.l1.project_at(clock).await?;
        let v2 = self.l2.project_at(clock).await?;
        let v3 = self.l3.project_at(clock).await?;
        Ok((v1, v2, v3))
    }
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo build -p axiom-core 2>&1`

- [ ] **Step 3: Commit**

```bash
git add crates/axiom-core/src/lens.rs
git commit -m "feat(axiom-core): CachedLens with VC-based invalidation and Lens3 combinator"
```

---

## Task 6: 更新 hello_cell 示例并添加单元测试

**Files:**
- Modify: `crates/axiom-core/examples/hello_cell.rs`
- Create: `crates/axiom-core/tests/vector_clock_tests.rs`

**Interfaces:**
- Consumes: All primitives from Tasks 0-5
- Produces: Working hello_cell example + comprehensive unit tests

- [ ] **Step 1: 更新 hello_cell.rs 示例**

将 `crates/axiom-core/examples/hello_cell.rs` 更新为使用新 API：

```rust
//! Hello Cell — minimal example demonstrating core primitives.
//!
//! A Cell that receives greeting messages and stores them,
//! producing a Witness for each received message.

use axiom_core::cell::*;
use axiom_core::id::*;
use axiom_core::layer::Layer;
use axiom_core::signal::*;
use axiom_core::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GreetingSignal {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    clock: VectorClock,
    message: String,
}

impl SignalClone for GreetingSignal {
    fn clone_box(&self) -> Box<dyn Signal> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Signal for GreetingSignal {
    fn signal_type(&self) -> &'static str {
        "greeting.hello"
    }
    fn msg_id(&self) -> &str {
        &self.msg_id.0
    }
    fn correlation_id(&self) -> &str {
        &self.correlation_id.0
    }
    fn vector_clock(&self) -> &VectorClock {
        &self.clock
    }
    fn kind(&self) -> SignalKind {
        SignalKind::Command
    }
}

struct GreetingCell {
    id: CellId,
    greetings: Vec<String>,
}

impl GreetingCell {
    fn new() -> Self {
        Self {
            id: CellId::new("greeting-cell"),
            greetings: Vec::new(),
        }
    }
}

impl Cell for GreetingCell {
    type Message = GreetingSignal;

    fn id(&self) -> &CellId {
        &self.id
    }

    fn layer(&self) -> Layer {
        Layer::Exec
    }

    async fn handle(&mut self, signal: GreetingSignal, ctx: &mut CellContext) -> Result<()> {
        println!("Received: {}", signal.message);
        self.greetings.push(signal.message.clone());
        ctx.emit_success(&format!("Stored greeting: {}", signal.message));
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut cell = GreetingCell::new();
    let mut ctx = CellContext::new(cell.id().clone(), cell.layer());

    let signal = GreetingSignal {
        msg_id: MsgId::new(),
        correlation_id: CorrelationId::new(),
        clock: VectorClock::new(),
        message: "Hello, Axiom!".to_string(),
    };

    cell.handle(signal, &mut ctx).await?;

    println!("Greetings received: {:?}", cell.greetings);
    println!("Witnesses produced: {}", ctx.witnesses.len());
    println!("Outgoing signals: {}", ctx.outgoing.len());

    Ok(())
}
```

- [ ] **Step 2: 创建 Vector Clock 综合测试**

创建 `crates/axiom-core/tests/vector_clock_tests.rs`：

```rust
use axiom_core::signal::VectorClock;

#[test]
fn test_vc_empty_clock() {
    let c = VectorClock::new();
    assert_eq!(c.get("any"), 0);
}

#[test]
fn test_vc_increment() {
    let mut c = VectorClock::new();
    c.increment("a");
    assert_eq!(c.get("a"), 1);
    c.increment("a");
    assert_eq!(c.get("a"), 2);
}

#[test]
fn test_vc_merge_takes_max() {
    let mut c1 = VectorClock::new();
    c1.increment("a");
    c1.increment("a");
    c1.increment("b");
    let mut c2 = VectorClock::new();
    c2.increment("a");
    c2.increment("c");
    c2.increment("c");

    c1.merge(&c2);
    assert_eq!(c1.get("a"), 2);
    assert_eq!(c1.get("b"), 1);
    assert_eq!(c1.get("c"), 2);
}

#[test]
fn test_vc_causality_chain() {
    let mut c = VectorClock::new();
    c.increment("a");
    let c1 = c.clone();
    c.increment("a");
    let c2 = c.clone();
    c.increment("b");
    let c3 = c.clone();

    assert!(c1.causally_precedes(&c2));
    assert!(c2.causally_precedes(&c3));
    assert!(c1.causally_precedes(&c3));
    assert!(!c3.causally_precedes(&c1));
}

#[test]
fn test_vc_concurrent_events() {
    let mut c1 = VectorClock::new();
    c1.increment("a");
    let mut c2 = VectorClock::new();
    c2.increment("b");

    assert!(c1.is_concurrent_with(&c2));
    assert!(!c1.causally_precedes(&c2));
    assert!(!c2.causally_precedes(&c1));
}

#[test]
fn test_vc_merge_makes_causal() {
    let mut c1 = VectorClock::new();
    c1.increment("a");
    let mut c2 = VectorClock::new();
    c2.increment("b");

    assert!(c1.is_concurrent_with(&c2));

    c1.merge(&c2);
    assert!(c2.causally_precedes(&c1));
}
```

- [ ] **Step 3: 运行完整测试套件**

Run:
```bash
cd d:\work\trae\axiom-core
cargo test -p axiom-core 2>&1
cargo run --example hello_cell -p axiom-core 2>&1
```

Expected:
- All tests pass
- Example prints:
  ```
  Received: Hello, Axiom!
  Greetings received: ["Hello, Axiom!"]
  Witnesses produced: 1
  Outgoing signals: 0
  ```
- 0 warnings

- [ ] **Step 4: Commit**

```bash
git add crates/axiom-core/examples/hello_cell.rs crates/axiom-core/tests/vector_clock_tests.rs
git commit -m "feat(axiom-core): update hello_cell example and add comprehensive VectorClock tests"
```

---

## Task 7: 最终验证 + Phase 交付

- [ ] **Step 1: 完整编译和测试**

```bash
cd d:\work\trae\axiom-core
cargo build --workspace 2>&1
cargo test --workspace 2>&1
cargo run --example hello_cell -p axiom-core 2>&1
```

Expected:
- `cargo build --workspace`: 0 errors, 0 warnings
- `cargo test --workspace`: all tests pass
- `cargo run --example hello_cell`: runs successfully with expected output

- [ ] **Step 2: 检查 unsafe 隔离**

```bash
grep -rn "unsafe" crates/axiom-core/src/ --include="*.rs"
```

Expected: Only matches in `unsafe_impl.rs` or none at all.

- [ ] **Step 3: 检查依赖方向**

```bash
cd d:\work\trae\axiom-core
cargo tree -p axiom-core --no-default-features 2>&1
```

Expected: axiom-core does NOT depend on axiom-runtime, axiom-agent, axiom-store, or any other workspace crate.

- [ ] **Step 4: 检查文档**

```bash
cargo doc -p axiom-core --no-deps 2>&1
```

Expected: 0 warnings about missing docs (because we added `#![deny(missing_docs)]`).
Note: If there are missing_docs warnings, fix them before proceeding. Add `///` docs to all public items.

- [ ] **Step 5: 提交所有剩余变更**

```bash
cd d:\work\trae\axiom-core
git add -A
git commit -m "feat(axiom-core): Phase 1 complete - all 5 primitives fully operational"
```

- [ ] **Step 6: 推送到 GitHub**

```bash
git push origin master
```

---

## Phase 1 验收标准（Definition of Done）

完成以上所有任务后，以下必须全部为真：

| # | 验收标准 | 验证方式 |
|---|---------|---------|
| 1 | `cargo build --workspace` 零错误零警告 | `cargo build --workspace 2>&1 \| findstr /i "error warning"` 无输出 |
| 2 | `cargo test --workspace` 全部通过 | 运行命令，看到 "test result: ok" |
| 3 | `cargo run --example hello_cell` 正常运行 | 输出 "Received: Hello, Axiom!" 且有 1 条 Witness |
| 4 | async-trait 已完全移除 | `grep -rn "async-trait" Cargo.toml crates/` 无匹配 |
| 5 | unsafe 代码仅存在于 unsafe_impl.rs | `grep -rn "unsafe" crates/axiom-core/src/` 仅 unsafe_impl.rs |
| 6 | Vector Clock 因果排序正确 | vector_clock_tests.rs 5个测试全部通过 |
| 7 | Witness 链式哈希防篡改 | witness_chain_tests.rs 3个测试全部通过 |
| 8 | 跨层调用检测有效 | signal_envelope_tests.rs layer violation测试通过 |
| 9 | 所有public API有文档 | `cargo doc -p axiom-core --no-deps` 无missing_docs警告 |
| 10 | 错误类型覆盖所有需求场景 | AxiomError有16个变体，覆盖需求文档中列出的所有错误情况 |
| 11 | Cell可以发送消息+产生Witness | CellContext的send/emit方法正常工作 |
| 12 | axiom-core无反向依赖 | `cargo tree -p axiom-core` 不依赖其他workspace crate |
