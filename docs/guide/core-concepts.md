# 核心概念

本文档解释 Axiom Core 的核心概念，帮助理解架构设计哲学和关键机制。

---

## 1. 五大原语

### 1.1 Cell

`Cell` 是状态单元的基本抽象，拥有私有状态和消息信箱。

```rust
pub trait Cell: Send + 'static {
    type Message: Signal;
    type State: 'static;
    type Layer: RuntimeTierMarker;

    fn id(&self) -> &CellId;
    fn layer(&self) -> RuntimeTier;
    fn handle<'a>(
        &'a mut self,
        signal: Self::Message,
        ctx: LayeredCellContext<'a, Self::Layer>,
    ) -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a;
}
```

**关键特性**：
- **单线程执行**：每个 Cell 在同一时间只处理一个消息
- **隔离状态**：私有状态不共享，通过消息通信
- **监督友好**：崩溃后可重建，状态可通过 Witness 恢复

### 1.2 Signal

`Signal` 是类型化的不可变消息，携带元数据用于路由和追踪。

```rust
pub trait Signal: Send + Sync + 'static {
    fn signal_type(&self) -> &'static str;
    fn msg_id(&self) -> &MsgId;            // 唯一 ID，用于幂等
    fn correlation_id(&self) -> &CorrelationId; // 链路追踪
    fn trace_id(&self) -> Option<&TraceId> { None }
    fn vector_clock(&self) -> &VectorClock;     // 因果排序
    fn timestamp_ns(&self) -> u64;              // 新鲜度
    fn kind(&self) -> SignalKind;               // Command/Event/Query/Response
    fn layer(&self) -> RuntimeTier;                   // 来源层
    fn schema_version(&self) -> SchemaVersion { SchemaVersion::new(1) }
    fn sender(&self) -> Option<&str> { None }

    fn as_any(&self) -> &dyn std::any::Any;
    fn clone_signal(&self) -> Box<dyn Signal>;
    fn validate(&self) -> ValidationResult;
    fn serialize_to_json(&self) -> Result<serde_json::Value>;
}
```

**关键特性**：
- **类型安全**：编译期保证消息类型正确
- **不可变**：一旦创建不可修改，避免竞态
- **链路追踪**：通过 `correlation_id` 和 `trace_id` 追踪请求链路
- **因果排序**：通过 `vector_clock` 保证因果一致性

### 1.3 Lens

`Lens` 提供状态的按需投影，避免暴露完整状态。

```rust
pub trait Lens: Send + Sync + 'static {
    type Input;
    type Output;

    fn id(&self) -> &LensId;
    fn project(&self, events: &[Event], input: &Self::Input) -> Self::Output;
}
```

**关键特性**：
- **渐进式披露**：只暴露必要信息
- **可组合**：多个 Lens 可以组合使用
- **缓存友好**：投影结果可缓存，提升性能

### 1.4 Axiom

`Axiom` 定义跨状态的不变量约束，违反即熔断。

```rust
#[axiom]
pub struct NonEmptyGreetingAxiom;

impl Axiom<GreetingState, GreetingCommand> for NonEmptyGreetingAxiom {
    fn name(&self) -> &'static str { "non-empty-greeting" }

    fn check(&self, _current: &Self::State, new: &Self::State, _msg: &Self::Message) -> Result<()> {
        if new.iter().any(|g| g.is_empty()) {
            return Err(KernelError::InvariantViolated {
                message: "greeting must not be empty".into(),
            });
        }
        Ok(())
    }

    fn applies_to_layer(&self, layer: RuntimeTier) -> bool {
        matches!(layer, RuntimeTier::Exec | RuntimeTier::Validate)
    }
}
```

**关键特性**：
- **硬约束**：违反 Axiom 的状态转换会被拒绝
- **可组合**：多个 Axiom 可以同时应用于同一状态
- **按层过滤**：通过 `applies_to_layer` 控制生效范围

### 1.5 Witness

`Witness` 是不可篡改的审计记录，每次状态转换自动产生，形成 SHA-256 哈希链。

```rust
pub struct Witness {
    pub witness_id: WitnessId,
    pub schema_version: SchemaVersion,
    pub cell_id: String,
    pub correlation_id: CorrelationId,
    pub prev_hash: Option<WitnessHash>,
    pub hash: WitnessHash,
    pub summary: String,
    pub outcome: TransitionOutcome,
    pub metrics: WitnessMetrics,
    pub signal_fingerprint: [u8; 32],
    pub payload_size_bytes: u64,
    pub kind: &'static str,
    pub trace_id: Option<TraceId>,
    pub triggering_msg_id: Option<MsgId>,
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
}
```

