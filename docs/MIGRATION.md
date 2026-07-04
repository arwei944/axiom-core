# Axiom Core 迁移指南

> **版本:** v0.3.0
> **最后更新:** 2026-07-04

---

## 1. 从 LangChain 迁移

### 1.1 核心概念映射

| LangChain 概念 | Axiom Core 对应 | 说明 |
|----------------|----------------|------|
| Agent | Cell + Signal | Cell 是状态单元，Signal 是消息 |
| Chain | Witness 链 | Witness 记录每次状态转换 |
| Memory | Lens + EventStore | Lens 提供投影，EventStore 持久化 |
| Tool | Tool trait + Guard | 通过 #[tool] 宏自动注入权限控制 |
| Callback | BusInterceptor | 运行时拦截器链 |
| Prompt | Signal payload | 结构化消息，支持 schema 验证 |

### 1.2 最小迁移步骤

1. **定义 Signal 类型**

```rust
use axiom_core::signal::Signal;

#[derive(Serialize, Deserialize, SignalPayload)]
struct MyAgentInput {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    prompt: String,
}
```

2. **创建 Cell**

```rust
use axiom_core::cell::Cell;

#[cell(layer = "exec")]
struct MyAgentCell {
    state: String,
}

#[async_trait]
impl Cell for MyAgentCell {
    type Message = MyAgentInput;
    // ...
}
```

3. **替换 Memory**

```rust
// LangChain
let memory = VectorStore::new();

// Axiom Core
let store = MemoryEventStore::new();
let lens = Lens::new("my-lens", |events| {
    events.iter().map(|e| e.payload.clone()).collect()
});
```

### 1.3 常见问题

**Q: LangChain 的 AgentExecutor 对应什么？**
A: Axiom Runtime 的 dispatch loop + Supervisor。Runtime 自动管理 Cell 生命周期、重试和 circuit break。

**Q: 如何实现 ReAct 模式？**
A: 使用多个 Cell 组成 pipeline，通过 Signal 传递思考-行动-观察循环。

**Q: 提示词模板如何处理？**
A: 使用 `serde_json::Value` 作为 Signal payload，在 Cell::handle 中动态生成。

---

## 2. 从 CrewAI 迁移

### 2.1 核心概念映射

| CrewAI 概念 | Axiom Core 对应 | 说明 |
|-------------|----------------|------|
| Crew | Runtime + 多个 Cell | Runtime 管理 Cell 生命周期 |
| Agent | Cell | 每个 Agent 是一个 Cell |
| Task | Signal | Task 作为 Signal 发送给 Agent Cell |
| Process | Execution Layer | 顺序/并行执行由 Layer 控制 |
| Tools | #[tool] 宏 | 自动权限注入 + audit logging |
| Memory | EventStore + Lens | 短期/长期记忆分离 |

### 2.2 最小迁移步骤

1. **定义 Crew（Runtime）**

```rust
let runtime = AxiomRuntime::new(RuntimeConfig::default());

// 注册 Agent Cells
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

### 2.3 常见问题

**Q: CrewAI 的 delegation 如何实现？**
A: 使用 `SignalEnvelope::to_cell()` 将任务委托给特定 Cell。

**Q: 如何实现 Agent 间的协作？**
A: 通过 Witness 链传播上下文，使用 `correlation_id` 追踪任务链。

**Q: 工具调用权限如何管理？**
A: `#[tool(permission = "read")]` 宏自动注入权限检查，无需手动实现。

---

## 3. 从自研框架迁移

### 3.1 Actor 模型迁移

如果你的框架基于 Actor 模型：

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

### 3.2 事件溯源迁移

如果你的框架使用事件溯源：

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

### 3.3 权限系统迁移

```rust
// 你的框架
if !user.can("read") { return Err(...); }

// Axiom Core
#[tool(perission = "read")]
fn read_data(&self) -> Result<Data> {
    // 权限检查自动注入
}
```

---

## 4. 最小可行示例

### 4.1 Hello World

```rust
use axiom_core::prelude::*;

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

    async fn handle(&mut self, msg: &Hello) -> HandlerResult {
        self.count += 1;
        tracing::info!("Hello {}! count={}", msg.name, self.count);
        Ok(())
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

### 4.2 带持久化的示例

```rust
let store = Arc::new(SqliteStore::connect("sqlite:myapp.db").await?);
runtime = runtime.with_event_store(store);
```

### 4.3 带监控的示例

```rust
let metrics = Arc::new(MetricsServer::new("0.0.0.0:9090".parse()?));
runtime = runtime.with_metrics(metrics);
```

---

## 5. 常见迁移问题 FAQ

**Q: 迁移需要重写所有代码吗？**
A: 不需要。Axiom Core 设计为增量迁移。可以从一个简单的 Cell 开始，逐步替换现有模块。

**Q: 如何保留现有数据？**
A: 使用 `EventStore::replay()` 从现有事件日志重建状态。

**Q: 是否支持热迁移？**
A: 支持。使用 Witness 链和快照，可以在不停机的情况下迁移状态。

**Q: 学习曲线如何？**
A: 核心概念（Cell、Signal、Layer）可在 1 天内掌握。完整掌握 Witness 和熵治理约需 1 周。

**Q: 社区支持？**
A: 查看 GitHub Issues 和 Discussions，或加入 Discord。

---

## 6. 迁移检查清单

- [ ] 阅读 [核心概念](guide/core-concepts.md)
- [ ] 定义 Signal 类型
- [ ] 实现 Cell trait
- [ ] 配置 Runtime
- [ ] 添加 EventStore 持久化
- [ ] 添加 MetricsServer 监控
- [ ] 编写单元测试
- [ ] 运行集成测试
- [ ] 性能基准测试
- [ ] 部署到 staging

---

## 7. 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| v0.3.0 | 2026-07-04 | 新增 LangChain/CrewAI/自研框架迁移指南 |
| v0.2.0 | 2025-12-01 | 初始版本 |
