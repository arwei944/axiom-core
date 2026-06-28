# 开发规则 Rules: axiom-builder 铁律

> 规则是我必须遵守的行为约束。Critical级别的规则违反等同于Axiom违反——立即熔断。

## 规则分级

- 🔴 **Critical（硬约束/熔断级）**: 违反即停止，必须修复后才能继续
- 🟠 **Strict（严格）**: 违反必须记录理由并修复，不能跳过
- 🟡 **Warning（警告）**: 尽量遵守，特殊情况可偏离但需说明
- 🔵 **Info（指南）**: 最佳实践，推荐遵守

---

## R-000: 强制约束加载（最高优先级）🔴 Critical

**规则**: 每次新会话开始或切换Task时，必须先完成 [preflight.md](../preflight.md) 预检清单全部打勾，才能修改任何代码。
**检查方式**: 预检清单是代码修改的前置门禁；未完成预检不得执行Edit/Write/Delete/代码修改类RunCommand。
**违反后果**: 违反R-000产生的代码提交一律回滚。此规则凌驾于所有其他规则之上。

---

## R-001: 编译零警告 🔴 Critical

**规则**: `cargo build --workspace` 和 `cargo clippy --workspace` 必须零错误零警告。
**检查方式**: 每个Task完成后运行编译命令。
**违反后果**: 立即停止后续步骤，修复警告后才能继续。

## R-002: 测试必须通过 🔴 Critical

**规则**: `cargo test --workspace` 必须全部通过。
**检查方式**: 每个Task完成后运行测试。
**违反后果**: 不得commit。

## R-003: TDD红绿循环 🔴 Critical

**规则**: 新功能必须先写失败测试→写代码→测试通过。
**检查方式**: 检查是否有对应测试文件和测试用例。
**违反后果**: 回滚未测试的代码，先补测试。

## R-004: 不引入async-trait依赖 🔴 Critical

**规则**: 全部使用Rust原生async fn in traits（Rust 1.75+），不使用async-trait宏。
**检查方式**: `grep -rn "async-trait" Cargo.toml crates/` 应无匹配。
**违反后果**: 立即移除async-trait依赖，重构为原生async fn。

## R-005: unsafe代码隔离 🔴 Critical

**规则**: 所有unsafe代码必须在 `crate::unsafe_impl` 模块中，且每个unsafe块有`// SAFETY:`注释。
**检查方式**: `grep -rn "unsafe" crates/*/src/` 只在unsafe_impl.rs中出现。
**违反后果**: 不得commit。

## R-006: 依赖方向铁律 🔴 Critical

**规则**: crate依赖只能从上到下（core←store←runtime←oversight←agent←cli），禁止反向依赖。
**检查方式**: `cargo tree -p axiom-core` 中不得出现其他workspace crate。
**违反后果**: 架构违规，立即重构。

## R-007: 不修改公共API签名 🟠 Strict

**规则**: plan中定义的公共trait/struct签名不得随意修改。如需修改，暂停并报告用户。
**检查方式**: 对比plan中的接口定义和实际代码。
**违反后果**: 暂停执行，请求用户确认。

## R-008: 不引入新依赖 🟠 Strict

**规则**: Cargo.toml中已定义的依赖外，不引入新的第三方crate。如需添加，报告用户。
**检查方式**: 检查Cargo.toml是否被修改新增[dependencies]。
**违反后果**: 暂停执行，请求用户确认。

## R-009: 错误类型全覆盖 🟠 Strict

**规则**: 每个错误路径必须使用AxiomError的明确变体，不能用unwrap/expect（测试代码除外）。
**检查方式**: 代码中不应有`.unwrap()`（测试代码允许）。
**违反后果**: 替换为?或match处理。

## R-010: Witness必须产生 🟠 Strict

**规则**: Cell的handle方法中，成功路径必须调用emit_success，错误路径必须emit_failure或emit_axiom_violation。
**检查方式**: 每个handle方法至少有一个emit调用。
**违反后果**: 补充Witness产生代码。

## R-011: 不写TODO/FIXME占位 🟠 Strict

**规则**: 代码中不得有TODO、FIXME、placeholder、"implement later"等占位符。
**检查方式**: `grep -rn "TODO\|FIXME\|placeholder\|implement later" crates/` 应无匹配。
**违反后果**: 要么实现，要么删除，不保留占位。

## R-012: 公共API必须有文档 🟠 Strict

**规则**: 所有pub struct、pub trait、pub fn、pub enum必须有`///` rustdoc注释。
**检查方式**: `cargo doc -p <crate> --no-deps` 无missing_docs警告。
**违反后果**: 补充文档。

## R-013: commit message规范 🟡 Warning

**规则**: commit格式为 `type(scope): description`，type ∈ {feat, fix, refactor, test, docs, chore}。
**检查方式**: git log 检查格式。
**违反后果**: 下次commit注意格式。

## R-014: 小步提交 🟡 Warning

**规则**: 每个Task至少一个commit，一个commit只做一件事，不攒大提交。
**检查方式**: 每个Task结束后必须有commit。

## R-015: 文件职责单一 🟡 Warning

**规则**: 每个.rs文件职责清晰，不把不相关的类型放在同一个文件中。
**建议**: 文件超过300行时考虑拆分。

## R-016: 函数长度控制 🟡 Warning

**规则**: 单个函数不超过50行（不含测试代码）。
**建议**: 超过50行的函数拆分为更小的函数。

## R-017: 不硬编码魔法数字 🔵 Info

**规则**: 常量使用const定义，给魔法数字起名字。
**示例**: `const MAX_RETRIES: u32 = 3;` 而非直接写 `3`。

## R-018: 使用thiserror而非anyhow 🔵 Info

**规则**: 库代码（所有crate除了axiom-cli）使用thiserror定义错误，anyhow仅用于二进制crate的应用边界。

## R-019: 发送消息前increment VC 🔵 Info

**规则**: CellContext::send内部自动increment VectorClock，不需要手动调用。
**注意**: 如果直接构造SignalEnvelope，必须手动increment。

## R-020: 遵循plan顺序 🔴 Critical

**规则**: 严格按照plan中的Task顺序执行，不跳步、不合并Step。
**检查方式**: 每个Step的checkbox确认后才进入下一个Step。
**违反后果**: 回退到未完成的Step。

---

## 规则执行机制

1. **每个Step开始前**: 确认涉及的规则
2. **每个Step完成后**: 自检相关规则是否被遵守
3. **每个Task完成后**: 运行所有Critical级检查（编译+测试+unsafe+依赖方向）
4. **发现规则违反**: 立即修复，不带着违规继续

## 规则更新

规则本身可以迭代——如果发现某条规则不合理，报告用户后可以修改。但修改规则的权限在用户，不在我。
