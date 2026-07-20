# Axiom Core

> **架构就是一切。** 一切失败都是架构失败；一切成功都是架构成功。

**Axiom Core** 是一个面向智能体（Agent）的确定性优先运行时架构——用五个核心原语构建低熵、可观测、可自愈的多智能体系统。

## ULE commercial path (single kernel) — U3–U5 complete

This monorepo is the **only host** for the Unified Low-Entropy (ULE) product.  
**`low-entropy-core` is archived/read-only assets** (see `../low-entropy-core/ARCHIVED.md`) — **not** a peer runtime for new work.

| Pillar | Authority |
|--------|-----------|
| Host | **AxiomRuntime** (Rust) |
| History | **Witness** only (no dual ExecutionStep authority) |
| Admit / entropy | **Governor** only (`axiom_isa::product_decide` / `product_admit`) |
| Business ISA | **Atom / Port / Adapter / Composer** (Composer-in-Cell) |
| Agent transfer | **HandoffRequest** as Signal payload + controlled Workbench |
| Observation | Single surface `/api/v1/surface` (not dual dashboards) |

Do **not** run a long-lived dual runtime or federation bridge as steady state.  

| 文档 | 说明 |
|------|------|
| [`AGENTS.md`](AGENTS.md) | **智能体根契约（自动加载 / 强制）** |
| [`docs/guide/agent-work-guide.md`](docs/guide/agent-work-guide.md) | 智能体工作指导与约束（细则） |
| [`docs/guide/AGENT_ONBOARDING_PACK.md`](docs/guide/AGENT_ONBOARDING_PACK.md) | **新智能体入职包（文档+门禁+DoD）** |
| [`docs/guide/frontend-integration.md`](docs/guide/frontend-integration.md) | 前端对接 / Surface vs gateway |
| [`docs/guide/secrets-and-llm.md`](docs/guide/secrets-and-llm.md) | 密钥与 LLM 环境变量 |
| [`.github/workflows/architecture-gates.yml`](.github/workflows/architecture-gates.yml) | **CI 架构门禁（archcheck / discipline / path）** |
| [`docs/COMMERCIAL_OPS.md`](docs/COMMERCIAL_OPS.md) | 运维 / 健康 / 鉴权 / 部署 |
| [`docs/ENGINEERING_HARDENING_v050.md`](docs/ENGINEERING_HARDENING_v050.md) | v0.5.0 工程清单与生产接线 |
| [`docs/TASK_CHECKLIST.md`](docs/TASK_CHECKLIST.md) | 升级任务清单（open = 0） |
| [`docs/unified/COMMERCIAL_DELIVERY.md`](docs/unified/COMMERCIAL_DELIVERY.md) | 商用交付说明 |
| [`docs/unified/FEATURE_THEME_MATRIX.md`](docs/unified/FEATURE_THEME_MATRIX.md) | 主题 T1–T15 **完全满足** |
| [`docs/unified/UNIFIED_MODEL.md`](docs/unified/UNIFIED_MODEL.md) | ULE 宪法 |
| [`docs/unified/DUAL_GOVERNOR_NOTE.md`](docs/unified/DUAL_GOVERNOR_NOTE.md) | 产品 admit 唯一说明 |
| [`docs/openapi.yaml`](docs/openapi.yaml) | OpenAPI 0.5.0（含 write/SSE） |

**Version:** workspace **0.5.0** · tag **v0.5.0-commercial**

```powershell
# 核心包测试（工程硬化 + ULE）
cargo test -p axiom-kernel -p axiom-runtime -p axiom-store --lib
cargo test -p axiom-isa -p axiom-resilience -p axiom-demo-taskflow

# 商用 CLI
cargo run -p axiom-demo-taskflow -- success        # task path
cargo run -p axiom-demo-taskflow -- handoff        # U3 agent path
cargo run -p axiom-demo-taskflow -- gateway        # write + SSE + ops shell
cargo run -p axiom-demo-taskflow -- surface        # same gateway floor
cargo run -p axiom-demo-taskflow -- health
```

## 为什么需要 Axiom Core？

UC Berkeley 对 1642+ 条多智能体执行轨迹的研究发现：**41%–86.7% 的失败源于架构缺陷，而非 AI 能力不足**。现有智能体框架（LangChain、CrewAI、AutoGPT 等）本质是"把 LLM 调用串起来"的工具库，没有解决分布式系统的经典问题——状态一致性、故障隔离、因果追踪、架构约束——这些问题在非确定性的 LLM 场景下被指数级放大。

Axiom Core 从底层重新设计，解决以下核心痛点：

