# Axiom Core 开发指南

本指南面向 Axiom Core 的贡献者，帮助你搭建开发环境、理解项目架构、掌握开发流程。

---

## 目录

- [开发环境准备](#开发环境准备)
- [项目结构](#项目结构)
- [构建与测试](#构建与测试)
- [架构约束](#架构约束)
- [添加新 Cell](#添加新-cell)
- [添加新 API 端点](#添加新-api-端点)
- [提交规范](#提交规范)
- [调试技巧](#调试技巧)

---

## 开发环境准备

### Rust 工具链

Axiom Core 要求 Rust 1.85+（支持 `async fn in traits` 等特性）。

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 确认版本
rustc --version  # 应输出 1.85.0 或更高

# 安装组件
rustup component add clippy rustfmt
```

### 系统依赖

```bash
# Ubuntu/Debian
sudo apt-get update && sudo apt-get install -y build-essential libssl-dev pkg-config

# macOS (Homebrew)
brew install openssl pkg-config

# Windows: 安装 Visual Studio Build Tools 和 OpenSSL
```

### 推荐 IDE 配置

#### VS Code

安装以下扩展：

- **rust-analyzer** — Rust 语言服务器，提供代码补全、跳转定义、类型推断
- **CodeLLDB** — 调试支持
- **Even Better TOML** — TOML 文件语法高亮
- ** crates** — 依赖版本查看

推荐配置（`.vscode/settings.json`）：

```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.procMacro.enable": true,
    "editor.formatOnSave": true,
    "[rust]": {
        "editor.defaultFormatter": "rust-lang.rust-analyzer"
    }
}
```

#### RustRover / IntelliJ Rust

启用 `clippy on save` 和 `fmt on save`。

### 克隆与首次构建

```bash
git clone https://github.com/arwei944/axiom-kernel.git
cd axiom-kernel

# 首次构建（会触发所有 crate 的 archcheck）
cargo build --workspace

# 运行测试
cargo test --workspace
```

---

## 项目结构

Axiom Core 采用 **9 层 Crate 分层架构**，所有架构规则定义在 `.axiom/architecture.toml` 中。

### 分层总览

```
Crate Layer 0: 顶层应用       — axiom-cli, axiom-bench
Crate Layer 1: 可视化与API    — axiom-viz, axiom-api
Crate Layer 2: Agent 门面     — axiom-identity, axiom-prompt
Crate Layer 3: 监督与集成      — axiom-agent, axiom-oversight, axiom-alert, axiom-mcp
Crate Layer 4: 运行时与协调    — axiom-runtime, axiom-planner, axiom-distributed
Crate Layer 5: 存储与工具      — axiom-store, axiom-tool, axiom-memory, axiom-llm
Crate Layer 7: 核心原语       — axiom-kernel
Crate Layer 8: Proc-macro     — axiom-macros（豁免层）
Crate Layer 9: Plugin SDK     — axiom-plugin-wasm-sdk, axiom-plugin-example-wasm
```

### 层间依赖规则

**铁律**：Crate Layer N 的 crate **只能依赖** Crate Layer >= N 的 crate（即只能向下依赖）。

```
Layer 0 (顶层应用)
    ↓ 可依赖
Layer 1 (可视化)
    ↓ 可依赖
Layer 2 (Agent 门面)
    ↓ 可依赖
Layer 3 (监督集成)
    ↓ 可依赖
Layer 4 (运行时)
    ↓ 可依赖
Layer 5 (存储工具)
    ↓ 可依赖
Layer 7 (核心原语)
    ↓ 可依赖
Layer 8 (Proc-macro) — 豁免层
Layer 9 (Plugin SDK)
```

> **注意**：Crate Layer 是编译期 crate 依赖层级，与 Runtime Tier（运行时分层）是两套独立体系，不要混淆。Runtime Tier 定义在 `crates/axiom-kernel/src/layer.rs`。

### 目录结构

```
axiom-core-project/
├── crates/                    # 所有 Rust crate
│   ├── axiom-kernel/          # 核心原语：Cell/Signal/Lens/Axiom/Witness
│   ├── axiom-runtime/         # Tokio 运行时：监督树 + 消息总线
│   ├── axiom-oversight/       # 监督层：熵治理 + 架构合规
│   ├── axiom-api/             # RESTful API 网关
│   ├── axiom-store/           # 事件存储
│   ├── axiom-macros/          # 过程宏
│   └── ...
├── tools/
│   └── archcheck/             # 架构检查工具（编译期门禁 + CLI）
├── xtask/                     # 任务运行器
├── templates/                 # 代码模板
│   ├── cell/                  # Cell 模板
│   └── crate/                 # Crate 模板
├── hooks/                     # Git hooks
│   ├── pre-commit
│   └── pre-push
├── docs/                      # 文档
├── .axiom/                    # 架构约束配置
│   ├── architecture.toml      # 分层规则、禁止依赖、审计依赖
│   └── ...
└── Cargo.toml                 # Workspace 配置
```

---

## 构建与测试

### 常用命令

```bash
# 构建整个 workspace
cargo build --workspace

# 构建 release 版本
cargo build --workspace --release

# 运行所有测试
cargo test --workspace

# 运行特定 crate 的测试
cargo test -p axiom-kernel

# 运行 clippy 检查（警告视为错误）
cargo clippy --workspace --all-targets -- -D warnings

# 格式化代码
cargo fmt --all

# 格式化检查（不修改文件）
cargo fmt --all -- --check
```

### 启用可选特性

```bash
# 启用 WASM 插件加载器
cargo build --workspace --features axiom-kernel/wasm-loader

# 启用 bincode 编解码
cargo build --workspace --features axiom-kernel/bincode-codec

# 启用遥测
cargo build --workspace --features axiom-runtime/telemetry
```

### 运行示例

```bash
# 运行 kernel 示例
cargo run --example hello_cell -p axiom-kernel
cargo run --example plugin_hello_world -p axiom-kernel

# 运行分布式示例
cargo run --example distributed_cluster -p axiom-distributed
```

### 基准测试

```bash
# 运行基准测试
cargo bench -p axiom-bench

# 运行特定基准
cargo bench -p axiom-bench -- bus_dispatch
```

---

## 架构约束

Axiom Core 使用编译期架构门禁（Architecture Gate）确保代码库的架构一致性。

### archcheck 工具

`archcheck` 是架构治理工具，位于 `tools/archcheck/`，提供两种检查方式：

#### 1. 编译期检查（build.rs）

每个 crate 的 `build.rs` 调用 `archcheck::build_hook::check_current_crate()` 进行编译期检查：

```rust
// crates/axiom-runtime/build.rs
fn main() {
    archcheck::build_hook::check_current_crate(env!("CARGO_PKG_NAME"));
}
```

检查内容：
- **依赖方向**：禁止逆向依赖（低 Layer 依赖高 Layer）
- **禁止依赖**：检查 `[forbidden-deps]` 中的依赖
- **审计依赖**：所有第三方依赖必须在 `[audited-deps]` 中注册

如果检查失败，编译会中断并输出详细的错误信息。

#### 2. CLI 检查（axm verify）

```bash
# 运行架构验证
cargo run -p axiom-cli -- verify

# 输出示例：
# === axiom verify (architecture constraints) ===
#   ✓ constraints integrity (hash check)
#   ✓ TODO/FIXME scan
#   ✓ unsafe code audit
#   ✓ third-party dependency audit
#   ✓ architecture dependency verification
# 5/5 architecture checks passed
```

#### 3. archcheck 独立工具

```bash
# 直接运行 archcheck
cargo run -p archcheck -- --workspace .

# 列出所有注册的 crate
cargo run -p archcheck -- --list-crates

# 输出 JSON 格式报告
cargo run -p archcheck -- --format json

# 验证 architecture.toml 语法
cargo run -p archcheck -- --validate-architecture
```

### 依赖方向规则详解

规则定义在 `.axiom/architecture.toml` 的 `[crate-layers]` 段：

```toml
[crate-layers]
axiom-cli = 0          # Layer 0
axiom-api = 1          # Layer 1
axiom-runtime = 4      # Layer 4
axiom-kernel = 7       # Layer 7
```

**规则**：Layer N 的 crate 只能依赖 Layer >= N 的 crate。

**示例**：
- `axiom-runtime`（Layer 4）可以依赖 `axiom-kernel`（Layer 7）✓
- `axiom-kernel`（Layer 7）不能依赖 `axiom-runtime`（Layer 4）✗

### 逆向依赖豁免

如果确实需要打破分层规则，可以在 `architecture.toml` 中申请豁免：

```toml
[reverse-dependency-exemptions]
axiom-agent = { allowed_deps = ["axiom-identity", "axiom-prompt"], reason = "Agent 需要调用 facade" }
```

### 添加新依赖

添加新的第三方依赖时，必须同步在 `architecture.toml` 的 `[audited-deps]` 中注册：

```toml
[audited-deps]
new-crate = "新依赖的用途说明"
```

未注册的依赖会在编译期触发 `UNAUDITED DEPENDENCY` 错误。

---

## 添加新 Cell

### 步骤说明

1. **确定 Cell 所属层**：根据 Cell 职责选择 Runtime Tier（Oversight/Agent/Validate/Exec）
2. **实现 Cell trait**：定义 Cell 结构体并实现 `Cell` trait
3. **注册到 Runtime**：使用 `CellRegistration` 注册到 `AxiomRuntime`
4. **编写测试**：验证 Cell 的信号处理逻辑

### 代码模板

以下是创建一个 Echo Cell 的完整示例（参考 `templates/cell/cell.rs.template`）：

```rust
use axiom_kernel::cell::{Cell, CellKind};
use axiom_kernel::id::CellId;

/// Echo Cell - 接收信号并原样返回
#[derive(Debug, Default)]
pub struct EchoCell {
    received: Vec<String>,
}

impl Cell for EchoCell {
    fn cell_id(&self) -> CellId {
        CellId::new("echo-cell")
    }

    fn cell_kind(&self) -> CellKind {
        CellKind::Exec
    }
}
```

### 注册到 Runtime

```rust
use axiom_kernel::cell::SupervisionStrategy;
use axiom_kernel::id::CellId;
use axiom_kernel::layer::RuntimeTier;
use axiom_runtime::{AxiomRuntime, CellRegistration};

// 创建 Runtime
let runtime = AxiomRuntime::default();

// 注册 Echo Cell
let registration = CellRegistration::new(
    CellId::new("echo-cell"),
    RuntimeTier::Exec,
)
.with_strategy(SupervisionStrategy::Restart { max_retries: 3 });

runtime.register_cell(registration).await?;

// 启动 Runtime
runtime.start().await?;
```

### Runtime Tier 选择指南

| Tier | 编号 | 职责 | 可发送目标 |
|------|------|------|-----------|
| Oversight | 0 | 最高监督层 | 所有层 |
| Agent | 3 | Agent 协调层 | Agent, Validate |
| Validate | 2 | 校验层 | Validate, Exec, Agent |
| Exec | 1 | 执行层 | 仅 Exec |

层间调用规则定义在 `crates/axiom-kernel/src/layer.rs` 的 `can_send_to()` 方法中。

### 使用过程宏简化

Axiom Core 提供过程宏自动注入必需字段（见 `crates/axiom-macros/`）：

```rust
#[axiom_macros::cell("exec")]
impl Cell for MyCell {
    // 宏自动注入层标记和 Witness 记录
}
```

---

## 添加新 API 端点

### 路由注册步骤

API 端点定义在 `crates/axiom-api/src/router/` 目录下。

#### 1. 在 `v1.rs` 中添加路由

```rust
// crates/axiom-api/src/router/v1.rs
use crate::router::{new_endpoint_handler, ApiState};
use axum::{routing::get, Router};

pub fn routes(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/cells", get(cells_handler))
        .route("/new-endpoint", get(new_endpoint_handler))  // 新增
        .with_state(state)
}
```

#### 2. 在 `mod.rs` 中实现 handler

```rust
// crates/axiom-api/src/router/mod.rs
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;

pub async fn new_endpoint_handler(State(state): State<ApiState>) -> impl IntoResponse {
    // 通过 state 中的 aggregator 获取数据
    // 返回 JSON 响应
    Json(serde_json::json!({ "status": "ok" }))
}
```

#### 3. 更新 OpenAPI 规范

在 `docs/openapi.yaml` 中添加端点描述：

```yaml
paths:
  /api/v1/new-endpoint:
    get:
      summary: New endpoint description
      description: Detailed description of the new endpoint
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/NewResponse'
```

### API 架构说明

API 采用聚合器（Aggregator）模式，位于 `crates/axiom-api/src/aggregator/`：

- `HealthAggregator` — 聚合健康状态数据
- `CellAggregator` — 聚合 Cell 列表数据
- `HeatmapAggregator` — 聚合热图数据
- `EntropyAggregator` — 聚合熵值数据

数据源通过 trait 抽象：

```rust
// crates/axiom-runtime/src/api/mod.rs
pub trait RuntimeDataSource: Send + Sync {
    fn get_health(&self) -> Pin<Box<dyn Future<Output = Result<RuntimeHealth>> + Send>>;
    fn get_cells(&self) -> Pin<Box<dyn Future<Output = Result<Vec<RegisteredCell>>> + Send>>;
    fn get_entropy_snapshot(&self) -> Pin<Box<dyn Future<Output = Result<EntropySnapshotData>> + Send>>;
    fn get_heatmap(&self) -> Pin<Box<dyn Future<Output = Result<UsageSnapshot>> + Send>>;
}
```

### 启动 API 服务

```rust
use axiom_api::builder::ApiServerBuilder;
use axiom_runtime::AxiomRuntime;
use axiom_oversight::OversightKernelAdapter;
use std::net::SocketAddr;
use std::sync::Arc;

let runtime = Arc::new(AxiomRuntime::default());
let oversight = Arc::new(OversightKernelAdapter::new());

let server = ApiServerBuilder::new()
    .addr("0.0.0.0:9092".parse().unwrap())
    .development()  // 开发环境配置
    .build(runtime, oversight);

server.serve().await?;
```

---

## 提交规范

### pre-commit hooks

项目使用 Git hooks 进行提交前检查，位于 `hooks/` 目录。安装方式：

```bash
# 使用 axm CLI 安装
cargo run -p axiom-cli -- init

# 或手动设置 git hooks 路径
git config core.hooksPath hooks
```

`hooks/pre-commit` 会依次执行以下检查（见 `hooks/pre-commit`）：

| 步骤 | 命令 | 说明 |
|------|------|------|
| 1/5 | `cargo fmt --check` | 格式化检查 |
| 2/5 | `cargo build --workspace` | 编译检查 |
| 3/5 | `cargo clippy -- -D warnings` | Lint 检查（警告视为错误） |
| 4/5 | `cargo test --workspace` | 测试检查 |
| 5/5 | `axm verify` | 架构约束检查 |

**跳过检查**（不推荐）：

```bash
git commit --no-verify
```

### pre-push hooks

`hooks/pre-push` 在推送前执行额外检查，确保不会推送未通过的代码。

### 提交信息规范

遵循 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>(<scope>): <subject>

<body>

<footer>
```

类型（type）：

| 类型 | 说明 |
|------|------|
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `docs` | 文档变更 |
| `style` | 代码格式（不影响功能） |
| `refactor` | 重构（既不是新功能也不是修 Bug） |
| `test` | 测试相关 |
| `chore` | 构建/工具链变更 |

示例：

```
feat(axiom-runtime): add throttle interceptor for entropy governance

Add ThrottleInterceptor that limits message rate when entropy reaches
red level. The interceptor uses exponential backoff with configurable
cooldown period.

Closes #123
```

---

## 调试技巧

### 日志查看

Axiom Core 使用 `tracing` 库进行结构化日志记录。日志级别通过环境变量配置：

```bash
# 设置日志级别
export AXIOM_LOG_LEVEL=debug

# 运行时查看实时日志
export AXIOM_LOG_FORMAT=text
cargo run -p axiom-runtime
```

日志输出示例：

```
2024-01-15T10:30:00.123Z INFO  axiom_runtime::runtime::start: preflight passed
2024-01-15T10:30:00.456Z INFO  axiom_runtime::runtime::start: runtime started with 3 cells
2024-01-15T10:30:01.789Z DEBUG axiom_runtime::bus: signal delivered target_cell=echo-cell signal_type=EchoCommand
2024-01-15T10:30:02.012Z WARN  axiom_runtime::supervisor: cell panicked cell_id=echo-cell restart_count=1
```

### tracing 分析

启用 OpenTelemetry 分布式追踪：

```bash
# 启动 Jaeger
docker run -d -p 16686:16686 -p 4318:4318 jaegertracing/all-in-one:latest

# 启用追踪
export AXIOM_TRACING_ENABLED=true
export AXIOM_OTLP_ENDPOINT=http://localhost:4318/v1/traces

# 运行
cargo run -p axiom-runtime

# 在 Jaeger UI 查看链路
# http://localhost:16686
```

### 使用 `axm` CLI 调试

```bash
# 查看系统拓扑
cargo run -p axiom-cli -- top

# 查看热力图
cargo run -p axiom-cli -- heatmap

# 查看熵值
cargo run -p axiom-cli -- entropy

# 追踪某个 correlation_id 的完整链路
cargo run -p axiom-cli -- why <correlation_id>

# 查看 Cell 详情
cargo run -p axiom-cli -- cell <cell_id>

# 查看 Witness 链
cargo run -p axiom-cli -- witness <witness_id>
```

### 调试 Cell 崩溃

当 Cell 崩溃时，Supervisor 会记录崩溃信息并决定是否重启：

1. 查看日志中 `WARN` 级别的 `cell panicked` 记录
2. 检查 Supervisor 的重启计数：
   ```bash
   cargo run -p axiom-cli -- cell <cell_id>
   ```
3. 查看死信队列（DLQ）中未被处理的消息：
   ```bash
   cargo run -p axiom-cli -- dashboard
   ```
4. 使用 `axm why` 追踪崩溃前的完整 Witness 链

### 常见编译问题

#### 1. ARCHITECTURE VIOLATION: REVERSE DEPENDENCY

```
╔══════════════════════════════════════════════════════════════╗
║  ARCHITECTURE VIOLATION: REVERSE DEPENDENCY                 ║
╠══════════════════════════════════════════════════════════════╣
║  axiom-runtime (level 4) depends on                          ║
║  axiom-api    (level 1) which is a HIGHER layer             ║
╚══════════════════════════════════════════════════════════════╝
```

**解决**：移除逆向依赖，或在 `architecture.toml` 中添加豁免。

#### 2. UNAUDITED DEPENDENCY

```
║  'new-crate' has not been audited (R-022).                    ║
║  Either:                                                       ║
║  1. Add it to audited-deps in .axiom/architecture.toml        ║
║  2. Remove it if unnecessary                                   ║
```

**解决**：在 `.axiom/architecture.toml` 的 `[audited-deps]` 中添加依赖。

#### 3. FORBIDDEN DEPENDENCY

```
║  'async-trait' is FORBIDDEN in axiom crates.                  ║
║  Reason: R-004: Rust 1.75+ 已支持原生 async fn in traits       ║
```

**解决**：移除被禁止的依赖，使用原生 Rust 替代方案。

### 内存与性能分析

```bash
# 运行基准测试
cargo bench -p axiom-bench

# 使用 valgrind 分析内存（Linux）
cargo build --workspace --release
valgrind --tool=massif ./target/release/axiom-runtime

# 查看编译时间
cargo build --workspace --timings
```

---

## 更多资源

- [架构设计文档](ARCHITECTURE.md) — 完整架构说明
- [用户指南](user-guide.md) — 面向使用者的指南
- [插件开发指南](plugin-development.md) — WASM/Native 插件开发
- [部署指南](deployment.md) — 生产环境部署
- [配置参考](configuration.md) — 全部配置项
- [安全策略](SECURITY.md) — 安全相关信息
