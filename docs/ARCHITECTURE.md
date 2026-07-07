# 架构文档

本文档详细描述 Axiom Core v0.4.0 的架构设计，包括分层结构、核心组件、数据流向、架构治理机制等。

---

## 目录

- [设计原则](#设计原则)
- [分层架构](#分层架构)
- [核心组件详解](#核心组件详解)
- [数据流向](#数据流向)
- [架构治理](#架构治理)
- [注册表系统](#注册表系统)
- [插件系统](#插件系统)
- [热图系统](#热图系统)
- [错误处理](#错误处理)
- [性能优化](#性能优化)

---

## 设计原则

### 1. 确定性优先
- 能确定的事情不放给 LLM
- 所有约束在编译期注入
- 运行时行为可预测、可验证

### 2. 编译期强制
- 层间调用方向编译期检查
- 类型安全的 ID 系统
- 自动注入必需字段和验证逻辑

### 3. 低熵化
- 熵值作为第一公民
- 实时监控、自动告警、主动减熵
- 违反约束即熔断

### 4. 可观测性
- 每次状态转换自动产生 Witness
- 完整的链路追踪
- 信号热图可视化

### 5. 自愈能力
- Erlang 风格"让它崩溃"
- 监督树自动重启
- 事件溯源恢复

---

## 分层架构

Axiom Core 采用 **9 层分层架构**，严格遵循单向依赖原则。

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 0: 顶层应用 (axiom-cli, axiom-bench)                 │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: 可视化 (axiom-viz)                               │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: Agent 门面 (axiom-identity, axiom-prompt)         │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: 监督与集成 (axiom-agent, axiom-oversight,        │
│                       axiom-alert, axiom-mcp)               │
├─────────────────────────────────────────────────────────────┤
│  Layer 4: 运行时与协调 (axiom-runtime, axiom-planner,       │
│                        axiom-distributed)                   │
├─────────────────────────────────────────────────────────────┤
│  Layer 5: 存储与工具 (axiom-store, axiom-tool,              │
│                       axiom-memory, axiom-llm)              │
├─────────────────────────────────────────────────────────────┤
│  Layer 7: 核心原语 (axiom-kernel) ⭐                        │
│           (axiom-core - deprecated)                         │
├─────────────────────────────────────────────────────────────┤
│  Layer 8: Proc-macro (axiom-macros) - 豁免层                │
├─────────────────────────────────────────────────────────────┤
│  Layer 9: Plugin SDK (axiom-plugin-wasm-sdk)               │
│           Plugin 示例                                       │
└─────────────────────────────────────────────────────────────┘
```

### 层间依赖规则

**铁律**：Layer N 的 crate **只能依赖** Layer >= N 的 crate。

| Layer | Crate | 职责 |
|-------|-------|------|
| 0 | axiom-cli, axiom-bench | 命令行工具、基准测试 |
| 1 | axiom-viz | 运行时可视化 |
| 2 | axiom-identity, axiom-prompt | Agent 身份管理、Prompt 模板 |
| 3 | axiom-agent, axiom-oversight, axiom-alert, axiom-mcp | Agent 协调、监督、告警、MCP |
| 4 | axiom-runtime, axiom-planner, axiom-distributed | 运行时、计划执行、分布式 |
| 5 | axiom-store, axiom-tool, axiom-memory, axiom-llm | 存储、工具、记忆、LLM |
| 7 | axiom-kernel | 核心原语（五大原语 + Plugin + Heatmap） |
| 8 | axiom-macros | 过程宏（豁免层） |
| 9 | axiom-plugin-wasm-sdk | WASM 插件 SDK |

---

## 核心组件详解

### axiom-kernel

`axiom-kernel` 是整个架构的核心原语层，提供所有基础运行时能力。

#### 模块结构

| 模块 | 职责 | 关键类型 |
|------|------|----------|
| **cell.rs** | Cell 单元抽象 | `Cell`, `CellHandle`, `DynCell` |
| **signal.rs** | 信号定义与序列化 | `Signal`, `SignalEnvelope`, `VectorClock` |
| **axiom.rs** | 约束验证引擎 | `Axiom`, `DynAxiom`, `KernelError` |
| **witness.rs** | 见证链系统 | `Witness`, `WitnessKernel`, `WitnessHash` |
| **guard.rs** | 拦截器/守卫 | `Guard`, `DynGuard`, `BoxedGuard` |
| **lens.rs** | 状态投影 | `Lens`, `DynLens` |
| **registry.rs** | 分布式注册表 | `AXIOM_REGISTRY`, `CAPABILITY_REGISTRY` |
| **plugin/** | 插件系统 | `PluginRegistry`, `WasmPluginLoader`, `NativePluginLoader` |
| **heatmap/** | 信号热图 | `HeatmapCollector`, `HeatmapExporter` |
| **gate.rs** | 架构门禁 | `crate_layers`, `verify_dependencies` |
| **error.rs** | 错误类型 | `KernelError` |
| **context.rs** | 上下文管理 | `CellContext`, `LayeredCellContext` |

#### 五大原语

##### 1. Cell

```rust
pub trait Cell: Send + 'static {
    type Message: Signal;
    type Layer: LayerMarker;
    
    fn id(&self) -> &CellId;
    fn handle<'a>(&'a mut self, signal: Self::Message, ctx: LayeredCellContext<'a, Self::Layer>)
        -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a;
}
```

##### 2. Signal

```rust
pub trait Signal: Send + Sync + 'static {
    fn signal_type(&self) -> &'static str;
    fn msg_id(&self) -> &MsgId;
    fn correlation_id(&self) -> &CorrelationId;
    fn vector_clock(&self) -> &VectorClock;
    fn kind(&self) -> SignalKind;
    fn layer(&self) -> Layer;
}
```

##### 3. Axiom

```rust
pub trait Axiom: Send + Sync {
    type State: 'static;
    type Message: 'static;
    
    fn name(&self) -> &'static str;
    fn check(&self, current: &Self::State, new: &Self::State, msg: &Self::Message) -> Result<()>;
}
```

##### 4. Witness

```rust
pub struct Witness {
    pub witness_id: WitnessId,
    pub correlation_id: CorrelationId,
    pub prev_hash: Option<WitnessHash>,
    pub hash: WitnessHash,
    pub summary: String,
    pub outcome: TransitionOutcome,
    // ...
}
```

##### 5. Lens

```rust
pub trait Lens: Send + Sync + 'static {
    type Input;
    type Output;
    
    fn id(&self) -> &LensId;
    fn project(&self, events: &[Event], input: &Self::Input) -> Self::Output;
}
```

---

## 数据流向

### 一次典型的消息处理流程

```
用户请求
    │
    ▼
┌──────────────┐
│  axiom-cli   │  (CLI 入口)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ axiom-agent  │  (Agent 协调)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│axiom-runtime │  (运行时调度)
│              │
│  ┌────────┐  │
│  │ 总线   │  │  SignalEnvelope 分发
│  │ 监督树 │  │  拦截器链处理
│  │ 信箱   │  │
│  └────────┘  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ axiom-kernel │  (核心原语)
│              │
│  Cell 处理   │
│  Signal 校验 │
│  Axiom 检查  │
│  Witness 记录│
│  Lens 投影   │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ axiom-store  │  (事件持久化)
└──────────────┘
```

### 层间调用方向

```
Oversight ──→ Agent ──→ Validate ──→ Exec
     │           │           │
     └───────────┴───────────┘
           只能向下调用
```

---

## 架构治理

### 架构门禁 (`gate.rs`)

架构门禁确保代码库的架构一致性：

1. **分层验证**：验证依赖方向正确
2. **约束完整性**：验证 `.axiom/` 目录下文件哈希
3. **unsafe 审计**：强制要求所有 unsafe 块有 `// SAFETY:` 注释
4. **依赖审计**：验证所有第三方依赖已审计

### 逆向依赖豁免

允许特定 crate 打破分层规则：

```toml
[reverse-dependency-exemptions]
axiom-agent = { allowed_deps = ["axiom-identity", "axiom-prompt"], reason = "Agent 需要调用 facade" }
```

### 架构验证命令

```bash
axm verify

# 输出：
# === axiom verify (architecture constraints) ===
#   ✓ constraints integrity (hash check)
#   ✓ TODO/FIXME scan
#   ✓ unsafe code audit
#   ✓ third-party dependency audit
#   ✓ architecture dependency verification
# 5/5 architecture checks passed
```

---

## 注册表系统

使用 `linkme` 的 `#[distributed_slice]` 实现编译时注册：

```rust
#[distributed_slice]
pub static AXIOM_REGISTRY: [&'static dyn DynAxiom] = [];

#[distributed_slice]
pub static CAPABILITY_REGISTRY: [&'static dyn DynCapability] = [];
```

### 注册机制

1. **编译期注册**：宏展开时自动添加到分布式切片
2. **零运行时开销**：无需初始化函数
3. **类型安全**：编译期检查注册项类型

### 查询方式

```rust
let chain = DynAxiomChain::from_registry_for_layer(Layer::Exec);
let capabilities: Vec<_> = CAPABILITY_REGISTRY.iter().copied().collect();
```

---

## 插件系统

v0.4.0 新增 WASM 插件系统，支持运行时动态加载插件。

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

## 热图系统

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

---

## 错误处理

### KernelError

```rust
pub enum KernelError {
    InvariantViolated { message: String },
    LayerViolation { from: Layer, to: Layer, ... },
    SignalValidationFailed { errors: Vec<String> },
    CellNotFound { cell_id: CellId },
    PluginLoadFailed { path: String, error: String },
    // ...
}
```

### 错误传播

- **编译期错误**：层间调用违规、类型不匹配
- **运行时错误**：信号校验失败、Cell 未找到、插件加载失败
- **熵值影响**：错误自动推高熵值，严重时触发熔断

---

## 性能优化

### 编译时优化

1. **编译期注册表**：零运行时注册开销
2. **类型擦除**：`DynCell`、`DynAxiom` 等 trait 对象
3. **宏展开**：自动生成样板代码，减少手动编写

### 运行时优化

1. **tokio RwLock**：更好的异步调度
2. **parking_lot Mutex**：更快的同步原语
3. **批量操作**：减少锁获取次数
4. **采样机制**：热图可配置采样率

### 内存优化

1. **对象池化**：减少频繁分配
2. **VecDeque**：高效的消息队列
3. **HashMap**：O(1) 的查找性能

---

## 总结

Axiom Core v0.4.0 的架构设计体现了以下核心价值：

1. **确定性优先**：编译期强制约束，运行时行为可预测
2. **低熵化**：熵值作为第一公民，自动监控和减熵
3. **可观测性**：完整的审计链和链路追踪
4. **可扩展性**：WASM 插件系统支持运行时扩展
5. **自愈能力**：监督树自动重启，事件溯源恢复
6. **架构治理**：自动验证依赖方向和约束完整性

这种设计使 Axiom Core 成为构建生产级智能体系统的理想选择。