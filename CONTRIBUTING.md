# Contributing to Axiom Core

感谢你对 Axiom Core 的关注与贡献。本文件说明如何搭建开发环境、代码风格要求、提交流程及测试规范。

---

## 1. 开发环境

### 1.1 必需工具

- **Rust**: 1.85+（工作区 `rust-version` 已锁定）
- **Git**: 2.40+
- **SQLite**: 3.40+（用于本地持久化测试）
- **Cargo**: 1.85+

### 1.2 克隆仓库

```bash
git clone <repository-url>
cd axiom-core-project
```

### 1.3 快速验证

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

---

## 2. 代码风格

### 2.1 格式化

- 使用 `cargo fmt --all` 统一格式
- 禁止提交未格式化的代码
- CI 中 `cargo fmt --all -- --check` 为门禁项

### 2.2 Clippy

- 使用 `cargo clippy --workspace -- -D warnings` 检查
- 所有 warning 必须修复，禁止 `#[allow(...)]` 掩盖问题（测试代码除外）

### 2.3 命名规范

- 函数/变量：`snake_case`
- 类型/枚举/ trait：`PascalCase`
- 常量：`SCREAMING_SNAKE_CASE`
- 模块名：`snake_case`

### 2.4 文档注释

- 所有 `pub` 项必须有 `///` 文档
- 模块级使用 `//!` 文档
- 复杂逻辑必须有行内注释说明“为什么”

---

## 3. 提交规范

### 3.1 Commit Message

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Type**:
- `feat`: 新功能
- `fix`: 修复 bug
- `docs`: 文档更新
- `refactor`: 重构
- `test`: 测试相关
- `chore`: 构建/工具链调整
- `perf`: 性能优化

**Scope**:
- `core`, `runtime`, `store`, `viz`, `macros`, `cli`, `bench`, `agent`, `alert`, `distributed`, `identity`, `llm`, `memory`, `mcp`, `oversight`, `planner`, `prompt`, `tool`

**示例**:
```
feat(runtime): add SignalCodec trait with bincode implementation

- Add SignalCodec trait with encode/decode methods
- Implement JsonCodec and BincodeCodec
- Wire codec into MessageBus for internal dispatch

Closes #123
```

### 3.2 分支策略

- `master`: 稳定分支，受保护
- `feat/*`: 功能分支
- `fix/*`: 修复分支
- `release/*`: 发布分支

### 3.3 PR 要求

- 代码必须通过所有 CI 检查
- 必须有对应的测试覆盖
- 需要至少 1 人 review 通过
- 文档已同步更新

---

## 4. 测试要求

### 4.1 测试分层

- **单元测试**: 放在 `src/` 文件内的 `#[cfg(test)]` 模块
- **集成测试**: 放在 `tests/` 目录
- **端到端测试**: 放在 `tests/e2e/` 目录
- **文档测试**: 使用 `///` 示例代码

### 4.2 测试命名

```
test_<functionality>_when_<condition>_should_<expected>
```

**示例**:
```rust
#[test]
fn test_mailbox_push_when_capacity_exceeded_should_reject() {
    // ...
}
```

### 4.3 测试覆盖

- 核心 crate（`axiom-core`、`axiom-runtime`、`axiom-store`）行覆盖率 ≥ 80%
- 新功能必须有对应测试
- 修复 bug 必须有 regression test

---

## 5. 架构规则

### 5.1 依赖方向

```
Layer 0: axiom-cli, axiom-bench
Layer 1: axiom-viz
Layer 2: axiom-identity, axiom-prompt
Layer 3: axiom-mcp, axiom-alert, axiom-agent, axiom-oversight
Layer 4: axiom-distributed, axiom-planner, axiom-runtime
Layer 5: axiom-llm, axiom-tool, axiom-memory, axiom-store
Layer 7: axiom-core
Layer 8: axiom-macros (proc-macro, 豁免)
```

### 5.2 禁止项

- 禁止 `async-trait`（Rust 1.75+ 已支持原生 async fn in traits）
- 禁止在核心 crate 使用 `anyhow`
- 禁止 `std::sync::Mutex` 在 async 上下文中使用
- 禁止生产代码中使用 `unwrap()`/`expect()`/`panic!`

### 5.3 审计依赖

- 所有新依赖必须加入 `.axiom/architecture.toml` 的 `[audited-deps]`
- 禁止引入 GPL 传染协议依赖

---

## 6. 发布流程

### 6.1 版本号

- 遵循 SemVer: `MAJOR.MINOR.PATCH`
- Breaking change → MAJOR
- 新功能 → MINOR
- Bug fix → PATCH

### 6.2 发布步骤

1. 更新 `CHANGELOG.md`
2. 更新各 crate `Cargo.toml` 版本号
3. 创建 Git tag: `git tag v0.X.Y`
4. 推送到远程: `git push origin v0.X.Y`

---

## 7. 问题反馈

- 使用 GitHub Issues 提交 bug 报告
- 包含复现步骤、环境信息、预期行为
- 性能问题请附上 benchmark 数据

---

## 8. 行为准则

- 尊重他人，保持专业
- 接受建设性批评
- 关注社区最佳利益
