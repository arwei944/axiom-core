# Axiom Core 项目交接文档

> **版本:** v0.1.0 (生产就绪)
> **交接日期:** 2026-07-04
> **代码基线:** master @ 949fc8c
> **仓库:** https://github.com/arwei944/axiom-core
> **维护者:** Axiom Core 团队

---

## 1. 项目概述

### 1.1 项目定位

Axiom Core 是一个面向智能体（Agent）的确定性优先运行时框架，核心目标是：
- **架构即约束** — 编译期自动注入架构规则，违规直接阻断
- **问题一秒定位** — Witness 哈希链 + CorrelationId + VectorClock
- **功能方便增删** — 宏 + 分布式注册表 (linkme) 实现插件化
- **低熵运行** — 熵值实时监控，黄线告警、红线熔断、自动减熵

### 1.2 核心设计理念

| 设计原则 | 实现机制 |
|---------|---------|
| 层级隔离 | 9 层架构，Layer N 只能依赖 Layer >= N |
| 私有状态 | Cell 状态完全私有，只能通过 `handle()` 修改 |
| 不变式保障 | Axiom trait + 编译期门禁，状态变更前自动检查 |
| 可追溯性 | Witness 哈希链 + CorrelationId + VectorClock |
| 熵监控 | EntropyScore 8 因子模型 + EntropyGovernor 冷却降级 |
| 自动注入 | `#[cell]/#[signal]/#[tool]/#[guard]/#[capability]` 宏编译期注入 |
| 版本管理 | 8大能力维度独立版本，自动兼容性检查 |
| 架构治理 | 单一数据源 `.axiom/architecture.toml` + 编译期强制 + 事前约束 |

### 1.3 架构约束（Hard Constraints）

- 禁止 `async-trait`（R-004）
- Crate 依赖必须遵循 N → ≥ N 方向
- 外部依赖必须在白名单（R-022，30 个 audited deps）
- Axiom `check()` 必须纯函数（无 async, 无 IO）
- Migration `migrate()` 必须纯函数
- Schema `validate()` 必须同步执行
- 层间通信必须遵循 `can_send_to` 规则
- 所有核心能力必须通过 `#[capability]` 宏注册版本
- 所有 crate 的 `build.rs` 必须调用 `archcheck::build_hook::check_current_crate()`

---

## 2. 代码结构

### 2.1 Crate 分层（9层）

```
Layer 0: 顶层应用 — axiom-cli, axiom-bench
Layer 1: 可视化   — axiom-viz
Layer 2: Agent 门面 — axiom-identity, axiom-prompt
Layer 3: 监督与集成 — axiom-mcp, axiom-alert, axiom-agent, axiom-oversight
Layer 4: 运行时与协调 — axiom-distributed, axiom-planner, axiom-runtime
Layer 5: 存储与工具 — axiom-llm, axiom-tool, axiom-memory, axiom-store
Layer 6: （预留）
Layer 7: 核心原语 — axiom-core
Layer 8: Proc-macro（豁免） — axiom-macros
```

**铁律**：Layer N 的 crate **只能依赖** Layer >= N 的 crate

### 2.2 关键文件索引

