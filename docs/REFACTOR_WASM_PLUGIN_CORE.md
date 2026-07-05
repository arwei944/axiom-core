# Axiom Core v0.4.0 — WASM 插件核心重构调研

> **Branch**: `feat/wasm-plugin-core`  
> **Version Target**: v0.4.0  
> **Date**: 2026-07-05  
> **Status**: 调研 / 设计阶段

---

## 1. 愿景：从核心延伸出万物

### 1.1 核心理念

Axiom Core 的终极形态不是"又一个 Agent 框架"，而是一个 **Agent 操作系统内核**：

- **最小内核**：只保留 Cell / Signal / Lens / Axiom / Witness 五个原语
- **一切皆插件**：LLM、Memory、MCP、Tool、Planner、Viz、Alert……都是运行时插件
- **自由组合**：通过配置或 DSL 描述系统结构，内核负责加载、连接、调度
- **热力可观测**：自动统计每个模块 / 原语的使用频率，可视化热点

### 1.2 与当前架构的质变

| 维度 | v0.3.0 (当前) | v0.4.0 (目标) |
|------|---------------|---------------|
| **模块加载** | 编译期静态依赖 | 运行时动态加载 |
| **扩展方式** | 新增 crate + 改 core | 实现 Plugin trait + 注册 |
| **Agent 构造** | `AgentBuilder` 硬编码组合 | 配置驱动 / DSL 描述 |
| **功能边界** | 18 个 crate 全量加载 | 核心 + 插件市场 |
| **可观测性** | Witness 链 + 指标 | Witness + 使用频率热力图 |
| **跨语言** | 纯 Rust | Rust core + WASM/JS/Go 插件 |

---

## 2. 为什么选择 WASM + 动态加载

### 2.1 动态加载的三种路径

| 方案 | 优势 | 劣势 | 适用场景 |
|------|------|------|----------|
| **Native dlopen/LoadLibrary** | 性能最优 | 平台差异大，安全风险高 | 系统级应用 |
| **Rust 宏 + 编译期注册** | 类型安全，零运行时开销 | 仍需编译，不够灵活 | 框架扩展 |
| **WASM + Component Model** | 安全沙箱，跨语言，热加载 | 运行时开销，生态早期 | **我们的选择** |

### 2.2 WASM 的优势

1. **安全沙箱**：插件崩溃不会影响内核
2. **跨语言**：用 Rust / Go / JS / Python 写插件，内核统一调度
3. **热加载**：无需重启，动态替换插件
4. **资源可控**：内存 / CPU / 调用次数可限制
5. **可验证**：WASM 字节码可静态分析，符合"约束者必先受约束"

### 2.3 风险与缓解

| 风险 | 缓解策略 |
|------|----------|
| **性能开销** | 核心原语保留 Native，只有可选功能走 WASM |
| **生态不成熟** | 先用 `wasmtime`/`wasmtime-wasi` 稳定 API，Component Model 渐进采用 |
| **类型边界** | 定义严格的 `PluginABI`，通过 WASM 线性内存传递序列化数据 |
| **调试困难** | 提供 WASM 插件模拟器，在 Native 模式下调试 |

---

## 3. 新核心架构设计

### 3.1 极简内核

