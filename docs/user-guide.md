# Axiom Core 用户指南

本指南面向 Axiom Core 的使用者，帮助你理解项目核心理念、快速上手运行时，并掌握日常使用所需的全部知识。

---

## 目录

- [项目简介](#项目简介)
- [快速开始](#快速开始)
- [核心概念](#核心概念)
- [熵治理体系](#熵治理体系)
- [监督树](#监督树)
- [API 使用](#api-使用)
- [常见问题（FAQ）](#常见问题faq)

---

## 项目简介

**Axiom Core** 是一个面向智能体（Agent）的确定性优先运行时架构。它用五个核心原语构建低熵、可观测、可自愈的多智能体系统，解决现有智能体框架在状态一致性、故障隔离、因果追踪和架构约束方面的痛点。

### 解决什么问题

UC Berkeley 对 1642+ 条多智能体执行轨迹的研究发现：**41%–86.7% 的失败源于架构缺陷，而非 AI 能力不足**。现有智能体框架本质是"把 LLM 调用串起来"的工具库，缺乏分布式系统经典问题的解决方案。Axiom Core 从底层重新设计，提供：

| 痛点 | Axiom Core 的解法 |
|------|------------------|
| 黑盒运行 | 每次状态转换自动产生 Witness（不可篡改审计记录） |
| 静默退化 | 熵值实时监控，黄线告警、红线熔断、自动减熵 |
| 消息字符串传递 | Signal 类型安全 + Vector Clock 因果追踪 |
| 上下文爆炸 | Lens 按需投影状态，渐进式披露 Skill 元数据 |
| 错误传染 | 四层架构 + 监督树 + Axiom 硬约束，故障不扩散 |
| 无法自愈 | Erlang 风格"让它崩溃"+ 监督树自动重启 + 事件溯源恢复 |

更多架构详情请参阅 [ARCHITECTURE.md](ARCHITECTURE.md)。

---

## 快速开始

### 环境要求

| 指标 | 最低配置 | 推荐配置 |
|------|---------|---------|
| CPU | 2 核 | 4 核+ |
| 内存 | 2GB | 8GB+ |
| 磁盘 | 10GB SSD | 50GB NVMe |
| Rust 版本 | 1.85+ | 1.85+ |

### 安装

```bash
# 克隆仓库
git clone https://github.com/arwei944/axiom-kernel.git
cd axiom-kernel

# 编译所有 crate
cargo build --workspace

# 运行测试验证环境
cargo test --workspace
```

### 配置

Axiom Core 支持通过环境变量、`.env` 文件和 TOML 配置文件进行配置。创建配置文件 `config.toml`：

```toml
[runtime]
mailbox_capacity = 2048
entropy_threshold = 100.0
entropy_cooldown_ms = 60000
dispatch_poll_interval_ms = 10

[api]
addr = "0.0.0.0:9092"

[logging]
level = "info"
format = "json"

[store]
backend = "sqlite"
database_url = "/data/axiom_events.db"
```

完整配置项请参阅 [configuration.md](configuration.md)。

### 启动

```bash
# 运行 Runtime
cargo run -p axiom-runtime -- --config config.toml

# 运行 API 服务
cargo run -p axiom-api

# 运行 CLI 工具
cargo run -p axiom-cli -- help
```

启动后访问 API：

```bash
# 健康检查
curl http://localhost:9092/api/v1/health

# 查看 Cell 列表
curl http://localhost:9092/api/v1/cells

# 查看熵值状态
curl http://localhost:9092/api/v1/entropy
```

---

## 核心概念

Axiom Core 用五个核心原语构建整个系统。理解这五个原语是使用 Axiom Core 的基础。

### Cell（单元）

**一句话**：隔离的状态单元——私有状态 + 消息信箱，单线程执行。

Cell 是 Axiom Core 的基本计算单元。你可以把它理解为 Erlang 中的 Actor：每个 Cell 有自己的私有状态，通过消息信箱（Mailbox）接收信号，单线程处理消息，互不干扰。

**通俗理解**：Cell 就像一个独立办公室里的员工，有自己的工作状态，只通过邮件（Signal）和别人沟通，不会被打扰。

定义方式见 `crates/axiom-kernel/src/cell.rs`：

```rust
pub trait Cell: Send + Sync {
    fn cell_id(&self) -> CellId;
    fn cell_kind(&self) -> CellKind;
}
```

### Signal（信号）

**一句话**：类型化不可变消息——Vector Clock + 链路追踪。

Signal 是 Cell 之间通信的唯一方式。每条 Signal 携带类型信息、消息 ID、关联 ID（Correlation ID）和向量时钟（Vector Clock），确保消息可追溯、可去重、可验证因果关系。

**通俗理解**：Signal 就像一封挂号信，有寄件人、收件人、信件类型、追踪编号，永远不会被篡改。

Signal 分为四种类型（见 `crates/axiom-kernel/src/signal.rs`）：

| 类型 | 说明 | 典型场景 |
|------|------|---------|
| `Command` | 命令 | 请求执行某个操作 |
| `Event` | 事件 | 通知某事已发生 |
| `Query` | 查询 | 请求返回数据 |
| `Response` | 响应 | 回复查询结果 |

### Axiom（公理）

**一句话**：全局不变量约束——违反即熔断，熵的减压阀。

Axiom 是系统级的不变量规则。与"软约束"的 Rules 不同，Axiom 是硬约束：一旦违反，系统立即熔断并推高熵值。Axiom 确保系统始终处于合法状态。

**通俗理解**：Axiom 就像物理定律——你不能违反能量守恒。在 Axiom Core 中，你不能违反架构约束，否则系统会自动停止。

### Witness（见证）

**一句话**：不可篡改审计链——每次状态转换自动记录。

Witness 是系统自带的"黑匣子记录仪"。每次 Cell 处理信号产生状态转换时，自动生成一条 Witness 记录，包含前一条的哈希，形成不可篡改的链式审计轨迹。

**通俗理解**：Witness 就像飞机的黑匣子，记录了系统做的每一件事，事后可以用 `axm why` 命令一秒回放定位问题。

### Lens（透镜）

**一句话**：按需状态投影——不是塞全部历史，而是精确查询。

Lens 让你从事件流中按需投影出当前状态，而不必加载全部历史数据。它定义了"如何从事件流中提取你需要的状态视图"。

**通俗理解**：Lens 就像数据库的视图（View）——你不需要看到所有原始数据，只需要看到你关心的那个角度。

---

## 熵治理体系

熵（Entropy）是 Axiom Core 的第一公民。它是一个可度量、可监控的指标，量化系统的"混乱程度"。所有异常事件（消息丢弃、约束违反、Cell 重启等）都会推高熵值。

### 四级熵值

熵值分为四个级别，定义在 `crates/axiom-kernel/src/entropy.rs`：

| 级别 | 阈值 | 含义 | 系统行为 |
|------|------|------|---------|
| **Green** | < 0.4 | 健康 | 正常运行 |
| **Yellow** | 0.4 ~ 0.8 | 警告 | 开始监控，记录告警 |
| **Red** | 0.8 ~ 1.5 | 危险 | 熔断器可能触发，限制流量 |
| **Critical** | ≥ 3.0 | 紧急 | 紧急停机，进入安全模式 |

### 熵值权重

不同异常事件对熵值的贡献权重不同：

| 事件 | 权重 | 说明 |
|------|------|------|
| Cell 重启 | 5.0 | 最严重——说明 Cell 频繁崩溃 |
| 熔断 | 4.0 | 熔断器打开 |
| Axiom 违反 | 3.0 | 约束被破坏 |
| 被守护者拒绝 | 2.0 | 架构守护者拦截 |
| 过期状态违反 | 2.0 | 状态过期 |
| 超时 | 1.5 | 请求超时 |
| 消息丢弃 | 1.0 | 消息被丢弃 |
| 重复消息 | 0.5 | 消息重复（最轻微） |

### 熵值衰减

熵值不是永久累积的——它采用**时间衰减**机制（半衰期默认 300 秒）。如果系统持续正常运行，熵值会自动下降，最终回到 Green 级别。

### 治理动作

当熵值升高时，系统会自动采取治理动作（见 `crates/axiom-runtime/src/entropy_gov.rs`）：

| 动作 | 触发条件 | 效果 |
|------|---------|------|
| `None` | Green | 无动作 |
| `Warn` | Yellow | 记录告警日志 |
| `Throttle` | Red | 限制特定 Cell 的消息流量 |
| `Emergency` | Critical | 紧急停机，进入安全模式 |

### 查看熵值

通过 API 查看当前熵值状态：

```bash
curl http://localhost:9092/api/v1/entropy
```

返回示例：

```json
{
  "value": 0.3,
  "level": "green",
  "per_cell": [["echo-cell", 0.1]],
  "last_action": null
}
```

---

## 监督树

监督树（Supervision Tree）是 Axiom Core 的自愈核心。每个 Cell 都运行在一个 Supervisor 之下，当 Cell 崩溃时，Supervisor 根据预设策略自动恢复。

### 四种监督策略

监督策略定义在 `crates/axiom-kernel/src/cell.rs`：

| 策略 | 说明 | 适用场景 |
|------|------|---------|
| **Restart** | 重启 Cell，达到最大重试次数后停止 | 临时性故障（如网络抖动） |
| **Stop** | 立即停止 Cell，不重启 | 不可恢复的故障 |
| **Escalate** | 将故障升级到上层 Supervisor | 当前层无法处理的故障 |
| **CircuitBreak** | 熔断器模式：失败达到阈值后断开，一段时间后半开重试 | 防止级联故障 |

### Restart 策略

```rust
SupervisionStrategy::Restart { max_retries: 3 }
```

Cell 崩溃后自动重启，最多重试 3 次。每次重启采用指数退避（100ms → 200ms → 400ms ...），超过最大重试次数后转为 Stop。

### CircuitBreak 策略

```rust
SupervisionStrategy::CircuitBreak {
    failure_threshold: 3,
    reset_after_ms: 30_000,
}
```

熔断器三种状态：

```
Closed（正常）──失败达阈值──▶ Open（断开）
                                 │
                          等待 reset_after_ms
                                 ▼
                            Half-Open（半开）
                                 │
                         成功 ▼     ▼ 失败
                       Closed    Open
```

- **Closed**：正常工作，记录失败次数
- **Open**：拒绝所有请求，等待冷却时间
- **Half-Open**：允许少量请求试探，成功则回到 Closed，失败则回到 Open

### 注册 Cell 时指定策略

```rust
use axiom_kernel::cell::SupervisionStrategy;
use axiom_runtime::CellRegistration;

let registration = CellRegistration::new(cell_id, layer)
    .with_strategy(SupervisionStrategy::Restart { max_retries: 5 });
```

---

## API 使用

Axiom Core 提供 RESTful API，用于查询系统状态和指标。完整规范请参阅 [openapi.yaml](openapi.yaml)。

### 端点总览

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v1/health` | GET | 获取系统健康状态 |
| `/api/v1/cells` | GET | 获取所有 Cell 列表 |
| `/api/v1/heatmap` | GET | 获取活动热力图 |
| `/api/v1/entropy` | GET | 获取熵值状态 |
| `/api/v1/metrics` | GET | 获取 Prometheus 指标 |

### 健康检查

```bash
curl http://localhost:9092/api/v1/health
```

返回系统整体健康状态，包括运行中的 Cell 数量、消息投递统计、熵值等：

```json
{
  "status": "ok",
  "cells_running": 5,
  "cells_stopped": 0,
  "total_restarts": 0,
  "messages_delivered": 1024,
  "messages_rejected": 3,
  "entropy_score": 0.15,
  "entropy_level": "green",
  "preflight_passed": true,
  "uptime_ms": 3600000,
  "version": "0.4.0"
}
```

### 获取 Cell 列表

```bash
curl http://localhost:9092/api/v1/cells
```

返回所有已注册的 Cell 及其状态：

```json
[
  {
    "id": "echo-cell",
    "layer": "exec",
    "state": "Running",
    "version": "0.1.0"
  }
]
```

### 获取熵值状态

```bash
curl http://localhost:9092/api/v1/entropy
```

返回当前熵值、级别、各 Cell 的熵值分布和最近治理动作：

```json
{
  "value": 0.15,
  "level": "green",
  "per_cell": [["echo-cell", 0.1]],
  "last_action": null
}
```

### 获取热力图

```bash
curl http://localhost:9092/api/v1/heatmap
```

返回实时的信号流量热图，显示活跃的 Cell、Signal 类型和工具调用：

```json
{
  "timestamp": 1690000000000000000,
  "hot_cells": [["echo-cell", 128]],
  "hot_signals": [["EchoCommand", 128]],
  "hot_tools": []
}
```

### 获取 Prometheus 指标

```bash
curl http://localhost:9092/api/v1/metrics
```

返回 Prometheus 格式的指标文本，可直接接入 Prometheus + Grafana 监控体系。

### 认证

生产环境建议启用认证（见 [configuration.md](configuration.md)）：

- **API Key 模式**：在请求头中携带 `Authorization: Bearer <your-api-key>`
- **JWT 模式**：在请求头中携带 JWT Token

---

## 常见问题（FAQ）

### Q1: 启动时报 "preflight failed" 错误

**原因**：启动前自检未通过，通常是 Cell 版本号不符合 0.x 预发布要求，或迁移链不完整。

**解决**：
- 检查所有 Cell 的 `version.major` 是否为 0（预发布阶段要求）
- 运行 `axm verify` 检查架构约束完整性
- 确认迁移链注册完整

### Q2: 熵值持续升高怎么办

**原因**：系统出现频繁的异常事件（Cell 重启、约束违反、消息丢弃等）。

**解决**：
- 通过 `/api/v1/entropy` 查看各 Cell 的熵值，定位问题 Cell
- 查看 `per_cell` 字段找出熵值最高的 Cell
- 检查日志中 `WARN` 和 `ERROR` 级别记录
- 熵值会在半衰期（默认 300 秒）后自动衰减

### Q3: Cell 崩溃后不自动重启

**原因**：Cell 的监督策略可能是 `Stop` 或已达到 `Restart` 的最大重试次数。

**解决**：
- 检查注册 Cell 时指定的 `SupervisionStrategy`
- 如果使用 `Restart { max_retries }`，确认未超过最大重试次数
- 查看日志中 Supervisor 的决策记录

### Q4: API 返回 500 错误

**原因**：通常是 Runtime 数据源连接异常。

**解决**：
- 确认 Runtime 已正常启动
- 检查 `/api/v1/health` 返回的 `preflight_passed` 字段
- 查看日志中的错误堆栈

### Q5: 消息投递失败

**原因**：可能是目标 Cell 未注册、层间调用违规或信箱已满。

**解决**：
- 确认目标 Cell 已注册（通过 `/api/v1/cells` 查看）
- 检查信号的方向是否符合层间调用规则（Oversight → Agent → Validate → Exec）
- 调整 `mailbox_capacity` 配置项增大信箱容量

### Q6: 如何查看完整的审计轨迹

使用 CLI 工具的 `why` 命令：

```bash
# 查看某个 correlation_id 的完整链路
cargo run -p axiom-cli -- why <correlation_id>
```

Witness 链会展示从信号发出到处理完成的完整记录。

### Q7: 如何启用分布式追踪

在配置文件中启用 OpenTelemetry：

```toml
[telemetry]
enabled = true
otlp_endpoint = "http://jaeger:4318/v1/traces"
```

启动后可在 Jaeger UI 中查看完整的链路追踪。详见 [deployment.md](deployment.md)。

### Q8: 如何备份和恢复数据

```bash
# 创建手动备份
cargo run -p axiom-cli -- backup create

# 列出所有备份
cargo run -p axiom-cli -- backup list

# 从备份恢复
cargo run -p axiom-cli -- backup restore <backup-path>
```

系统也支持自动定时备份，详见 [deployment.md](deployment.md)。

---

## 更多资源

- [架构设计文档](ARCHITECTURE.md) — 完整的架构设计说明
- [部署指南](deployment.md) — 生产环境部署
- [配置参考](configuration.md) — 全部配置项说明
- [开发指南](development.md) — 参与项目开发
- [插件开发指南](plugin-development.md) — 开发 WASM/Native 插件
- [安全策略](SECURITY.md) — 安全相关信息
- [商用就绪路线图](commercial-readiness-roadmap.md) — 商用化计划
