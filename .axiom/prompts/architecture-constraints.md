# Axiom Core 架构约束提示词

> **使用说明**：将此文件内容嵌入到智能体的系统提示词中，或在每次会话开始时作为上下文加载。

---

## 你是 Axiom Core 项目的智能体开发者

所有你生成的代码、编辑的文件、提交的变更，都必须遵守以下架构规则。
**违反这些规则会导致编译失败，必须回滚修复。**

---

## 1. 分层架构（9层）

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

**铁律**：Layer N 的 crate **只能依赖** Layer >= N 的 crate。

**示例**：
- `axiom-cli` (Layer 0) 可以依赖 `axiom-core` (Layer 7) ✅
- `axiom-core` (Layer 7) 不能依赖 `axiom-cli` (Layer 0) ❌

---

## 2. 禁止依赖

| 依赖 | 规则ID | 原因 |
|------|--------|------|
| `async-trait` | R-004 | Rust 1.75+ 已支持原生 `async fn in traits` |

**任何 crate 引入 `async-trait` 都会导致编译失败。**

---

## 3. 审计依赖

引入任何第三方依赖前，必须先检查 `.axiom/architecture.toml` 中的 `[audited-deps]`。

**如果依赖不在 audited-deps 中，必须**：
1. 评估是否真的需要
2. 如果是，添加到 audited-deps 并说明理由
3. 如果不是，寻找替代方案

**当前 audited deps（30个）**：
```
tokio, serde, serde_json, thiserror, anyhow, tracing, tracing-subscriber,
sha2, uuid, futures, clap, ratatui, crossterm, syn, quote, proc-macro2,
linkme, trybuild, regex, parking_lot, dashmap, sqlx, snap, tempfile,
criterion, schemars, reqwest, url, axum, toml, walkdir, once_cell, archcheck
```

---

## 4. 豁免机制

### Proc-macro 豁免
- `axiom-macros` → 允许依赖 `axiom-core`
- 原因：Proc-macro 必须引用 core 类型以进行宏展开

### 反向依赖豁免
- `axiom-agent` → 允许依赖 `axiom-identity`, `axiom-prompt`
- 原因：Agent 需要调用 identity/prompt facade 完成用户交互

**豁免必须显式声明在 `.axiom/architecture.toml` 中，并写明原因。**

---

## 5. 代码生成规则

### 5.1 创建新 Crate

**必须使用 CLI**：
```bash
cargo run -p xtask -- new_crate --name <crate-name> --layer <0-7>
```

**禁止手动创建 Cargo.toml 后忘记注册。**

CLI 会自动完成：
- 创建目录结构
- 生成 Cargo.toml（只允许依赖同层或更低层）
- 生成 src/lib.rs
- 自动更新 `.axiom/architecture.toml`
- 创建 build.rs（编译期门禁）

### 5.2 所有 build.rs 必须调用 archcheck

```rust
// 每个 crate 的 build.rs 必须包含：
fn main() {
    archcheck::build_hook::check_current_crate(env!("CARGO_PKG_NAME"));
}
```

**禁止删除或修改 build.rs 中的 archcheck 调用。**

### 5.3 禁止硬编码架构常量

**禁止在代码中硬编码**：
- 层数字（如 `const LAYER: usize = 7;`）
- 依赖列表
- 架构规则

**必须从 `.axiom/architecture.toml` 或 `gate.rs` API 读取。**

---

## 6. 开发流程

### 标准开发循环

```bash
# 1. 编写代码
vim crates/axiom-newfeature/src/lib.rs

# 2. 编译检查（自动触发架构门禁）
cargo check -p axiom-newfeature
#    ├── 如果违规 → panic，显示详细错误信息
#    └── 如果合规 → 继续编译

# 3. 运行测试
cargo test -p axiom-newfeature

# 4. 架构检查
cargo run -p archcheck --
#    ├── 检查所有 crate 注册状态
#    ├── 检查依赖方向
#    ├── 检查 forbidden/audited deps
#    └── 输出 violations 报告

# 5. 提交
git add .
git commit -m "feat: add newfeature"
git push
```

### 添加依赖时的检查

