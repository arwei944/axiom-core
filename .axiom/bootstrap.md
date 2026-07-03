# Axiom Core 会话引导协议

> **每次新会话开始的强制 checklist。**
> 不完成此 checklist，不得执行任何文件修改操作。

## 第零步：加载架构约束提示词（强制）

在开始任何工作之前，**必须**读取以下提示词文件，确保完全理解架构规则：

1. 读取 [`.axiom/prompts/architecture-constraints.md`](.axiom/prompts/architecture-constraints.md) — **系统提示词模板，包含完整架构规则**
2. 确保理解并记住以下核心规则：
   - 9 层分层架构，Layer N 只能依赖 Layer >= N
   - 禁止依赖：`async-trait`（R-004）
   - 审计依赖：30 个 audited deps，未审计依赖阻止引入
   - 豁免机制：proc-macro 豁免 + 反向依赖豁免
   - 代码生成规则：必须使用 CLI 创建 crate，禁止硬编码常量
   - 违反后果：编译 panic + CI 报告 + 必须回滚

**不读取此文件，不得执行任何文件修改操作。**

---

## 第一步：加载架构约束（强制）

1. 读取 [`.axiom/architecture.toml`](.axiom/architecture.toml) — 单一数据源，了解当前架构规则
2. 读取 [`.axiom/AGENTS.md`](.axiom/AGENTS.md) — 约束入口和身份定义
3. 读取 [`.axiom/rules/axiom-builder-rules.md`](.axiom/rules/axiom-builder-rules.md) — 开发铁律
4. 读取 [`.axiom/identity/axiom-builder.md`](.axiom/identity/axiom-builder.md) — 开发者身份和权限边界
5. 读取 [`.axiom/skills/axiom-builder-skills.md`](.axiom/skills/axiom-builder-skills.md) — 可用技能包
6. 读取 [`.axiom/tools.md`](.axiom/tools.md) — 工具权限边界

---

## 第二步：检查项目状态

```bash
# 1. 检查工作区是否能正常编译
cargo check --workspace

# 2. 运行测试确保没有回归
cargo test --workspace

# 3. 检查架构门禁状态
cargo run -p xtask -- gatecheck --format text
```

---

## 第三步：确认当前任务

读取 `docs/plans/` 目录下的 plan 文件，确认：
- 当前处于哪个 Phase
- 当前 Task 的原子化子任务列表
- 每个子任务的验收标准

---

## 第四步：验证架构一致性

```bash
# 运行架构检查工具
cargo run -p archcheck --

# 检查是否有未注册的 crate
cargo run -p archcheck -- --list-crates

# 验证 architecture.toml 语法正确
cargo run -p archcheck -- --validate-architecture
```

---

## 第五步：确认理解架构规则

在开始编码前，**必须**能准确回答以下问题：

1. **当前 crate 属于哪一层？**
   - 使用 `cargo run -p archcheck -- --list-crates` 查看
   - 或读取 `crates/axiom-xxx/Cargo.toml` 中的 `build.rs`

2. **可以依赖哪些层？**
   - Layer N 只能依赖 Layer >= N
   - 查看 `.axiom/architecture.toml` 中的 `[crate-layers]`

3. **添加新依赖前需要检查什么？**
   - 是否是 `axiom-*` 内部依赖 → 检查层方向
   - 是否是第三方依赖 → 检查 `[audited-deps]`
   - 是否是禁止依赖 → 检查 `[forbidden-deps]`

4. **创建新 crate 的正确方法？**
   - **必须**使用 `cargo run -p xtask -- new_crate --name <name> --layer <0-7>`
   - **禁止**手动创建 Cargo.toml 后忘记注册

5. **如果遇到架构违规怎么办？**
   - 查看编译错误信息
   - 根据错误类型修复（移除依赖 / 添加豁免 / 调整层）
   - 重新运行 `cargo check` 验证

**无法回答以上问题，不得开始编码。**

---

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
- **事前约束**：在写代码前就知晓规则，在提交前就拦截违规

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

# 创建新 crate
cargo run -p xtask -- new_crate --name <name> --layer <0-7>
```

## 事前约束检查清单

在每次文件修改前，确认：

- [ ] 已读取 `.axiom/prompts/architecture-constraints.md`
- [ ] 已读取 `.axiom/architecture.toml`
- [ ] 已运行 `cargo check --workspace` 确认当前状态
- [ ] 已运行 `cargo run -p archcheck --` 确认零违规
- [ ] 理解当前 crate 的层和允许依赖
- [ ] 知道如何创建新 crate（使用 CLI）
- [ ] 知道如何添加新依赖（检查 audited-deps）

## 开发循环

```
1. 读取架构约束提示词
2. 检查项目状态
3. 确认当前任务
4. 验证架构一致性
5. 确认理解架构规则
6. 开始编码
7. 运行 cargo check（自动触发架构门禁）
8. 运行 cargo test
9. 运行架构检查
10. 提交代码
```
