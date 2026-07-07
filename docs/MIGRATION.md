# Axiom Core 迁移指南

> **版本:** v0.4.0
> **最后更新:** 2026-07-08
> **主题:** 从 v0.3.0 (axiom-core) 迁移到 v0.4.0 (axiom-kernel)

---

## 目录

- [概述](#概述)
- [从 v0.3.0 迁移](#从-v030-迁移)
- [从 LangChain 迁移](#从-langchain-迁移)
- [从 CrewAI 迁移](#从-crewai-迁移)
- [从自研框架迁移](#从自研框架迁移)
- [最小可行示例](#最小可行示例)
- [常见迁移问题 FAQ](#常见迁移问题-faq)
- [迁移检查清单](#迁移检查清单)

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
use axiom_kernel::sealed::ExecLayer; // 用于 LayeredCellContext
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
let lens = OrderHistoryLens; // 使用 #[lens] 宏注册
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

## 从自研框架迁移

### Actor 模型迁移

```rust
// 你的框架
actor.handle(message)

// Axiom Core
impl Cell for MyCell {
    async fn handle(&mut self, msg: impl Signal) -> HandlerResult {
        // ...
    }
}
```

### 事件溯源迁移

```rust
// 你的框架
event_store.append(Event { ... })

// Axiom Core
let store = SqliteStore::connect("sqlite:events.db").await?;
store.append(Event { ... }).await?;

// 自动获得：
// - 订阅机制
// - 快照加速回放
// - Witness 审计链
```

### 权限系统迁移

```rust
// 你的框架
if !user.can("read") { return Err(...); }

// Axiom Core
#[tool(permission = "read")]
fn read_data(&self) -> Result<Data> {
    // 权限检查自动注入
}
```

---

## 最小可行示例

### Hello World

```rust
use axiom_kernel::prelude::*;

#[signal(kind = "command", layer = "exec")]
struct Hello {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    name: String,
}

#[cell(layer = "exec")]
struct HelloCell {
    count: u64,
}

impl Cell for HelloCell {
    type Message = Hello;

    fn id(&self) -> &CellId { &CellId::new("hello") }

    fn handle<'a>(&'a mut self, msg: Hello, ctx: LayeredCellContext<'a, Self::Layer>) -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a {
        async move {
            self.count += 1;
            tracing::info!("Hello {}! count={}", msg.name, self.count);
            
            ctx.emit_witness(
                ctx.witness()
                    .summary(format!("greeted: {}", msg.name))
                    .outcome(TransitionOutcome::Success)
                    .processing_time_us(42),
            )?;
            
            let (outgoing, witnesses) = ctx.end_processing();
            (Ok(()), outgoing, witnesses)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let runtime = AxiomRuntime::new(RuntimeConfig::default());
    runtime.register_cell(Registration {
        id: CellId::new("hello"),
        layer: Layer::Exec,
        version: Version::new(1, 0, 0),
        supervision_strategy: SupervisionStrategy::Restart,
        cell: Some(Arc::new(Mutex::new(HelloCell { count: 0 }))),
        factory: None,
    }).await?;
    runtime.start().await?;
    Ok(())
}
```

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

---

## 迁移检查清单

- [ ] 更新 `Cargo.toml`：将 `axiom-core` 替换为 `axiom-kernel`
- [ ] 更新所有导入路径：`axiom_core::` → `axiom_kernel::`
- [ ] 更新宏调用：`#[axiom_core::...]` → `#[axiom_kernel::...]`
- [ ] 更新 Cell trait 实现：`handle` 方法签名、`LayeredCellContext`
- [ ] 更新 Witness 记录：使用 `ctx.witness()` builder 模式
- [ ] 更新 Axiom 注册：使用 `DynAxiomChain::from_registry_for_layer`
- [ ] 添加层标记导入：`use axiom_kernel::sealed::ExecLayer;`
- [ ] 运行 `cargo check` 检查编译错误
- [ ] 运行 `cargo test` 检查测试通过
- [ ] 运行 `cargo clippy` 检查代码质量

---

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| v0.4.0 | 2026-07-08 | 新增从 v0.3.0 迁移指南，`axiom-kernel` 替代 `axiom-core` |
| v0.3.0 | 2026-07-04 | 新增 LangChain/CrewAI/自研框架迁移指南 |
| v0.2.0 | 2025-12-01 | 初始版本 |