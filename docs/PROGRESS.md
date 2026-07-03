# Axiom Core 项目进度总览

> **最后更新:** 2026-07-04
> **当前阶段:** v0.1.0 完成，v0.2.0 待开始
> **代码基线:** master @ 949fc8c
> **测试总数:** 390+ (全部通过)
> **质量门禁:** ✅ build / ✅ test / ✅ archcheck / ✅ xtask gatecheck --strict

---

## v0.1.0 完成情况

| 阶段 | 状态 | 描述 | 测试数 |
|------|------|------|--------|
| P0 基础设施 | ✅ 完成 | 三层门禁系统 + 9 层架构 + 18 个 crate | - |
| P1 核心原语 | ✅ 完成 | Cell / Signal / Axiom / Witness / Lens / Entropy | 197 |
| P2 架构债务修复 | ✅ 完成 | 编译期架构门禁 + 事前约束体系 + 死代码清理 | 391 |
| P5 Agent工具链 | ✅ 完成 | LLM / Tool / Memory / Planner / Prompt / Identity | 391 |
| P6 Agent集成 | ✅ 完成 | axiom-agent门面 + AgentBuilder + 端到端测试 | 391 |
| Phase 6 生产就绪 | ✅ 完成 | 性能基准 / 压力测试 / CI/CD / 文档 / 发布准备 | 391 |
| 自动注入机制 | ✅ 完成 | #[cell]/#[signal]/#[tool]/#[guard]/#[capability] 宏系统 | 391 |
| 能力维度版本管理 | ✅ 完成 | 8大能力维度自动注册与兼容性检查 | 391 |
| 架构治理现代化 | ✅ 完成 | 单一数据源 + 编译期强制 + 事前约束 + 自约束 | 391 |

---

## v0.1.0 已交付功能

### Task 1: 自动注入机制 — ✅ 完成
- [x] `#[cell(layer="...")]` 宏 — 层标记、WitnessGenerator、监督策略
- [x] `#[signal(kind="...", layer="...")]` 宏 — 必需字段、Schema验证、序列化
- [x] `#[tool]` 宏 — Tool trait、权限检查、Witness记录
- [x] `#[guard]` 宏 — Guard trait、检查逻辑、Witness记录
- [x] `#[capability(dim="...", version="...")]` 宏 — 版本注册、兼容性策略

### Task 2: 能力维度版本管理 — ✅ 完成
- [x] 8大能力维度定义 (Witness/Schema/Layer/Tool/Guard/Identity/Entropy/Runtime)
- [x] CapabilityDescriptor 结构体
- [x] CAPABILITY_REGISTRY 分布式切片 (linkme)
- [x] CapabilityVersionRegistry 管理类
- [x] 自动兼容性检查
- [x] 集成测试

### Task 3: Witness 增强 — ✅ 完成
- [x] 添加 kind 字段和 WitnessKind 枚举
- [x] WITNESS_REGISTRY 分布式切片
- [x] 哈希链完整性验证增强

### Task 4: 编译期约束强化 — ✅ 完成
- [x] CanSendTo trait 编译期强制层间调用规则
- [x] 约束7: 能力维度版本管理规则
- [x] 所有核心能力必须通过 #[capability] 宏注册版本

### Task 5: 架构治理现代化 — ✅ 完成
- [x] `.axiom/architecture.toml` 单一数据源
- [x] `tools/archcheck` 独立架构检查工具
- [x] `xtask` 统一任务入口（gatecheck / precommit / state / new_crate）
- [x] 18 个 crate 的编译期门禁（build.rs 自动执行）
- [x] 事前约束体系（提示词 + 脚手架 + pre-commit + 编译期 + CI）
- [x] 自约束机制（archcheck 自身也受规则约束）
- [x] 会话引导协议（bootstrap.md）
- [x] 提示词模板（architecture-constraints.md）
- [x] 严格代码审查修复（6 Critical + 9 Warning）

