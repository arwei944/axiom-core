# 核心概念

Axiom Core 用**五个核心原语**加上**四层架构**构建整个确定性优先的智能体运行时。本文档逐一讲解每个原语的设计意图、核心 trait、使用方式，以及四层架构的层间调用约束规则。理解这些概念是高效使用 Axiom Core 的前提。

---

## 目录

- [设计哲学](#设计哲学)
- [核心原语总览](#核心原语总览)
- [Cell：隔离状态单元](#cell隔离状态单元)
- [Signal：类型化消息](#signal类型化消息)
- [Axiom：全局不变量约束](#axiom全局不变量约束)
- [Witness：审计记录](#witness审计记录)
- [Lens：状态投影](#lens状态投影)
- [四层架构](#四层架构)
- [层间调用约束规则](#层间调用约束规则)
- [原语协作总览](#原语协作总览)

---

## 设计哲学

Axiom Core 的核心信念是：**架构就是一切**。模型会犯错，架构不能。UC Berkeley 的研究表明，多智能体系统中 41%–86.7% 的失败源于架构缺陷而非 AI 能力不足。因此 Axiom Core 把分布式系统的经典工程手段——状态隔离、因果追踪、不变量约束、审计链——下放到语言层与编译期，让"确定性"成为系统的底色，而非事后补救。

五个原语各自解决一类痛点：

| 痛点 | 原语 | 一句话 |
|------|------|--------|
| 错误传染 | Cell | 隔离的状态单元，单线程执行，故障不扩散 |
| 消息字符串传递 | Signal | 类型化不可变消息 + Vector Clock 因果追踪 |
| 静默退化 | Axiom | 全局不变量，违反即熔断 |
| 黑盒运行 | Witness | 不可篡改审计链，每次状态转换自动记录 |
| 上下文爆炸 | Lens | 按需状态投影，不塞全部历史 |

---

## 核心原语总览

```
┌─────────────────────────────────────────────────────────┐
│  Cell  │ 隔离的状态单元——私有状态 + 消息信箱，单线程执行 │
├────────┼────────────────────────────────────────────────┤
│ Signal │ 类型化不可变消息——Vector Clock + 链路追踪       │
├────────┼────────────────────────────────────────────────┤
│  Lens  │ 按需状态投影——不是塞全部历史，而是精确查询      │
├────────┼────────────────────────────────────────────────┤
│ Axiom  │ 全局不变量约束——违反即熔断，熵的减压阀          │
├────────┼────────────────────────────────────────────────┤
│Witness │ 不可篡改审计链——每次状态转换自动记录            │
└─────────────────────────────────────────────────────────┘
```

---

## Cell：隔离状态单元

`Cell` 是 Axiom Core 的执行单元，借鉴自 Actor 模型与 Erlang 的进程理念。每个 Cell 拥有**私有状态**与**消息信箱**，单线程处理消息，状态对外不可直接访问。这种隔离保证了一个 Cell 的崩溃不会污染其他 Cell。

### 核心 trait

```rust
pub trait Cell: Send + 'static {
    type Message: Signal;
    type Layer: LayerMarker;

    fn id(&self) -> &CellId;

    fn supervision_strategy(&self) -> SupervisionStrategy {
        SupervisionStrategy::default() // 默认 Restart { max_retries: 3 }
    }

    fn heartbeat_interval_ms(&self) -> Option<u64> { None }

    /// 处理信号，返回结果 + 外发信封 + 见证记录
    fn handle<'a>(
        &'a mut self,
        signal: Self::Message,
        ctx: LayeredCellContext<'a, Self::Layer>,
    ) -> impl Future<
        Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>),
    > + Send + 'a;

    fn on_start<'a>(&'a mut self, _ctx: &'a mut CellContext<'a>) -> impl Future<Output = Result<()>> + Send + 'a {
        async { Ok(()) }
    }

    fn on_stop<'a>(&'a mut self, _ctx: &'a mut CellContext<'a>) -> impl Future<Output = Result<()>> + Send + 'a {
        async { Ok(()) }
    }
}
```

### 关键设计点

1. **关联类型 `Message`**：每个 Cell 只接受一种信号类型，编译期类型安全。
2. **关联类型 `Layer`**：每个 Cell 归属某一层（`ExecLayer` / `ValidateLayer` / `AgentLayer` / `OversightLayer`），决定它能向哪些层发消息。
3. **`handle` 返回 `impl Future`**：异步处理，但状态隔离——同一时刻只有一个 `&mut self`。
4. **`ctx.end_processing()` 必须调用**：框架在 future resolve 后不再访问 ctx，由实现负责取出外发缓冲。
5. **监督策略**：`Restart`、`Stop`、`Escalate`、`CircuitBreak`，Erlang 风格的"让它崩溃"自愈。

### 层级特化 trait

为不同层提供标记 trait，配合宏自动实现：

```rust
pub trait ExecCell: Cell {}       // 执行层
pub trait ValidateCell: Cell {}   // 验证层
pub trait AgentCell: Cell {}      // 推理层
pub trait OversightCell: Cell {}  // 监督层
```

### 用宏声明 Cell

```rust
#[cell("exec")] // 自动 impl ExecCell + 设置 type Layer = ExecLayer
impl Cell for HelloCell {
    type Message = HelloCommand;
    // ...
}
```

### CellHandle：类型擦除句柄

`CellHandle` 把具体 Cell 包装成 `Box<dyn DynHandleCell>`，便于运行时统一调度。它支持 `downcast_ref` 回到具体类型用于测试：

```rust
let handle = CellHandle::new(HelloCell::new());
println!("Cell ID: {}", handle.id());
println!("Cell Layer: {:?}", handle.layer());
assert!(handle.downcast_ref::<HelloCell>().is_some());
```

---

## Signal：类型化消息

`Signal` 是 Cell 之间唯一的通信方式。它**不可变**、**类型化**，并携带因果追踪元数据。这避免了"消息是字符串"导致的解析错误与调试困难。

### 核心 trait

```rust
pub trait Signal: Send + Sync + 'static {
    fn signal_type(&self) -> &'static str;
    fn msg_id(&self) -> &MsgId;            // 唯一 ID，用于幂等
    fn correlation_id(&self) -> &CorrelationId; // 链路追踪
    fn trace_id(&self) -> Option<&TraceId> { None }
    fn vector_clock(&self) -> &VectorClock;     // 因果排序
    fn timestamp_ns(&self) -> u64;              // 新鲜度
    fn kind(&self) -> SignalKind;               // Command/Event/Query/Response
    fn layer(&self) -> Layer;                   // 来源层
    fn schema_version(&self) -> SchemaVersion { SchemaVersion::new(1) }
    fn sender(&self) -> Option<&str> { None }

    fn as_any(&self) -> &dyn std::any::Any;
    fn clone_signal(&self) -> Box<dyn Signal>;
    fn validate(&self) -> ValidationResult;
    fn serialize_to_json(&self) -> Result<serde_json::Value>;
}
```

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
struct HelloCommand {
    // 必需字段
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    // 可选字段（宏会自动识别并实现对应方法）
    // trace_id: Option<TraceId>,  // 若存在，trace_id() 返回 Some
    // sender: Option<String>,     // 若存在，sender() 返回 self.sender.as_deref()
    // 业务字段
    message: String,
}
```

| 宏属性 | 作用 |
|--------|------|
| `#[signal(kind="command", layer="exec")]` | 设置 `kind()` 与 `layer()` |
| `#[schema_version(N)]` | 设置 `schema_version()`，N 必须 ≥ 1 |
| `#[schema(skip)]` | 跳过默认空 `Schema` 实现，由你手写校验 |

### Vector Clock：因果追踪

每个 Signal 携带 `VectorClock`，用于判断消息间的因果关系：

```rust
let mut vc1 = VectorClock::new();
vc1.increment("cell-a"); // cell-a 处理了一次

let mut vc2 = vc1.clone();
vc2.increment("cell-b"); // cell-b 在 vc1 之后处理

assert!(vc1.causally_precedes(&vc2)); // vc1 因果先于 vc2
assert!(!vc2.causally_precedes(&vc1));
assert!(!vc1.concurrent_with(&vc2));
```

这让你能在分布式场景下识别"哪个消息先发生"，而不是依赖不可靠的物理时钟。

### SignalEnvelope：类型擦除传输

消息总线上传输的是 `SignalEnvelope`——把任意 Signal 序列化为 JSON 负载，并携带路由元数据：

```rust
pub struct SignalEnvelope {
    pub msg_id: MsgId,
    pub correlation_id: CorrelationId,
    pub trace_id: Option<TraceId>,
    pub signal_type: String,
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
    pub kind: SignalKind,
    pub source_layer: Layer,
    pub target_layer: Layer,
    pub source_cell: Option<String>,
    pub target_cell: Option<String>,
    pub payload: serde_json::Value,
    pub schema_version: SchemaVersion,
    pub parent_msg_id: Option<MsgId>,
    pub hop_count: u32,
}
```

`SignalEnvelope` 内置多项校验：`validate_layer_transition()` 检查层间方向、`validate_payload_size()` 限制体积、`increment_hop()` 防止消息无限跳转（最多 8 跳）。

---

## Axiom：全局不变量约束

`Axiom` 是确定性的纯函数（无 async、无 IO），用于校验状态转换是否违反系统不变量。它们是"熵的减压阀"——一旦违反，可触发 `Reject`、`Warn`、`CircuitBreak` 或 `Rollback`。

### 核心 trait

```rust
pub trait Axiom: Send + Sync {
    type State: 'static;
    type Message: 'static;

    fn name(&self) -> &'static str;

    fn check(&self, current: &Self::State, new: &Self::State, msg: &Self::Message) -> Result<()>;

    fn violation_action(&self) -> ViolationAction {
        ViolationAction::Reject // 默认拒绝
    }

    fn applies_to_layer(&self, _layer: Layer) -> bool {
        true // 默认对所有层生效
    }
}

pub enum ViolationAction {
    Reject,       // 拒绝该状态转换
    Warn,         // 仅告警，放行
    CircuitBreak, // 触发熔断
    Rollback,     // 回滚到上一个状态
}
```

### 用宏注册 Axiom

`#[axiom]` 宏会把 Axiom 注册到全局分布式切片（`linkme::distributed_slice`），运行时可按层查询：

```rust
#[axiom]
struct NonEmptyGreetingAxiom;

impl Axiom for NonEmptyGreetingAxiom {
    type State = Vec<String>;
    type Message = HelloCommand;

    fn name(&self) -> &'static str { "non-empty-greeting" }

    fn check(&self, _current: &Self::State, new: &Self::State, _msg: &Self::Message) -> Result<()> {
        if new.iter().any(|g| g.is_empty()) {
            return Err(AxiomError::InvariantViolated {
                message: "greeting must not be empty".into(),
            });
        }
        Ok(())
    }

    fn applies_to_layer(&self, layer: Layer) -> bool {
        matches!(layer, Layer::Exec | Layer::Validate)
    }
}
```

### 运行时查询与批量校验

`DynAxiomChain` 从注册表构建某层的 Axiom 链，对状态转换做批量校验：

```rust
let chain = DynAxiomChain::from_registry_for_layer(Layer::Exec);
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

### Axiom 与 Schema 的区别

| 维度 | Schema | Axiom |
|------|--------|-------|
| 作用对象 | 单条信号的字段结构 | 跨信号的状态不变量 |
| 执行时机 | 信号发出/接收时 | 状态转换前后 |
| 位置 | Layer 2 验证层 | 全局，可声明 `applies_to_layer` |
| 性质 | 数据校验（非空、长度、范围） | 业务不变量（余额非负、唯一性等） |
| 纯函数 | 是 | 是 |

---

## Witness：审计记录

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
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
    pub prev_hash: Option<WitnessHash>,      // 链接前一条
    pub state_before_hash: Option<WitnessHash>,
    pub state_after_hash: Option<WitnessHash>,
    pub hash: WitnessHash,                    // 本条哈希
    pub summary: String,
    pub outcome: TransitionOutcome,
    pub metrics: WitnessMetrics,
    pub version_info: VersionInfo,
    pub signal_fingerprint: [u8; 32],
    pub payload_size_bytes: usize,
}

pub enum TransitionOutcome {
    Success,
    Failed { reason: String },
    AxiomViolated { axiom_name: String, message: String },
}
```

### 用 WitnessBuilder 构造

在 Cell 的 `handle` 内通过上下文构造：

```rust
ctx.emit_witness(
    ctx.witness()
        .summary("processed greeting: Hello, Axiom!")
        .outcome(TransitionOutcome::Success)
        .processing_time_us(42)
)?;
```

或使用快捷方法：

```rust
ctx.emit_success("done")?;                    // 成功
ctx.emit_failure("db error", "timeout")?;     // 失败
ctx.emit_axiom_violation("non-empty", "empty greeting")?; // 违反约束
```

### 哈希链完整性校验

每条 Witness 的 `hash` 由 `prev_hash` + 内容计算得出，任何篡改都会破坏链条：

```rust
// 验证一批 Witness 是否构成完整链条
let ok = Witness::verify_chain_integrity(&witnesses);
assert!(ok);
```

### 为什么 Witness 重要

- **可调试**：通过 `correlation_id` 串联一次完整调用的所有状态转换。
- **可回放**：Witness 链 + 事件日志可实现事件溯源恢复。
- **可审计**：`state_before_hash` / `state_after_hash` 证明状态未被篡改。
- **可监控**：`TransitionOutcome::AxiomViolated` 直接喂给熵值计算。

---

## Lens：状态投影

`Lens` 是按需状态投影机制。与其把全部历史塞进上下文（导致上下文爆炸），Lens 让你从事件日志中**精确查询**所需的状态视图。

### 设计意图

在传统 Agent 架构中，LLM 的上下文里堆满了历史消息，导致 token 成本爆炸、注意力分散。Lens 反其道而行：

- **事件日志是真相源**（Source of Truth）。
- **Lens 是只读视图**，按需从事件日志派生。
- **渐进式披露**：只暴露当前任务需要的字段。
- **增量更新**：基于 VectorClock 自动失效缓存，只重新投影变化的部分。

### 与其他原语的关系

```
  事件日志（Append-Only）
         │
         ▼
   Lens（按需投影）──→ 返回精简状态视图
         │
         ▼
   供 LLM / 验证层 / 监督层消费
```

Lens 与 `axiom-store`（事件存储 crate）协作：`axiom-store` 负责持久化事件日志与快照，Lens 负责从日志中重建特定视图。

### 当前状态 (v0.1.0)

> **注意**：Lens 原语目前仅定义了 `LensId` 类型，完整实现在 v0.2.0 中交付。

### v0.2.0 设计目标

#### Lens Trait

```rust
pub trait Lens: Send + Sync + 'static {
    type Input;
    type Output;
    
    fn id(&self) -> &LensId;
    
    fn project(&self, events: &[Event], input: &Self::Input) -> Self::Output;
    
    fn cache_key(&self, input: &Self::Input) -> Option<String> { None }
    
    fn depends_on(&self) -> &[LensId] { &[] }
}
```

#### LensRegistry 自动注册

使用 `#[lens]` 宏自动注册到全局注册表：

```rust
#[lens]
struct OrderHistoryLens;

impl Lens for OrderHistoryLens {
    type Input = CustomerId;
    type Output = Vec<OrderSummary>;
    
    fn id(&self) -> &LensId { &LensId::new("order-history") }
    
    fn project(&self, events: &[Event], customer_id: &CustomerId) -> Vec<OrderSummary> {
        events
            .iter()
            .filter(|e| e.customer_id == *customer_id)
            .map(OrderSummary::from_event)
            .collect()
    }
}
```

#### ProjectionCache

Lens 结果自动缓存，基于 VectorClock 失效：

```rust
let cache = InMemoryProjectionCache::new();
let result = cache.get_or_compute(lens_id, input, || lens.project(events, input));
```

### 核心价值

| 价值 | 说明 |
|------|------|
| **避免上下文爆炸** | 只投影需要的状态，不塞全部历史 |
| **Token 预算感知** | 投影结果自动估算 Token 数，超预算时自动摘要 |
| **权限边界** | 编译期保证一个 Lens 只能看到授权的状态子集 |
| **时间旅行** | 支持查询任意历史时间点的状态（事件重放） |
| **可组合** | Lens 可以组合其他 Lens，像函数式编程的透镜组合子 |

详细实现计划请参考 [v0.2.0 开发计划](../plans/v0.2.0-development-plan.md)。

---

## 四层架构

Axiom Core 把系统划分为四个层级，每层职责不同，确定性程度不同。**调用方向只能从上往下**（Oversight → Agent → Validate → Exec），编译期与运行时双重检查。

```
┌─────────────────────────────────────────────────────┐
│  Layer 0: 监督层（Oversight）← 元层，监督一切       │
│  熵治理 · 架构合规 · 意图审计 · 资源管控 · 死锁检测  │
│  确定性：高（不执行业务逻辑，只监督）                │
├─────────────────────────────────────────────────────┤
│  Layer 3: 推理层（Agent/LLM）  ← 可以犯错            │
│  输出必须经过 Axiom 验证，不直接产生副作用           │
│  确定性：低（LLM 是非确定性的）                      │
├─────────────────────────────────────────────────────┤
│  Layer 2: 验证层（Validate）   ← 守门人              │
│  Schema 校验 · 规则引擎 · Axiom 不变量检查           │
│  确定性：高（纯函数校验）                            │
├─────────────────────────────────────────────────────┤
│  Layer 1: 执行层（Exec）       ← 不出错              │
│  数据库 · API 调用 · 计算，幂等 + 自动重试           │
│  确定性：高（幂等设计）                              │
└─────────────────────────────────────────────────────┘
```

### Layer 枚举

```rust
pub enum Layer {
    Oversight = 0, // 监督一切，不执行业务逻辑
    Agent = 3,     // LLM / 非确定性推理
    Validate = 2,  // Schema / 规则 / Axiom 校验
    Exec = 1,      // 确定性执行：DB / API / IO
}
```

注意：枚举数值并非 0/1/2/3 顺序，而是按"确定性"与"调用链"语义编排。调用链语义为 Oversight → Agent → Validate → Exec。

### 各层职责

| 层 | 数值 | 职责 | 确定性 | 示例 |
|----|------|------|--------|------|
| Oversight | 0 | 监督、熵治理、架构合规、死锁检测 | 高 | `EntropyGovernor`、`ArchitectureGuardian` |
| Agent | 3 | LLM 推理、规划、非确定性决策 | 低 | `AgentCell`（axiom-agent） |
| Validate | 2 | Schema 校验、规则引擎、Axiom 检查 | 高 | 校验信号的 `Schema::validate` |
| Exec | 1 | DB / API / IO，幂等执行 | 高 | `HelloCell`、数据库写入 Cell |

### 设计原则：确定性分层

- **能确定的事不放给 LLM**：Exec 层做幂等 DB 操作，不让 LLM 决定。
- **LLM 的输出必经校验**：Agent 层产出的信号要过 Validate 层的 Axiom，不直接产生副作用。
- **监督层不碰业务**：Oversight 只看熵值、合规、健康，不执行业务逻辑。
- **让能崩溃的崩溃**：Erlang 风格，Cell 崩溃由监督树重启，不传染。

---

## 层间调用约束规则

层间调用方向是 Axiom Core 的"铁律"，通过**编译期类型系统**与**运行时校验**双层保证。

### 方向矩阵（CanSendTo）

源层 → 目标层 的合法调用关系（`sealed.rs`）：

| 源层 \ 目标层 | Oversight | Agent | Validate | Exec |
|---------------|-----------|-------|----------|------|
| **Oversight** | ✅ | ✅ | ✅ | ✅ |
| **Agent** | ❌ | ✅ | ✅ | ❌ |
| **Validate** | ❌ | ✅ | ✅ | ✅ |
| **Exec** | ❌ | ❌ | ❌ | ✅ |

**核心规则**：

1. **Oversight 可向所有层发消息**——它是元层，监督一切。
2. **Agent 可向 Agent、Validate 发消息**——LLM 的输出要进验证层，不能直接执行。
3. **Validate 可向 Validate、Exec、Agent 发消息**——校验通过后下发执行，必要时可回传 Agent 做进一步推理。
4. **Exec 只能向 Exec 发消息**——执行层结果不回灌上层，避免副作用传染。

### 编译期强制：LayeredCellContext + CanSendTo

`LayeredCellContext<'a, L>` 包装 `CellContext`，通过 trait bound `L: CanSendTo<Target>` 在**编译期**拒绝非法调用：

```rust
impl<'a, L: LayerMarker> LayeredCellContext<'a, L> {
    pub fn send_to<Target: LayerMarker, S: Signal>(
        &mut self,
        signal: S,
        target_cell: &str,
    ) -> Result<()>
    where
        L: CanSendTo<Target>, // ← 编译期 trait bound
    { ... }

    pub fn emit_to<Target: LayerMarker, S: Signal>(
        &mut self,
        signal: S,
    ) -> Result<()>
    where
        L: CanSendTo<Target>, // ← 编译期 trait bound
    { ... }
}
```

`CanSendTo` 用 sealed trait 模式实现，下游无法扩展，保证方向矩阵穷尽：

```rust
// 只有这些组合实现了 CanSendTo，其他组合编译失败
impl CanSendTo<OversightLayer> for OversightLayer {}
impl CanSendTo<AgentLayer>     for OversightLayer {}
impl CanSendTo<ValidateLayer>  for OversightLayer {}
impl CanSendTo<ExecLayer>      for OversightLayer {}

impl CanSendTo<AgentLayer>     for AgentLayer {}
impl CanSendTo<ValidateLayer>  for AgentLayer {}

impl CanSendTo<ValidateLayer>  for ValidateLayer {}
impl CanSendTo<ExecLayer>      for ValidateLayer {}
impl CanSendTo<AgentLayer>     for ValidateLayer {}

impl CanSendTo<ExecLayer>      for ExecLayer {}
```

### 编译失败示例

如果你在 Exec 层 Cell 里尝试向 Agent 层发消息，**代码无法编译**：

```rust
#[cell("exec")]
impl Cell for MyExecCell {
    fn handle<'a>(&'a mut self, signal: ..., ctx: LayeredCellContext<'a, Self::Layer>) -> ... {
        async move {
            // ❌ 编译错误：ExecLayer 没有实现 CanSendTo<AgentLayer>
            ctx.emit_to::<AgentLayer, _>(some_signal)?;

            // ✅ 只能向同层（Exec）发消息
            ctx.emit_to::<ExecLayer, _>(some_event)?;
            Ok(())
        }
    }
}
```

### 运行时校验：Layer::can_send_to

即使绕过类型系统（如通过 `SignalEnvelope` 直接构造），运行时仍有兜底校验：

```rust
impl Layer {
    pub fn can_send_to(&self, target: Layer) -> bool {
        match self {
            Layer::Oversight => true,
            Layer::Agent => matches!(target, Layer::Agent | Layer::Validate),
            Layer::Validate => matches!(target, Layer::Validate | Layer::Exec | Layer::Agent),
            Layer::Exec => matches!(target, Layer::Exec),
        }
    }
}

// SignalEnvelope 自带校验
env.validate_layer_transition()?; // 违反方向则返回 LayerViolation 错误
```

### 违反方向的后果

运行时若检测到层间越界，返回 `AxiomError::LayerViolation`：

```rust
pub enum AxiomError {
    LayerViolation {
        from: Layer,
        to: Layer,
        signal_type: String,
        source_cell: String,
    },
    // ...
}
```

该错误会被监督层捕获，推高熵值，严重时触发熔断。

### Sealed 模式：防止扩展

`LayerMarker` 使用 sealed trait 模式，下游 crate 无法新增层标记，保证方向矩阵**穷尽且不可篡改**：

```rust
mod private {
    pub trait Sealed {}
    impl Sealed for super::OversightLayer {}
    impl Sealed for super::AgentLayer {}
    impl Sealed for super::ValidateLayer {}
    impl Sealed for super::ExecLayer {}
}

pub trait LayerMarker: private::Sealed + Send + Sync + 'static {
    const LAYER: Layer;
}
```

---

## 原语协作总览

下图展示一次典型的消息处理流程中，五个原语如何协作：

```
         ┌─────────────┐
信号 ───→ │   Signal    │  类型化消息，携带 VectorClock + correlation_id
         └──────┬──────┘
                │ 校验 Schema
                ▼
         ┌─────────────┐
         │  Validate   │  Layer 2: Schema + Axiom 校验
         └──────┬──────┘
                │ 校验通过，下发
                ▼
         ┌─────────────┐
         │    Cell     │  Layer 1/3: 处理消息，更新私有状态
         │  (handle)   │
         └──────┬──────┘
                │ 每次状态转换自动产出
                ├──────────────┐
                ▼              ▼
         ┌─────────────┐ ┌─────────────┐
         │  Witness    │ │   Axiom     │  审计记录 + 不变量检查
         │ (哈希链)    │ │ (熵减压阀)  │
         └──────┬──────┘ └──────┬──────┘
                │              │
                ▼              ▼
         ┌─────────────┐ ┌─────────────┐
         │  事件日志    │ │  熵值监控    │  Green/Yellow/Red/Critical
         └──────┬──────┘ └─────────────┘
                │ 按需投影
                ▼
         ┌─────────────┐
         │    Lens     │  返回精简状态视图
         └─────────────┘
```

### 一次处理的代码视角

```rust
fn handle<'a>(&'a mut self, signal: HelloCommand, ctx: LayeredCellContext<'a, Self::Layer>)
    -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a
{
    async move {
        let mut ctx = ctx;
        // 1. Signal 已通过 Schema 校验进入
        // 2. Cell 更新私有状态
        self.greetings.push(signal.message.clone());

        // 3. 发出后续事件（受 CanSendTo 约束）
        ctx.emit_to::<ExecLayer, _>(GreetedEvent::new(...))?;

        // 4. 产出 Witness 审计记录（自动加入哈希链）
        ctx.emit_witness(
            ctx.witness()
                .summary("processed greeting")
                .outcome(TransitionOutcome::Success)
        )?;

        // 5. 取出输出缓冲返回给运行时
        let (outgoing, witnesses) = ctx.end_processing();
        (Ok(()), outgoing, witnesses)
    }
}
```

运行时会把 `witnesses` 写入事件日志，把 `outgoing` 投递到目标 Cell，把 Axiom 违规与异常推入熵值计算。Lens 则在外部按需从事件日志投影状态。

---

## 小结

| 原语 | 一句话 | 关键 trait/类型 | 关键宏 |
|------|--------|-----------------|--------|
| **Cell** | 隔离状态单元 | `Cell`、`CellHandle`、`LayeredCellContext` | `#[cell("exec")]` |
| **Signal** | 类型化消息 | `Signal`、`SignalEnvelope`、`VectorClock` | `#[derive(SignalPayload)]`、`#[signal(...)]` |
| **Axiom** | 全局不变量 | `Axiom`、`DynAxiomChain`、`ViolationAction` | `#[axiom]` |
| **Witness** | 审计哈希链 | `Witness`、`WitnessBuilder`、`TransitionOutcome` | `ctx.emit_witness(...)` |
| **Lens** | 状态投影 | `LensId`、`Lens`、`ProjectionCache`（v0.2.0） | `#[lens]`（v0.2.0） |

四层架构通过 `Layer` 枚举 + `CanSendTo` sealed trait + `LayeredCellContext` 在**编译期**锁死调用方向，让架构违规根本无法编译。

下一节 [创建一个 Agent](./creating-an-agent.md) 将展示如何用 `AgentBuilder` 把这些原语组装成一个完整的智能体。