| 模块 | 文件 | 核心类型/函数 |
|------|------|-------------|
| 架构规则 | [`.axiom/architecture.toml`](.axiom/architecture.toml) | 唯一真相源 |
| 架构检查 | [`tools/archcheck/src/build_hook.rs`](tools/archcheck/src/build_hook.rs) | `check_current_crate()` |
| 任务运行器 | [`xtask/src/main.rs`](xtask/src/main.rs) | `gatecheck`, `precommit`, `state` |
| 会话引导 | [`.axiom/bootstrap.md`](.axiom/bootstrap.md) | 强制 checklist |
| 提示词模板 | [`.axiom/prompts/architecture-constraints.md`](.axiom/prompts/architecture-constraints.md) | 系统提示词 |
| Cell | [`crates/axiom-core/src/cell.rs`](crates/axiom-core/src/cell.rs) | `Cell`, `DynHandleCell`, `CellHandle` |
| Signal | [`crates/axiom-core/src/signal.rs`](crates/axiom-core/src/signal.rs) | `Signal`, `SignalEnvelope`, `VectorClock` |
| Axiom | [`crates/axiom-core/src/axiom.rs`](crates/axiom-core/src/axiom.rs) | `Axiom`, `DynAxiom`, `DynAxiomChain` |
| Witness | [`crates/axiom-core/src/witness.rs`](crates/axiom-core/src/witness.rs) | `Witness`, `WitnessBuilder`, `WitnessBatch` |
| Lens | [`crates/axiom-core/src/lens.rs`](crates/axiom-core/src/lens.rs) | Lens 原语（v0.2.0 完善） |
| 运行时 | [`crates/axiom-runtime/src/runtime.rs`](crates/axiom-runtime/src/runtime.rs) | `AxiomRuntime`, `RuntimeBuilder` |
| 消息总线 | [`crates/axiom-runtime/src/bus.rs`](crates/axiom-runtime/src/bus.rs) | `MessageBus` |
| 监督 | [`crates/axiom-runtime/src/supervisor.rs`](crates/axiom-runtime/src/supervisor.rs) | `Supervisor` |
| 熵治理 | [`crates/axiom-oversight/src/entropy_governor.rs`](crates/axiom-oversight/src/entropy_governor.rs) | `EntropyGovernorCell` |
| 宏 | [`crates/axiom-macros/src/lib.rs`](crates/axiom-macros/src/lib.rs) | `#[cell]`, `#[signal]`, `#[tool]`, `#[guard]`, `#[capability]` |
| 创建 crate | [`crates/axiom-cli/src/commands/new_crate.rs`](crates/axiom-cli/src/commands/new_crate.rs) | `xtask new_crate` |
| 预提交 | [`xtask/src/commands/precommit.rs`](xtask/src/commands/precommit.rs) | `xtask precommit` |

---

## 3. 核心设计模式

### 3.1 Cell::handle — "Drain Inside, Return Everything"

**问题:** RPITIT 的不透明 future 会将 `&mut ctx` 借用绑定到 `'a` 生命周期。

**解决方案:**
```rust
fn handle<'a>(
    &'a mut self,
    signal: Self::Message,
    ctx: &'a mut CellContext<'a>,
) -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a;
```

- 实现内部调用 `ctx.end_processing()` 排空所有数据
- 返回三元组，调用方**永远不要**在 `handle().await` 后访问 `ctx`

### 3.2 循环中多次调用 handle

使用 `Arc<Mutex<Cell>>` 包装，每次循环获取本地 guard。

### 3.3 闭包模式处理 `?` 运算符

```rust
async move {
    let result: Result<()> = (|| {
        ctx.emit_event(event, Layer::Exec)?;
        Ok(())
    })();
    let (outgoing, witnesses) = ctx.end_processing();
    (result, outgoing, witnesses)
}
```

### 3.4 DynHandleCell — 类型擦除调度

从 JSON 反序列化为具体类型，调用 `Cell::handle`，通过 `CellHandle` 包装后统一调度。

### 3.5 分布式注册表 (linkme)

- `#[cell]` 宏 → 自动注册
- `#[capability]` 宏 → `CAPABILITY_REGISTRY` slice
- 零成本静态注册，运行时直接遍历

### 3.6 自动注入宏系统

| 宏 | 自动注入内容 |
|----|-------------|
| `#[cell(layer="...")]` | 层标记、`LayerOf`、`WitnessGenerator` |
| `#[signal(kind="...", layer="...")]` | 必需字段、`Signal` trait、`Schema`验证、序列化 |
| `#[tool]` | `Tool` trait、权限检查、Witness记录 |
| `#[guard]` | `Guard` trait、检查逻辑、Witness记录 |
| `#[capability(dim="...", version="...")]` | 版本注册、兼容性策略（8个维度） |

---

## 4. 当前状态与已知问题

### 4.1 已完成（v0.1.0）

- ✅ 9 层分层架构，18 个 crate 全部注册
- ✅ 编译期架构门禁（每个 crate 的 build.rs 自动执行）
- ✅ 事前约束体系（提示词 + 脚手架 + pre-commit + 编译期 + CI）
- ✅ 五大核心原语（Cell/Signal/Axiom/Witness/Lens/Entropy）
- ✅ 8 大能力维度版本管理
- ✅ 自动注入机制（5 个宏）
- ✅ 390+ 测试全部通过
- ✅ Clippy 零警告
- ✅ `cargo publish --dry-run` 通过

### 4.2 v0.2.0 待完成