---

## v0.2.0 开发计划

### 阶段目标：生产深度 > 功能广度

| Phase | 周期 | 目标 |
|-------|------|------|
| Phase 1 | 1周 | Lens 原语实现 — 完成5原语故事 |
| Phase 2 | 2周 | Store 持久化 — SQLite + 文件系统后端 |
| Phase 3 | 1周 | 约束运行时统一 — 编译期约束在运行时总线层强制执行 |
| Phase 4 | 1周 | 现有 crate 深化 — identity/prompt/planner/memory |
| Phase 5 | 1周 | API 稳定与发布 — v0.2.0 发布 |

### v0.2.0 关键里程碑

| 里程碑 | 状态 | 说明 |
|--------|------|------|
| 核心原语完整 (5/5) | 📋 待开始 | Lens 原语实现 |
| Witness 链持久化 | 📋 待开始 | SQLite + 文件系统后端 |
| 约束运行时闭环 | 📋 待开始 | 编译期 + 运行期约束统一 |
| API 稳定 | 📋 待开始 | v1 API 边界定义 |
| 测试数量 > 500 | 📋 待开始 | 新增测试补齐 |

---

## 代码质量指标

| 指标 | 当前值 | v0.2.0 目标 |
|------|--------|------------|
| 测试总数 | 390+ | 500+ |
| Clippy 警告 | 0 | 0 |
| 死代码率 | ~0% | 0% |
| 文档覆盖率 | 高 | 高 |
| Crate 数量 | 18 | 18 (不新增) |
| 基准测试 | 4组 (17个bench) | 扩展 |
| 压力测试 | 1 (stress binary) | 扩展 |
| 核心原语完整性 | 5/5 | 5/5 |
| 持久化支持 | 仅内存 | SQLite + 文件系统 |
| 约束覆盖 | 编译期 + 事前 | 编译期 + 运行期闭环 |
| 架构违规 | 0 | 0 |

---

## 关键文件索引

| 文件 | 说明 |
|------|------|
| [CHANGELOG.md](../CHANGELOG.md) | 变更日志 |
| [docs/HANDOVER.md](HANDOVER.md) | 项目交接文档 |
| [docs/architecture-diagram.md](architecture-diagram.md) | 架构设计图 |
| [docs/plans/architecture-governance-implementation.md](plans/architecture-governance-implementation.md) | 架构治理计划 |
| [docs/plans/pre-constraint-enforcement.md](plans/pre-constraint-enforcement.md) | 事前约束计划 |
| [docs/guide/getting-started.md](guide/getting-started.md) | 快速上手 |
| [docs/guide/core-concepts.md](guide/core-concepts.md) | 核心概念 |
| [docs/guide/creating-an-agent.md](guide/creating-an-agent.md) | Agent创建教程 |
| [docs/guide/best-practices.md](guide/best-practices.md) | 最佳实践 |
| [crates/axiom-bench/](../crates/axiom-bench/) | 性能基准和压力测试 |
| [.github/workflows/ci.yml](../.github/workflows/ci.yml) | CI/CD配置 |
| [.axiom/architecture.toml](../.axiom/architecture.toml) | 架构规则唯一真相源 |
| [.axiom/bootstrap.md](../.axiom/bootstrap.md) | 会话引导协议 |
| [.axiom/prompts/architecture-constraints.md](../.axiom/prompts/architecture-constraints.md) | 提示词模板 |

---

## 下一阶段准备

v0.2.0 将聚焦以下三大架构缺口的修复：

1. **Lens 原语实现** — 完成 5 个核心原语的故事
2. **Witness 链持久化** — SQLite + 文件系统后端，重启后审计链不丢失
3. **约束运行时闭环** — 编译期约束在运行时总线层强制执行

详细计划请参考 [pre-constraint-enforcement.md](plans/pre-constraint-enforcement.md) 和 [architecture-diagram.md](architecture-diagram.md)。
