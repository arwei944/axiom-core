# 文档审查报告：遗漏、矛盾与不足

> 审查日期: 2026-06-29
> 审查范围: docs/architecture/、docs/plans/、.axiom/、当前代码骨架
> 审查维度: 九大理念覆盖度、技术一致性、需求完整性、路线图逻辑、约束体系完备性

---

## 一、🔴 Critical 问题（阻塞开发，必须先修复）

### C1. 当前代码仍然依赖 async-trait，违反 R-004

**位置**: [axiom-core/Cargo.toml:13](file:///D:/work/trae/axiom-core/crates/axiom-core/Cargo.toml#L13) 根 [Cargo.toml](file:///D:/work/trae/axiom-core/Cargo.toml)
**问题**: workspace 和 axiom-core 的 Cargo.toml 仍然把 `async-trait` 列为依赖，与需求文档（第十二节）和规则 R-004 直接矛盾。cell.rs 和 lens.rs 仍然使用 `#[async_trait]` 宏。
**修复**: Phase 0 Task 0 的第一步就应该移除这个依赖，但当前代码骨架是plan之前创建的，没有按plan更新。开始编码前必须先执行 Task 0。

### C2. Signal trait 的 msg_id/correlation_id 默认实现是 panic 炸弹

**位置**: plan 01-phase0-1 Task 1 Step 1
**问题**: Plan 中写的 `fn msg_id(&self) -> &str { unimplemented!() }` 是极差的API设计——如果实现者忘记覆写，运行时直接panic。这违反"工程化"和"结构化"理念。
**修复**: msg_id 和 correlation_id 必须是 required method（无默认实现），或者提供 `#[derive(Signal)]` 宏自动生成字段和实现。Phase 1 不能用 unimplemented!() 作为默认实现。

### C3. 需求文档的三层架构图和四层架构图编号矛盾

**位置**: [00-requirements.md:54-63](file:///D:/work/trae/axiom-core/docs/architecture/00-requirements.md#L54-L63) vs [00-requirements.md:137-152](file:///D:/work/trae/axiom-core/docs/architecture/00-requirements.md#L137-L152)
**问题**: 第二节"确定性优先"中的三层图把执行层标为 Layer 1、验证层 Layer 2、推理层 Layer 3（这是对的），但没有 Layer 0。第三节加了 Layer 0 Oversight 后，第二节的图没有更新编号说明。读者会困惑Layer到底是3层还是4层。
**修复**: 更新第二节的图为四层，或在图下加注"监督层为第零层"。

### C4. hello_cell 验收标准与 Phase 0-1 范围矛盾

**位置**: roadmap P1 验收标准 "hello_cell 可以收发消息、产生Witness"
**问题**: Phase 0-1 只完成 axiom-core 的原语，没有 runtime/mailbox/bus。没有消息总线，Cell 之间无法实际收发消息。hello_cell 示例直接调用 handle() 不算"收发消息"。
**修复**: P1 验收标准改为"CellContext 可以发送 SignalEnvelope 到 outbox、产生 Witness"；真正的多Cell消息通信放到 P3 runtime 完成后验证。

### C5. Witness 缺少"前状态hash/后状态hash"字段

**位置**: Witness 需求表第3行明确要求记录"前状态hash、后状态hash"，但当前 Witness 结构体和 WitnessBuilder 都没有这两个字段。
**修复**: Witness 需要增加 `state_before_hash` 和 `state_after_hash` 字段（Option类型，因为不是所有Cell都实现状态hash）。

---

## 二、🟠 Major 问题（需求遗漏，必须补充）

### M1. 心跳检测完全缺失

**位置**: Cell需求第6行"心跳检测：内置心跳，静默失败能被检测"——无任何phase覆盖。
**说明**: 心跳是检测"Cell死锁/无限循环/静默挂起"的关键机制，监督树的重启依赖它。
**建议**: 在 P3 runtime 中加入 heartbeat 机制（CellContext 设置 heartbeat_deadline，runtime watchdog 检查）。

### M2. 背压机制缺失

**位置**: Cell需求第7行"有界信箱+背压"——P3 mailbox 没有背压设计。
**说明**: 背压是防止OOM和雪崩的关键。当消费者速度跟不上生产者时，需要有策略：丢弃旧消息/阻塞发送者/返回错误。
**建议**: 在 P3 mailbox 设计中明确背压策略。

### M3. 编译期分层强制未实现

**位置**: 架构自约束6.1节说"ExecCell trait没有发消息给AgentCell的方法；编译期阻止直接引用"——但plan只有运行期检查（SignalEnvelope.validate_layer_transition）。
**说明**: 这是"架构自约束"的核心承诺——仅靠运行期检查不够（绕过检查怎么办？）。
**建议**: 定义 `ExecCell`/`ValidateCell`/`AgentCell`/`OversightCell` 子trait，各自的CellContext只暴露合法的发送目标。例如 ExecCellContext 只有 send_to_exec() 方法，没有 send_to_agent()。

### M4. Schema trait 从未定义

**位置**: Signal需求第1行"必须实现Schema trait，编译期检查消息类型"——但Schema trait在代码和plan中从未出现。
**修复**: 需要定义 Schema trait（类似 serde 的 Serialize/Deserialize，但专门用于消息验证，包含字段校验逻辑）。

### M5. 请求-响应消息模式缺失

**位置**: Bus需求"直接投递、发布-订阅、请求-响应模式"——但Signal只有单向 fire-and-forget。
**说明**: Agent场景经常需要 ask-pattern（请求并等待回复），比如"查询订单状态→等待返回"。
**建议**: 在 P3 bus 中增加 Request-Response 模式（使用 oneshot channel）。

### M6. 热升级机制完全没有

**位置**: Cell需求"热升级：不停止系统替换Cell实现"——无任何phase覆盖。
**建议**: 延后到 v2 或在 P3 中预留 trait 接口（Cell::upgrade()），但标注 v1 不实现。

### M7. Cell暂停/恢复未覆盖

**位置**: 类型状态中有 Suspended 状态，但没有pause/resume机制。
**建议**: 在 P3 supervisor 中增加 suspend/resume。

### M8. Witness 采样率缺失

**位置**: Witness需求"可采样：高吞吐场景支持配置采样率"——无任何实现计划。
**建议**: 在 WitnessBuilder 或 Runtime 配置中增加 sample_rate 参数。

### M9. Lens Token 预算感知缺失

**位置**: Lens需求"Token预算感知：Lens投影结果自动估算Token数，超预算时自动摘要"——无覆盖。
**修复**: Lens trait 需要增加 token_estimate() 方法或 TokenBudget 参数。

### M10. Lens 编译期权限边界缺失

**位置**: Lens需求"编译期保证一个Lens只能看到授权的状态子集"——当前Lens trait无权限机制。
**建议**: 通过 LensId + 权限标记（如 const 泛型或 marker type）实现编译期权限隔离。

### M11. 确定性执行器（测试用）缺失

**位置**: 可测试性需求"Cell可在模拟时间中测试，无flaky test"——无覆盖。
**说明**: 这是TDD和"一秒速查"的基础设施——没有确定性执行器，测试依赖系统时间和tokio调度，会产生flaky test。
**建议**: P3 增加 DeterministicRuntime（手动驱动时间和消息，类似 Erlang 的确定性调度）。

### M12. 安全模式（Fail-Safe）缺失

**位置**: 监督层安全保障"监督层异常时，系统进入安全模式（推理层暂停，只允许确定性操作）"——无覆盖。
**建议**: P4 oversight 实现 Fail-Safe 模式。

### M13. 熔断机制不完整

**位置**: 自愈机制"连续N次Axiom违反/崩溃率超阈值→熔断"——当前SupervisionStrategy只有Restart/Stop/Escalate，没有CircuitBreaker状态。
**建议**: 增加 CircuitBreaker 状态机（Closed→Open→Half-Open）。

### M14. MCP缺少沙箱隔离设计

**位置**: MCP安全层只有四层检查，但MCP Server可以执行任意代码/访问文件系统——没有沙箱隔离。
**建议**: 补充MCP安全设计：进程隔离、权限白名单、文件系统路径限制。

---

## 三、🟡 Moderate 问题（设计不足，建议改进）

### O1. no_std 兼容性与依赖矛盾

**位置**: 第十二节技术约束说"core crate支持no_std（可选）"，但 Phase 0-1 引入了 tokio、uuid、futures 等std-only依赖。
**建议**: 明确 v1 不做 no_std，移除这条约束或标注 v2 考虑。

### O2. Feature flags 完全未规划

**位置**: "核心crate依赖尽量少，可选择启用feature"——所有 Cargo.toml 没有定义任何 feature。
**建议**: 在 Cargo.toml 中规划 features = ["macros", "sha2", "uuid"] 等可选功能。

### O3. CellHandle 提到但未实现

**位置**: plan File Structure 列出 CellHandle，但所有 Step 都没有创建它。
**建议**: CellHandle 是 runtime 需要的类型擦除句柄，应该在 P3 实现，P1 不需要它。修正 plan。

### O4. Signaled trait 提到但未定义

**位置**: plan File Structure 列出 "+ Signaled trait"，但从未定义。
**建议**: 删除或明确定义其用途。

### O5. correlation_id 传播链断裂

**位置**: CellContext.send() 创建新 SignalEnvelope 时生成新的 CorrelationId::new()，不从接收的消息中继承/传播。
**说明**: 这会导致全链路追踪断裂——一个请求的correlation_id应该传播到它产生的所有后续消息。
**修复**: CellContext 需要记录当前处理消息的 correlation_id，send() 时复用或派生（如 parent_id → child_id）。

### O6. Plan Step 不包含 cargo clippy 验证

**位置**: Global Constraints 说零警告，但每个 Step 只运行 cargo build/test，没有 cargo clippy。
**修复**: 每个 Task 完成后增加 `cargo clippy --workspace -- -D warnings` 步骤。

### O7. 路线图阶段顺序问题：P9(MCP) 在 P10(LLM+Tool) 之前

**位置**: 00-roadmap.md P9→P10
**问题**: MCP Bridge 需要 ToolRegistry（来自 axiom-tool，P10），但 P9 在 P10 之前。
**修复**: P10（LLM+Tool）应提前到 P9 之前，或 P9/P10 合并。

### O8. axiom-macros 从未排期

**位置**: Crates 规划中有 axiom-macros（#[cell]、#[axiom]、#[signal] 宏），但 roadmap 18个phase没有一个是实现过程宏的。
**建议**: 过程宏在 P1 完成后单独作为一个phase（或者放在P0中）。没有 #[derive(Signal)] 宏，手动实现 Signal trait 非常繁琐。

### O9. 可视化P5依赖P4过强

**位置**: P5 只在 P4 之后，但 topology/timeline/entropy 数据在 P3 runtime 完成后就有了。
**建议**: P5 可在 P3 后开始基础版，P4 后增加监督层面板。

### O10. CLI阶段拆分不合理

**位置**: P11 CLI基础、P16 CLI完善——但 axm new（脚手架）不需要runtime存在，axm run 需要runtime。
**建议**: CLI 按功能分阶段而非"基础/完善"二分，如：axm new 在 P1 后即可做，axm run 在 P3 后。

### O11. Skill 触发机制性能隐患

**位置**: Skill "on_message_contains" 触发——runtime需要扫描每个信号的文本内容匹配关键词列表。
**问题**: 全局关键词扫描每条消息会成为性能瓶颈，尤其在100k msg/s时。
**建议**: 触发匹配应该在Agent层（推理层之前）做，不是在runtime层。明确触发只对Agent Cell的入站消息生效。

### O12. Identity 热切换语义未定义

**位置**: Identity "可以在运行时动态挂载/卸载"——但没有定义in-flight消息怎么处理。
**建议**: 增加 Identity versioning——消息携带identity_version，Witness记录当时版本。

---

## 四、🔵 开发约束体系（.axiom/）不足

### D1. 规则自修改防护缺失

**问题**: axiom-builder 可以修改 .axiom/rules/ 下的规则文件来削弱自己的约束——没有"修改约束需要用户确认"的规则。
**修复**: 增加 R-021: "修改 .axiom/ 目录下的任何文件必须先获得用户明确确认"。

### D2. 缺少CI/CD流水线定义

**问题**: 没有 GitHub Actions 配置来自动执行 R-001/R-002（编译+测试）。
**建议**: 在P1完成后增加CI配置phase。

### D3. 缺少版本发布策略

**问题**: 没有 semver 规范、release process、changelog 约定。
**建议**: 补充到开发文档。

### D4. 第三方依赖安全审计规则缺失

**问题**: R-008说"不引入新依赖需报告"，但没有说引入依赖时需要做什么检查（维护状态、下载量、漏洞审计）。
**建议**: 增加依赖引入检查清单。

### D5. 性能回归检测缺失

**问题**: 需求有明确性能目标（<10µs/msg、>100k msg/s），但没有benchmark和性能回归检测。
**建议**: 增加criterion benchmarks，P3开始建立基线。

### D6. Preflight 缺少分支确认

**问题**: C2说"与远程同步（git pull）"但没有确认当前在正确的分支上。
**修复**: 增加 "当前在master/main分支" 检查项。

---

## 五、📊 九大理念覆盖度评分

| 理念 | 覆盖度 | 缺失项 |
|------|--------|--------|
| **可视化** | 80% | 内生数据都想到了，但Viz crate排期偏后；缺少性能火焰图的具体实现方案 |
| **工程化** | 60% | 缺少CI/CD、benchmark、确定性执行器、版本策略 |
| **结构化** | 70% | Signal Schema trait缺失、编译期分层未实现、CellHandle等类型不完整 |
| **极简化** | 85% | 5原语设计很好，但async-trait清理未执行、Signal trait的unimplemented!()反极简 |
| **低熵化** | 75% | EntropyScore骨架有了，但熵度量公式、减熵动作触发条件、交接次数上限未细化 |
| **一秒速查** | 85% | Witness链+axm why设计很好，但correlation_id传播链断裂会影响速查准确性 |
| **自愈化** | 55% | 监督树有了，但熔断状态机、安全模式、心跳检测、背压未覆盖 |
| **架构就是一切** | 65% | 架构自约束理念写得好，但编译期强制未实现、当前代码违反R-004 |
| **智能体专用** | 80% | 工具链规划全面，但MCP沙箱、Skill触发性能、Identity热切换语义不足 |

**综合覆盖度: 72%** — 理念表达很清晰（尤其是"约束者必先受约束"），但多个关键机制在从"文档"到"可执行plan"的转化中丢失了。

---

## 六、修复优先级建议

### 立即修复（开始编码前）：
1. C1: 移除 async-trait 依赖（执行P0 Task 0）
2. C2: 修复 Signal trait，msg_id/correlation_id 改为 required method
3. C3: 修复需求文档四层架构图编号
4. C5: Witness 增加 state_before/after_hash
5. D1: 增加约束自修改防护规则

### Phase 1 完成前修复：
6. C4: 调整 P1 验收标准
7. O5: 修复 correlation_id 传播
8. O6: plan 增加 clippy 步骤
9. M4: 定义 Schema trait（或合并到 Signal trait）
10. O2: 规划 Cargo.toml feature flags

### Phase 2-3 补充：
11. M1: 心跳检测
12. M2: 背压机制
13. M3: 编译期分层trait
14. M5: 请求-响应模式
15. M11: 确定性执行器
16. M13: 熔断器
17. O7: 修复路线图P9/P10顺序
18. O8: 补充 axiom-macros phase
