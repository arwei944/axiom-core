# 技能 Skills: axiom-builder 可用技能包

> 技能定义"我会什么"。每个技能是一个可组合的能力单元，按需激活。

## 技能列表

### Skill: rust-trait-design

**触发条件**: 定义trait、设计公共API、类型抽象
**层级约束**: 遵循Rust原生async fn in traits，不用async-trait宏
**指令**:
1. 先写trait定义（带rustdoc注释）
2. 为每个方法写单元测试（或使用example）
3. trait必须是object-safe（如果需要dyn）
4. 泛型约束最小化——不要加不必要的trait bound
5. 错误类型使用 `crate::Result` / `crate::AxiomError`

### Skill: error-type-design

**触发条件**: 添加新错误类型、处理错误路径
**层级约束**: 使用thiserror，不使用anyhow（应用边界除外）
**指令**:
1. 每个错误变体必须有上下文信息（cell_id、msg_id等字段）
2. 错误信息使用 `{field}` 格式直接嵌入Display
3. 不为"正常情况"创建错误变体
4. From实现用于标准库错误转换（IO、Serde等）
5. 错误类型不能panic——所有panic路径必须转化为Result

### Skill: test-driven-dev

**触发条件**: 实现新功能、修复bug
**层级约束**: 红→绿→重构，严格TDD
**指令**:
1. 先写失败测试（包含具体断言）
2. 运行测试确认失败
3. 写最少代码让测试通过
4. 运行测试确认通过
5. 重构（测试继续通过）
6. 每个功能至少一个happy path + 一个error path测试

### Skill: vector-clock-causality

**触发条件**: 处理消息顺序、因果关系、状态版本
**层级约束**: 所有Signal必须携带VectorClock
**指令**:
1. 发送消息前increment自己的clock
2. 接收消息后merge对方的clock
3. 用causally_precedes判断因果关系
4. 不依赖系统时间判断happens-before
5. 并发消息（is_concurrent_with）需要merge

### Skill: witness-chain-integrity

**触发条件**: 状态转换、产生审计记录
**层级约束**: 每个handle调用至少产生一个Witness
**指令**:
1. 成功路径调用ctx.emit_success()
2. 失败路径调用ctx.emit_failure()
3. Axiom违反调用ctx.emit_axiom_violation()
4. Witness通过WitnessBuilder构造，不直接构造Witness
5. 前一个Witness的hash必须作为prev_hash传入

### Skill: layer-enforcement

**触发条件**: Cell间发送Signal、定义Cell归属
**层级约束**: 跨层调用必须通过validate_layer_transition
**指令**:
1. Oversight可以发往任意层
2. Agent只能发往Agent和Validate
3. Validate只能发往Validate和Exec
4. Exec只能发往Exec
5. 反向调用、跨层跳跃一律报错LayerViolation

### Skill: dependency-direction

**触发条件**: 添加use语句、修改Cargo.toml
**层级约束**: crate依赖只能从上到下
**指令**:
1. axiom-core不依赖任何workspace crate
2. axiom-store只依赖axiom-core
3. axiom-runtime依赖axiom-core和axiom-store
4. axiom-oversight依赖axiom-core和axiom-runtime
5. axiom-agent依赖axiom-core和axiom-runtime
6. 上层crate不能被下层crate引用——反向依赖即架构违规

### Skill: code-formatting

**触发条件**: 代码写完后、commit前
**层级约束**: 统一格式，零clippy警告
**指令**:
1. 写完代码运行 `cargo fmt`
2. 运行 `cargo clippy` 修复所有警告
3. 公共API有 `///` rustdoc
4. 不添加任何注释除非用户要求（代码自解释）
5. 函数不超过50行，超过则拆分

### Skill: commit-discipline

**触发条件**: 每个Task完成后
**层级约束**: 小步提交，commit message规范
**指令**:
1. 每个Task至少一个commit
2. 格式：`type(scope): description`
3. type ∈ {feat, fix, refactor, test, docs, chore}
4. 一个commit只做一件事
5. commit前必须 `cargo test -p <crate>` 通过

### Skill: zero-warning-policy

**触发条件**: 任何编译、clippy、doc警告
**层级约束**: 零警告是硬性要求
**指令**:
1. `cargo build` 出现任何warning → 立即修复
2. `cargo clippy` 出现任何warning → 立即修复
3. `cargo doc` 出现missing_docs → 补充文档
4. 不用`#[allow(dead_code)]`等压制警告——删除无用代码
5. unused import → 删除，不要注释掉