| 痛点 | Axiom Core 的解法 |
|------|------------------|
| 🔴 **黑盒运行** | 每次状态转换自动产生 Witness（不可篡改审计记录），一秒定位根因 |
| 🔴 **静默退化** | 熵值实时监控，黄线告警、红线熔断、自动减熵 |
| 🔴 **消息字符串传递** | Signal 类型安全 + Vector Clock 因果追踪 |
| 🔴 **上下文爆炸** | Lens 按需投影状态，渐进式披露 Skill 元数据 |
| 🔴 **错误传染** | 四层架构 + 监督树 + Axiom 硬约束，故障不扩散 |
| 🔴 **无法自愈** | Erlang 风格"让它崩溃"+ 监督树自动重启 + 事件溯源恢复 |
| 🔴 **调试地狱** | `axm why` 一秒速查，Witness 链即"时间线录像机" |
| 🔴 **工具碎片化** | 内置 Identity/Skill/Rules/MCP 完整配套，不用从零造轮子 |

## 核心理念

- **可视化**：系统内部状态像仪表盘一样一目了然
- **工程化**：生产级运行时——可监控、可调试、可回滚
- **结构化**：一切有类型、有边界、有 Schema
- **极简化**：五个原语构建整个系统
- **低熵化**：熵是第一公民，可度量、可监控、可主动消减
- **一秒速查**：任何问题一秒内定位根因
- **自愈化**：局部崩溃自动恢复，不扩散
- **架构就是一切**：模型会犯错，架构不能；约束者必先受约束
- **智能体专用**：从零为 Agent 设计，不是把 Web 框架硬套上去

## 五大核心原语

```
┌─────────────────────────────────────────────────────────┐
│  Cell  │ 隔离的状态单元——私有状态 + 消息信箱，单线程执行 │
├────────┼────────────────────────────────────────────────┤
│ Signal │ 类型化不可变消息——Vector Clock + 链路追踪       │
├────────┼────────────────────────────────────────────────┤
│  Lens  │ 按需状态投影——不是塞全部历史，而是精确查询      │
├────────┼────────────────────────────────────────────────┤
│  Axiom │ 全局不变量约束——违反即熔断，熵的减压阀          │
├────────┼────────────────────────────────────────────────┤
│Witness │ 不可篡改审计链——每次状态转换自动记录            │
└─────────────────────────────────────────────────────────┘
```

## 自动注入机制（硬约束）

所有约束在**编译期自动注入**，无需手动调用 API：

```rust
#[axiom_kernel::signal]          // 自动添加 msg_id/correlation_id/vector_clock
struct GreetingSignal {
    message: String,
}

#[axiom_kernel::cell("exec")]    // 自动注入层标记 + Witness记录
impl Cell for MyCell { ... }

#[axiom_kernel::tool(permission = "read")]  // 自动注入权限检查 + Witness记录
struct DatabaseTool;

#[axiom_kernel::guard(layer = "exec")]     // 自动注入检查逻辑 + Witness记录
struct RateLimitGuard;

#[axiom_kernel::capability(dim = "witness", version = "1.0.0")]  // 自动版本注册
struct WitnessCapability;
```

| 宏 | 自动注入内容 |
|----|-------------|
| `#[signal]` | 必需字段、`Signal` trait、`Schema`验证、序列化 |
| `#[cell]` | 层标记、`LayerOf`、`WitnessGenerator` |
| `#[tool]` | `Tool` trait、权限检查、Witness记录 |
| `#[guard]` | `Guard` trait、检查逻辑、Witness记录 |
| `#[capability]` | 版本注册、兼容性策略、迁移链关联（8个维度） |

### 8大能力维度

| 维度 | 用途 | 典型场景 |
|------|------|---------|
| **Witness** | 审计链版本 | 状态转换记录格式 |
| **Schema** | 信号协议版本 | 消息序列化格式 |
| **Layer** | 架构层版本 | 层间调用规则 |
| **Tool** | 工具接口版本 | 工具执行协议 |
| **Guard** | 约束规则版本 | 权限检查规则 |
| **Identity** | 身份协议版本 | Agent身份/权限集 |
| **Entropy** | 熵治理版本 | 阈值策略/治理动作 |
| **Runtime** | 运行时协议版本 | 监督策略/邮箱配置 |

## 架构总览

Axiom Core 采用 **9 层分层架构**，所有架构规则定义在 [`.axiom/architecture.toml`](.axiom/architecture.toml) 中。

```
Crate Layer 0: 顶层应用 — axiom-cli, axiom-bench
Crate Layer 1: 可视化   — axiom-viz
Crate Layer 2: Agent 门面 — axiom-identity, axiom-prompt
Crate Layer 3: 监督与集成 — axiom-mcp, axiom-alert, axiom-agent, axiom-oversight
Crate Layer 4: 运行时与协调 — axiom-distributed, axiom-planner, axiom-runtime
Crate Layer 5: 存储与工具 — axiom-llm, axiom-tool, axiom-memory, axiom-store
Crate Layer 6: （预留）
Crate Layer 7: 核心原语 — axiom-kernel
Crate Layer 8: Proc-macro（豁免） — axiom-macros
Crate Layer 9: Plugin SDK 和示例 — axiom-plugin-wasm-sdk, axiom-plugin-example-wasm
```