```bash
# 1. 编辑 Cargo.toml
vim crates/axiom-myapp/Cargo.toml

# 2. 编译（自动触发架构门禁）
cargo check -p axiom-myapp

# 3. 如果编译失败，查看错误信息
#    ├── REVERSE DEPENDENCY: 层违规
#    ├── FORBIDDEN DEP: 禁止依赖
#    └── NOT AUDITED: 未审计依赖

# 4. 修复后重新编译
```

---

## 7. 违反后果

| 违规类型 | 后果 | 修复方法 |
|----------|------|----------|
| **反向依赖** | 编译 panic | 移除依赖或添加豁免 |
| **禁止依赖** | 编译 panic | 移除依赖（如 async-trait） |
| **未审计依赖** | 编译 panic | 添加到 audited-deps 或移除 |
| **未注册 crate** | CI 报告 | 运行 `xtask new_crate` 注册 |
| **豁免滥用** | CI 报告 + 人工审查 | 必须写明合理原因 |

**所有违规必须修复后才能合并代码。**

---

## 8. 常用命令

```bash
# 架构检查
cargo run -p archcheck -- --validate-architecture   # 验证 TOML 语法
cargo run -p archcheck -- --list-crates              # 列出 18 个注册 crate
cargo run -p archcheck --                            # 完整架构检查
cargo run -p archcheck -- --format json --output report.json  # JSON 报告

# 统一入口
cargo run -p xtask -- gatecheck --strict             # 严格模式，违规则退出 1
cargo run -p xtask -- gatecheck                      # 非严格模式，仅警告
cargo run -p xtask -- state --output .axiom/state.toml  # 生成状态快照

# 创建新 crate
cargo run -p xtask -- new_crate --name <name> --layer <0-7>

# 编译和测试
cargo check --workspace
cargo test --workspace
```

---

## 9. 关键文件

| 文件 | 用途 |
|------|------|
| `.axiom/architecture.toml` | **唯一真相源**，所有架构规则定义在这里 |
| `.axiom/bootstrap.md` | 会话引导协议，每次新会话必须执行 |
| `.axiom/prompts/architecture-constraints.md` | 本文件，系统提示词模板 |
| `tools/archcheck/` | 架构检查工具 |
| `xtask/` | 统一任务入口 |
| `crates/axiom-core/src/gate.rs` | 运行时架构 API |
| `docs/plans/pre-constraint-enforcement.md` | 事前约束体系完整开发计划 |

---

## 10. 设计原则

| 原则 | 说明 |
|------|------|
| **零信任** | 不依赖开发者自觉，编译期自动拦截 |
| **单一数据源** | `.axiom/architecture.toml` 是唯一真相源 |
| **可追溯** | 所有规则变更记录在案 |
| **向后兼容** | 公共 API 不变 |
| **自约束** | 架构工具自身也受规则约束 |
| **零违规** | 当前工作区完全合规 |

---

## 11. 快速决策树

```
需要添加新依赖？
    │
    ├── 是 internal (axiom-*) 依赖？
    │   ├── 是 → 检查层方向（Layer N 只能依赖 Layer >= N）
    │   │       ├── 合规 → 继续
    │   │       └── 违规 → 移除或添加豁免
    │   └── 否 → 继续检查
    │
    ├── 在 audited-deps 中？
    │   ├── 是 → 继续
    │   └── 否 → 添加到 audited-deps 或寻找替代方案
    │
    └── 是 forbidden dep？
        ├── 是 → 移除（如 async-trait）
        └── 否 → 继续

需要创建新 crate？
    │
    ├── 使用 CLI：cargo run -p xtask -- new_crate --name <name> --layer <0-7>
    └── 禁止手动创建

需要修改 architecture.toml？
    │
    ├── 运行 cargo run -p archcheck -- --validate-architecture
    └── 运行 cargo run -p archcheck -- 检查影响范围
```

---

## 12. 联系与反馈

如果对架构规则有疑问：
1. 查看 `.axiom/architecture.toml` 中的 `reason` 字段
2. 查看 `docs/plans/architecture-governance-implementation.md`
3. 查看 `docs/plans/pre-constraint-enforcement.md`
4. 联系架构维护者

---

**记住**：架构约束不是障碍，而是**保障系统长期可维护性的基础设施**。
遵守规则 = 减少技术债务 = 更快交付业务价值。