```
┌─────────────────────────────────────────────────────────────┐
│                    Axiom Kernel (v0.4.0)                    │
│                                                             │
│   ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐  │
│   │  Cell       │  │  Signal     │  │  Lens / Axiom /   │  │
│   │  Kernel     │  │  Kernel     │  │  Witness Kernel   │  │
│   └──────┬──────┘  └──────┬──────┘  └────────┬──────────┘  │
│          │                │                   │              │
│   ┌──────▼────────────────▼───────────────────▼──────────┐  │
│   │              Plugin Runtime (WASM Host)              │  │
│   │  - Plugin Loader  - Plugin Registry  - Composer      │  │
│   └──────────────────────────────────────────────────────┘  │
│                                                             │
│   ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐  │
│   │  Entropy     │  │  Supervisor │  │  Heatmap / Telemetry│ │
│   │  Governor    │  │  Kernel     │  │                    │  │
│   └─────────────┘  └─────────────┘  └───────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 核心层定义

#### Layer 0: Primitives Kernel（不可变）
- `Cell`：状态单元，消息信箱
- `Signal`：类型化消息，Vector Clock
- `Lens`：状态投影
- `Axiom`：全局约束
- `Witness`：审计记录

#### Layer 1: Governance Kernel（可选插件）
- `EntropyGovernor`：熵治理
- `Supervisor`：监督自愈
- `ArchitectureGuardian`：架构守护

#### Layer 2: Extension Kernel（可选插件）
- `LLMProvider`：LLM 接口
- `MemoryStore`：记忆存储
- `ToolExecutor`：工具执行
- `MCPBridge`：MCP 协议桥接
- `Planner`：规划策略
- `AlertRouter`：告警路由

#### Layer 3: Observability（可选插件）
- `HeatmapCollector`：使用频率热力收集
- `VizExporter`：可视化导出
- `MetricsServer`：Prometheus 端点

### 3.3 Plugin ABI（应用二进制接口）

每个插件必须实现以下 trait：

```rust
/// Axiom Plugin ABI - 所有插件的契约
pub trait AxiomPlugin: Send + Sync {
    /// 插件唯一标识
    fn id(&self) -> &'static str;
    
    /// 插件版本
    fn version(&self) -> &'static str;
    
    /// 依赖的其他插件 ID
    fn dependencies(&self) -> &[&'static str];
    
    /// 插件能力声明
    fn capabilities(&self) -> &[CapabilityDescriptor];
    
    /// 初始化插件，接收内核引用
    fn init(&mut self, ctx: PluginContext) -> Result<(), PluginError>;
    
    /// 启动插件
    fn start(&mut self) -> Result<(), PluginError>;
    
    /// 停止插件
    fn stop(&mut self) -> Result<(), PluginError>;
    
    /// 处理插件间消息
    fn handle_message(&mut self, msg: PluginMessage) -> Result<PluginReply, PluginError>;
}
```

#### PluginContext（内核提供给插件的上下文）

```rust
pub struct PluginContext {
    /// 访问 Cell 内核
    pub cells: CellKernel,
    /// 访问 Signal 内核
    pub signals: SignalKernel,
    /// 访问 Lens 内核
    pub lens: LensKernel,
    /// 访问 Axiom 内核
    pub axioms: AxiomKernel,
    /// 访问 Witness 内核
    pub witness: WitnessKernel,
    /// 访问其他插件
    pub plugins: PluginRegistry,
    /// 日志
    pub logger: PluginLogger,
    /// 指标
    pub metrics: PluginMetrics,
}
```

### 3.4 Composer：积木式组合引擎

```rust
/// 系统组合描述
pub struct SystemComposition {
    /// 系统名称
    pub name: String,
    /// 核心插件列表
    pub plugins: Vec<PluginSpec>,
    /// 连接关系：谁向谁发送消息
    pub connections: Vec<ConnectionSpec>,
    /// 资源限制
    pub resources: ResourceSpec,
}

/// 插件规格
pub struct PluginSpec {
    pub id: String,
    pub kind: PluginKind,
    pub config: toml::Value,
    pub instance: usize,  // 支持多实例
}

/// 连接规格
pub struct ConnectionSpec {
    pub from: (String, Port),
    pub to: (String, Port),
    pub transform: Option<String>,
}
```

**DSL 示例**：

```toml
[system]
name = "my-agent"

[[plugins]]
id = "llm-openai"
kind = "llm"
config.api_key = "env:OPENAI_API_KEY"
instance = 1

[[plugins]]
id = "memory-redis"
kind = "memory"
config.url = "env:REDIS_URL"
instance = 1

[[plugins]]
id = "tool-web-search"
kind = "tool"
instance = 3  # 3 个实例并行

[[connections]]
from = "agent-cell:outgoing"
to = "llm-openai:incoming"

[[connections]]
from = "llm-openai:outgoing"
to = "memory-redis:store"

[[connections]]
from = "memory-redis:retrieve"
to = "llm-openai:context"
```

---

## 4. 使用频率热力显示

### 4.1 热力数据模型

```rust
/// 使用频率指标
pub struct UsageMetrics {
    /// 每个 Cell 的消息接收量
    pub cell_message_count: HashMap<CellId, u64>,
    /// 每个 Signal 类型的发送量
    pub signal_type_count: HashMap<String, u64>,
    /// 每个 Tool 的调用量
    pub tool_invocation_count: HashMap<String, u64>,
    /// 每个 Axiom 的检查量
    pub axiom_check_count: HashMap<String, u64>,
    /// 每个 Lens 的查询量
    pub lens_query_count: HashMap<String, u64>,
    /// 时间序列采样
    pub timeline: Vec<UsageSnapshot>,
}

/// 单次快照
pub struct UsageSnapshot {
    pub timestamp: u64,
    pub hot_cells: Vec<(CellId, u64)>,
    pub hot_signals: Vec<(String, u64)>,
    pub hot_tools: Vec<(String, u64)>,
}
```

### 4.2 热力可视化

| 视图 | 实现方式 | 用途 |
|------|----------|------|
| **拓扑热力图** | `axiom-viz` 导出 + 着色 | 一眼看出瓶颈 Cell |
| **时间轴热点** | Timeline + 颜色映射 | 发现峰值时段 |
| **模块排名** | CLI `axm heatmap` | 识别未使用模块 |
| **实时仪表盘** | Prometheus + Grafana | 生产监控 |

### 4.3 CLI 命令

```bash
# 实时热力显示
axm heatmap --live

# 导出 JSON
axm heatmap --export heatmap.json

# 按模块过滤
axm heatmap --module llm --module memory

# 时间范围
axm heatmap --since 1h --until now
```

---

## 5. 核心 API 设计

### 5.1 Kernel API

```rust
/// 极简内核，不可变
pub struct AxiomKernel {
    cells: CellKernel,
    signals: SignalKernel,
    lens: LensKernel,
    axioms: AxiomKernel,
    witness: WitnessKernel,
}

impl AxiomKernel {
    /// 创建内核
    pub fn new() -> Self;
    
