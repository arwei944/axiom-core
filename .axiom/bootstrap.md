# Axiom Core 会话引导协议

> **每次新会话开始的强制 checklist。**
> 不完成此 checklist，不得执行任何文件修改操作。

## 第一步：加载架构约束（强制）

1. 读取 [`.axiom/architecture.toml`](.axiom/architecture.toml) — 单一数据源，了解当前架构规则
2. 读取 [`.axiom/AGENTS.md`](.axiom/AGENTS.md) — 约束入口和身份定义
3. 读取 [`.axiom/rules/axiom-builder-rules.md`](.axiom/rules/axiom-builder-rules.md) — 开发铁律
4. 读取 [`.axiom/identity/axiom-builder.md`](.axiom/identity/axiom-builder.md) — 开发者身份和权限边界
5. 读取 [`.axiom/skills/axiom-builder-skills.md`](.axiom/skills/axiom-builder-skills.md) — 可用技能包
6. 读取 [`.axiom/tools.md`](.axiom/tools.md) — 工具权限边界

## 第二步：检查项目状态

```bash
# 1. 检查工作区是否能正常编译
cargo check --workspace

# 2. 运行测试确保没有回归
cargo test --workspace

# 3. 检查架构门禁状态
cargo run -p xtask -- gatecheck --format text
```

## 第三步：确认当前任务

读取 `docs/plans/` 目录下的 plan 文件，确认：
- 当前处于哪个 Phase
- 当前 Task 的原子化子任务列表
- 每个子任务的验收标准

## 第四步：验证架构一致性

```bash
# 运行架构检查工具
cargo run -p archcheck --

# 检查是否有未注册的 crate
cargo run -p archcheck -- --list-crates

# 验证 architecture.toml 语法正确
cargo run -p archcheck -- --validate-architecture
```

## 会话记忆协议

当会话中断或需要恢复时：

1. **重新执行本 checklist**
2. **读取最近的 git commit** 了解当前进度
3. **检查 `.axiom/state.toml`**（如果存在）获取上次会话状态
4. **确认未完成的 TODO 项**，继续执行

## 关键原则

- **单一数据源**：`.axiom/architecture.toml` 是架构规则的唯一来源
- **编译期强制**：不信任运行时检查，所有规则在编译期强制执行
- **零信任**：假设所有代码提交都可能违反架构，需要自动检查
- **可追溯**：每次架构变更必须记录在案

## 快速命令参考

```bash
# 编译检查
cargo check --workspace

# 测试
cargo test --workspace

# 架构门禁（严格模式，失败则退出）
cargo run -p xtask -- gatecheck --strict

# 架构报告（JSON 格式）
cargo run -p xtask -- gatecheck --format json --output arch-report.json

# 列出所有已注册 crate
cargo run -p archcheck -- --list-crates
```
