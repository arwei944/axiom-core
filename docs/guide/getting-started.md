# 快速上手指南

本指南将带你从零开始使用 Axiom Core——一个面向智能体（Agent）的确定性优先运行时架构。通过最小可运行示例（Hello Cell），你将了解如何安装、编写、运行并理解一个基于五大核心原语构建的 Cell。

---

## 目录

- [前置条件](#前置条件)
- [安装](#安装)
- [最小示例：Hello Cell](#最小示例hello-cell)
- [运行示例](#运行示例)
- [项目结构说明](#项目结构说明)
- [下一步](#下一步)

---

## 前置条件

Axiom Core 是一个 Rust 项目，开始之前请确保你的开发环境满足以下要求：

| 工具 | 最低版本 | 说明 |
|------|---------|------|
| Rust 工具链 | 1.75+ | 通过 `rustup` 安装稳定版通道 |
| Cargo | 随 Rust 附带 | 包管理与构建工具 |
| Tokio 运行时 | 由依赖自动引入 | 示例使用 `#[tokio::main]` |

> **提示**：Axiom Core 默认启用 `sha2-id` 与 `uuid` 特性，用于生成 SHA-256 哈希与唯一 ID。如果你在受限环境（如 `no_std`）下使用，可在 `Cargo.toml` 中关闭默认特性。

---

## 安装

Axiom Core 已发布到 crates.io，你可以通过 `cargo add` 直接添加依赖：

```bash
# 添加核心原语 crate
cargo add axiom-kernel

# 如果要构建完整的 Agent，添加 agent 配套 crate
cargo add axiom-agent
```

这等价于在 `Cargo.toml` 的 `[dependencies]` 中写入：

```toml
[package]
name = "my-axiom-app"
version = "0.1.0"
edition = "2021"

[dependencies]
axiom-kernel = "0.4"
axiom-agent = "0.4"

tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### 验证安装

创建一个新项目并验证依赖能够编译：

```bash
cargo new my-axiom-app
cd my-axiom-app
cargo add axiom-kernel
cargo build
```

若构建成功，说明环境已就绪。

---

## 最小示例：Hello Cell

下面是一个最小可运行示例，它演示了 Axiom Core 的五个核心原语中至少四个的协作：**Cell**（隔离状态单元）、**Signal**（类型化消息）、**Axiom**（全局约束）、**Witness**（审计记录）。这个示例源自仓库内的 `crates/axiom-kernel/examples/hello_cell.rs`。

### 完整代码

```rust
use axiom_kernel::cell::{Cell, CellHandle};
use axiom_kernel::context::{CellContext, LayeredCellContext, OutgoingEnvelope, OutgoingWitness};
use axiom_kernel::entropy::EntropyScore;
use axiom_kernel::id::{CellId, CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::schema::{validators, ValidationResult};
use axiom_kernel::sealed::ExecLayer;
use axiom_kernel::signal::{Signal, SignalKind, VectorClock};
use axiom_kernel::witness::TransitionOutcome;
use axiom_kernel::{axiom, cell, schema_version, Axiom, DynAxiomChain, SignalPayload};
use std::future::Future;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, SignalPayload)]
#[signal(kind = "command", layer = "exec")]
#[schema_version(1)]
#[schema(skip)]
struct HelloCommand {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    message: String,
}

impl HelloCommand {
    fn new(message: &str) -> Self {
        Self {
            msg_id: MsgId::generate(),
            correlation_id: CorrelationId::generate(),
            vector_clock: VectorClock::new(),
            message: message.to_string(),
        }
    }
}

impl axiom_kernel::Schema for HelloCommand {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::ok();
        result += validators::require_non_empty("message", &self.message);
        result += validators::require_max_length("message", &self.message, 1024);
        result
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, SignalPayload)]
#[signal(kind = "event", layer = "exec")]
#[schema_version(1)]
struct GreetedEvent {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    greeting: String,
}

impl GreetedEvent {
    fn new(correlation_id: CorrelationId, greeting: &str) -> Self {
        Self {
            msg_id: MsgId::generate(),
            correlation_id,
            vector_clock: VectorClock::new(),
            greeting: greeting.to_string(),
        }
    }
}

#[axiom]
struct NonEmptyGreetingAxiom;

impl Axiom for NonEmptyGreetingAxiom {
    type State = Vec<String>;
    type Message = HelloCommand;

    fn name(&self) -> &'static str {
        "non-empty-greeting"
    }

    fn check(&self, _current: &Self::State, new: &Self::State, _msg: &Self::Message) -> axiom_kernel::Result<()> {
        if new.iter().any(|g| g.is_empty()) {
            return Err(axiom_kernel::KernelError::InvariantViolated {
                message: "greeting must not be empty".into(),
            });
        }
        Ok(())
    }

    fn applies_to_layer(&self, layer: Layer) -> bool {
        matches!(layer, Layer::Exec | Layer::Validate)
    }
}

struct HelloCell {
    id: CellId,
    greetings: Vec<String>,
}

impl HelloCell {
    fn new() -> Self {
        Self {
            id: CellId::new("hello-cell"),
            greetings: Vec::new(),
        }
    }
}

#[cell("exec")]
impl Cell for HelloCell {
    type Message = HelloCommand;

    fn id(&self) -> &CellId {
        &self.id
    }

    fn handle<'a>(
        &'a mut self,
        signal: HelloCommand,
        ctx: LayeredCellContext<'a, Self::Layer>,
    ) -> impl Future<Output = (axiom_kernel::Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a {
        async move {
            let mut ctx = ctx;
            println!("Received: {}", signal.message);
            self.greetings.push(signal.message.clone());

            let event = GreetedEvent::new(signal.correlation_id.clone(), &signal.message);
            let result: axiom_kernel::Result<()> = (|| {
                ctx.emit_to::<ExecLayer, _>(event)?;
                ctx.emit_witness(
                    ctx.witness()
                        .summary(format!("processed greeting: {}", signal.message))
                        .outcome(TransitionOutcome::Success)
                        .processing_time_us(42),
                )?;
                Ok(())
            })();
            let (outgoing, witnesses) = ctx.end_processing();
            (result, outgoing, witnesses)
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Axiom Core - Hello Cell example");

    let cell = HelloCell::new();
    let handle = CellHandle::new(cell);
    println!("Cell ID: {}", handle.id());
    println!("Cell Layer: {:?}", handle.layer());

    let mut cell = HelloCell::new();
    let cell_id = CellId::new("hello-cell");
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);

    let signal = HelloCommand::new("Hello, Axiom!");
    assert!(axiom_kernel::Schema::validate(&signal).is_valid(), "signal should validate");
    assert_eq!(signal.kind(), SignalKind::Command);
    assert_eq!(signal.layer(), Layer::Exec);

    let layered = ctx.as_layered::<ExecLayer>();
    let (result, _outgoing, witnesses) = cell.handle(signal, layered).await;
    result.unwrap();

    println!("Greetings received: {:?}", cell.greetings);
    println!("Witnesses produced: {}", witnesses.len());

    let chain = DynAxiomChain::from_registry_for_layer(Layer::Exec);
    println!("Registered axioms for Exec layer: {}", chain.count());

    let mut entropy = EntropyScore::new();
    assert!(entropy.is_green());
    println!("Initial entropy: {:.3} [{:?}]", entropy.compute(), entropy.level());

    for _ in 0..3 {
        entropy.record_axiom_violation();
        entropy.record_cell_restart();
        entropy.record_circuit_break();
    }
    println!("After multiple factors: {:.3} [{:?}]", entropy.compute(), entropy.level());

    entropy.reset();
    println!("After reset: {:.3} [{:?}]", entropy.compute(), entropy.level());

    println!("\n=== All core primitives verified successfully! ===");
}
```

### 代码要点解读

1. **信号定义**：`HelloCommand` 通过 `#[derive(SignalPayload)]` + `#[signal(kind="command", layer="exec")]` 自动实现 `Signal` trait。`#[schema(skip)]` 表示我们手写 `Schema` 校验逻辑。

2. **Cell 实现**：`#[cell("exec")]` 宏在编译期把 `HelloCell` 绑定到 Exec 层，并自动实现 `type Layer = ExecLayer`。`handle` 是消息处理入口，接收 `LayeredCellContext`——它只能在编译期允许的方向上发送消息。

3. **Axiom 约束**：`#[axiom]` 宏把 `NonEmptyGreetingAxiom` 注册到全局分布式注册表，运行时可通过 `DynAxiomChain::from_registry_for_layer` 查询。

4. **Witness 记录**：通过 `ctx.witness().summary(...).outcome(...).emit()` 构造审计记录，每条 Witness 自动加入 SHA-256 哈希链。

5. **熵值监控**：`EntropyScore` 量化系统无序度，分为 Green/Yellow/Red/Critical 四级，违规、重启、熔断都会推高熵值。

---

## 运行示例

### 方式一：运行仓库内置示例

仓库自带上述示例，直接在仓库根目录执行：

```bash
cargo run --example hello_cell -p axiom-kernel
```

预期输出（节选）：

```
Axiom Core - Hello Cell example
Cell ID: hello-cell
Cell Layer: Exec
Received: Hello, Axiom!
Greetings received: ["Hello, Axiom!"]
Witnesses produced: 1
Registered axioms for Exec layer: 1
Initial entropy: 0.000 [Green]
After multiple factors: 6.000 [Critical]
After reset: 0.000 [Green]

=== All core primitives verified successfully! ===
```

### 方式二：在自己的项目中运行

1. 用 `cargo new` 创建二进制项目。
2. 按上文「安装」一节添加 `axiom-kernel`、`tokio`、`serde` 等依赖。
3. 把上面的「完整代码」粘贴到 `src/main.rs`。
4. 执行：

```bash
cargo run
```

### 方式三：开启调试日志

示例中调用了 `tracing_subscriber::fmt::init()`，你可以通过环境变量控制日志级别：

```bash
# Windows PowerShell
$env:RUST_LOG="axiom_kernel=debug,info"
cargo run --example hello_cell -p axiom-kernel

# Linux / macOS
RUST_LOG="axiom_kernel=debug,info" cargo run --example hello_cell -p axiom-kernel
```

---

## 项目结构说明

Axiom Core 采用 workspace 多 crate 组织，每个 crate 职责单一。下表列出与入门最相关的 crate：

| Crate | 职责 | 入门优先级 |
|-------|------|-----------|
| `axiom-kernel` | 五大原语：Cell / Signal / Lens / Axiom / Witness + Layer / Entropy + Plugin / Heatmap | ⭐⭐⭐ 必学 |
| `axiom-macros` | 过程宏：`#[cell]`、`#[axiom]`、`#[derive(SignalPayload)]`、`#[schema_version]` | ⭐⭐ 由 core 自动引入 |
| `axiom-agent` | Agent 开发配套（LLM + Tool + Memory + Planner + Identity + Prompt） | ⭐⭐ 构建智能体时必学 |
| `axiom-runtime` | Tokio 运行时：监督树 + 消息总线 + MPSC 信箱 | ⭐ 生产部署时学习 |
| `axiom-oversight` | 监督层：熵治理 + 架构合规 | ⭐ 高级运维 |
| `axiom-store` | 事件存储：Append-Only Event Log + 快照 + 重放 | ⭐ 持久化场景 |
| `axiom-cli` | `axm` 命令行工具：诊断、追踪、可视化 | ⭐ 调试辅助 |
| `axiom-plugin-wasm-sdk` | WASM 插件开发 SDK | ⭐ 扩展开发 |

### 仓库目录概览

```
axiom-core-project/
├── crates/
│   ├── axiom-kernel/                 # 核心原语（本指南的主角）
│   │   ├── examples/
│   │   │   └── hello_cell.rs       # ← 本指南使用的最小示例
│   │   └── src/
│   │       ├── cell.rs             # Cell trait + CellHandle
│   │       ├── signal.rs           # Signal trait + SignalEnvelope + VectorClock
│   │       ├── axiom.rs            # Axiom trait + DynAxiomChain
│   │       ├── witness.rs          # Witness + WitnessBuilder + 哈希链
│   │       ├── schema.rs           # Schema trait + validators
│   │       ├── layer.rs            # 四层枚举 Layer
│   │       ├── sealed.rs           # CanSendTo 编译期方向矩阵
│   │       ├── context.rs          # CellContext + LayeredCellContext
│   │       ├── entropy.rs          # EntropyScore 熵值模型
│   │       ├── plugin/             # 插件子系统
│   │       └── heatmap/            # 热图子系统
│   ├── axiom-agent/                # Agent 配套
│   ├── axiom-macros/               # 过程宏
│   ├── axiom-runtime/              # Tokio 运行时
│   ├── axiom-plugin-wasm-sdk/      # WASM 插件 SDK
│   └── ...                         # 其他配套 crate
├── tools/
│   ├── archcheck/                  # 架构检查工具
│   └── xtask/                      # 任务运行器
└── docs/
    ├── ARCHITECTURE.md             # 架构文档
    ├── PLUGIN_SYSTEM.md            # 插件系统文档
    ├── HEATMAP_SYSTEM.md           # 热图系统文档
    └── guide/                      # 用户指南（本目录）
```

### 入门后的典型开发路径

1. **定义信号**：用 `#[derive(SignalPayload)]` 定义 Command / Event / Query。
2. **实现 Schema**：为信号编写 `Schema::validate`，确保字段合法。
3. **实现 Cell**：用 `#[cell("exec")]` 把状态单元绑定到某一层，编写 `handle`。
4. **声明 Axiom**：用 `#[axiom]` 注册全局不变量。
5. **产出 Witness**：在 `handle` 内调用 `ctx.emit_witness(...)` 记录审计。
6. **接入运行时**（可选）：用 `axiom-runtime` 把 Cell 装进监督树与消息总线。
7. **构建 Agent**（可选）：用 `axiom-agent` 的 `AgentBuilder` 把 LLM、Tool、Memory 组装成完整智能体。
8. **编写插件**（可选）：用 `axiom-plugin-wasm-sdk` 编写 WASM 插件扩展功能。

---

## 下一步

完成本指南后，建议按以下顺序继续阅读：

- **[核心概念](./core-concepts.md)**：深入理解 Cell、Signal、Axiom、Witness、Lens 与四层架构的设计原理与层间调用约束。
- **[创建一个 Agent](./creating-an-agent.md)**：用 `AgentBuilder` 链式构建一个具备 LLM、Tool、Memory、Identity 的完整智能体。
- **[最佳实践](./best-practices.md)**：学习架构设计原则、性能优化、安全实践、错误处理与测试策略。
- **[插件系统](../PLUGIN_SYSTEM.md)**：学习如何编写和使用 WASM 插件扩展系统功能。

如果在运行示例时遇到问题，可开启 `RUST_LOG=debug` 查看详细日志，或参考仓库根目录的 `README.md` 与 `docs/ARCHITECTURE.md`。