    /// 加载插件组合
    pub fn compose(&mut self, spec: SystemComposition) -> Result<(), KernelError>;
    
    /// 启动系统
    pub fn start(&mut self) -> Result<(), KernelError>;
    
    /// 停止系统
    pub fn stop(&mut self) -> Result<(), KernelError>;
}
```

### 5.2 Plugin API

```rust
/// 插件 trait
pub trait AxiomPlugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn dependencies(&self) -> &[&'static str];
    fn capabilities(&self) -> &[CapabilityDescriptor];
    fn init(&mut self, ctx: PluginContext) -> Result<(), PluginError>;
    fn start(&mut self) -> Result<(), PluginError>;
    fn stop(&mut self) -> Result<(), PluginError>;
    fn handle_message(&mut self, msg: PluginMessage) -> Result<PluginReply, PluginError>;
}

/// 插件加载器
pub trait PluginLoader {
    fn load(&self, path: &Path) -> Result<Box<dyn AxiomPlugin>, PluginError>;
    fn unload(&self, plugin: &dyn AxiomPlugin) -> Result<(), PluginError>;
}
```

### 5.3 Heatmap API

```rust
/// 热力收集器
pub trait HeatmapCollector {
    fn record_cell_message(&mut self, cell_id: CellId);
    fn record_signal_send(&mut self, signal_type: &str);
    fn record_tool_invoke(&mut self, tool_id: &str);
    fn record_axiom_check(&mut self, axiom_id: &str);
    fn record_lens_query(&mut self, lens_id: &str);
    