**铁律**：Crate Layer N 的 crate **只能依赖** Crate Layer >= N 的 crate（即只能向下依赖）。

### 架构治理

Axiom Core 使用编译期架构门禁（Architecture Gate）确保代码库的架构一致性：

- **单一数据源**：`.axiom/architecture.toml` 定义所有架构规则
- **编译期强制**：每个 crate 的 `build.rs` 在编译时自动检查依赖方向、禁止依赖、审计依赖
- **零信任原则**：不依赖开发者自觉，编译期自动拦截架构违规
- **豁免机制**：支持 `proc-macro-exemptions` 和 `reverse-dependency-exemptions`

## 插件系统

v0.4.0 新增 **WASM 插件系统**，支持运行时动态加载插件：

```rust
// 使用 WASM 插件
use axiom_kernel::plugin::{PluginRegistry, PluginKind};

let registry = PluginRegistry::new();
registry.load_wasm("plugins/echo.wasm").await?;

// 使用 Native 插件
registry.load_native("plugins/libcounter.so").await?;
```

## Agent 配套体系

在五大原语之上，提供完整的 Agent 开发配套：

| 概念 | 一句话 | 说明 |
|------|--------|------|
| **Identity（身份）** | Agent 是谁 | Persona + 能力边界 + 权限集，可组合、可热切换 |
| **Skill（技能）** | Agent 会什么 | 遵循 agentskills.io 开放标准，渐进式披露三层加载，支持绑定 Axiom/Lens/Permission |
| **Rules（规则）** | Agent 守什么底线 | 软约束（区别于 Axiom 硬约束），Prompt注入+输出验证+升级Axiom三层执行 |
| **MCP（模型上下文协议）** | Agent 连什么外部世界 | 双向桥接（Client+Server），四层安全检查（Permission→Rules→Axiom→Human-in-the-loop） |

## 项目结构

```
axiom-core-project/
├── crates/
│   ├── axiom-kernel/          # 核心原语：Cell/Signal/Lens/Axiom/Witness + Plugin/Heatmap
│   │   ├── src/
│   │   │   ├── cell.rs
│   │   │   ├── signal.rs
│   │   │   ├── witness.rs
│   │   │   ├── plugin/        # 插件子系统
│   │   │   ├── heatmap/       # 热图子系统
│   │   │   └── ...
│   ├── axiom-runtime/         # Tokio 运行时：监督树 + 消息总线 + MPSC 信箱
│   ├── axiom-oversight/       # 监督层：熵治理 + 架构合规
│   ├── axiom-store/           # 事件存储：Append-Only Event Log + 快照 + 重放
│   ├── axiom-agent/           # Agent 开发配套：Identity + Skill + Rules 引擎
│   ├── axiom-mcp/             # MCP 协议桥接
│   ├── axiom-viz/             # 可视化数据导出：拓扑/时间轴/熵值
│   ├── axiom-macros/          # 过程宏：#[signal] #[cell] #[tool] #[guard] #[capability]
│   ├── axiom-plugin-wasm-sdk/ # WASM 插件开发 SDK
│   ├── axiom-plugin-example-wasm/ # WASM 插件示例
│   └── axiom-cli/             # axm 命令行工具
├── tools/
│   ├── archcheck/             # 架构检查工具（编译期门禁 + CLI）
│   └── xtask/                 # 任务运行器（gatecheck / state）
└── docs/
    ├── ARCHITECTURE.md        # 架构文档
    ├── PLUGIN_SYSTEM.md       # 插件系统文档
    ├── HEATMAP_SYSTEM.md      # 热图系统文档
    ├── guide/                 # 使用指南
    └── ...
```

## 快速开始

```rust
use axiom_kernel::*;

// 定义一个 Greeting Cell
struct GreetingCell {
    greetings: Vec<String>,
}

impl Cell for GreetingCell {
    type Signal = GreetingSignal;
    type State = Vec<String>;

    fn cell_id(&self) -> CellId { CellId("greeting-cell".into()) }
    fn layer(&self) -> RuntimeTier { RuntimeTier::Agent }

    async fn handle(&mut self, signal: Self::Signal, ctx: &mut CellContext) -> Result<()> {
        self.greetings.push(signal.message.clone());
        println!("Received: {}", signal.message);
        ctx.emit_witness(TransitionOutcome::Success).await?;
        Ok(())
    }
}
```

## 状态

✅ **v0.4.0 已发布** — 核心原语迁移完成，`axiom-kernel` 作为运行时层完全替代 `axiom-core`，新增 WASM 插件系统和热图系统。

## 设计文档

- [架构设计](docs/ARCHITECTURE.md)
- [插件系统](docs/PLUGIN_SYSTEM.md)
- [热图系统](docs/HEATMAP_SYSTEM.md)
- [状态转换图](docs/STATE_TRANSITION.md)
- [API 边界](docs/API_BOUNDARY.md)

## License

MIT