| Phase | 周期 | 目标 |
|-------|------|------|
| Phase 1 | 1周 | Lens 原语实现 — 完成5原语故事 |
| Phase 2 | 2周 | Store 持久化 — SQLite + 文件系统后端 |
| Phase 3 | 1周 | 约束运行时统一 — 编译期约束在运行时总线层强制执行 |
| Phase 4 | 1周 | 现有 crate 深化 — identity/prompt/planner/memory |
| Phase 5 | 1周 | API 稳定与发布 — v0.2.0 发布 |

### 4.3 已知问题 / 技术债务

| 问题 | 位置 | 计划修复 |
|------|------|---------|
| Lens 原语待完善 | `axiom-core/src/lens.rs` | v0.2.0 Phase 1 |
| Store 仅内存后端 | `axiom-store/src/store.rs` | v0.2.0 Phase 2 |
| 部分 crate 实现较薄 | identity/prompt/planner/memory | v0.2.0 Phase 4 |

### 4.4 设计待决策

1. **Lens 缓存策略** — LRU 还是基于 VectorClock 的失效策略？
2. **Store 后端选型** — SQLite vs 文件系统，还是双后端？

---

## 5. 开发工作流

### 5.1 质量门禁（必须全部通过）

```bash
# 1. 编译检查（自动触发架构门禁）
cargo check --workspace

# 2. 测试
cargo test --workspace

# 3. 架构检查
cargo run -p archcheck -- --validate-architecture
cargo run -p archcheck -- --list-crates
cargo run -p archcheck --

# 4. 严格模式
cargo run -p xtask -- gatecheck --strict

# 5. 预提交检查
cargo run -p xtask -- precommit
```

### 5.2 新功能开发步骤

1. **读取约束** — 读取 `.axiom/prompts/architecture-constraints.md`
2. **检查状态** — 运行 `cargo check --workspace` 和 `cargo run -p archcheck --`
3. **创建 crate**（如需要）— `cargo run -p xtask -- new_crate --name <name> --layer <0-7>`
4. **添加依赖** — 编辑 Cargo.toml，编译期自动检查
5. **实现代码** — 编写 Rust 代码
6. **运行测试** — `cargo test -p <crate>`
7. **架构检查** — `cargo run -p xtask -- gatecheck --strict`
8. **提交** — `git add . && git commit && git push`

### 5.3 添加新依赖 Checklist

- [ ] 是 `axiom-*` 内部依赖？→ 检查层方向（Layer N 只能依赖 Layer >= N）
- [ ] 是第三方依赖？→ 检查 `[audited-deps]` 是否已审计
- [ ] 是禁止依赖？→ 检查 `[forbidden-deps]`（如 async-trait）
- [ ] 需要豁免？→ 添加到 `[reverse-dependency-exemptions]` 并写明原因

### 5.4 常见坑

| 坑 | 现象 | 解决方案 |
|----|------|---------|
| RPITIT 借用冲突 | E0499 二次借用 | 使用 drain-inside 模式，不要在 handle 后访问 ctx |
| 循环中调用 handle | E0499 循环借用 | 用 `Arc<Mutex<Cell>>` 包装，每次循环取 guard |
| async block 中用 `?` | 编译错误，返回类型不匹配 | 用闭包模式 `(|| { ...?; Ok(()) })()` |
| 宏展开后调试困难 | 无法看到宏生成的代码 | `cargo expand` 或 trybuild 测试 |
| Windows 增量编译 ICE | rustc 内部错误 | `cargo clean -p <crate>` 后重编 |

---

## 6. 架构治理体系

### 6.1 约束时机全景

```
Layer -1: 提示词约束 — 生成代码前，智能体已知晓规则
Layer 0: 脚手架约束 — 创建 crate 时自动注册，无法忘记
Layer 1: IDE/LSP — 待实施（实时反馈）
Layer 2: 预提交约束 — git commit 前自动检查
Layer 3: 编译期约束 — build.rs 自动执行，违规 panic
Layer 4: CI 约束 — push/PR 自动检查，非阻塞报告
```

### 6.2 工具链

| 工具 | 命令 | 能力 |
|------|------|------|
| **archcheck** | `cargo run -p archcheck -- --validate-architecture` | 验证 TOML 语法 |
| | `cargo run -p archcheck -- --list-crates` | 列出 18 个注册 crate |
| | `cargo run -p archcheck --` | 完整架构检查 |
| | `cargo run -p archcheck -- --format json --output report.json` | JSON 报告 |
| **xtask** | `cargo run -p xtask -- gatecheck --strict` | 严格模式，违规则退出 1 |
| | `cargo run -p xtask -- gatecheck` | 非严格模式，仅警告 |
| | `cargo run -p xtask -- state --output .axiom/state.toml` | 生成状态快照 |
| | `cargo run -p xtask -- precommit --install` | 安装 git pre-commit 钩子 |
| | `cargo run -p xtask -- precommit` | 运行预提交检查 |
| **new_crate** | `cargo run -p xtask -- new_crate --name <name> --layer <0-7>` | 创建新 crate |