    fn snapshot(&self) -> UsageSnapshot;
    fn timeline(&self, range: TimeRange) -> Vec<UsageSnapshot>;
    fn top_cells(&self, n: usize) -> Vec<(CellId, u64)>;
    fn top_tools(&self, n: usize) -> Vec<(String, u64)>;
}
```

---

## 6. WASM 插件实现路径

### 6.1 Phase 1：Native 插件原型（2-3 周）

**目标**：验证 Plugin ABI 和 Composer 设计

- 定义 `AxiomPlugin` trait
- 实现 Native 插件加载器（`dlopen` / `LoadLibrary`）
- 实现 Composer：从 TOML 配置加载插件
- 实现 2-3 个示例插件（LLM Mock、Memory In-Memory、Tool Echo）

### 6.2 Phase 2：WASM 运行时（3-4 周）

**目标**：用 WASM 替代 Native 插件

- 集成 `wasmtime` / `wasmtime-wasi`
- 定义 WASM 插件 ABI（通过 wasm 导出函数）
- 实现 WASM 插件加载器
- 实现插件间通信（通过内核转发）

### 6.3 Phase 3：热力系统（2 周）

**目标**：使用频率统计与可视化

- 在 Kernel 中埋点
- 实现 `HeatmapCollector`
- CLI `axm heatmap` 命令
- `axiom-viz` 热力图导出

### 6.4 Phase 4：插件市场与工具链（持续）

- 插件打包格式（`.wasm` + manifest）
- 插件版本管理与依赖解析
- `axm plugin install/uninstall/list`
- 插件开发 SDK（Rust / Go / JS）

---

## 7. 迁移路径

### 7.1 兼容性策略

| 方面 | 策略 |
|------|------|
| **API 兼容** | v0.3.0 Stable API 继续维护，v0.4.0 新 API 在 `unstable` 后稳定 |
| **数据兼容** | Witness 链、Event 日志格式不变 |
| **渐进迁移** | 上层模块可同时支持 v0.3.0 和 v0.4.0 API |

### 7.2 迁移步骤

1. **v0.4.0-alpha**：新内核 + 插件系统，旧 API 兼容层
2. **v0.4.0-beta**：核心模块迁移为插件，测试覆盖
3. **v0.4.0-stable**：旧 API 标记 deprecated，新 API 稳定
4. **v0.5.0**：移除旧 API，完成迁移

---

## 8. 目录结构草案

```
crates/
  axiom-kernel/           # 新极简内核
    src/
      lib.rs
      cell.rs
      signal.rs
      lens.rs
      axiom.rs
      witness.rs
      plugin/
        mod.rs
        abi.rs
        loader/
          mod.rs
          native.rs
          wasm.rs
        registry.rs
        composer.rs
      heatmap/
        mod.rs
        collector.rs
        exporter.rs
  axiom-plugin-macros/    # 插件开发宏
  axiom-plugin-sdk/       # 插件 SDK

tools/
  axm-plugin/             # 插件管理 CLI
    src/
      install.rs
      uninstall.rs
      list.rs
      pack.rs

docs/
  REFACTOR_WASM_PLUGIN_CORE.md  # 本文档
  PLUGIN_DEVELOPMENT.md         # 插件开发指南
  MIGRATION_V0.3_TO_V0.4.md     # 迁移指南
```

---

## 9. 风险与决策

| 风险 | 影响 | 缓解 | 决策 |
|------|------|------|------|
| **WASM 性能** | 高频路径延迟 | 核心原语保留 Native | 接受 |
| **生态不成熟** | 插件生态弱 | 提供 Native + WASM 双模式 | 接受 |
| **复杂度增加** | 学习曲线陡峭 | 文档 + SDK + 示例 | 接受 |
| **迁移成本** | 现有用户需适配 | 兼容层 + 迁移指南 | 接受 |

---

## 10. 下一步行动

1. **评审此文档**，确认方向正确
2. **实现 Phase 1**：Native 插件原型 + Composer
3. **写插件开发指南**，降低门槛
4. **社区反馈**：邀请核心用户试用 alpha

---

## 11. 参考

- [wasmtime](https://github.com/bytecodealliance/wasmtime)
- [WASI](https://wasi.dev/)
- [WebAssembly Component Model](https://github.com/WebAssembly/component-model)
- [tokio::task::JoinSet](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html) - 动态任务管理
- [rust-Plugin-System](https://github.com/bytecodealliance/rusty-plugin-system) - Rust 插件系统参考

---

*本文档是 v0.4.0 重构的起点，所有设计决策基于当前 v0.3.0 的实践经验。*
