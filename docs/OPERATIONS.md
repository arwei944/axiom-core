# Axiom Core 运维手册

> **版本:** v0.1.0
> **最后更新:** 2026-07-04

---

## 1. 环境要求

### 1.1 必需工具

| 工具 | 版本 | 用途 |
|------|------|------|
| Rust | 1.75+ | 编译（需要原生 async fn in traits） |
| Git | 2.0+ | 版本控制 |
| Node.js | 18+ | 前端可视化（可选） |
| SQLite | 3.40+ | 持久化后端（v0.2.0+） |

### 1.2 验证环境

```bash
# 验证 Rust 版本
rustc --version
# 应 >= 1.75.0

# 验证 Git 版本
git --version

# 验证项目可编译
cargo check --workspace
```

---

## 2. 日常运维命令

### 2.1 编译与测试

```bash
# 快速编译检查
cargo check --workspace

# 完整编译
cargo build --workspace --all-targets

# 运行所有测试
cargo test --workspace

# 运行特定 crate 测试
cargo test -p axiom-core

# 运行基准测试
cargo bench -p axiom-bench
```

### 2.2 架构治理

```bash
# 验证 architecture.toml 语法
cargo run -p archcheck -- --validate-architecture

# 列出所有已注册 crate
cargo run -p archcheck -- --list-crates

# 完整架构检查
cargo run -p archcheck --

# 生成 JSON 报告
cargo run -p archcheck -- --format json --output arch-report.json

# 严格模式检查（违规则退出码 1）
cargo run -p xtask -- gatecheck --strict

# 生成状态快照
cargo run -p xtask -- state --output .axiom/state.toml
```

### 2.3 预提交检查

```bash
# 安装 git pre-commit 钩子
cargo run -p xtask -- precommit --install

# 手动运行预提交检查
cargo run -p xtask -- precommit

# 卸载 pre-commit 钩子
cargo run -p xtask -- precommit --uninstall
```

### 2.4 创建新 Crate

```bash
# 创建新 crate（自动注册 + 自动 build.rs）
cargo run -p xtask -- new_crate --name myfeature --layer 4

# 最小模板（无测试/示例/CI）
cargo run -p xtask -- new_crate --name myfeature --layer 4 --minimal

# 完整模板（含测试/示例/CI）
cargo run -p xtask -- new_crate --name myfeature --layer 4 --full
```

---

## 3. CI/CD 流程

### 3.1 GitHub Actions 工作流

```
push/PR → architecture-observer.yml → archcheck → artifact 上传
```

**关键点**：
- 非阻塞模式，不阻止合并
- 自动上传 `arch-report.json`
- 人工审查 violations

### 3.2 本地 CI 模拟

```bash
# 模拟 CI 检查
cargo check --workspace
cargo test --workspace
cargo run -p archcheck -- --validate-architecture
cargo run -p archcheck --
cargo run -p xtask -- gatecheck --strict
```

---

## 4. 故障排查

### 4.1 编译期架构违规

**现象**：编译失败，显示 `ARCHITECTURE VIOLATION`

**解决方案**：
1. 查看错误信息中的 crate 名和层号
2. 检查 `.axiom/architecture.toml` 中的 `[crate-layers]`
3. 确认依赖方向是否正确
4. 如果是设计需要，添加 `[reverse-dependency-exemptions]`

```bash
# 查看当前 crate 层
cargo run -p archcheck -- --list-crates

# 查看架构规则
cat .axiom/architecture.toml
```

### 4.2 未审计依赖

**现象**：`'xxx' has not been audited (R-022)`

**解决方案**：
1. 确认依赖是否必要
2. 如果是，添加到 `.axiom/architecture.toml` 的 `[audited-deps]`
3. 如果否，寻找替代方案或移除

### 4.3 禁止依赖

**现象**：`'async-trait' is FORBIDDEN`