**关键特性**：
- **不可篡改**：一旦创建不能修改
- **哈希链**：每个 Witness 包含前一个 Witness 的哈希
- **完整审计**：记录状态转换的所有关键信息
- **一秒速查**：任何问题都能一秒回溯根因

---

## 2. Signal 详解

### 信号种类

```rust
pub enum SignalKind {
    Command,  // 命令：要求执行某动作（可改变状态）
    Event,    // 事件：已发生的事实（不可改变状态，仅通知）
    Query,    // 查询：请求信息（只读）
    Response, // 响应：对查询/命令的回复
}
```

### 用宏自动实现 Signal

通过 `#[derive(SignalPayload)]` + `#[signal(kind=..., layer=...)]` 自动生成 trait 实现：

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, SignalPayload)]
#[signal(kind = "command", layer = "exec")]
#[schema_version(1)]
pub struct GreetingSignal {
    pub message: String,
}
```

---

## 3. Layer 体系

### Crate Layer（编译期依赖分层）

Crate Layer 定义 crate 之间的编译期依赖方向，确保架构层次清晰。

| Crate Layer | 职责 |
|-------------|------|
| 0 | 顶层应用（CLI、Benchmark） |
| 1 | 可视化导出 |
| 2 | Agent 门面（Identity、Prompt） |
| 3 | 监督与集成（Agent、Oversight、Alert、MCP） |
| 4 | 运行时与协调（Runtime、Planner、Distributed） |
| 5 | 存储与工具（Store、Tool、Memory、LLM） |
| 7 | 核心原语（Kernel） |
| 8 | 过程宏（豁免层） |
| 9 | Plugin SDK |

**铁律**：Layer N 只能依赖 Layer >= N。

### Runtime Tier（运行时分层）

Runtime Tier 定义 Cell 和 Signal 在运行时的路由约束。

| Runtime Tier | 编号 | 职责 |
|--------------|------|------|
| Oversight | 0 | 最高监督层，可向所有层发送指令 |
| Agent | 3 | Agent 协调层，可向 Validate/Exec 发送任务 |
| Validate | 2 | 校验层，可向 Exec 发送验证请求 |
| Exec | 1 | 执行层，最底层，执行具体任务 |

**层间调用规则**：
- `CanSendTo<SourceTier, TargetTier>` 编译期约束调用方向
- 只有 `Oversight` 可以向所有层发送消息
- `Agent` 可以向 `Validate` 和 `Exec` 发送消息
- `Validate` 只能向 `Exec` 发送消息
- `Exec` 不能向其他层发送消息

---

## 4. Axiom 系统

### 什么是 Axiom

Axiom 是**跨状态的不变量约束**，与 Schema（单条信号校验）不同，Axiom 关注的是**状态转换的业务规则**。

### Axiom 与 Schema 的区别

| 维度 | Schema | Axiom |
|------|--------|-------|
| 作用对象 | 单条信号的字段结构 | 跨信号的状态不变量 |
| 执行时机 | 信号发出/接收时 | 状态转换前后 |
| 位置 | Runtime Tier 2 验证层 | 全局，可声明 `applies_to_layer` |
| 性质 | 数据校验（非空、长度、范围） | 业务不变量（余额非负、唯一性等） |
| 纯函数 | 是 | 是 |

### 运行时查询与批量校验

`DynAxiomChain` 从注册表构建某层的 Axiom 链，对状态转换做批量校验：

```rust
let chain = DynAxiomChain::from_registry_for_layer(RuntimeTier::Exec);
println!("Registered axioms for Exec layer: {}", chain.count());

let violations = chain.check_all(
    &current_state as &dyn Any,
    &new_state as &dyn Any,
    &msg as &dyn Any,
);
for v in &violations {
    println!("{} violated: {:?} (action: {:?})", v.axiom_name, v.error, v.action);
}
```

---

## 5. Witness：审计记录

`Witness` 是不可篡改的审计记录，**每次状态转换自动产生**，形成 SHA-256 哈希链。它是"时间线录像机"——任何问题都能一秒回溯根因。

### Witness 结构

```rust
pub struct Witness {
    pub witness_id: WitnessId,
    pub schema_version: SchemaVersion,
    pub cell_id: String,
    pub correlation_id: CorrelationId,
    pub trace_id: Option<TraceId>,
    pub triggering_msg_id: Option<MsgId>,
    pub prev_hash: Option<WitnessHash>,
    pub hash: WitnessHash,
    pub summary: String,
    pub outcome: TransitionOutcome,
    pub metrics: WitnessMetrics,
    pub signal_fingerprint: [u8; 32],
    pub payload_size_bytes: u64,
    pub kind: &'static str,
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
}
```

### Witness 哈希计算

`Witness::compute_hash` 纳入以下字段生成 SHA-256 哈希（`sha2-id` feature 下）：

- 身份字段：`witness_id`、`cell_id`、`correlation_id`、`trace_id`、`triggering_msg_id`
- 时序字段：`vector_clock`、`timestamp_ns`、`schema_version`
- 内容字段：`summary`、`outcome`、`metrics`、`signal_fingerprint`、`payload_size_bytes`、`kind`
- 链字段：`prev_hash`、`state_before_hash`、`state_after_hash`

### 验证哈希链

```rust
// 验证单个 witness 哈希
assert!(witness.verify_hash().is_ok());