### 6.3 审计依赖清单（30 个）

```
tokio, serde, serde_json, thiserror, anyhow, tracing, tracing-subscriber,
sha2, uuid, futures, clap, ratatui, crossterm, syn, quote, proc-macro2,
linkme, trybuild, regex, parking_lot, dashmap, sqlx, snap, tempfile,
criterion, schemars, reqwest, url, axum, toml, walkdir, once_cell, archcheck
```

### 6.4 豁免机制

| 类型 | 规则 | 原因 |
|------|------|------|
| **Proc-macro 豁免** | `axiom-macros` → `axiom-core` | Proc-macro 必须引用 core 类型 |
| **反向依赖豁免** | `axiom-agent` → `axiom-identity`, `axiom-prompt` | Agent 需要调用 facade |

---

## 7. 8 大能力维度版本管理

| 维度 | 用途 | 典型场景 |
|------|------|---------|
| Witness | 审计链版本 | 状态转换记录格式 |
| Schema | 信号协议版本 | 消息序列化格式 |
| Layer | 架构层版本 | 层间调用规则 |
| Tool | 工具接口版本 | 工具执行协议 |
| Guard | 约束规则版本 | 权限检查规则 |
| Identity | 身份协议版本 | Agent身份/权限集 |
| Entropy | 熵治理版本 | 阈值策略/治理动作 |
| Runtime | 运行时协议版本 | 监督策略/邮箱配置 |

---

## 8. 相关文档

| 文档 | 用途 |
|------|------|
| [README.md](../README.md) | 项目介绍 |
| [DEVELOPMENT.md](../DEVELOPMENT.md) | 开发门禁 |
| [PROGRESS.md](PROGRESS.md) | 进度总览 |
| [architecture-diagram.md](architecture-diagram.md) | 架构设计图 |
| [architecture-governance-implementation.md](plans/architecture-governance-implementation.md) | 架构治理计划 |
| [pre-constraint-enforcement.md](plans/pre-constraint-enforcement.md) | 事前约束计划 |
| [.axiom/bootstrap.md](../.axiom/bootstrap.md) | 会话引导协议 |
| [.axiom/prompts/architecture-constraints.md](../.axiom/prompts/architecture-constraints.md) | 提示词模板 |
| [.axiom/architecture.toml](../.axiom/architecture.toml) | 架构规则唯一真相源 |

---

## 9. 快速命令参考

```bash
# 编译检查（自动触发架构门禁）
cargo check --workspace

# 测试
cargo test --workspace

# 架构检查
cargo run -p archcheck -- --validate-architecture
cargo run -p archcheck -- --list-crates
cargo run -p archcheck --

# 严格模式
cargo run -p xtask -- gatecheck --strict

# 预提交检查
cargo run -p xtask -- precommit --install
cargo run -p xtask -- precommit

# 创建新 crate
cargo run -p xtask -- new_crate --name <name> --layer <0-7>

# 状态快照
cargo run -p xtask -- state --output .axiom/state.toml
```

---

## 10. 联系人

如有疑问，优先查阅：
1. [.axiom/bootstrap.md](../.axiom/bootstrap.md) — 会话引导协议
2. [.axiom/prompts/architecture-constraints.md](../.axiom/prompts/architecture-constraints.md) — 架构规则
3. [.axiom/architecture.toml](../.axiom/architecture.toml) — 单一真相源
4. [docs/plans/pre-constraint-enforcement.md](plans/pre-constraint-enforcement.md) — 事前约束计划
5. 对应模块的单元测试（最准确的使用示例）

---

## 11. 性能指标（v0.1.0）

| 指标 | 当前值 | 目标 |
|------|--------|------|
| 测试总数 | 390+ | ≥ 200 ✅ |
| Clippy 警告 | 0 | 0 ✅ |
| 死代码率 | ~0% | 0% ✅ |
| Crate 数量 | 18 | 18 |
| 架构违规 | 0 | 0 ✅ |
| 编译期门禁 | 100% 覆盖 | 100% ✅ |