**解决方案**：
1. 移除 `async-trait` 依赖
2. 使用 Rust 1.75+ 原生 `async fn in traits`

### 4.4 预提交钩子失败

**现象**：`git commit` 被阻止

**解决方案**：
1. 查看错误信息，修复架构违规
2. 紧急情况可使用 `git commit --no-verify` 跳过
3. 修复后重新提交

### 4.5 编译缓慢

**现象**：`cargo check` 时间过长

**解决方案**：
1. 检查是否触发了全量重编译
2. 使用 `cargo check -p <crate>` 只检查单个 crate
3. 清理增量编译缓存：`cargo clean -p <crate>`

---

## 5. 监控与诊断

### 5.1 架构健康检查

```bash
# 检查架构违规
cargo run -p archcheck --

# 检查 crate 注册状态
cargo run -p archcheck -- --list-crates

# 验证 TOML 语法
cargo run -p archcheck -- --validate-architecture
```

### 5.2 运行时诊断

```bash
# 查看 Cell 状态（TODO: v0.2.0+）
axm cell list

# 查看 Witness 链（TODO: v0.2.0+）
axm why <correlation-id>

# 系统健康诊断（TODO: v0.2.0+）
axm doctor
```

### 5.3 性能监控

```bash
# 运行基准测试
cargo bench -p axiom-bench

# 运行压力测试
cargo run -p axiom-bench -- stress
```

---

## 6. 备份与恢复

### 6.1 重要文件备份

| 文件 | 备份频率 | 说明 |
|------|---------|------|
| `.axiom/architecture.toml` | 每次修改 | 架构规则唯一真相源 |
| `.axiom/state.toml` | 每日 | 架构状态快照 |
| `Cargo.toml`（workspace） | 每次修改 | 工作区配置 |
| `.github/workflows/` | 每次修改 | CI/CD 配置 |

### 6.2 状态恢复

```bash
# 恢复 architecture.toml
git checkout HEAD -- .axiom/architecture.toml

# 恢复状态快照
git checkout HEAD -- .axiom/state.toml

# 完整回滚到某个 commit
git revert <commit-hash>
```

---

## 7. 安全考虑

### 7.1 架构规则保护

- `.axiom/architecture.toml` 是唯一真相源，必须严格保护
- 任何修改必须经过代码审查
- 禁止直接修改 `gate.rs` 或 `gate_check.rs` 中的硬编码常量

### 7.2 依赖审计

- 所有新依赖必须经过审计才能加入 `[audited-deps]`
- 禁止引入未审计的第三方依赖
- 定期审查 `[audited-deps]` 列表，移除不再使用的依赖

### 7.3 豁免管理

- 所有豁免必须写明原因
- 豁免必须经过团队 review
- 定期审查豁免列表，移除不再需要的豁免

---

## 8. 常见操作速查

| 操作 | 命令 |
|------|------|
| 编译检查 | `cargo check --workspace` |
| 运行测试 | `cargo test --workspace` |
| 架构检查 | `cargo run -p archcheck --` |
| 严格检查 | `cargo run -p xtask -- gatecheck --strict` |
| 预提交检查 | `cargo run -p xtask -- precommit` |
| 安装钩子 | `cargo run -p xtask -- precommit --install` |
| 创建 crate | `cargo run -p xtask -- new_crate --name <name> --layer <0-7>` |
| 生成快照 | `cargo run -p xtask -- state --output .axiom/state.toml` |
| 格式化 | `cargo fmt --all` |
| Clippy | `cargo clippy --workspace --all-targets --all-features -D warnings` |

---

## 9. 联系与支持

- **架构规则问题**：查看 [.axiom/architecture.toml](.axiom/architecture.toml)
- **开发流程问题**：查看 [docs/HANDOVER.md](HANDOVER.md)
- **约束体系问题**：查看 [docs/plans/pre-constraint-enforcement.md](plans/pre-constraint-enforcement.md)
- **Bug 报告**：提交 GitHub Issue
