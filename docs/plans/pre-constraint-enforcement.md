# 事前约束体系开发计划

> **细分到最小任务单元的完整开发文档**
>
> 目标：在智能体写代码之前、生成代码之前、提交代码之前，实现多层事前约束，
> 将架构违规拦截从"编译期/事后"前移到"生成前/实时"。

---

## 目录

1. [整体架构](#1-整体架构)
2. [Phase -1: 提示词工程](#2-phase--1-提示词工程)
3. [Phase 0: 脚手架增强](#3-phase-0-脚手架增强)
4. [Phase 1: IDE/LSP 实时反馈](#4-phase-1-idelsp-实时反馈)
5. [Phase 2: 预提交钩子](#5-phase-2-预提交钩子)
6. [Phase 3: 编译期增强](#6-phase-3-编译期增强)
7. [Phase 4: 运行时 API 扩展](#7-phase-4-运行时-api-扩展)
8. [验收标准总表](#8-验收标准总表)
9. [依赖关系图](#9-依赖关系图)
10. [风险与回滚](#10-风险与回滚)

---

## 1. 整体架构

### 1.1 约束时机全景

```
智能体意图
    │
    ▼
[Layer -1] 提示词约束 ─── 生成代码前，智能体已知晓规则
    │
    ▼
[Layer 0] 脚手架约束 ─── 创建 crate 时自动注册，无法忘记
    │
    ▼
[Layer 1] IDE/LSP 约束 ─── 编辑 Cargo.toml 时实时标红
    │
    ▼
[Layer 2] 预提交约束 ─── git commit 前自动检查
    │
    ▼
[Layer 3] 编译期约束 ─── build.rs 自动执行，违规 panic
    │
    ▼
[Layer 4] CI 约束 ─── push/PR 自动检查，非阻塞报告
    │
    ▼
代码合并
```

### 1.2 设计原则

| 原则 | 说明 | 实现方式 |
|------|------|----------|
| **零信任** | 不依赖智能体自觉 | 编译期/事前自动执行 |
| **单一数据源** | 规则只定义在一处 | `.axiom/architecture.toml` |
| **多层防御** | 多层约束，逐级拦截 | Layer -1 到 Layer 4 |
| **即时反馈** | 越早发现问题越好 | LSP > pre-commit > 编译期 |
| **不可绕过** | 智能体无法跳过约束 | build.rs panic + git hook |
| **向后兼容** | 不破坏现有代码 | gate.rs API 稳定 |

### 1.3 当前状态

| 约束层 | 当前状态 | 说明 |
|--------|----------|------|
| Layer -1 | ❌ 缺失 | 提示词未嵌入架构规则 |
| Layer 0 | ✅ 部分实现 | `xtask new_crate` 已实现 |
| Layer 1 | ❌ 缺失 | 无 LSP 服务器 |
| Layer 2 | ❌ 缺失 | 无 pre-commit 钩子 |
| Layer 3 | ✅ 已实现 | 18 个 crate 全覆盖 |
| Layer 4 | ✅ 已实现 | CI observer 已配置 |

**缺口**：Layer -1、Layer 1、Layer 2 缺失。

---

## 2. Phase -1: 提示词工程

### 目标

在智能体会话开始时，通过系统提示词和会话记忆，让智能体**知晓**架构规则，
在生成代码前就考虑约束。

### 任务分解

#### Task -1.1: 设计系统提示词模板

**描述**：创建标准化的系统提示词模板，包含完整的架构规则。

**验收标准**：
- [ ] 提示词模板包含 9 层架构定义
- [ ] 提示词模板包含 forbidden-deps 列表
- [ ] 提示词模板包含 audited-deps 列表
- [ ] 提示词模板包含豁免机制说明
- [ ] 提示词模板包含代码生成规则
- [ ] 提示词模板包含违反后果说明
- [ ] 提示词长度 < 2000 字符（避免超出上下文窗口）

**输出文件**：
- `.axiom/prompts/architecture-constraints.md`

**依赖**：无

---

#### Task -1.2: 创建智能体会话记忆协议

**描述**：创建 `.axiom/bootstrap.md` 增强版，确保每次新会话自动加载。

**验收标准**：
- [ ] bootstrap.md 包含架构规则摘要
- [ ] bootstrap.md 包含常用命令参考
- [ ] bootstrap.md 包含检查清单（checklist）
- [ ] 智能体在开始工作前必须执行 checklist

**输出文件**：
- `.axiom/bootstrap.md`（更新）

**依赖**：Task -1.1

---

#### Task -1.3: 集成到 CI 提示词

**描述**：在 GitHub Actions 的 CI 提示词中嵌入架构检查要求。

**验收标准**：
- [ ] CI 提示词包含架构检查步骤
- [ ] CI 提示词包含 violations 处理流程
- [ ] CI 提示词包含修复建议

**输出文件**：
- `.github/prompts/ci-architecture-check.md`

**依赖**：Task -1.1

---

## 3. Phase 0: 脚手架增强

### 目标

通过 CLI 模板生成代码，让智能体**无法绕过**架构规则。

### 任务分解

#### Task 0.1: 增强 `new_crate` 命令

**描述**：增强 `xtask new_crate` 命令，添加更多模板和检查。

**验收标准**：
- [ ] 支持 `--minimal` 模板（无测试/示例）
- [ ] 支持 `--full` 模板（含测试/示例/CI）
- [ ] 自动生成 `build.rs`（调用 archcheck）
- [ ] 自动生成 `tests/` 目录和测试模板
- [ ] 自动生成 `.github/workflows/` CI 模板
- [ ] 创建后自动运行 `cargo check` 验证
- [ ] 创建后自动运行 `cargo test` 验证
- [ ] 失败时自动回滚（删除已创建文件）

**输出文件**：
- `xtask/src/commands/new_crate.rs`（更新）

**依赖**：无（已有基础实现）

---

#### Task 0.2: 新增 `new_signal` 命令

**描述**：创建 `xtask new_signal` 命令，自动生成 Signal 类型。

**验收标准**：
- [ ] 支持 `--kind` 参数（command/event/query/response）
- [ ] 自动生成 `#[signal]` 宏代码
- [ ] 自动添加必需字段（msg_id, correlation_id, vector_clock）
- [ ] 自动生成序列化代码
- [ ] 自动添加到模块导出

**输出文件**：
- `xtask/src/commands/new_signal.rs`

**依赖**：Task 0.1

---

#### Task 0.3: 新增 `new_cell` 命令

**描述**：创建 `xtask new_cell` 命令，自动生成 Cell 类型。

**验收标准**：
- [ ] 支持 `--layer` 参数（exec/validate/agent/oversight）
- [ ] 自动生成 `#[cell]` 宏代码
- [ ] 自动添加层标记
- [ ] 自动生成 `handle` 方法模板
- [ ] 自动生成 Witness 记录代码
- [ ] 自动添加到模块导出

**输出文件**：
- `xtask/src/commands/new_cell.rs`

**依赖**：Task 0.1

---

#### Task 0.4: 新增 `new_tool` 命令

**描述**：创建 `xtask new_tool` 命令，自动生成 Tool 类型。

**验收标准**：
- [ ] 支持 `--permission` 参数（read/write/admin）
- [ ] 自动生成 `#[tool]` 宏代码
- [ ] 自动添加权限检查
- [ ] 自动生成执行逻辑模板
- [ ] 自动添加到 ToolRegistry

**输出文件**：
- `xtask/src/commands/new_tool.rs`

**依赖**：Task 0.1

---

#### Task 0.5: 新增 `new_guard` 命令

**描述**：创建 `xtask new_guard` 命令，自动生成 Guard 类型。

**验收标准**：
- [ ] 支持 `--layer` 参数
- [ ] 自动生成 `#[guard]` 宏代码
- [ ] 自动生成检查逻辑模板
- [ ] 自动添加到 GuardRegistry

**输出文件**：
- `xtask/src/commands/new_guard.rs`

**依赖**：Task 0.1

---

## 4. Phase 1: IDE/LSP 实时反馈

### 目标

在智能体编辑代码时，实时显示架构违规，不等编译。

### 任务分解

#### Task 1.1: 设计 LSP 协议

**描述**：设计 archcheck LSP 服务器的协议规范。

**验收标准**：
- [ ] 定义 `textDocument/didOpen` 处理逻辑
- [ ] 定义 `textDocument/didChange` 处理逻辑
- [ ] 定义 `textDocument/didSave` 处理逻辑
- [ ] 定义 `workspace/diagnostic` 报告格式
- [ ] 定义 `codeAction` 修复建议格式
- [ ] 定义 `workspace/configuration` 配置项

**输出文件**：
- `tools/archcheck-lsp/SPEC.md`

**依赖**：无

---

#### Task 1.2: 实现 LSP 服务器核心

**描述**：实现 archcheck LSP 服务器基础框架。

**验收标准**：
- [ ] 使用 `tower-lsp` 框架
- [ ] 支持 `initialize` 请求
- [ ] 支持 `initialized` 通知
- [ ] 支持 `shutdown` 请求
- [ ] 支持 `exit` 通知
- [ ] 支持 `textDocument/didOpen`
- [ ] 支持 `textDocument/didChange`
- [ ] 支持 `textDocument/didSave`
- [ ] 支持 `workspace/diagnostic`（可选）

**输出文件**：
- `tools/archcheck-lsp/src/main.rs`
- `tools/archcheck-lsp/src/server.rs`
- `tools/archcheck-lsp/Cargo.toml`

**依赖**：Task 1.1

---

#### Task 1.3: 实现 Cargo.toml 实时检查

**描述**：实现 Cargo.toml 文件的实时架构检查。

**验收标准**：
- [ ] 解析 TOML 内容
- [ ] 检查 `[dependencies]` 中的 axiom-* 依赖方向
- [ ] 检查 `[dependencies]` 中的第三方依赖（forbidden/audited）
- [ ] 检查 `[build-dependencies]`
- [ ] 检查 `[dev-dependencies]`（如果启用）
- [ ] 实时发布 Diagnostic（错误/警告）
- [ ] 支持 `codeAction` 提供修复建议

**输出文件**：
- `tools/archcheck-lsp/src/checks/cargo_toml.rs`

**依赖**：Task 1.2

---

#### Task 1.4: 实现 architecture.toml 实时检查

**描述**：实现 architecture.toml 文件的实时检查。

**验收标准**：
- [ ] 解析 TOML 内容
- [ ] 验证语法正确性
- [ ] 检查 crate-layers 完整性
- [ ] 检查 audited-deps 引用有效性
- [ ] 检查豁免配置有效性
- [ ] 实时发布 Diagnostic

**输出文件**：
- `tools/archcheck-lsp/src/checks/architecture_toml.rs`

**依赖**：Task 1.2

---

#### Task 1.5: 实现 Rust 源码检查

**描述**：实现 Rust 源码中架构相关代码的实时检查。

**验收标准**：
- [ ] 检查 `#[cell]` 宏的 layer 参数合法性
- [ ] 检查 `#[signal]` 宏的 layer 参数合法性
- [ ] 检查 `#[tool]` 宏的 permission 参数合法性
- [ ] 检查 `#[guard]` 宏的 layer 参数合法性
- [ ] 实时发布 Diagnostic

**输出文件**：
- `tools/archcheck-lsp/src/checks/rust_src.rs`

**依赖**：Task 1.2

---

#### Task 1.6: 实现 Code Action 修复

**描述**：提供自动修复建议。

**验收标准**：
- [ ] 未审计依赖 → 提示添加到 audited-deps
- [ ] 反向依赖 → 提示添加豁免或移除依赖
- [ ] 禁止依赖 → 提示移除依赖
- [ ] 未注册 crate → 提示运行 `xtask new_crate`
- [ ] 支持一键修复（通过 `codeAction`）

**输出文件**：
- `tools/archcheck-lsp/src/code_actions.rs`

**依赖**：Task 1.3, Task 1.4

---

#### Task 1.7: LSP 集成测试

**描述**：为 LSP 服务器编写集成测试。

**验收标准**：
- [ ] 测试 `didOpen` 触发检查
- [ ] 测试 `didChange` 实时更新
- [ ] 测试 `didSave` 触发检查
- [ ] 测试 Diagnostic 发布
- [ ] 测试 Code Action 修复
- [ ] 测试错误恢复

**输出文件**：
- `tools/archcheck-lsp/tests/integration.rs`

**依赖**：Task 1.3, Task 1.4, Task 1.5, Task 1.6

---

## 5. Phase 2: 预提交钩子

### 目标

在 git commit 前自动检查，阻止违规提交。

### 任务分解

#### Task 2.1: 设计预提交检查规范

**描述**：设计预提交钩子的检查规范和接口。

**验收标准**：
- [ ] 定义检查触发时机（pre-commit）
- [ ] 定义检查范围（staging area）
- [ ] 定义检查项（Cargo.toml 变更、新增文件）
- [ ] 定义输出格式（人类可读 + JSON）
- [ ] 定义失败处理（阻止提交 + 错误信息）

**输出文件**：
- `tools/archcheck-precommit/SPEC.md`

**依赖**：无

---

#### Task 2.2: 实现 staging area 检查

**描述**：实现检查 git staging area 中变更的功能。

**验收标准**：
- [ ] 使用 `git diff --cached` 获取变更
- [ ] 识别新增/修改的 Cargo.toml 文件
- [ ] 识别新增的 crate 目录
- [ ] 解析变更的依赖项
- [ ] 调用 archcheck 进行验证
- [ ] 输出 violations 报告

**输出文件**：
- `tools/archcheck-precommit/src/checker.rs`

**依赖**：Task 2.1

---

#### Task 2.3: 实现 pre-commit 钩子脚本

**描述**：创建 git pre-commit 钩子脚本。

**验收标准**：
- [ ] 脚本可执行（chmod +x）
- [ ] 调用 archcheck-precommit
- [ ] 检查失败时阻止提交
- [ ] 检查失败时输出详细错误信息
- [ ] 检查失败时输出修复建议
- [ ] 支持 `--no-verify` 跳过（紧急情况）
- [ ] 支持 `--strict` 模式（检查所有文件，不仅是 staging）

**输出文件**：
- `tools/archcheck-precommit/install.sh`
- `tools/archcheck-precommit/install.ps1`
- `.githooks/pre-commit`

**依赖**：Task 2.2

---

#### Task 2.4: 实现自动修复建议

**描述**：在 pre-commit 失败时，提供自动修复脚本。

**验收标准**：
- [ ] 检测未审计依赖 → 提示运行 `cargo run -p xtask -- audit-add <dep>`
- [ ] 检测未注册 crate → 提示运行 `cargo run -p xtask -- new_crate`
- [ ] 检测反向依赖 → 提示添加豁免或移除依赖
- [ ] 检测禁止依赖 → 提示移除依赖
- [ ] 提供 `--fix` 自动修复选项（可选）

**输出文件**：
- `tools/archcheck-precommit/src/fixer.rs`

**依赖**：Task 2.2

---

#### Task 2.5: 集成到 xtask

**描述**：将 pre-commit 检查集成到 xtask。

**验收标准**：
- [ ] 支持 `cargo xtask precommit` 命令
- [ ] 支持 `--install` 安装钩子
- [ ] 支持 `--uninstall` 卸载钩子
- [ ] 支持 `--check` 手动检查
- [ ] 支持 `--fix` 自动修复

**输出文件**：
- `xtask/src/commands/precommit.rs`

**依赖**：Task 2.3, Task 2.4

---

## 6. Phase 3: 编译期增强

### 目标

增强现有编译期检查，覆盖更多场景。

### 任务分解

#### Task 3.1: 增强 build_hook.rs

**描述**：增强 `archcheck::build_hook::check_current_crate()` 功能。

**验收标准**：
- [ ] 检查 `[dependencies]`、`[build-dependencies]`、`[dev-dependencies]`
- [ ] 检查内部依赖方向 + 豁免
- [ ] 检查 forbidden deps
- [ ] 检查 audited deps
- [ ] 检查 dev-dep-audit 开关
- [ ] 错误信息包含修复建议
- [ ] 错误信息包含行号（如可能）
- [ ] 支持 `AXIOM_ARCHITECTURE_TOML` 环境变量

**输出文件**：
- `tools/archcheck/src/build_hook.rs`（更新）

**依赖**：无（已有基础实现）

---

#### Task 3.2: 实现 build.rs 模板生成

**描述**：在 `new_crate` 命令中自动生成 build.rs。

**验收标准**：
- [ ] build.rs 调用 `archcheck::build_hook::check_current_crate()`
- [ ] build.rs 包含 `cargo:rerun-if-changed` 指令
- [ ] build.rs 包含 rustc 版本检查（如需要）
- [ ] build.rs 通过 `cargo check` 验证

**输出文件**：
- `xtask/src/commands/new_crate.rs`（更新）

**依赖**：Task 0.1, Task 3.1

---

#### Task 3.3: 添加编译期测试

**描述**：添加编译期架构检查的单元测试。

**验收标准**：
- [ ] 测试反向依赖检测
- [ ] 测试禁止依赖检测
- [ ] 测试未审计依赖检测
- [ ] 测试豁免机制
- [ ] 测试 dev-dependencies 检查
- [ ] 测试 build-dependencies 检查
- [ ] 测试错误信息格式

**输出文件**：
- `tools/archcheck/tests/build_hook_tests.rs`

**依赖**：Task 3.1

---

## 7. Phase 4: 运行时 API 扩展

### 目标

扩展运行时 API，支持动态架构查询和验证。

### 任务分解

#### Task 4.1: 增强 gate.rs API

**描述**：增强 `axiom-core/src/gate.rs` 的公共 API。

**验收标准**：
- [ ] 添加 `crate_levels()` 返回所有 crate 层映射
- [ ] 添加 `is_registered(crate_name)` 检查 crate 是否注册
- [ ] 添加 `get_allowed_deps(crate_name)` 返回允许的依赖
- [ ] 添加 `get_forbidden_deps()` 返回禁止依赖列表
- [ ] 添加 `get_audited_deps()` 返回审计依赖列表
- [ ] 保持向后兼容（旧 API 不变）
- [ ] 所有新 API 有完整文档注释
- [ ] 所有新 API 有单元测试

**输出文件**：
- `crates/axiom-core/src/gate.rs`（更新）

**依赖**：无（已有基础实现）

---

#### Task 4.2: 实现动态架构验证

**描述**：实现运行时动态验证架构规则的功能。

**验收标准**：
- [ ] 支持运行时检查依赖方向
- [ ] 支持运行时检查 forbidden/audited deps
- [ ] 支持运行时检查 crate 注册状态
- [ ] 返回详细的 violations 列表
- [ ] 与编译期检查结果一致

**输出文件**：
- `crates/axiom-core/src/gate.rs`（更新）

**依赖**：Task 4.1

---

#### Task 4.3: 实现架构变更通知

**描述**：实现 architecture.toml 变更时的通知机制。

**验收标准**：
- [ ] 监听 `.axiom/architecture.toml` 文件变更
- [ ] 变更时清除缓存
- [ ] 变更时发布通知（可选）
- [ ] 支持热重载（开发模式）

**输出文件**：
- `crates/axiom-core/src/gate.rs`（更新）

**依赖**：Task 4.1

---

## 8. 验收标准总表

### 8.1 功能验收

| 功能 | 验收标准 | 测试方法 |
|------|----------|----------|
| **提示词约束** | 系统提示词包含完整架构规则 | 人工审查提示词模板 |
| **脚手架约束** | `new_crate` 自动注册 + 自动 build.rs | 运行 `xtask new_crate` 验证 |
| **LSP 实时检查** | 编辑 Cargo.toml 时实时标红 | 使用 VS Code 测试 |
| **预提交钩子** | `git commit` 前自动检查 | 运行 `git commit` 测试 |
| **编译期检查** | build.rs 自动执行，违规 panic | 运行 `cargo check` 测试 |
| **运行时 API** | gate.rs API 完整且向后兼容 | 运行单元测试 |

### 8.2 性能验收

| 指标 | 目标 | 测试方法 |
|------|------|----------|
| **LSP 响应时间** | < 100ms | 编辑 Cargo.toml 测量 |
| **预提交检查时间** | < 5s | 运行 pre-commit 测量 |
| **编译期检查时间** | < 1s | 运行 `cargo check` 测量 |
| **内存占用** | < 50MB | 运行时监控 |

### 8.3 安全验收

| 检查项 | 验收标准 | 测试方法 |
|--------|----------|----------|
| **权限检查** | pre-commit 钩子不可绕过 | 尝试 `--no-verify` 测试 |
| **注入防护** | LSP 服务器输入验证 | fuzz 测试 |
| **路径遍历** | build.rs 路径安全 | 代码审查 |
| **错误信息** | 不泄露敏感信息 | 代码审查 |

### 8.4 兼容性验收

| 检查项 | 验收标准 | 测试方法 |
|--------|----------|----------|
| **Rust 版本** | 支持 Rust 1.75+ | CI 测试 |
| **平台兼容** | Windows/Linux/macOS | CI 矩阵测试 |
| **向后兼容** | 旧代码可继续编译 | 运行 `cargo check` 测试 |
| **API 稳定** | gate.rs 公共 API 不变 | 编译测试 |

### 8.5 文档验收

| 文档 | 验收标准 |
|------|----------|
| **提示词模板** | 包含完整规则，长度 < 2000 字符 |
| **LSP 规范** | 包含协议定义、示例、错误码 |
| **pre-commit 文档** | 包含安装、使用、故障排查 |
| **开发文档** | 包含任务分解、验收标准、依赖关系 |

---

## 9. 依赖关系图

```
Phase -1: 提示词工程
    ├── Task -1.1: 提示词模板
    ├── Task -1.2: 会话记忆协议 ─── 依赖 -1.1
    └── Task -1.3: CI 提示词 ─── 依赖 -1.1

Phase 0: 脚手架增强
    ├── Task 0.1: new_crate 增强
    ├── Task 0.2: new_signal ─── 依赖 0.1
    ├── Task 0.3: new_cell ─── 依赖 0.1
    ├── Task 0.4: new_tool ─── 依赖 0.1
    └── Task 0.5: new_guard ─── 依赖 0.1

Phase 1: IDE/LSP
    ├── Task 1.1: LSP 协议设计
    ├── Task 1.2: LSP 服务器核心 ─── 依赖 1.1
    ├── Task 1.3: Cargo.toml 检查 ─── 依赖 1.2
    ├── Task 1.4: architecture.toml 检查 ─── 依赖 1.2
    ├── Task 1.5: Rust 源码检查 ─── 依赖 1.2
    ├── Task 1.6: Code Action 修复 ─── 依赖 1.3, 1.4
    └── Task 1.7: LSP 集成测试 ─── 依赖 1.3, 1.4, 1.5, 1.6

Phase 2: 预提交钩子
    ├── Task 2.1: 检查规范设计
    ├── Task 2.2: staging area 检查 ─── 依赖 2.1
    ├── Task 2.3: pre-commit 钩子脚本 ─── 依赖 2.2
    ├── Task 2.4: 自动修复建议 ─── 依赖 2.2
    └── Task 2.5: 集成到 xtask ─── 依赖 2.3, 2.4

Phase 3: 编译期增强
    ├── Task 3.1: build_hook.rs 增强
    ├── Task 3.2: build.rs 模板生成 ─── 依赖 3.1, 0.1
    └── Task 3.3: 编译期测试 ─── 依赖 3.1

Phase 4: 运行时 API
    ├── Task 4.1: gate.rs API 增强
    ├── Task 4.2: 动态架构验证 ─── 依赖 4.1
    └── Task 4.3: 架构变更通知 ─── 依赖 4.1
```

---

## 10. 风险与回滚

### 10.1 风险清单

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| **LSP 性能问题** | 智能体编辑卡顿 | 中 | 缓存 + 增量检查 |
| **pre-commit 误报** | 智能体绕过钩子 | 中 | 提供 `--no-verify` + 详细错误信息 |
| **build.rs 编译时间** | 编译变慢 | 低 | OnceLock 缓存 + 增量解析 |
| **向后兼容破坏** | 旧代码无法编译 | 低 | 保持 gate.rs API 稳定 |
| **智能体忽略提示词** | 事前约束失效 | 高 | 多层防御，不依赖单一层 |

### 10.2 回滚策略

| 组件 | 回滚方法 |
|------|----------|
| **提示词** | 移除 `.axiom/prompts/` 目录 |
| **LSP** | 卸载 LSP 服务器，不影响编译 |
| **pre-commit** | 删除 `.git/hooks/pre-commit` |
| **build.rs** | 回退到旧版本 build.rs |
| **gate.rs** | 保持 API 稳定，无需回滚 |

---

## 11. 实施优先级

### 11.1 优先级矩阵

| 优先级 | Phase | 任务 | 效果 | 成本 | 依赖 |
|--------|-------|------|------|------|------|
| **P0** | -1 | 提示词工程 | 智能体知晓规则 | 低 | 无 |
| **P0** | 2 | pre-commit 钩子 | 阻止违规提交 | 低 | 无 |
| **P1** | 0 | new_crate 增强 | 自动注册 | 低 | 无 |
| **P1** | 3 | build_hook 增强 | 更详细的错误信息 | 低 | 无 |
| **P2** | 1 | LSP 实时反馈 | 实时标红 | 中 | P0 |
| **P2** | 4 | gate.rs API 扩展 | 运行时查询 | 低 | 无 |
| **P3** | 0 | new_signal/cell/tool/guard | 完整脚手架 | 中 | P1 |

### 11.2 建议实施顺序

```
Week 1: P0 任务
  Day 1-2: Task -1.1, -1.2 (提示词工程)
  Day 3-4: Task 2.1, 2.2, 2.3 (pre-commit 钩子)

Week 2: P1 任务
  Day 1-2: Task 0.1 (new_crate 增强)
  Day 3-4: Task 3.1, 3.2 (build_hook 增强)

Week 3: P2 任务
  Day 1-3: Task 1.1, 1.2, 1.3 (LSP 基础)
  Day 4-5: Task 1.4, 1.5, 1.6 (LSP 检查)

Week 4: P3 任务
  Day 1-2: Task 0.2, 0.3 (new_signal, new_cell)
  Day 3-4: Task 0.4, 0.5 (new_tool, new_guard)
  Day 5: Task 4.1, 4.2 (gate.rs 扩展)
```

---

## 12. 验收检查清单

### 12.1 每日检查

- [ ] `cargo check --workspace` 通过
- [ ] `cargo test --workspace` 通过
- [ ] `cargo run -p archcheck --` 零违规
- [ ] 新代码符合架构规则

### 12.2 每周检查

- [ ] 所有新增 crate 已注册
- [ ] 所有新增依赖已审计
- [ ] 所有 build.rs 已更新
- [ ] 提示词模板已更新
- [ ] 文档已更新

### 12.3 发布检查

- [ ] 所有 Phase 任务完成
- [ ] 所有验收标准通过
- [ ] 性能测试通过
- [ ] 安全测试通过
- [ ] 兼容性测试通过
- [ ] 文档完整
- [ ] 已推送到 GitHub

---

## 13. 附录

### 13.1 术语表

| 术语 | 说明 |
|------|------|
| **事前约束** | 在代码生成/提交前的约束 |
| **事中约束** | 在代码编辑时的约束 |
| **事后约束** | 在代码提交/编译后的约束 |
| **LSP** | Language Server Protocol |
| **pre-commit** | Git 预提交钩子 |
| **build.rs** | Cargo 构建脚本 |
| **archcheck** | 架构检查工具 |
| **gate.rs** | 运行时架构 API |

### 13.2 参考文档

- [架构治理实施计划](architecture-governance-implementation.md)
- [架构设计图](../architecture-diagram.md)
- [审查标准](../.axiom/review-standards.md)
- [会话引导协议](../.axiom/bootstrap.md)

---

**文档版本**：v1.0
**最后更新**：2026-07-04
**维护者**：Axiom Core 团队
