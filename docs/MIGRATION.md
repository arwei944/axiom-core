# Axiom Core 迁移指南

> **版本:** v0.4.0
> **最后更新:** 2026-07-08
> **主题:** 从 v0.3.0 (axiom-core) 迁移到 v0.4.0 (axiom-kernel)

---

## 目录

- [概述](#概述)
- [自动迁移工具](#自动迁移工具)
- [从 v0.3.0 迁移](#从-v030-迁移)
- [从 LangChain 迁移](#从-langchain-迁移)
- [从 CrewAI 迁移](#从-crewai-迁移)
- [从自研框架迁移](#从自研框架迁移)
- [API 变更详细对照表](#api-变更详细对照表)
- [常见迁移问题 FAQ](#常见迁移问题-faq)
- [迁移检查清单](#迁移检查清单)
- [迁移案例](#迁移案例)

---

## 概述

v0.4.0 完成了从 `axiom-core` 到 `axiom-kernel` 的完整迁移。`axiom-kernel` 作为新的运行时层完全替代 `axiom-core`，带来了以下改进：

| 改进 | 说明 |
|------|------|
| **编译期注册表** | 使用 `linkme::distributed_slice`，零运行时注册开销 |
| **更好的锁原语** | `tokio::sync::RwLock` + `parking_lot::Mutex` |
| **WASM 插件系统** | 支持运行时动态加载 WASM 和 Native 插件 |
| **热图系统** | 实时信号流量监控 |
| **Witness 哈希链** | SHA-256 不可篡改审计链 |
| **编译期层间检查** | `CanSendTo` trait bound 在编译期拒绝非法跨层调用 |

**兼容性策略**：提供 `axiom-core` 兼容性层，旧代码可继续运行，但会收到 deprecation 警告。

---

## 自动迁移工具

### 使用 CLI 迁移工具

```bash
# 安装 CLI（如果尚未安装）
cargo install --path crates/axiom-cli

# 分析项目（报告模式）
axm migrate

# 分析并应用自动修复
axm migrate --apply

# 详细输出模式
axm migrate --apply --verbose

# 指定项目路径
axm migrate --path /path/to/project
```

### 迁移工具功能

| 功能 | 说明 |
|------|------|
| **Cargo.toml 分析** | 自动替换 `axiom-core` → `axiom-kernel` |
| **导入路径替换** | 自动替换 `axiom_core::*` → `axiom_kernel::*` |
| **宏调用替换** | 自动替换 `#[axiom_core::*]` → `#[axiom_kernel::*]` |
| **手动操作检测** | 检测需要手动修改的代码（如 `Cell::handle` 签名变更） |
| **Dry-run 模式** | 预览变更而不实际修改文件 |

### 迁移报告示例

```
=== Axiom Migration Tool ===
Project: /path/to/project
Mode: report

📦 Cargo.toml changes:
  ✓ Replace 'axiom-core' with 'axiom-kernel'

📝 Source file changes:
  src/cell/my_cell.rs
    3 changes
  src/signal/my_signal.rs
    1 changes

⚠️  Manual actions required:
  1. src/cell/my_cell.rs: Update `Cell::handle` method signature. See MIGRATION.md for details.

Total changes needed: 4
```

---

## 从 v0.3.0 迁移

### 1. 更新依赖

将 `Cargo.toml` 中的 `axiom-core` 替换为 `axiom-kernel`：

```toml
# v0.3.0
[dependencies]
axiom-core = "0.3"

# v0.4.0
[dependencies]
axiom-kernel = "0.4"
```

### 2. 更新导入路径

```rust
// v0.3.0
use axiom_core::cell::Cell;
use axiom_core::signal::Signal;
use axiom_core::axiom::Axiom;
use axiom_core::witness::Witness;

// v0.4.0
use axiom_kernel::cell::Cell;
use axiom_kernel::signal::Signal;
use axiom_kernel::axiom::Axiom;
use axiom_kernel::witness::Witness;
```

### 3. 更新宏调用

```rust
// v0.3.0
#[axiom_core::signal]
#[axiom_core::cell("exec")]
#[axiom_core::axiom]
#[axiom_core::guard(layer = "exec")]

// v0.4.0
#[axiom_kernel::signal]
#[axiom_kernel::cell("exec")]
#[axiom_kernel::axiom]
#[axiom_kernel::guard(layer = "exec")]
```

### 4. 更新 Cell trait 实现

`handle` 方法签名变化：

```rust
// v0.3.0
async fn handle(&mut self, signal: Self::Message, ctx: &mut CellContext) -> Result<()>;

// v0.4.0
fn handle<'a>(
    &'a mut self,
    signal: Self::Message,
    ctx: LayeredCellContext<'a, Self::Layer>,
) -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a;
```

完整示例对比：

```rust
// v0.3.0
#[cell(layer = "exec")]
struct MyCell {
    count: u64,
}

impl Cell for MyCell {
    type Message = MySignal;

    fn id(&self) -> &CellId { /* ... */ }

    async fn handle(&mut self, signal: Self::Message, ctx: &mut CellContext) -> Result<()> {
        self.count += 1;
        ctx.emit_witness(TransitionOutcome::Success).await?;
        Ok(())
    }
}

// v0.4.0
#[cell(layer = "exec")]
struct MyCell {
    count: u64,
}

impl Cell for MyCell {
    type Message = MySignal;
    type Layer = ExecLayer;

    fn id(&self) -> &CellId { /* ... */ }

    fn handle<'a>(&'a mut self, signal: Self::Message, ctx: LayeredCellContext<'a, Self::Layer>) -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a {
        async move {
            self.count += 1;
            
            ctx.emit_witness(
                ctx.witness()
                    .summary("processed signal")
                    .outcome(TransitionOutcome::Success)
                    .processing_time_us(42),
            )?;
            
            let (outgoing, witnesses) = ctx.end_processing();
            (Ok(()), outgoing, witnesses)
        }
    }
}
```

### 5. 更新 Witness 记录

```rust
// v0.3.0
ctx.emit_witness(TransitionOutcome::Success).await?;

// v0.4.0
ctx.emit_witness(
    ctx.witness()
        .summary("processed signal")
        .outcome(TransitionOutcome::Success)
        .processing_time_us(42),
)?;
let (outgoing, witnesses) = ctx.end_processing();
(result, outgoing, witnesses)
```

### 6. 更新 Axiom 注册

```rust
// v0.3.0
let chain = AxiomChain::from_registry();

// v0.4.0
let chain = DynAxiomChain::from_registry_for_layer(Layer::Exec);
```

### 7. 更新 Layer 相关代码

```rust
// v0.3.0
use axiom_core::layer::Layer;

// v0.4.0 - 新增编译期层标记
use axiom_kernel::layer::Layer;
use axiom_kernel::sealed::ExecLayer;
```

### 8. 更新 Runtime 注册

```rust
// v0.3.0
runtime.register_cell(CellRegistration {
    id: CellId::new("my-cell"),
    layer: Layer::Exec,
    version: Version::new(1, 0, 0),
    supervision_strategy: SupervisionStrategy::Restart,
    cell: Some(Arc::new(Mutex::new(MyCell::new()))),
    factory: None,
}).await;

// v0.4.0 - 新增 factory 字段用于重启机制
runtime.register_cell(CellRegistration {
    id: CellId::new("my-cell"),
    layer: Layer::Exec,
    version: Version::new(1, 0, 0),
    supervision_strategy: SupervisionStrategy::Restart,
    cell: Some(Arc::new(Mutex::new(MyCell::new()))),
    factory: Some(Box::new(|| Ok(Arc::new(Mutex::new(MyCell::new()))))),
}).await;
```

---

## 从 LangChain 迁移

### 核心概念映射

| LangChain 概念 | Axiom Core 对应 | 说明 |
|----------------|----------------|------|
| Agent | Cell + Signal | Cell 是状态单元，Signal 是消息 |
| Chain | Witness 链 | Witness 记录每次状态转换 |
| Memory | Lens + EventStore | Lens 提供投影，EventStore 持久化 |
| Tool | Tool trait + Guard | 通过 `#[tool]` 宏自动注入权限控制 |
| Callback | BusInterceptor | 运行时拦截器链 |
| Prompt | Signal payload | 结构化消息，支持 schema 验证 |

### 最小迁移步骤

1. **定义 Signal 类型**

```rust
use axiom_kernel::signal::Signal;

#[derive(Serialize, Deserialize, SignalPayload)]
#[signal(kind = "command", layer = "exec")]
struct MyAgentInput {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    prompt: String,
}
```

2. **创建 Cell**

```rust
use axiom_kernel::cell::Cell;

#[cell(layer = "exec")]
struct MyAgentCell {
    state: String,
}

impl Cell for MyAgentCell {
    type Message = MyAgentInput;
    type Layer = ExecLayer;

    fn id(&self) -> &CellId { /* ... */ }

    fn handle<'a>(&'a mut self, signal: Self::Message, ctx: LayeredCellContext<'a, Self::Layer>) -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a {
        async move { /* ... */ }
    }
}
```

3. **替换 Memory**

```rust
// LangChain
let memory = VectorStore::new();

// Axiom Core
let store = MemoryEventStore::new();
let lens = OrderHistoryLens;
```

---

## 从 CrewAI 迁移

### 核心概念映射

| CrewAI 概念 | Axiom Core 对应 | 说明 |
|-------------|----------------|------|
| Crew | Runtime + 多个 Cell | Runtime 管理 Cell 生命周期 |
| Agent | Cell | 每个 Agent 是一个 Cell |
| Task | Signal | Task 作为 Signal 发送给 Agent Cell |
| Process | Execution Layer | 顺序/并行执行由 Layer 控制 |
| Tools | `#[tool]` 宏 | 自动权限注入 + audit logging |
| Memory | EventStore + Lens | 短期/长期记忆分离 |

### 最小迁移步骤

1. **定义 Runtime**

```rust
let runtime = AxiomRuntime::new(RuntimeConfig::default());

runtime.register_cell(AgentRegistration {
    id: CellId::new("researcher"),
    layer: Layer::Agent,
    version: Version::new(1, 0, 0),
    supervision_strategy: SupervisionStrategy::Restart,
    cell: Some(Arc::new(Mutex::new(ResearcherCell::new()))),
    factory: None,
}).await;
```

2. **定义 Task（Signal）**

```rust
#[signal(kind = "command", layer = "agent")]
struct ResearchTask {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    topic: String,
    depth: u32,
}
```

3. **执行流程**

```rust
// CrewAI
result = crew.kickoff()

// Axiom Core
let task = ResearchTask::new("AI 最新进展", 3);
runtime.submit_signal(&task, None, Layer::Agent).await?;
```

---

## API 变更详细对照表

### 核心类型变更

| v0.3.0 (axiom-core) | v0.4.0 (axiom-kernel) | 变更说明 |
|---------------------|----------------------|---------|
| `Cell` | `Cell` | trait 签名变更 |
| `CellContext` | `LayeredCellContext<'a, L>` | 添加层类型参数 |
| `AxiomChain` | `DynAxiomChain` | 动态链替代静态链 |
| `TransitionOutcome` | `TransitionOutcome` | 保持不变 |
| `Witness` | `Witness` | 新增 builder 模式 |
| `Signal` | `Signal` | 保持不变 |

### Context API 变更

| v0.3.0 | v0.4.0 | 说明 |
|--------|--------|------|
| `ctx.emit_witness(outcome).await` | `ctx.emit_witness(ctx.witness().outcome(outcome))` | Builder 模式 |
| `ctx.emit_event(event, layer).await` | `ctx.emit_event(event)` | 层由 context 自动推断 |
| `ctx.send_to(target, signal).await` | `ctx.send_to(target, signal)` | 编译期检查 |
| `ctx.end_processing()` | `ctx.end_processing()` | 返回 `(outgoing, witnesses)` |

### Layer API 变更

| v0.3.0 | v0.4.0 | 说明 |
|--------|--------|------|
| `Layer::Exec` | `Layer::Exec` | 保持不变 |
| `Layer::Agent` | `Layer::Agent` | 保持不变 |
| - | `ExecLayer`, `AgentLayer`, etc. | 新增编译期标记 |
| - | `CanSendTo` | 新增编译期层间调用检查 |

---

## 常见迁移问题 FAQ

**Q: 迁移需要重写所有代码吗？**
A: 不需要。v0.4.0 设计为增量迁移。可以从一个简单的 Cell 开始，逐步替换现有模块。

**Q: 如何保留现有数据？**
A: 使用 `EventStore::replay()` 从现有事件日志重建状态。

**Q: 是否支持热迁移？**
A: 支持。使用 Witness 链和快照，可以在不停机的情况下迁移状态。

**Q: `CellContext` 变成了 `LayeredCellContext`，如何处理？**
A: `LayeredCellContext` 通过 `CanSendTo` trait bound 在编译期强制层间调用规则。使用 `ctx.as_layered::<ExecLayer>()` 获取层特定上下文。

**Q: `handle` 方法返回值变了，如何适配？**
A: 新的返回值包含 `(Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)`。在 `handle` 结束时调用 `ctx.end_processing()` 获取外发消息和见证。

**Q: `AxiomChain` 变成了 `DynAxiomChain`，如何适配？**
A: 使用 `DynAxiomChain::from_registry_for_layer(Layer::Exec)` 按层查询注册的 Axiom。

**Q: 学习曲线如何？**
A: 核心概念（Cell、Signal、Layer）可在 1 天内掌握。完整掌握 Witness 和熵治理约需 1 周。

**Q: 为什么我的代码编译时出现 "REVERSE DEPENDENCY" 错误？**
A: 这是因为新架构强制层间依赖方向。检查 `.axiom/architecture.toml` 中的 crate 层配置，或添加豁免。

**Q: 为什么 `async-trait` 不再被允许？**
A: v0.4.0 使用 Rust 1.75+ 原生 `async fn in traits`，不再需要 `async-trait` 宏。

**Q: 如何处理 `ActivationCondition` 和 `DisclosureLevel` 的歧义导入？**
A: 这两个类型在 `axiom-agent` 和 `axiom-identity` 中都有定义。使用显式导入：

```rust
use axiom_identity::{ActivationCondition, DisclosureLevel};
```

---

## 迁移检查清单

- [ ] 使用 `axm migrate` 自动分析和修复项目
- [ ] 更新 `Cargo.toml`：将 `axiom-core` 替换为 `axiom-kernel`
- [ ] 更新所有导入路径：`axiom_core::` → `axiom_kernel::`
- [ ] 更新宏调用：`#[axiom_core::...]` → `#[axiom_kernel::...]`
- [ ] 更新 Cell trait 实现：`handle` 方法签名、`LayeredCellContext`、返回三元组
- [ ] 更新 Witness 记录：使用 `ctx.witness()` builder 模式
- [ ] 更新 Axiom 注册：使用 `DynAxiomChain::from_registry_for_layer`
- [ ] 添加层标记导入：`use axiom_kernel::sealed::ExecLayer;`
- [ ] 更新 Runtime 注册：添加 `factory` 字段
- [ ] 运行 `cargo check` 检查编译错误
- [ ] 运行 `cargo test` 检查测试通过
- [ ] 运行 `cargo clippy` 检查代码质量

---

## 迁移案例

### 案例 1：简单 Cell 迁移

**v0.3.0 代码**：

```rust
use axiom_core::cell::Cell;
use axiom_core::context::CellContext;

#[axiom_core::cell("exec")]
struct CounterCell {
    count: u64,
}

impl Cell for CounterCell {
    type Message = CounterSignal;

    fn id(&self) -> &CellId {
        static ID: CellId = CellId::new_static("counter");
        &ID
    }

    async fn handle(&mut self, signal: Self::Message, ctx: &mut CellContext) -> Result<()> {
        self.count += signal.amount;
        ctx.emit_witness(TransitionOutcome::Success).await?;
        Ok(())
    }
}
```

**v0.4.0 代码**：

```rust
use axiom_kernel::cell::Cell;
use axiom_kernel::context::LayeredCellContext;
use axiom_kernel::sealed::ExecLayer;

#[axiom_kernel::cell("exec")]
struct CounterCell {
    count: u64,
}

impl Cell for CounterCell {
    type Message = CounterSignal;
    type Layer = ExecLayer;

    fn id(&self) -> &CellId {
        static ID: CellId = CellId::new_static("counter");
        &ID
    }

    fn handle<'a>(&'a mut self, signal: Self::Message, ctx: LayeredCellContext<'a, Self::Layer>) -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a {
        async move {
            self.count += signal.amount;
            
            ctx.emit_witness(
                ctx.witness()
                    .summary(format!("incremented by {}", signal.amount))
                    .outcome(TransitionOutcome::Success)
                    .processing_time_us(10),
            )?;
            
            let (outgoing, witnesses) = ctx.end_processing();
            (Ok(()), outgoing, witnesses)
        }
    }
}
```

### 案例 2：Runtime 注册迁移

**v0.3.0 代码**：

```rust
let runtime = AxiomRuntime::new(RuntimeConfig::default());

runtime.register_cell(CellRegistration {
    id: CellId::new("my-cell"),
    layer: Layer::Exec,
    version: Version::new(1, 0, 0),
    supervision_strategy: SupervisionStrategy::Restart,
    cell: Some(Arc::new(Mutex::new(MyCell::new()))),
    factory: None,
}).await?;
```

**v0.4.0 代码**：

```rust
let runtime = RuntimeBuilder::new().build();

runtime.register_cell(CellRegistration {
    id: CellId::new("my-cell"),
    layer: Layer::Exec,
    version: Version::new(1, 0, 0),
    supervision_strategy: SupervisionStrategy::Restart,
    cell: Some(Arc::new(Mutex::new(MyCell::new()))),
    factory: Some(Box::new(|| Ok(Arc::new(Mutex::new(MyCell::new()))))),
}).await?;
```

---

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| v0.4.0 | 2026-07-08 | 新增自动迁移工具 `axm migrate`，完善 API 对照表 |
| v0.3.0 | 2026-07-04 | 新增 LangChain/CrewAI/自研框架迁移指南 |
| v0.2.0 | 2025-12-01 | 初始版本