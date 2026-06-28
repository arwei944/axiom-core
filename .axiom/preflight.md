# 🔴 预检清单 Preflight Checklist

> **本清单是开发的硬门禁。所有项必须全部打勾，才能开始修改代码。**
> 未完成预检而修改代码 = 违反 R-000 = 代码回滚。

---

## 使用方式

每次新会话开始或切换Task时：
1. 逐项检查以下所有条目
2. 全部打 `[x]` 后，才能执行第一个文件修改操作
3. 如果任何一项无法打勾，先解决该问题，再继续

---

## A. 约束加载检查（必须全部✅）

- [ ] **A1**: 我已读取 [AGENTS.md](AGENTS.md)，理解R-000强制加载规则
- [ ] **A2**: 我已读取 [identity/axiom-builder.md](identity/axiom-builder.md)，知道我的身份、权限和边界
- [ ] **A3**: 我已读取 [rules/axiom-builder-rules.md](rules/axiom-builder-rules.md)，知道所有Critical级规则
- [ ] **A4**: 我已读取 [skills/axiom-builder-skills.md](skills/axiom-builder-skills.md)，知道有哪些可用技能
- [ ] **A5**: 我已读取 [tools.md](tools.md)，知道哪些命令/操作是允许的，哪些是禁止的

## B. 任务上下文检查（必须全部✅）

- [ ] **B1**: 我已读取当前阶段的plan文件（`docs/plans/`目录下对应的plan）
- [ ] **B2**: 我知道当前是哪个Phase、哪个Task、哪个Step
- [ ] **B3**: 我知道本Step要修改哪些文件（精确路径）
- [ ] **B4**: 我知道本Step要新增哪些文件（精确路径）
- [ ] **B5**: 我知道本Step完成后的验收标准（期望输出）
- [ ] **B6**: 我没有跳过任何前置Step

## C. 代码状态检查（必须全部✅）

- [ ] **C1**: 工作目录是干净的或我了解当前所有未提交的变更（`git status`）
- [ ] **C2**: 当前在主分支（main/master）上，且与远程同步（`git pull`）
- [ ] **C3**: 我已确认当前分支正确，不在detached HEAD或feature分支上（除非明确要求）
- [ ] **C4**: 我已读取要修改文件的最新内容（没有用过期的上下文）
- [ ] **C5**: 上一个Task的clippy检查已通过（`cargo clippy --workspace -- -D warnings`）

## D. 规则确认（必须全部✅）

- [ ] **D1**: 我承诺本Step不引入async-trait依赖（R-004）
- [ ] **D2**: 我承诺本Step不添加unsafe代码（R-005）
- [ ] **D3**: 我承诺不引入新的第三方依赖（R-008），如需要将先报告用户
- [ ] **D4**: 我承诺不修改公共API签名（R-007），如需要将先报告用户
- [ ] **D5**: 我承诺每个Task完成后`cargo build`零警告（R-001）
- [ ] **D6**: 我承诺每个Task完成后`cargo test`全通过（R-002）
- [ ] **D7**: 我承诺每个Task完成后`cargo clippy --workspace -- -D warnings`零警告
- [ ] **D8**: 我承诺不写TODO/FIXME占位符（R-011）
- [ ] **D9**: 我承诺修改文件前先Read获取最新内容（工具铁律1）
- [ ] **D10**: 我承诺Write/Edit后立即编译验证（工具铁律2）
- [ ] **D11**: 我承诺不写项目路径外的文件（工具铁律3）
- [ ] **D12**: 我承诺不修改.axiom/目录下的任何约束文件（R-021），除非用户明确授权
- [ ] **D13**: 我承诺引入新依赖前通过安全审计检查（R-022）

## E. 技能激活（按需✅，不适用的打N/A）

- [ ] **E1**: rust-trait-design — 本Step涉及trait定义/公共API设计
- [ ] **E2**: error-type-design — 本Step涉及错误类型
- [ ] **E3**: test-driven-dev — 本Step需要先写测试
- [ ] **E4**: vector-clock-causality — 本Step涉及消息顺序/因果
- [ ] **E5**: witness-chain-integrity — 本Step涉及Witness产生
- [ ] **E6**: layer-enforcement — 本Step涉及Cell间通信
- [ ] **E7**: dependency-direction — 本Step涉及use/import/Cargo.toml
- [ ] **E8**: code-formatting — 本Step完成后需要cargo fmt/clippy
- [ ] **E9**: commit-discipline — 本Step完成后需要commit
- [ ] **E10**: zero-warning-policy — 本Step需要检查零警告

---

## 预检结果

**预检时间**: _______(填写日期时间)_______  
**当前Phase/Task/Step**: _______(填写)_______  
**所有A-D项是否全部✅**: □ 是 → 可以开始编码  □ 否 → 先解决未✅项

---

## ⛔ 警告

如果你在没有完成此清单的情况下开始写代码：
- 你违反了 R-000（最高优先级规则）
- 你产生的代码将被视为不可信，需要回滚重写
- 你的行为不符合"约束者必先受约束"的架构哲学

**约束先于代码。** 这不是建议，是铁律。