// 验证整个链
assert!(WitnessKernel::verify_chain(&witnesses).is_ok());
```

---

## 6. 插件系统

### 架构

```
┌──────────────────────────────────────────────┐
│              PluginRegistry                  │
│  ┌────────────────────────────────────────┐  │
│  │         PluginLoader                  │  │
│  │  ┌─────────┐    ┌─────────────────┐  │  │
│  │  │ WASM    │    │ Native          │  │  │
│  │  │ Loader  │    │ Loader          │  │  │
│  │  └────┬────┘    └────────┬────────┘  │  │
│  └───────┼──────────────────┼────────────┘  │
│          │                  │               │
│          ▼                  ▼               │
│  ┌─────────────┐  ┌─────────────────┐      │
│  │ WASM Plugin │  │ Native Plugin   │      │
│  │ (.wasm)     │  │ (.so/.dll)      │      │
│  └─────────────┘  └─────────────────┘      │
└──────────────────────────────────────────────┘
```

### 核心类型

| 类型 | 职责 |
|------|------|
| `PluginRegistry` | 插件注册表，管理已加载的插件 |
| `PluginKind` | 插件类型枚举（Wasm/Native） |
| `WasmPluginLoader` | WASM 插件加载器 |
| `NativePluginLoader` | Native 插件加载器 |
| `AxiomPlugin` | 插件 trait |

---

## 7. 热图系统

### 功能

- **信号热图收集**：实时收集信号流量数据
- **时间维度分析**：按时间窗口统计信号数量
- **层维度分析**：按层统计信号分布
- **导出功能**：支持导出为 JSON 格式

### 核心类型

| 类型 | 职责 |
|------|------|
| `HeatmapCollector` | 热图数据收集器 |
| `HeatmapExporter` | 热图数据导出器 |
| `HeatmapData` | 热图数据结构 |

### 使用示例

```rust
let collector = HeatmapCollector::new();
collector.record_signal(&signal_envelope);

let data = collector.get_data();
println!("Total signals: {}", data.total_signals);
println!("Exec layer: {}", data.layer_stats.get(&RuntimeTier::Exec).unwrap_or(&0));
```

---

## 8. 熵治理

### 熵值作为第一公民

- **实时监控**：每个 Cell 的熵值实时追踪
- **黄线告警**：熵值超过阈值时发出警告
- **红线熔断**：熵值过高时自动熔断
- **主动减熵**：通过 Axiom 和 Guardian 主动降低熵值

### 熵事件

```rust
pub enum EntropyEvent {
    DroppedMessage,      // 消息被丢弃
    AxiomViolation,      // Axiom 违反
    CircuitBreaker,      // 熔断器触发
    SupervisionRestart,  // 监督重启
}
```

---

## 9. 异步锁策略

为避免锁混用导致的死锁和性能问题，制定统一锁使用规范：

| 上下文 | 推荐锁 | 禁止使用 |
|--------|--------|----------|
| async 函数/方法 | `tokio::sync::RwLock` | `std::sync::RwLock` |
| sync 函数/方法 | `parking_lot::RwLock` | `std::sync::RwLock` |
| 全局静态状态 | `parking_lot::Mutex` | `std::sync::Mutex` |

### 典型场景

- `AxiomRuntime` 的 async 方法中使用 `tokio::sync::RwLock`
- `Supervisor`、`EntropyGovernorCell` 的 sync 方法中使用 `parking_lot::RwLock`
- `DeadLetterQueue`、`WitnessRegistry` 等全局注册表使用 `parking_lot::Mutex`

---

## 10. 总结

Axiom Core 的核心思想是：

1. **确定性优先**：编译期强制约束，运行时行为可预测
2. **低熵化**：熵值作为第一公民，自动监控和减熵
3. **可观测性**：完整的审计链和链路追踪
4. **可扩展性**：WASM 插件系统支持运行时扩展
5. **自愈能力**：监督树自动重启，事件溯源恢复
6. **架构治理**：自动验证依赖方向和约束完整性

这种设计使 Axiom Core 成为构建生产级智能体系统的理想选择。