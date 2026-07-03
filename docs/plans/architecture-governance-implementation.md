# AI 智能体驱动的架构治理
## 完整开发计划 v3.0 — 原子任务拆解

> **范围**：以编译期强制执行的、面向 AI 智能体对齐的新系统，完全替代原有架构治理体系。
> **原则**：每个任务都是原子化的、可验证的单元，具备明确的前置条件、执行步骤和验收标准。
> **策略**：Parachute replacement（ parachute 替换）—— 4 个阶段，零影响着陆、渐进切换、完全退役。
> **状态**：就绪，可原子执行。

---

## 目录

1. [项目概述](#1-项目概述)
2. [现状分析](#2-现状分析)
3. [设计原则](#3-设计原则)
4. [架构设计](#4-架构设计)
5. [Phase 0: 新系统着陆](#5-phase-0-新系统着陆)
6. [Phase 1: 观察者模式](#6-phase-1-观察者模式)
7. [Phase 2: 切换依赖](#7-phase-2-切换依赖)
8. [Phase 3: 退役旧系统](#8-phase-3-退役旧系统)
9. [验收标准](#9-验收标准)
10. [回滚程序](#10-回滚程序)
11. [风险登记册](#11-风险登记册)
12. [时间线](#12-时间线)
13. [完成定义](#13-完成定义)
14. [附录](#14-附录)

---

## 1. 项目概述

### 1.1 问题陈述

当前架构治理体系由以下部分组成：
- `crates/axiom-core/src/gate.rs` — 硬编码常量
- `tools/gate_check.rs` — 独立二进制，数据重复
- 无 CI 集成
- 无编译期强制执行

**关键失效点**：
1. `gate.rs` 与 `gate_check.rs` 之间的数据不一致
2. 9 个 crate 未纳入 `CRATE_LAYERS`
3. 4 个 crate 存在被禁止的 `async-trait` 依赖
4. `dev-dependencies` 未被检查
5. 无自动化执行 — 违规 silently 累积

### 1.2 根因

为人类开发者设计的架构约束假设：
- 基于社会/职业激励的自愿遵守
- 长期思维和延迟满足能力
- 记忆并应用复杂规则的能力

AI 智能体的运行约束不同：
- 目标优化：以最小 token/延迟成本完成用户任务
- 短注意力窗口：上下文压缩会降低早期指令的显著性
- 工具成本不对称：编写代码成本低，运行测试成本高
- 无社会激励：无职业声誉、无团队压力

**结论**：当前治理体系与 AI 智能体开发模式不兼容。

### 1.3 解决方案

用**编译期强制执行**替代自愿遵守：

| 层级 | 机制 | 强制方式 |
|------|------|----------|
| 数据 | `.axiom/architecture.toml` | 唯一数据源 |
| 工具 | `tools/archcheck/` | 自动化检查 |
| 集成 | 每个 crate 的 `build.rs` | 编译期阻断 |
| 记忆 | `.axiom/state.toml` + `bootstrap.md` | 会话感知 |
| 目标 | 系统提示编码 | 硬约束优先级 |

### 1.4 成功标准

- `cargo xtask gatecheck --strict` 可检测到的架构违规为零
- 所有违规产生**编译期错误**，而非警告
- 旧系统完全退役
- 现有功能零回归

---

## 2. 现状分析

### 2.1 现有文件清单

| 路径 | 类型 | 状态 | 所需操作 |
|------|------|------|----------|
| `crates/axiom-core/src/gate.rs` | 源码 | 活跃，硬编码数据 | 重构为加载器 |
| `tools/gate_check.rs` | 工具 | 活跃，独立运行 | 替换为 archcheck 库 |
| `crates/*/Cargo.toml`（18 个文件） | 配置 | 活跃 | Phase 3 添加 `archcheck` feature |
| `.github/workflows/` | CI | 无架构检查 | Phase 1 添加观察者，Phase 3 添加门禁 |

### 2.2 已验证违规

| ID | 违规 | 影响文件 | 严重度 | 修复策略 |
|----|------|----------|--------|----------|
| V-01 | 依赖中包含 `async-trait` | `axiom-llm`、`axiom-mcp`、`axiom-planner`、`axiom-tool` | BLOCKER | 替换为 `BoxFuture` |
| V-02 | `axiom-macros` 反向依赖 `axiom-core` | `crates/axiom-macros/Cargo.toml` | BLOCKER | 将 `axiom-macros` 提升至 level 8 |
| V-03 | `axiom-macros` dev-dep 包含 `async-trait` | `crates/axiom-macros/Cargo.toml` | BLOCKER | 从 dev-dep 移除 |
| V-04 | 9 个 crate 未纳入 `CRATE_LAYERS` | `gate.rs`、`gate_check.rs` | BLOCKER | 添加到 `architecture.toml` |
| V-05 | `AUDITED_DEPS` 不一致 | `gate.rs` vs `gate_check.rs` | BLOCKER | 统一到 `architecture.toml` |
| V-06 | `pub mod gate` 泄露内部 API | `crates/axiom-core/src/lib.rs` | WARNING | 添加 `#[doc(hidden)]` |
| V-07 | `pub use linkme` 暴露第三方 crate | `crates/axiom-core/src/lib.rs` | WARNING | 移除 re-export |

### 2.3 Crate 注册表（18 个 crate）

| 层级 | Crates |
|------|--------|
| 0 | `axiom-cli`、`axiom-bench` |
| 1 | `axiom-viz` |
| 2 | `axiom-identity`、`axiom-prompt` |
| 3 | `axiom-mcp`、`axiom-alert`、`axiom-agent`、`axiom-oversight` |
| 4 | `axiom-distributed`、`axiom-planner`、`axiom-runtime` |
| 5 | `axiom-llm`、`axiom-tool`、`axiom-memory`、`axiom-store` |
| 6 | *（预留）* |
| 7 | `axiom-core` |
| 8 | `axiom-macros`（proc-macro 豁免） |

---

## 3. 设计原则

### P-1: 零影响着陆
新系统在 Phase 0 期间不得修改任何现有代码。所有新文件均为纯新增。

### P-2: 唯一数据源
`.axiom/architecture.toml` 是维护架构数据的**唯一位置**。所有其他文件均从此数据源生成或加载。

### P-3: 编译期强制执行
违规必须产生编译器错误，而非警告。构建必须失败。

### P-4: 渐进式增强
每个阶段建立在前一阶段之上。回滚始终可行，只需删除新文件。

### P-5: 向后兼容
`gate.rs` 的公共 API 保持不变。`gate_check.rs` CLI 在过渡期间保持不变。

### P-6: 原子任务
每个任务都是单一、可验证的单元，具有明确的通过/失败标准。

---

## 4. 架构设计

### 4.1 文件结构

```
.axiom/
├── architecture.toml      # 唯一数据源（Phase 0）
├── state.toml             # 自动生成的状态快照（Phase 1）
├── bootstrap.md           # 智能体会话启动（Phase 3）
├── violations/            # 违规账本（Phase 1+）
│   └── YYYY-MM-DD.md
└── baseline-report.json   # Phase 0 基线

tools/
├── archcheck/             # 新架构检查器（Phase 0）
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── loader.rs
│       ├── checker.rs
│       └── reporter.rs
└── gate_check.rs          # 旧系统（Phase 3: 删除或替换）

xtask/
├── Cargo.toml             # 任务封装（Phase 0）
└── src/
    └── main.rs            # cargo xtask gatecheck、cargo xtask state

crates/*/build.rs          # 编译期检查（Phase 3）
```

### 4.2 数据流

```
.axiom/architecture.toml
        │
        ├── tools/archcheck/ ── cargo xtask gatecheck ── CI/智能体
        │
        ├── crates/axiom-core/src/gate.rs ── axiom-cli
        │
        ├── crates/*/build.rs ── cargo check ── 编译期强制
        │
        └── .axiom/state.toml ── 智能体会话启动
```

### 4.3 技术选型

| 组件 | 技术 | 理由 |
|------|------|------|
| 配置格式 | TOML | 人类可读，Rust 原生 |
| CLI 解析 | `clap` | 已存在于 workspace 依赖 |
| TOML 解析 | `toml` crate | 成熟，广泛使用 |
| 构建脚本 | `std::env!` 宏 | 无外部依赖 |
| 状态生成 | `serde_json` | 已存在于 workspace |

---

## 5. Phase 0: 新系统着陆

**目标**：建立新治理系统作为独立模块。零修改现有代码。

**工期**：1 天
**回滚**：`git rm -rf .axiom tools/archcheck xtask`

### Task 0.1: 创建 `.axiom/architecture.toml`

**前置条件**：无

**子任务**：

#### T-0.1.1: 创建 `.axiom/` 目录
- **动作**：`mkdir -p .axiom/violations`
- **验证**：`ls -la .axiom/` 显示目录存在
- **验收**：目录存在，为空

#### T-0.1.2: 创建 `.axiom/architecture.toml` — crate 层级
- **动作**：编写 `[crate-layers]` 段，包含全部 18 个 crate 及正确层级分配
- **验证**：`cat .axiom/architecture.toml | grep -c "axiom-"` 等于 18
- **验收**：全部 18 个 crate 以正确层级存在

#### T-0.1.3: 创建 `.axiom/architecture.toml` — 禁止依赖
- **动作**：编写 `[forbidden-deps]` 段，包含 `async-trait = "R-004: Rust 1.75+ 已支持原生 async fn in traits"`
- **验证**：`grep "async-trait" .axiom/architecture.toml`
- **验收**：`async-trait` 已列出并附带原因

#### T-0.1.4: 创建 `.axiom/architecture.toml` — 审计依赖
- **动作**：编写 `[audited-deps]` 段，统一包含 `gate.rs` 和 `gate_check.rs` 中的所有依赖
- **验证**：`cargo xtask gatecheck --validate-architecture`（在 T-0.2 之后）
- **验收**：所有 workspace 依赖都被覆盖

#### T-0.1.5: 创建 `.axiom/architecture.toml` — dev-dependencies 审计
- **动作**：编写 `[dev-dependencies-audit] = true`
- **验证**：`grep "dev-dependencies-audit" .axiom/architecture.toml`
- **验收**：段存在且值为 `true`

#### T-0.1.6: 创建 `.axiom/architecture.toml` — proc-macro 豁免
- **动作**：编写 `[proc-macro-exemptions]`，记录 `axiom-macros` 对 `axiom-core` 的反向依赖
- **验证**：`grep -A 3 "proc-macro-exemptions" .axiom/architecture.toml`
- **验收**：豁免已记录并附有理由

#### T-0.1.7: 验证 TOML 语法
- **动作**：运行 `cargo run --bin archcheck --validate-architecture`（在 T-0.2.1 之后）
- **验证**：命令退出码为 0
- **验收**：无解析错误

**交付件**：`.axiom/architecture.toml` — 完整、有效、唯一数据源

---

### Task 0.2: 创建 `tools/archcheck/`

**前置条件**：T-0.1 完成

**子任务**：

#### T-0.2.1: 创建 `tools/archcheck/Cargo.toml`
- **动作**：编写 `Cargo.toml`，依赖包括：`toml`、`serde`、`clap`、`thiserror`、`walkdir`、`anyhow`
- **验证**：`cat tools/archcheck/Cargo.toml` 显示所有依赖
- **验收**：`cargo check -p archcheck` 退出码为 0

#### T-0.2.2: 创建 `tools/archcheck/src/main.rs`
- **动作**：编写 `main.rs`，定义 `clap` CLI：
  - `--format <text|json>`
  - `--output <path>`
  - `--validate-architecture`
  - `--list-crates`
- **验证**：`cargo run --bin archcheck -- --help` 显示所有选项
- **验收**：CLI 正确解析所有参数

#### T-0.2.3: 创建 `tools/archcheck/src/loader.rs`
- **动作**：实现 `Architecture::load(path: &str) -> Result<Architecture>`
  - 读取 `.axiom/architecture.toml`
  - 解析为结构体，字段：`crate_layers`、`forbidden_deps`、`audited_deps`、`dev_dep_audit_enabled`、`proc_macro_exemptions`
- **验证**：单元测试加载有效 TOML，返回正确结构体
- **验收**：`cargo test -p archcheck --lib loader` 通过

#### T-0.2.4: 创建 `tools/archcheck/src/checker.rs`
- **动作**：实现检查函数：
  - `check_crate_registered(arch, crate_name) -> Result<(), Violation>`
  - `check_dependency_direction(arch, crate_name, dep) -> Result<(), Violation>`
  - `check_forbidden_deps(arch, crate_name, dep) -> Result<(), Violation>`
  - `check_audited_deps(arch, dep) -> Result<(), Violation>`
  - `check_all(arch, workspace_path) -> Vec<Violation>`
- **验证**：针对已知违规的每个检查函数的单元测试
- **验收**：全部单元测试通过

#### T-0.2.5: 创建 `tools/archcheck/src/reporter.rs`
- **动作**：实现报告器：
  - `report_text(violations) -> String`
  - `report_json(violations) -> String`
- **验证**：单元测试验证输出格式
- **验收**：`--format json` 时输出为有效 JSON

#### T-0.2.6: 连接 main.rs
- **动作**：将 CLI 连接到 loader、checker、reporter
- **验证**：`cargo run --bin archcheck` 在干净仓库上无错误运行
- **验收**：干净仓库退出码为 0，有违规时退出码为 1

#### T-0.2.7: 验证零影响
- **动作**：运行 `cargo test --workspace`
- **验证**：所有测试通过
- **验收**：无现有测试被破坏

**交付件**：`tools/archcheck/` — 独立、可运行的架构检查器

---

### Task 0.3: 创建 `xtask/` 命令封装

**前置条件**：T-0.2 完成

**子任务**：

#### T-0.3.1: 创建 `xtask/Cargo.toml`
- **动作**：编写 `Cargo.toml`，依赖：`clap`、`anyhow`、`serde_json`
- **验证**：`cargo check -p xtask` 退出码为 0
- **验收**：包可编译

#### T-0.3.2: 创建 `xtask/src/main.rs` — `gatecheck` 子命令
- **动作**：实现 `gatecheck` 子命令：
  - 加载 `.axiom/architecture.toml`
  - 扫描 `crates/*/Cargo.toml`
  - 运行 `archcheck::check_all()`
  - 打印结果
  - 干净时退出 0，有违规时退出 1
- **验证**：`cargo xtask gatecheck` 产生输出
- **验收**：检测到已知违规 V-01 至 V-07

#### T-0.3.3: 创建 `xtask/src/main.rs` — `state` 子命令
- **动作**：实现 `state` 子命令：
  - 扫描 workspace 中所有 `Cargo.toml`
  - 提取 crate 元数据
  - 生成 `.axiom/state.toml`
- **验证**：`cargo xtask state` 创建文件
- **验收**：`.axiom/state.toml` 是有效 TOML

#### T-0.3.4: 验证 xtask 输出与 archcheck 一致
- **动作**：运行 `cargo run --bin archcheck` 和 `cargo xtask gatecheck`
- **验证**：比较输出，应等效
- **验收**：两者检测到相同的违规

**交付件**：`xtask/` — 架构治理命令封装

---

### Task 0.4: 建立基线

**前置条件**：T-0.1、T-0.2、T-0.3 完成

**子任务**：

#### T-0.4.1: 运行基线测试套件
- **动作**：`cargo test --workspace`
- **验证**：所有测试通过
- **验收**：退出码 0

#### T-0.4.2: 运行基线文档构建
- **动作**：`cargo doc --workspace --no-deps`
- **验证**：文档构建无错误
- **验收**：退出码 0

#### T-0.4.3: 运行 archcheck 基线
- **动作**：`cargo xtask gatecheck --format json --output .axiom/baseline-report.json`
- **验证**：文件已创建，包含全部 7 个已知违规
- **验收**：报告列出 V-01 至 V-07

#### T-0.4.4: 验证零影响
- **动作**：`git status --short`
- **验证**：仅新文件，无修改
- **验收**：输出仅显示 `A`（新增）状态，无 `M`（修改）

#### T-0.4.5: 提交 Phase 0
- **动作**：`git add .axiom/ tools/archcheck/ xtask/ && git commit -m "feat: Phase 0 — 新架构治理系统着陆"`
- **验证**：提交成功
- **验收**：`git log --oneline -1` 显示提交信息

**Phase 0 退出标准**：
- [ ] EC-0.1: `cargo test --workspace` 通过
- [ ] EC-0.2: `cargo doc --workspace --no-deps` 通过
- [ ] EC-0.3: `cargo xtask gatecheck` 运行并检测全部 7 个违规
- [ ] EC-0.4: `git status` 仅显示新文件，无修改
- [ ] EC-0.5: `.axiom/architecture.toml` 包含全部 18 个 crate

---

## 6. Phase 1: 观察者模式

**目标**：将新系统以非阻断观察者模式集成到 CI 中。验证与旧系统的准确性。

**工期**：2 天
**回滚**：`git rm .github/workflows/architecture-observer.yml`

### Task 1.1: 与旧系统并行验证

**前置条件**：Phase 0 完成

**子任务**：

#### T-1.1.1: 运行旧版 gatecheck
- **动作**：`cargo run --bin gatecheck > /tmp/legacy-report.txt 2>&1`
- **验证**：文件存在，非空
- **验收**：旧版报告已生成

#### T-1.1.2: 运行新系统
- **动作**：`cargo xtask gatecheck > /tmp/new-report.txt 2>&1`
- **验证**：文件存在，非空
- **验收**：新版报告已生成

#### T-1.1.3: 对比报告
- **动作**：`diff /tmp/legacy-report.txt /tmp/new-report.txt`
- **验证**：捕获 diff 输出
- **验收**：无差异，或差异已记录

#### T-1.1.4: 分析差异
- **若新系统更严格**：记录为"增强检查"，更新 `.axiom/architecture.toml` 理由
- **若新系统漏检**：在继续前修复 checker.rs
- **验收**：所有差异已解决或记录

#### T-1.1.5: 提交验证
- **动作**：`git add .axiom/ && git commit -m "chore: 将 archcheck 与旧版 gatecheck 并行验证"`
- **验证**：提交成功
- **验收**：验证结果已保留

**交付件**：验证报告，显示新系统匹配或超越旧系统准确度

---

### Task 1.2: CI 观察者工作流

**前置条件**：T-1.1 完成

**子任务**：

#### T-1.2.1: 创建 `.github/workflows/architecture-observer.yml`
- **动作**：编写 YAML 文件：
  ```yaml
  name: Architecture Observer
  on: [push, pull_request]
  jobs:
    observe:
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
        - run: cargo xtask gatecheck --format json --output arch-report.json
        - uses: actions/upload-artifact@v4
          with:
            name: architecture-report
            path: arch-report.json
  ```
- **验证**：`cat .github/workflows/architecture-observer.yml` 显示有效 YAML
- **验收**：文件存在，YAML 有效

#### T-1.2.2: 触发 CI 运行
- **动作**：`git push` 或创建测试 PR
- **验证**：GitHub Actions 显示工作流运行
- **验收**：工作流在 push/PR 时触发

#### T-1.2.3: 验证产物上传
- **动作**：检查 GitHub Actions 产物
- **验证**：`arch-report.json` 出现在产物中
- **验收**：产物可下载

#### T-1.2.4: 验证非阻断
- **动作**：检查工作流设置
- **验证**：无 `if: failure()` 条件，无必选检查
- **验收**：工作流失败不阻断合并

#### T-1.2.5: 提交 CI 工作流
- **动作**：`git add .github/workflows/architecture-observer.yml && git commit -m "ci: 添加架构观察者工作流"`
- **验证**：提交成功
- **验收**：工作流在 git 历史中

**交付件**：非阻断 CI 观察者，生成架构报告

---

### Task 1.3: 状态文件生成

**前置条件**：Phase 0 完成

**子任务**：

#### T-1.3.1: 实现 `cargo xtask state` 核心逻辑
- **动作**：在 `xtask/src/main.rs` 中实现状态生成：
  - 扫描 `crates/*/Cargo.toml`
  - 提取：crate 名称、版本、依赖
  - 构建 `State` 结构体
- **验证**：`cargo xtask state` 无错误运行
- **验收**：命令退出码 0

#### T-1.3.2: 生成 `.axiom/state.toml`
- **动作**：运行 `cargo xtask state --output .axiom/state.toml`
- **验证**：文件存在
- **验收**：`cat .axiom/state.toml` 显示有效 TOML

#### T-1.3.3: 验证状态完整性
- **动作**：`grep -c "\[crates\]" .axiom/state.toml`
- **验证**：计数匹配 workspace crate 数量
- **验收**：全部 18 个 crate 存在

#### T-1.3.4: 提交状态生成
- **动作**：`git add .axiom/state.toml && git commit -m "feat: 添加架构状态生成器"`
- **验证**：提交成功
- **验收**：状态文件在 git 中

**交付件**：`.axiom/state.toml` — 自动生成的架构快照

---

### Task 1.4: 准确度验证期

**前置条件**：T-1.1、T-1.2、T-1.3 完成

**子任务**：

#### T-1.4.1: 运行观察者 5 天
- **动作**：等待 5 天，收集所有 CI 产物
- **验证**：收集到 5 个 `arch-report.json` 文件
- **验收**：每日报告已生成

#### T-1.4.2: 分析一致性
- **动作**：对比全部 5 个报告
- **验证**：报告稳定（无随机失败）
- **验收**：零误报、零漏报

#### T-1.4.3: 修复问题
- **动作**：若发现差异，修复 `archcheck`
- **验证**：重新运行验证
- **验收**：所有报告一致

#### T-1.4.4: 记录发现
- **动作**：将发现添加到 `.axiom/validation-report.md`
- **验证**：文件存在
- **验收**：报告记录准确度指标

#### T-1.4.5: 提交验证
- **动作**：`git add .axiom/validation-report.md && git commit -m "chore: Phase 1 观察者验证完成"`
- **验证**：提交成功
- **验收**：验证结果已保留

**Phase 1 退出标准**：
- [ ] EC-1.1: 新系统输出与旧系统在所有检查上一致
- [ ] EC-1.2: CI 观察者运行 5 天且不阻断合并
- [ ] EC-1.3: `.axiom/state.toml` 有效且完整
- [ ] EC-1.4: 零误报/漏报已记录

---

## 7. Phase 2: 切换依赖

**目标**：旧版 `gate.rs` 和 `gate_check.rs` 成为新系统的瘦客户端。数据源从硬编码切换为 `.axiom/architecture.toml`。

**工期**：1 天
**回滚**：`git revert HEAD~N` 恢复各提交

### Task 2.1: 重构 `gate.rs`

**前置条件**：Phase 1 完成

**子任务**：

#### T-2.1.1: 读取并分析当前 `gate.rs`
- **动作**：读取 `crates/axiom-core/src/gate.rs`
- **验证**：识别所有硬编码常量和公共函数
- **验收**：列出需保留项：
  - 常量：`CRATE_LAYERS`、`FORBIDDEN_DEPS`、`AUDITED_DEPS`
  - 函数：`crate_level()`、`verify_dependencies()`、`audit_dependency()`

#### T-2.1.2: 为 architecture.toml 添加 `include_str!`
- **动作**：在 `gate.rs` 中添加：
  ```rust
  static ARCHITECTURE_TOML: &str = include_str!("../../../.axiom/architecture.toml");
  ```
- **验证**：`cargo check -p axiom-core` 成功
- **验收**：文件编译通过

#### T-2.1.3: 实现 TOML 解析器
- **动作**：实现 `parse_architecture_toml(toml: &str) -> Result<Architecture>`
- **验证**：单元测试解析有效 TOML
- **验收**：`cargo test -p axiom-core --lib gate_loader_tests` 通过

#### T-2.1.4: 实现缓存加载器
- **动作**：使用 `once_cell` 或 `lazy_static` 缓存解析结果
- **验证**：多次调用返回相同缓存实例
- **验收**：性能测试显示首次后每次调用 < 1ms

#### T-2.1.5: 重写 `crate_level()` 并添加 fallback
- **动作**：
  ```rust
  pub fn crate_level(name: &str) -> Option<usize> {
      parse_architecture_toml(ARCHITECTURE_TOML)
          .ok()
          .and_then(|arch| arch.crate_layers.get(name).copied())
          .or_else(|| LEGACY_CRATE_LAYERS.iter().find(|(n, _)| *n == name).map(|(_, l)| *l))
  }
  ```
- **验证**：单元测试使用有效 TOML 返回正确层级
- **验收**：对所有 18 个 crate 返回值与之前相同

#### T-2.1.6: 重写 `verify_dependencies()` 并添加 fallback
- **动作**：同 T-2.1.5 模式
- **验证**：单元测试验证输出与旧版一致
- **验收**：输出与重构前完全相同

#### T-2.1.7: 重写 `audit_dependency()` 并添加 fallback
- **动作**：同模式
- **验证**：单元测试验证输出一致
- **验收**：输出与重构前完全相同

#### T-2.1.8: 验证公共 API 不变
- **动作**：运行 `cargo test -p axiom-core --lib` 和 `cargo test -p axiom-cli --lib`
- **验证**：所有测试通过
- **验收**：零测试失败

#### T-2.1.9: 提交重构
- **动作**：`git add crates/axiom-core/src/gate.rs && git commit -m "refactor: gate.rs 从 .axiom/architecture.toml 加载"`
- **验证**：提交成功
- **验收**：提交在历史中

**交付件**：`gate.rs` — 数据从文件加载，API 不变

---

### Task 2.2: 重构 `gate_check.rs`

**前置条件**：T-2.1 完成

**子任务**：

#### T-2.2.1: 读取并分析当前 `gate_check.rs`
- **动作**：读取 `tools/gate_check.rs`
- **验证**：识别硬编码常量和主逻辑
- **验收**：列出需替换组件

#### T-2.2.2: 添加 `archcheck` 依赖
- **动作**：在 `tools/gate_check/Cargo.toml` 中添加：
  ```toml
  archcheck = { path = "../archcheck" }
  ```
- **验证**：`cargo check -p gatecheck` 成功
- **验收**：依赖已解析

#### T-2.2.3: 替换硬编码 `CRATE_LAYERS`
- **动作**：删除硬编码 `CRATE_LAYERS` 常量，从 `archcheck::Architecture::load()` 加载
- **验证**：`cargo run --bin gatecheck` 运行
- **验收**：干净仓库退出码为 0

#### T-2.2.4: 替换硬编码 `AUDITED_DEPS`
- **动作**：删除硬编码列表，从 architecture 加载
- **验证**：在有违规的仓库上运行 `cargo run --bin gatecheck`
- **验收**：检测到与之前相同的违规

#### T-2.2.5: 添加 dev-dependencies 检查
- **动作**：在 `archcheck` 中实现 dev-dep 扫描，通过 `gate_check.rs` 暴露
- **验证**：向测试 crate 的 `[dev-dependencies]` 添加 `async-trait`，运行 `gatecheck`
- **验收**：检测到违规

#### T-2.2.6: 验证 CLI 兼容性
- **动作**：比较 `cargo run --bin gatecheck` 重构前后的输出
- **验证**：输出格式相同
- **验收**：CLI 无破坏性变更

#### T-2.2.7: 提交重构
- **动作**：`git add tools/gate_check.rs && git commit -m "refactor: gate_check.rs 使用 archcheck 库"`
- **验证**：提交成功
- **验收**：提交在历史中

**交付件**：`gate_check.rs` — `archcheck` 的瘦封装，CLI 不变

---

### Task 2.3: 统一数据源

**前置条件**：T-2.1、T-2.2 完成

**子任务**：

#### T-2.3.1: 验证 `.axiom/architecture.toml` 完整性
- **动作**：交叉检查 `[audited-deps]` 与所有 `Cargo.toml` 文件
- **验证**：脚本扫描所有依赖，与审计列表比对
- **验收**：零未审计依赖

#### T-2.3.2: 验证 `.axiom/architecture.toml` 禁止依赖
- **动作**：交叉检查 `[forbidden-deps]` 与所有 `Cargo.toml`
- **验证**：`grep -r "async-trait" crates/*/Cargo.toml` 仅返回已知违规
- **验收**：无意外禁止依赖

#### T-2.3.3: 从 `gate.rs` 删除硬编码 fallback
- **动作**：删除 `LEGACY_CRATE_LAYERS` 和 fallback 逻辑
- **验证**：`grep -n "LEGACY" crates/axiom-core/src/gate.rs` 返回空
- **验收**：无遗留数据

#### T-2.3.4: 从 `gate_check.rs` 删除硬编码常量
- **动作**：删除任何剩余硬编码数据
- **验证**：`grep -n "const CRATE_LAYERS" tools/gate_check.rs` 返回空
- **验收**：无硬编码数据

#### T-2.3.5: 验证唯一数据源
- **动作**：`grep -r "CRATE_LAYERS" crates/ tools/ | grep -v "architecture.toml" | grep -v "target/"`
- **验证**：仅加载器代码保留
- **验收**：无其他架构数据源

#### T-2.3.6: 提交统一
- **动作**：`git add -A && git commit -m "refactor: 将架构数据统一到 .axiom/architecture.toml"`
- **验证**：提交成功
- **验收**：唯一数据源已建立

**交付件**：`.axiom/architecture.toml` 是唯一数据源

---

### Task 2.4: 集成测试

**前置条件**：T-2.1、T-2.2、T-2.3 完成

**子任务**：

#### T-2.4.1: 完整测试套件
- **动作**：`cargo test --workspace`
- **验证**：所有测试通过
- **验收**：退出码 0

#### T-2.4.2: 完整文档构建
- **动作**：`cargo doc --workspace --no-deps`
- **验证**：文档构建成功
- **验收**：退出码 0

#### T-2.4.3: 旧版 gatecheck 兼容性
- **动作**：`cargo run --bin gatecheck`
- **验证**：输出与 Phase 1 基线一致
- **验收**：检测到相同违规

#### T-2.4.4: 新系统准确度
- **动作**：`cargo xtask gatecheck`
- **验证**：输出与旧版一致
- **验收**：检测到相同违规

#### T-2.4.5: 模拟违规 — 禁止依赖
- **动作**：向测试 crate 的 `[dependencies]` 添加 `async-trait = "0.1"`
- **验证**：`cargo check -p <test-crate>` 失败
- **验收**：错误信息提及 `.axiom/architecture.toml`

#### T-2.4.6: 模拟违规 — 反向依赖
- **动作**：向 `axiom-macros` 添加 `axiom-runtime` 依赖
- **验证**：`cargo check -p axiom-macros` 失败
- **验收**：错误提及层级违规

#### T-2.4.7: 模拟违规 — 未注册 crate
- **动作**：创建未在 `architecture.toml` 中的临时 crate
- **验证**：`cargo check -p temp-crate` 失败
- **验收**：错误提及缺失注册

#### T-2.4.8: 提交集成测试
- **动作**：`git add -A && git commit -m "test: 架构治理集成测试"`
- **验证**：提交成功
- **验收**：测试已保留

**Phase 2 退出标准**：
- [ ] EC-2.1: `gate.rs` 中零硬编码架构常量
- [ ] EC-2.2: `gate_check.rs` 使用共享 archcheck 库
- [ ] EC-2.3: 所有公共 API 不变
- [ ] EC-2.4: `cargo test --workspace` 通过
- [ ] EC-2.5: 违规产生带可操作消息的编译期错误

---

## 8. Phase 3: 退役旧系统

**目标**：移除遗留代码。新系统成为唯一系统。编译期强制执行激活。

**工期**：0.5 天
**回滚**：`git revert` Phase 3 提交

### Task 3.1: 启用编译期强制执行

**前置条件**：Phase 2 完成

**子任务**：

#### T-3.1.1: 创建 `tools/archcheck/src/build_hook.rs`
- **动作**：实现 `check_current_crate()` 函数：
  - 读取 `CARGO_PKG_NAME` 环境变量
  - 加载 `.axiom/architecture.toml`
  - 检查 crate 注册
  - 检查所有依赖
  - 违规时 `panic!` 并附带清晰消息
- **验证**：`cargo test -p archcheck --lib build_hook_tests` 通过
- **验收**：函数存在且正常工作

#### T-3.1.2: 将 `archcheck` 添加到 workspace 依赖
- **动作**：编辑根 `Cargo.toml`：
  ```toml
  archcheck = { path = "tools/archcheck" }
  ```
- **验证**：`cargo check --workspace` 成功
- **验收**：依赖对所有 crate 可用

#### T-3.1.3: 为 `axiom-core` 添加 `build.rs`
- **动作**：创建 `crates/axiom-core/build.rs`：
  ```rust
  fn main() {
      archcheck::check_current_crate();
  }
  ```
- **验证**：`cargo check -p axiom-core` 触发构建脚本
- **验收**：构建脚本运行

#### T-3.1.4: 为 `axiom-runtime` 添加 `build.rs`
- **动作**：同 T-3.1.3
- **验证**：`cargo check -p axiom-runtime` 触发
- **验收**：构建脚本运行

#### T-3.1.5: 为 `axiom-store` 添加 `build.rs`
- **动作**：同上
- **验证**：`cargo check -p axiom-store` 触发
- **验收**：构建脚本运行

#### T-3.1.6: 为 `axiom-oversight` 添加 `build.rs`
- **动作**：同上
- **验证**：`cargo check -p axiom-oversight` 触发
- **验收**：构建脚本运行

#### T-3.1.7: 为 `axiom-agent` 添加 `build.rs`
- **动作**：同上
- **验证**：`cargo check -p axiom-agent` 触发
- **验收**：构建脚本运行

#### T-3.1.8: 为 `axiom-cli` 添加 `build.rs`
- **动作**：同上
- **验证**：`cargo check -p axiom-cli` 触发
- **验收**：构建脚本运行

#### T-3.1.9: 批量为剩余 12 个 crate 添加 `build.rs`
- **动作**：对每个剩余 crate：创建 `build.rs`，运行 `cargo check -p <crate>`
- **验证**：全部 18 个 crate 触发构建脚本
- **验收**：`cargo check --workspace` 运行全部 18 个构建脚本

#### T-3.1.10: 验证无误报
- **动作**：`cargo check --workspace`
- **验证**：所有检查通过（干净代码库无违规）
- **验收**：退出码 0

**交付件**：全部 18 个 crate 具备编译期架构强制执行

---

### Task 3.2: 移除遗留 fallback

**前置条件**：T-3.1 完成

**子任务**：

#### T-3.2.1: 从 `gate.rs` 移除遗留数据
- **动作**：删除任何 `LEGACY_*` 常量和 fallback 逻辑
- **验证**：`grep -n "LEGACY" crates/axiom-core/src/gate.rs` 返回空
- **验收**：无遗留数据

#### T-3.2.2: 从 `gate_check.rs` 移除遗留逻辑
- **动作**：删除任何 `legacy_*` 函数
- **验证**：`grep -n "legacy" tools/gate_check.rs` 返回空
- **验收**：无遗留逻辑

#### T-3.2.3: 验证无其他遗留引用
- **动作**：`grep -r "LEGACY_CRATE_LAYERS\|legacy_check" crates/ tools/`
- **验证**：无结果
- **验收**：零遗留引用

#### T-3.2.4: 提交清理
- **动作**：`git add -A && git commit -m "refactor: 移除所有遗留 fallback 数据"`
- **验证**：提交成功
- **验收**：状态干净

**交付件**：零遗留代码剩余

---

### Task 3.3: 删除遗留 `gate_check.rs`

**前置条件**：T-3.2 完成

**子任务**：

#### T-3.3.1: 验证 `xtask gatecheck` 完全替代 `gate_check.rs`
- **动作**：对比功能：`cargo xtask gatecheck --help` vs `cargo run --bin gatecheck --help`
- **验证**：所有旧版功能在 xtask 中存在
- **验收**：功能对等确认

#### T-3.3.2: 删除 `tools/gate_check.rs`
- **动作**：`git rm tools/gate_check.rs`
- **验证**：文件不再存在
- **验收**：`test -f tools/gate_check.rs` 返回 false

#### T-3.3.3: 更新 `README.md`
- **动作**：将 `gate_check.rs` 引用替换为 `cargo xtask gatecheck`
- **验证**：`grep "gatecheck" README.md` 仅显示新引用
- **验收**：文档已更新

#### T-3.3.4: 更新 `DEVELOPMENT.md`
- **动作**：添加章节："Architecture Checks"
  - 记录 `cargo xtask gatecheck`
  - 记录 `cargo xtask state`
  - 记录编译期强制执行
- **验证**：章节存在于文件
- **验收**：文档完整

#### T-3.3.5: 提交退役
- **动作**：`git add -A && git commit -m "feat: 退役遗留 gate_check.rs，xtask 成为主要工具"`
- **验证**：提交成功
- **验收**：遗留已移除

**交付件**：遗留 `gate_check.rs` 完全被 `xtask` 替代

---

### Task 3.4: 会话记忆集成

**前置条件**：Phase 2 完成

**子任务**：

#### T-3.4.1: 创建 `.axiom/bootstrap.md` 模板
- **动作**：在 `cargo xtask state` 中实现：
  - 从 `state.toml` 生成 `bootstrap.md`
  - 包含：crate 数量、层级摘要、最近违规、活跃约束
- **验证**：`cargo xtask state` 创建两个文件
- **验收**：`.axiom/bootstrap.md` 存在且为有效 Markdown

#### T-3.4.2: 记录 bootstrap 协议
- **动作**：添加到 `.axiom/bootstrap.md`：
  ```
  ## 智能体会话启动
  
  每次会话开始时：
  1. 读取本文件
  2. 验证 `.axiom/architecture.toml` 存在
  3. 运行 `cargo xtask gatecheck` 检查违规
  4. 若发现违规：停止并向用户报告
  5. 若干净：继续开发
  ```
- **验证**：文件包含协议
- **验收**：协议已记录

#### T-3.4.3: 提交会话记忆
- **动作**：`git add .axiom/bootstrap.md && git commit -m "feat: 添加智能体会话启动协议"`
- **验证**：提交成功
- **验收**：协议在 git 中

**交付件**：`.axiom/bootstrap.md` — 智能体会话启动协议

---

### Task 3.5: 最终验证

**前置条件**：全部 Phase 3 任务完成

**子任务**：

#### T-3.5.1: 完整测试套件
- **动作**：`cargo test --workspace`
- **验证**：所有测试通过
- **验收**：退出码 0

#### T-3.5.2: 完整文档构建
- **动作**：`cargo doc --workspace --no-deps`
- **验证**：文档构建成功
- **验收**：退出码 0

#### T-3.5.3: 严格 gatecheck
- **动作**：`cargo xtask gatecheck --strict`
- **验证**：在干净仓库上通过
- **验收**：退出码 0

#### T-3.5.4: 编译期强制执行验证
- **动作**：向测试 crate 添加 `async-trait`，运行 `cargo check`
- **验证**：失败并显示清晰错误
- **验收**：非零退出码，错误提及 `architecture.toml`

#### T-3.5.5: 验证无遗留引用
- **动作**：`grep -r "gate_check" crates/ tools/ || true`
- **验证**：无结果（或仅在 git 历史中）
- **验收**：零引用

#### T-3.5.6: 验证唯一数据源
- **动作**：`grep -r "CRATE_LAYERS" crates/ tools/ | grep -v "target/" | grep -v "architecture.toml"`
- **验证**：仅加载器代码保留
- **验收**：无其他数据源

#### T-3.5.7: Clippy 检查
- **动作**：`cargo clippy --workspace`
- **验证**：无警告
- **验收**：退出码 0

#### T-3.5.8: 最终提交
- **动作**：`git add -A && git commit -m "feat: 架构治理 v1.0 — 遗留系统完全替代"`
- **验证**：提交成功
- **验收**：最终状态已提交

**Phase 3 退出标准**：
- [ ] EC-3.1: `gate.rs` 或 `gate_check.rs` 中无硬编码架构数据
- [ ] EC-3.2: `.axiom/architecture.toml` 是唯一数据源
- [ ] EC-3.3: `cargo test --workspace` 通过
- [ ] EC-3.4: `cargo xtask gatecheck --strict` 通过
- [ ] EC-3.5: 全部 18 个 crate 激活编译期强制执行
- [ ] EC-3.6: `.axiom/bootstrap.md` 存在且为最新
- [ ] EC-3.7: 遗留代码完全移除或废弃

---

## 9. 验收标准

### 硬性门禁（全部必须通过）

| ID | 标准 | 验证命令 | 预期结果 |
|----|------|----------|----------|
| H-01 | `cargo check --workspace` 成功 | `cargo check --workspace` | 退出码 0 |
| H-02 | `cargo test --workspace` 成功 | `cargo test --workspace` | 退出码 0 |
| H-03 | `cargo doc --workspace --no-deps` 成功 | `cargo doc --workspace --no-deps` | 退出码 0 |
| H-04 | `cargo xtask gatecheck --strict` 通过 | `cargo xtask gatecheck --strict` | 退出码 0 |
| H-05 | 依赖中零 `async-trait` | `grep -r "async-trait" crates/*/Cargo.toml` | 空 |
| H-06 | 全部 18 个 crate 在 `architecture.toml` 中 | `cargo xtask gatecheck --list-crates` | 列出 18 个 |
| H-07 | `gate.rs` 零硬编码常量 | `grep -n "const CRATE_LAYERS" crates/axiom-core/src/gate.rs` | 无结果 |
| H-08 | `gate_check.rs` 使用 archcheck 库 | `grep -n "archcheck::" tools/gate_check.rs` | 找到（Phase 2） |
| H-09 | 编译期强制执行激活 | 添加 `async-trait` → `cargo check` | 非零退出 |
| H-10 | `.axiom/state.toml` 有效 | `cargo xtask state && cat .axiom/state.toml` | 有效 TOML |
| H-11 | `.axiom/bootstrap.md` 存在 | `test -f .axiom/bootstrap.md` | True |
| H-12 | CI 观察者工作流存在 | `test -f .github/workflows/architecture-observer.yml` | True |
| H-13 | `cargo clippy --workspace` 通过 | `cargo clippy --workspace` | 退出码 0 |
| H-14 | 无遗留代码引用 | `grep -r "LEGACY_CRATE_LAYERS" .` | 空 |
| H-15 | `axiom-cli` verify 工作 | `cargo run --bin axm -- verify` | 退出码 0 |

### 软性门禁（尽力而为）

| ID | 标准 | 目标 |
|----|------|------|
| S-01 | 构建时间开销 < 5% | `time cargo check` 在基线 5% 内 |
| S-02 | 智能体在会话开始时读取 bootstrap | 100% 会话 |
| S-03 | 零手动 crate 注册 | 100% 自动化 |
| S-04 | 文档从单一数据源生成 | 100% |

---

## 10. 回滚程序

### R-0.1: Phase 0 回滚

**触发条件**：新系统有严重 bug
**动作**：
```bash
git rm -rf .axiom tools/archcheck xtask
git commit -m "rollback: 移除 Phase 0 架构治理"
```
**时间**：< 30 秒
**风险**：零 — 无现有代码被修改

### R-1.1: Phase 1 回滚

**触发条件**：CI 观察者引发问题
**动作**：
```bash
git rm .github/workflows/architecture-observer.yml
git commit -m "rollback: 移除架构观察者工作流"
```
**时间**：< 30 秒
**风险**：零 — 工作流是非阻断的

### R-2.1: Phase 2 回滚

**触发条件**：`gate.rs` 重构破坏 `axiom-cli`
**动作**：
```bash
git revert HEAD~2  # 恢复 T-2.1 和 T-2.2 提交
```
**时间**：< 5 分钟
**风险**：低 — 公共 API 不变，有 fallback 逻辑

### R-3.1: Phase 3 回滚

**触发条件**：编译期强制执行导致大规模构建失败
**动作**：
```bash
# 选项 A: 恢复所有 Phase 3 提交
git revert HEAD~5..HEAD

# 选项 B: 在所有 Cargo.toml 中禁用 archcheck feature
cargo xtask disable-archcheck
```
**时间**：< 10 分钟
**风险**：中 — `build.rs` 文件需要移除

---

## 11. 风险登记册

| ID | 风险 | 可能性 | 影响 | 缓解措施 | 阶段 |
|-----|------|--------|------|----------|------|
| RK-01 | 构建时间增加 > 5% | 中 | 高 | 缓存解析后的 architecture；优化 `build.rs` 到 < 100ms | 3 |
| RK-02 | 智能体绕过 `build.rs` | 中 | 严重 | 要求显式 `--unsafe-skip-arch` 标志；记录所有跳过 | 3 |
| RK-03 | `.axiom/` 被智能体忽略 | 中 | 中 | 在系统提示中添加"会话启动时始终读取" | 1 |
| RK-04 | `architecture.toml` 过时 | 低 | 高 | `gatecheck` 每次运行验证一致性 | 全部 |
| RK-05 | Phase 2 破坏 `axiom-cli` | 低 | 高 |  Extensive fallback 逻辑；API 兼容性测试 | 2 |
| RK-06 | Phase 3 `build.rs` 编译循环 | 低 | 高 | 在全局启用前在所有 18 个 crate 上测试 | 3 |
| RK-07 | 遗留代码删除过早 | 低 | 中 | 删除前先废弃；保留在 git 历史中 | 3 |

---

## 12. 时间线

### 第 1 周

| 日期 | 任务 | 交付件 |
|------|------|--------|
| 周一 | T-0.1、T-0.2、T-0.3 | `.axiom/architecture.toml`、`tools/archcheck/`、`xtask/` |
| 周二 | T-0.4、T-1.1 | 基线报告、验证报告 |
| 周三 | T-1.2 | CI 观察者工作流 |
| 周四 | T-1.3 | `.axiom/state.toml` 生成器 |
| 周五 | T-1.4 | Phase 1 验证完成 |

### 第 2 周

| 日期 | 任务 | 交付件 |
|------|------|--------|
| 周一 | T-2.1 | `gate.rs` 重构完成 |
| 周二 | T-2.2、T-2.3 | `gate_check.rs` 重构完成，数据统一 |
| 周三 | T-2.4 | 集成测试通过 |
| 周四 | 缓冲 / 修复问题 | Phase 2 完成 |
| 周五 | Phase 2 评审 | Phase 3 的 go/no-go |

### 第 3 周

| 日期 | 任务 | 交付件 |
|------|------|--------|
| 周一 | T-3.1 | 全部 18 个 crate 拥有 `build.rs` |
| 周二 | T-3.2、T-3.3 | 遗留代码移除 |
| 周三 | T-3.4 | 会话记忆集成 |
| 周四 | T-3.5 | 最终验证 |
| 周五 | 回顾 | Phase 3 完成 |

---

## 13. 完成定义

本计划在以下条件全部满足时视为完成：

1. [ ] 全部 15 项硬性门禁验收标准通过
2. [ ] `cargo test --workspace` 退出码 0
3. [ ] `cargo xtask gatecheck --strict` 退出码 0
4. [ ] 遗留系统完全退役
5. [ ] 编译期强制执行在全部 18 个 crate 上激活
6. [ ] `.axiom/` 目录是唯一数据源
7. [ ] 零遗留硬编码数据引用剩余
8. [ ] 文档已更新
9. [ ] CI 强制执行架构合规
10. [ ] 会话记忆运行正常

---

## 14. 附录

### 附录 A: 完整文件清单

#### 新创建文件

| 路径 | 阶段 | 用途 |
|------|------|------|
| `.axiom/architecture.toml` | 0 | 唯一数据源 |
| `.axiom/state.toml` | 1 | 架构状态快照 |
| `.axiom/bootstrap.md` | 3 | 智能体会话启动 |
| `.axiom/violations/*.md` | 1+ | 违规账本 |
| `.axiom/baseline-report.json` | 0 | Phase 0 基线 |
| `.axiom/validation-report.md` | 1 | Phase 1 验证 |
| `tools/archcheck/Cargo.toml` | 0 | archcheck 包 |
| `tools/archcheck/src/main.rs` | 0 | CLI 入口 |
| `tools/archcheck/src/loader.rs` | 0 | TOML 加载器 |
| `tools/archcheck/src/checker.rs` | 0 | 检查逻辑 |
| `tools/archcheck/src/reporter.rs` | 0 | 输出格式化 |
| `tools/archcheck/src/build_hook.rs` | 3 | 编译期钩子 |
| `xtask/Cargo.toml` | 0 | xtask 包 |
| `xtask/src/main.rs` | 0 | 命令封装 |
| `crates/*/build.rs`（18 个文件） | 3 | 编译期检查 |
| `.github/workflows/architecture-observer.yml` | 1 | CI 观察者 |
| `.github/workflows/architecture-gate.yml` | 3 | CI 门禁 |

#### 修改文件

| 路径 | 阶段 | 变更 |
|------|------|------|
| `crates/axiom-core/src/gate.rs` | 2 | 从文件加载，API 不变 |
| `tools/gate_check.rs` | 2 | 使用 archcheck 库 |
| 根 `Cargo.toml` | 3 | 添加 `archcheck` workspace 依赖 |
| `crates/*/Cargo.toml`（18 个文件） | 3 | 添加 `archcheck` feature |
| `README.md` | 3 | 更新工作流文档 |
| `DEVELOPMENT.md` | 3 | 添加架构章节 |

#### 删除文件

| 路径 | 阶段 | 原因 |
|------|------|------|
| `tools/gate_check.rs` | 3 | 被 `xtask gatecheck` 替代 |
| `crates/*/build.rs`（回滚） | 3 | 如需回滚 |

### 附录 B: 命令参考

#### 新命令

```bash
# 架构检查（文本）
cargo xtask gatecheck

# 架构检查（JSON）
cargo xtask gatecheck --format json --output report.json

# 严格模式（CI）
cargo xtask gatecheck --strict

# 列出所有注册 crate
cargo xtask gatecheck --list-crates

# 生成状态快照
cargo xtask state

# 生成状态到指定路径
cargo xtask state --output .axiom/state.toml
```

#### 验证命令

```bash
# 完整验证
cargo test --workspace && cargo doc --workspace --no-deps && cargo xtask gatecheck --strict

# 快速检查
cargo xtask gatecheck

# 状态检查
cat .axiom/state.toml
cat .axiom/bootstrap.md

# 旧版对比
cargo run --bin gatecheck > legacy.txt
cargo xtask gatecheck > new.txt
diff legacy.txt new.txt
```

### 附录 C: `.axiom/architecture.toml` 完整 Schema

```toml
[crate-layers]
# Layer 0: 顶层应用
axiom-cli = 0
axiom-bench = 0

# Layer 1: 可视化
axiom-viz = 1

# Layer 2: Agent 门面
axiom-identity = 2
axiom-prompt = 2

# Layer 3: 监督与集成
axiom-mcp = 3
axiom-alert = 3
axiom-agent = 3
axiom-oversight = 3

# Layer 4: 运行时与协调
axiom-distributed = 4
axiom-planner = 4
axiom-runtime = 4

# Layer 5: 存储与工具
axiom-llm = 5
axiom-tool = 5
axiom-memory = 5
axiom-store = 5

# Layer 6: （预留）

# Layer 7: 核心原语
axiom-core = 7

# Layer 8: Proc-macro（豁免）
axiom-macros = 8

[forbidden-deps]
async-trait = "R-004: Rust 1.75+ 已支持原生 async fn in traits"

[audited-deps]
# 异步运行时
tokio = "async 运行时和工具"
serde = "序列化框架"
serde_json = "JSON 序列化"
thiserror = "错误处理"
anyhow = "错误处理"
tracing = "结构化日志"
tracing-subscriber = "日志订阅器"
sha2 = "加密哈希"
uuid = "唯一标识符"
futures = "future 组合器"
clap = "CLI 解析"
ratatui = "终端 UI"
crossterm = "终端操作"
syn = "Rust 解析（proc-macro）"
quote = "代码生成（proc-macro）"
proc-macro2 = "token 表示（proc-macro）"
linkme = "分布式 slice 链接器"
trybuild = "编译测试运行器"
regex = "正则表达式"
parking_lot = "同步原语"
dashmap = "并发 hashmap"
sqlx = "SQL 工具包（runtime）"
snap = "snappy 压缩（store）"
tempfile = "临时文件（store）"
criterion = "基准测试（bench）"

[dev-dependencies-audit]
enabled = true
note = "所有 dev-dependencies 也必须遵守 forbidden-deps 和 audited-deps"

[proc-macro-exemptions]
axiom-macros = { allowed_deps = ["axiom-core"], reason = "Proc-macro 必须引用 core 类型以进行宏展开" }
```

### 附录 D: 构建脚本模板

```rust
// crates/<crate-name>/build.rs
fn main() {
    // 加载架构配置
    let arch_toml = include_str!("../../../.axiom/architecture.toml");
    let arch: Architecture = parse_architecture_toml(arch_toml)
        .expect("无法解析 .axiom/architecture.toml");

    // 获取当前 crate 名称
    let crate_name = std::env::var("CARGO_PKG_NAME")
        .expect("CARGO_PKG_NAME 未设置");

    // 检查 1: Crate 已注册
    if !arch.crate_layers.contains_key(&crate_name) {
        panic!(
            "ARCHITECTURE VIOLATION: crate '{}' 未在 .axiom/architecture.toml [crate-layers] 中注册。 \
            运行: cargo xtask new_crate --name {} --layer <0-8>",
            crate_name,
            crate_name.strip_prefix("axiom-").unwrap_or(&crate_name)
        );
    }

    // 检查 2: 依赖
    let cargo_toml = include_str!("../Cargo.toml");
    let manifest: CargoManifest = parse_cargo_toml(cargo_toml)
        .expect("无法解析 Cargo.toml");

    for (dep_name, _) in manifest.dependencies.iter().chain(manifest.dev_dependencies.iter()) {
        // 跳过 axiom-* 内部依赖
        if dep_name.starts_with("axiom-") {
            continue;
        }

        // 检查禁止
        if arch.forbidden_deps.contains_key(dep_name) {
            panic!(
                "ARCHITECTURE VIOLATION: crate '{}' 依赖了禁止的依赖 '{}'。原因: {}",
                crate_name,
                dep_name,
                arch.forbidden_deps[dep_name]
            );
        }

        // 检查审计
        if !arch.audited_deps.contains_key(dep_name) {
            panic!(
                "ARCHITECTURE VIOLATION: crate '{}' 依赖了未审计的依赖 '{}'。 \
                添加到 .axiom/architecture.toml [audited-deps] 或移除依赖。",
                crate_name,
                dep_name
            );
        }
    }

    // 检查 3: 依赖方向
    let crate_layer = arch.crate_layers[&crate_name];
    for dep_name in manifest.dependencies.keys() {
        if dep_name.starts_with("axiom-") {
            if let Some(&dep_layer) = arch.crate_layers.get(dep_name) {
                if dep_layer < crate_layer {
                    // 检查豁免
                    let is_exempt = arch.proc_macro_exemptions.get(&crate_name)
                        .map(|e| e.allowed_deps.contains(&dep_name.as_str()))
                        .unwrap_or(false);

                    if !is_exempt {
                        panic!(
                            "ARCHITECTURE VIOLATION: crate '{}' (layer {}) 反向依赖了 '{}' (layer {})",
                            crate_name, crate_layer, dep_name, dep_layer
                        );
                    }
                }
            }
        }
    }

    println!("cargo:rerun-if-changed=.axiom/architecture.toml");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
```

---

**文档版本**：3.0
**最后更新**：2026-07-04
**状态**：就绪，可原子执行
