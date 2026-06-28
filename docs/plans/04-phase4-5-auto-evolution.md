# Phase 4.5: 自动进化引擎 - 最小任务单元与验收标准

> **Spec参考**：[04-auto-evolution.md](../architecture/04-auto-evolution.md)
>
> **前置依赖**：P4（L2运行时门禁）必须全部验收通过。
>
> **本Phase完成标志**：系统可以自动观察问题→生成提案→沙盒验证→金丝雀测试→采纳/回滚，进化全程受7条元公理保护。

---

## 依赖准备

### TE01: 添加axiom-evolution crate到workspace

**文件**：
- 修改：`Cargo.toml`（根workspace members加入`"crates/axiom-evolution"`）
- 新建：`crates/axiom-evolution/Cargo.toml`
- 新建：`crates/axiom-evolution/src/lib.rs`

**具体操作**：
1. 在根Cargo.toml的`[workspace.members]`中添加`"crates/axiom-evolution"`
2. 创建axiom-evolution crate，依赖axiom-core/axiom-runtime/axiom-store/axiom-oversight/sha2/tokio/serde
3. lib.rs中导出所有模块骨架（`pub mod meta_axioms;`等，模块文件暂时空或用todo!())

**验收标准**：
- [ ] `cargo build -p axiom-evolution` 编译通过零警告
- [ ] `cargo test -p axiom-evolution` 通过
- [ ] crate出现在workspace中

---

## M1-M7元公理实现

### TE02: 定义MetaAxiom类型与ComplianceCheck

**文件**：
- 新建：`crates/axiom-evolution/src/meta_axioms.rs`

**具体操作**：
1. 定义`MetaAxiom`枚举（M1/M2/M3/M4/M5/M6/M7）
2. 定义`ComplianceCheck`结构体，包含每个元公理的通过/失败状态+原因
3. 定义`ComplianceCheck::all_passed() -> bool`方法
4. 定义`META_AXIOMS_HASH: &str`常量（初始值待TE03计算后填入）

**验收标准**：
- [ ] 所有7个元公理有明确的枚举变体和文档注释
- [ ] ComplianceCheck可序列化(serde)
- [ ] 单元测试覆盖ComplianceCheck::all_passed()

### TE03: 实现元公理hash校验（M2）

**文件**：
- 修改：`crates/axiom-evolution/src/meta_axioms.rs`

**具体操作**：
1. 将7条元公理文本定义为`META_AXIOMS_TEXT: &str`常量
2. 启动时计算SHA-256 hash，与硬编码常量比对
3. 提供`verify_meta_axioms_integrity() -> Result<()>`函数
4. hash不匹配返回`AxiomError::MetaAxiomTampered`

**验收标准**：
- [ ] hash值正确（手动计算一次后填入）
- [ ] 单元测试：篡改任一条元公理文本→hash不匹配→返回错误
- [ ] 单元测试：正确文本→验证通过
- [ ] 验证函数是纯函数，无IO

### TE04: 实现速率限制器（M7）

**文件**：
- 新建：`crates/axiom-evolution/src/rate_limiter.rs`

**具体操作**：
1. 定义`RateLimiter`结构体，内部用Mutex<HashMap<ProposalType, VecDeque<Timestamp>>>记录每个类型的采纳时间
2. 定义速率常量：
   - L1参数：≤5/小时
   - L2规则/Lens/路由：≤3/天
   - L3 Cell：≤1/周
   - L4架构：≤1/月
3. 实现`can_propose(proposal_type: ProposalType) -> Result<()>`检查是否超限
4. 实现`record_adoption(proposal_type: ProposalType)`记录采纳时间

**验收标准**：
- [ ] 单元测试：L1在1小时内第6个提案→拒绝
- [ ] 单元测试：L3连续两天各提1个→拒绝（需间隔7天）
- [ ] 线程安全（Mutex保护）
- [ ] 过期记录自动清理（不无限增长）

---

## Proposal与Change类型

### TE05: 定义ProposalId/ProposalType/EvolutionLayer/Change类型

**文件**：
- 新建：`crates/axiom-evolution/src/proposal.rs`

**具体操作**：
1. `ProposalId`：newtype wrapper around UUIDv4
2. `EvolutionLayer`枚举：L1Param/L2aAxiom/L2bLens/L2cRoute/L3Cell/L4Arch
3. `ProposalType`枚举：对应所有提案类型（ParamTune/NewAxiom/NewLens/RouteOpt/NewCell/ArchChange）
4. `Change`枚举：
   - ParamTune { param_name, old_value, new_value }
   - NewAxiom { axiom_id, axiom_code_hash, definition }
   - NewLens { lens_id, lens_definition }
   - RouteChange { from_cell, to_cell, new_route }
   - NewCell { cell_template, config }
   - ArchChange { description, migration_plan }
5. `RollbackPlan`：包含snapshot_id + reverse_migration_id + estimated_rollback_time
6. `FitnessDelta`：expected_entropy_reduction/expected_error_reduction/expected_latency_improvement
7. `FitnessMetric`枚举：Entropy/ErrorRate/LatencyP99/AxiomCatchRate/MemoryUsage
8. `EvolutionProposal`结构体（含所有字段，见spec 4.2节）

**验收标准**：
- [ ] 所有类型可序列化/反序列化(serde)
- [ ] ProposalId生成UUIDv4
- [ ] 单元测试：Proposal序列化round-trip正确

### TE06: 实现EvolutionWitness类型

**文件**：
- 新建：`crates/axiom-evolution/src/witness.rs`

**具体操作**：
1. `EvolutionAction`枚举：Observed/Hypothesized/SandboxPassed/SandboxFailed/CanaryPassed/CanaryFailed/Adopted/AutoRolledBack/ManualRolledBack/Rejected/Approved/Paused/Resumed
2. `EvolutionWitness`结构体：
   - witness_id: WitnessId
   - proposal_id: Option<ProposalId>
   - action: EvolutionAction
   - timestamp: Timestamp
   - before_snapshot_hash: Option<Hash>
   - after_snapshot_hash: Option<Hash>
   - metrics_before: Option<FitnessSnapshot>
   - metrics_after: Option<FitnessSnapshot>
   - reason: Option<String>
   - previous_witness_hash: Hash
   - witness_hash: Hash  // SHA-256(previous_hash + content)
3. 实现hash计算和链接验证逻辑
4. 实现`verify_chain(witnesses: &[EvolutionWitness]) -> Result<()>`

**验收标准**：
- [ ] EvolutionWitness实现了Witness trait（如果存在）或能融入主Witness链
- [ ] hash链接验证：篡改任一witness→链断裂
- [ ] 单元测试：3条witness链→验证通过；篡改第2条→验证失败
- [ ] FitnessSnapshot包含entropy/error_rate/latency_p99/axiom_catch_rate

---

## 观察引擎

### TE07: 定义ImprovementSignal类型

**文件**：
- 新建：`crates/axiom-evolution/src/observer.rs`（部分）

**具体操作**：
1. `SignalType`枚举：HotCell/RepeatedViolation/PerformanceDegradation/MissingAxiom/SchemaPattern/UnusedField/HighReplayRate/MailboxBacklog
2. `ImprovementSignal`结构体：
   - signal_type
   - evidence: serde_json::Value（量化数据）
   - affected_components: Vec<ComponentId>
   - confidence: f64 (0.0-1.0)
   - detected_at: Timestamp

**验收标准**：
- [ ] 类型可序列化
- [ ] confidence有范围校验（0.0-1.0，超出panic或clamp）

### TE08: 实现Witness分析器

**文件**：
- 修改：`crates/axiom-evolution/src/observer.rs`

**具体操作**：
1. `WitnessAnalyzer::analyze_recent(witnesses: &[Witness], window: Duration) -> Vec<ImprovementSignal>`
2. 检测逻辑：
   - RepeatedViolation: 同一AxiomViolation类型在window内出现≥N次（默认N=5）
   - HighReplayRate: 某Cell的消息重放率>20%
3. 返回置信度>0.7的信号

**验收标准**：
- [ ] 单元测试：构造5条相同类型AxiomViolation witness→检测到RepeatedViolation信号
- [ ] 单元测试：构造正常witness→不产生信号
- [ ] 单元测试：高重放率witness→检测到HighReplayRate
- [ ] 纯函数分析，不修改状态

### TE09: 实现熵值热力图分析

**文件**：
- 修改：`crates/axiom-evolution/src/observer.rs`

**具体操作**：
1. `EntropyHeatmap::build(entropy_readings: &[(ComponentId, f64, Timestamp)]) -> Vec<ImprovementSignal>`
2. 计算各Cell熵值的均值μ和标准差σ
3. 熵值>μ+2σ的Cell标记为HotCell信号
4. 熵值上升速率>阈值（0.1/小时）的标记为信号

**验收标准**：
- [ ] 单元测试：大部分Cell熵在0.2-0.4，一个Cell熵=0.9→检测为HotCell
- [ ] 单元测试：所有Cell熵均匀→不产生信号
- [ ] 统计计算正确

### TE10: 实现性能退化检测

**文件**：
- 修改：`crates/axiom-evolution/src/observer.rs`

**具体操作**：
1. `PerformanceAnalyzer::detect_degradation(latency_series: &[(ComponentId, Duration, Timestamp)]) -> Vec<ImprovementSignal>`
2. 计算各Cell P99延迟的滑动窗口对比（当前窗口vs前一个窗口）
3. P99延迟上升>50%→PerformanceDegradation信号

**验收标准**：
- [ ] 单元测试：延迟从10ms升至60ms→检测到退化
- [ ] 单元测试：延迟稳定→无信号

---

## 假设生成器

### TE11: 实现参数自调优提案模板（L1）

**文件**：
- 新建：`crates/axiom-evolution/src/hypothesis.rs`（部分）

**具体操作**：
1. 定义参数调优规则模板：
   - P99延迟 > threshold × 2 → 建议timeout增加到1.5×（上限max_timeout）
   - 错误率 > 10% → 建议circuit_breaker阈值收紧
   - Mailbox队列深度 > high_watermark → 建议增加mailbox_capacity
2. `ParamTuneGenerator::generate(signal: &ImprovementSignal, current_config: &SystemConfig) -> Option<EvolutionProposal>`
3. 每个提案包含回滚方案（恢复到当前参数值）

**验收标准**：
- [ ] 单元测试：PerformanceDegradation信号(P99=2×threshold)→生成timeout调整提案
- [ ] 单元测试：提案的rollback_plan包含原始参数值
- [ ] 参数不会超过安全上限（有max_timeout等硬约束）

### TE12: 实现Axiom提案生成（L2a）

**文件**：
- 修改：`crates/axiom-evolution/src/hypothesis.rs`

**具体操作**：
1. `AxiomProposalGenerator::generate(signal: &ImprovementSignal) -> Option<EvolutionProposal>`
2. 仅支持预定义模板类型：
   - RepeatedViolation(violation_type="empty_field") → 提案`RequiredFieldAxiom`
   - RepeatedViolation(violation_type="value_out_of_range") → 提案`ValueRangeAxiom`
   - 通用模式：`RepetitionGuardAxiom`（检测某Signal类型N分钟内重复次数上限）
3. L2a初期不做任意代码生成，只从预定义Axiom模板中选取

**验收标准**：
- [ ] 单元测试：RepeatedViolation(empty_field)信号→生成RequiredFieldAxiom提案
- [ ] 单元测试：提案包含rollback_plan（禁用新Axiom即可回滚）
- [ ] 提案必须通过M3/M4合规检查才能返回

### TE13: 实现合规检查器（M1/M3/M4/M6）

**文件**：
- 新建：`crates/axiom-evolution/src/compliance.rs`

**具体操作**：
1. `ComplianceChecker::check(proposal: &EvolutionProposal) -> ComplianceCheck`
2. 逐项检查：
   - M1: 检查提案是否引入第6原语（通过白名单：只能修改/添加属于5原语的东西）
   - M3: 检查提案是否删除/减弱现有门禁（简单文本匹配+规则检查：不允许删除check函数/Axiom/Guardian审查项）
   - M4: 检查提案是否新增CanSendTo方向（不允许修改CanSendTo impl）
   - M6: 检查提案是否包含rollback_plan
3. 每项检查返回pass/fail+原因

**验收标准**：
- [ ] 单元测试：提案新增CanSendTo方向→M4检查失败
- [ ] 单元测试：提案删除一个Axiom→M3检查失败
- [ ] 单元测试：提案无rollback_plan→M6检查失败
- [ ] 单元测试：合法提案（参数调优）→全部通过
- [ ] ComplianceCheck::all_passed()正确反映结果

---

## 沙盒验证

### TE14: 实现SandboxRuntime

**文件**：
- 新建：`crates/axiom-evolution/src/sandbox.rs`（部分）

**具体操作**：
1. `SandboxRuntime::from_snapshot(snapshot: SystemSnapshot) -> Result<Self>`从Snapshot创建隔离运行时
2. 沙盒runtime与生产runtime完全隔离：
   - 独立的Mailbox/Bus/Dispatcher
   - 不连接真实外部IO
   - Mock外部服务（LLM/数据库/HTTP）
3. 实现`apply_change(&mut self, change: &Change) -> Result<()>`应用变更
   - L1参数变更：直接修改config
   - L2a Axiom：注册新Axiom到沙盒
   - L2b Lens：注册新Lens到沙盒
   - L2c路由：更新沙盒路由表

**验收标准**：
- [ ] 从Snapshot创建沙盒后，沙盒初始状态与Snapshot一致
- [ ] 沙盒中的变更不影响任何全局状态
- [ ] 可以成功应用L1/L2a/L2b/L2c类型变更
- [ ] 单元测试：沙盒创建→应用参数变更→查询参数=新值→原runtime参数不变

### TE15: 实现确定性重放测试

**文件**：
- 修改：`crates/axiom-evolution/src/sandbox.rs`

**具体操作**：
1. `SandboxRuntime::replay(events: &[Event], duration: Duration) -> Result<ReplayResult>`
2. 从EventStore加载历史事件，按时间戳顺序在沙盒中重放
3. ReplayResult包含：
   - 是否有panic
   - Witness链完整性（重放后）
   - 处理的消息数/失败数
   - 最终entropy
   - 各Cell的延迟统计

**验收标准**：
- [ ] 重放100条历史事件→沙盒处理完成无panic
- [ ] 确定性：同样的events重放两次→ReplayResult完全相同
- [ ] 有panic的事件序列→replay返回Err

### TE16: 实现回滚验证

**文件**：
- 修改：`crates/axiom-evolution/src/rollback.rs`

**具体操作**：
1. `RollbackManager::verify_rollback(sandbox: &SandboxRuntime, plan: &RollbackPlan) -> Result<Duration>`
2. 在沙盒中：应用变更→执行rollback→验证恢复到原始状态
3. 测量回滚耗时，要求<30s
4. 返回实际回滚耗时

**验收标准**：
- [ ] 单元测试：应用参数变更→回滚→参数恢复原值
- [ ] 回滚耗时<30s（测试用小snapshot）
- [ ] 回滚后状态hash与原始snapshot hash一致

---

## 适应度评估

### TE17: 实现FitnessEvaluator

**文件**：
- 新建：`crates/axiom-evolution/src/fitness.rs`

**具体操作**：
1. `FitnessEvaluator::evaluate(baseline: &ReplayResult, after: &ReplayResult, metrics: &[FitnessMetric]) -> FitnessEvaluation`
2. 对比沙盒重放前后的指标：
   - entropy: 不升高
   - error_rate: 不升高超过5%
   - latency_p99: 不升高超过10%
   - axiom_catch_rate: 不降低超过5%
   - 至少一个目标指标改善>5%
3. `FitnessEvaluation`包含：passed(bool)、各指标delta、整体评价

**验收标准**：
- [ ] 单元测试：熵降低、错误率不变→passed=true
- [ ] 单元测试：延迟翻倍→passed=false
- [ ] 单元测试：所有指标无变化→passed=false（没有改善则不采纳）

---

## 金丝雀部署

### TE18: 实现影子流量复制

**文件**：
- 新建：`crates/axiom-evolution/src/canary.rs`（部分）

**具体操作**：
1. `CanaryDeployment::start(proposal: &EvolutionProposal, sandbox: &SandboxRuntime) -> Result<Self>`
2. 在Bus上挂载影子拦截器：
   - 复制所有经过Bus的SignalEnvelope到CanaryRuntime
   - CanaryRuntime处理影子消息，结果丢弃（不写回真实Mailbox）
   - 记录CanaryRuntime的输出用于对比
3. 实现`stop(self) -> CanaryResult`停止金丝雀并返回结果

**验收标准**：
- [ ] 金丝雀运行时不影响生产流量（消息投递延迟增加<1ms）
- [ ] 影子消息被CanaryRuntime处理但不影响生产状态
- [ ] 金丝雀期间产生的所有消息有trace标记可区分

### TE19: 实现金丝雀结果对比

**文件**：
- 修改：`crates/axiom-evolution/src/canary.rs`

**具体操作**：
1. `CanaryResult::evaluate(&self, baseline: &BaselineMetrics) -> CanaryEvaluation`
2. 对比指标（按spec 4.4节）：
   - 错误率 ≤ baseline×1.05
   - 延迟P99 ≤ baseline×1.1
   - 熵增量 ≤ 0
   - Axiom拦截率 ≥ baseline×0.95
   - 无panic/crash
3. 持续时间/消息数达到spec要求才认为样本足够

**验收标准**：
- [ ] 单元测试：canary错误率>baseline×1.05→失败
- [ ] 单元测试：所有指标正常→通过
- [ ] 样本不足（消息数不够）→返回InsufficientSample不做判断

---

## 采纳与回滚

### TE20: 实现原子采纳部署

**文件**：
- 新建：`crates/axiom-evolution/src/deployer.rs`

**具体操作**：
1. `Deployer::adopt(proposal: &EvolutionProposal, runtime: &mut Runtime) -> Result<AdoptResult>`
2. 原子采纳流程：
   - 创建采纳前Snapshot
   - 应用变更（与sandbox.apply_change逻辑复用）
   - 创建采纳后Snapshot
   - 更新注册表/路由表/配置
   - 产生EvolutionWitness(adopted)
   - 如果任一步骤失败→自动执行回滚
3. L2a Axiom采纳：注册Axiom到AxiomChain
4. L1参数采纳：热更新config（发送ConfigUpdated信号）
5. L2b Lens采纳：注册Lens到LensRegistry

**验收标准**：
- [ ] 单元测试：采纳参数变更→runtime.config更新为新值
- [ ] 单元测试：采纳过程中注入失败→自动回滚到原始状态
- [ ] 采纳产生EvolutionWitness(adopted)

### TE21: 实现RollbackManager自动回滚

**文件**：
- 修改：`crates/axiom-evolution/src/rollback.rs`

**具体操作**：
1. `RollbackManager::rollback(proposal_id: ProposalId, runtime: &mut Runtime) -> Result<()>`
2. 从回滚点Snapshot恢复
3. 执行反向Migration（如有数据变更）
4. 产生EvolutionWitness(auto_rolled_back/manual_rolled_back)
5. 实现24h延迟监控：采纳后启动后台task，定期检查指标，触发M6条件时自动回滚

**验收标准**：
- [ ] 单元测试：采纳→回滚→状态恢复
- [ ] 回滚产生EvolutionWitness
- [ ] 延迟监控：采纳后熵升高>20%→自动触发回滚

---

## 进化守门人

### TE22: 实现EvolutionGuardian

**文件**：
- 新建：`crates/axiom-evolution/src/evolution_guardian.rs`

**具体操作**：
1. `EvolutionGuardian`作为Oversight层的Cell运行
2. 在每个进化步骤前调用compliance_checker
3. 拦截所有违反元公理的操作，产生EvolutionWitness(rejected, meta-axiom-violation)
4. 实现速率限制检查（调用rate_limiter）
5. 监控进化引擎自身的健康状态（如果EvolutionCell崩溃→Supervisor重启）

**验收标准**：
- [ ] 尝试采纳一个M3违规的提案→Guardian拦截
- [ ] 超过速率限制的提案→排队或拒绝
- [ ] EvolutionGuardian自身panic→被Supervisor捕获（需要TE27集成）

---

## 主进化循环

### TE23: 实现EvolutionGovernor主循环

**文件**：
- 新建：`crates/axiom-evolution/src/lib.rs`（整合）

**具体操作**：
1. `EvolutionGovernor`结构体，持有：
   - observer/witness_analyzer/entropy_heatmap/performance_analyzer
   - hypothesis_generators (Vec<Box<dyn HypothesisGenerator>>)
   - compliance_checker
   - sandbox_factory
   - canary_manager
   - fitness_evaluator
   - deployer
   - rollback_manager
   - rate_limiter
   - evolution_guardian
   - proposal_queue: VecDeque<EvolutionProposal>
   - active_canary: Option<CanaryDeployment>
2. 实现`async fn run_tick(&mut self, runtime: &mut Runtime, store: &dyn EventStore)`：
   - 观察：收集ImprovementSignal
   - 假设：对每个信号生成提案→合规检查→速率检查→入队
   - 如果有active_canary：检查是否完成→评估→采纳/回滚
   - 如果没有active_canary且队列非空：取队首→沙盒创建→重放→fitness→通过则启动canary
3. 循环间隔：10s一次tick

**验收标准**：
- [ ] tick函数能正确走完整个流程
- [ ] 同一时间最多1个active_canary（M7）
- [ ] 所有步骤产生EvolutionWitness
- [ ] 集成测试（TE29）覆盖完整流程

---

## EvolutionCell集成

### TE24: EvolutionCell注册为Oversight层Cell

**文件**：
- 修改：`crates/axiom-evolution/src/lib.rs`
- 修改：`crates/axiom-oversight/src/lib.rs`

**具体操作**：
1. 实现`EvolutionCell`，包装EvolutionGovernor，impl OversightCell trait
2. EvolutionCell可以：
   - 读取Witness链（通过CellContext）
   - 读取系统熵值（通过OversightContext）
   - 发送治理信号（如ConfigUpdated/AxiomRegistered等）
   - 受Supervisor监督（panic自动重启）
3. 在axiom-oversight中注册EvolutionCell为标准监督项

**验收标准**：
- [ ] EvolutionCell impl Cell trait + OversightCell trait
- [ ] Cell panic时能被Supervisor重启
- [ ] 重启后从EvolutionWitness链恢复状态（已采纳的提案不丢失）

### TE25: 启动时元公理完整性验证

**文件**：
- 修改：`crates/axiom-evolution/src/lib.rs`

**具体操作**：
1. 在EvolutionGovernor::new()或start()中调用verify_meta_axioms_integrity()
2. 如果元公理hash不匹配→返回Err（启动失败）
3. 同时验证已有EvolutionWitness链完整性

**验收标准**：
- [ ] 正常启动→验证通过
- [ ] 篡改元公理文本→启动失败
- [ ] Witness链断裂→启动失败

---

## CLI命令

### TE26: 实现axm evolution子命令

**文件**：
- 修改：`crates/axiom-cli/src/main.rs`（或对应文件）
- 新建：`crates/axiom-cli/src/evolution.rs`

**具体操作**：
1. 实现以下CLI子命令：
   - `axm evolution status`：显示引擎状态（运行中/暂停/观察到的信号数/活跃提案/金丝雀）
   - `axm evolution proposals [--status pending|adopted|rejected]`：列提案
   - `axm evolution show <id>`：显示提案详情
   - `axm evolution log [--limit N]`：显示最近N条EvolutionWitness
   - `axm evolution rollback <id>`：手动回滚
   - `axm evolution pause`/`resume`：暂停/恢复自动进化
2. CLI通过runtime的health endpoint或Unix socket与EvolutionCell通信

**验收标准**：
- [ ] 所有命令可执行，输出清晰
- [ ] `axm evolution status`在runtime运行时返回有效状态
- [ ] `axm evolution rollback`执行回滚并确认
- [ ] `axm evolution pause`后引擎不再生成新提案

---

## 集成测试

### TE27: 端到端测试——参数自调优完整流程

**文件**：
- 新建：`crates/axiom-evolution/tests/l1_param_tuning.rs`

**具体操作**：
1. 构造测试runtime：一个CommandExecutorCell，timeout=1s
2. 注入性能退化：让Executor处理时间=2s，触发持续timeout
3. 运行evolution governor足够tick数
4. 验证完整流程：
   - Observer检测到PerformanceDegradation信号
   - HypothesisGenerator生成timeout调整提案
   - Compliance检查通过（L1参数调优不违反M1-M7）
   - Sandbox重放验证通过
   - Canary影子流量验证通过
   - 采纳后timeout=1.5s
   - 产生EvolutionWitness(observed→hypothesized→sandbox_passed→canary_passed→adopted)
5. 采纳后性能恢复（不再timeout）

**验收标准**：
- [ ] 完整流程自动走完，无人工干预
- [ ] 所有EvolutionWitness链完整、hash正确
- [ ] 最终timeout参数为1.5s（不是任意值）

### TE28: 端到端测试——元公理拦截违规提案

**文件**：
- 新建：`crates/axiom-evolution/tests/meta_axiom_guard.rs`

**具体操作**：
1. 测试M3保护：构造一个提案试图删除一个Axiom（或等价地减弱门禁）
2. ComplianceChecker在hypothesis阶段拒绝
3. 产生EvolutionWitness(rejected, reason="M3-violation")
4. 测试M4保护：构造一个新增CanSendTo方向的提案→拒绝
5. 测试M7速率限制：在短时间内注入5个L1采纳→第6个被限速

**验收标准**：
- [ ] M3违规提案被拦截，不进入sandbox阶段
- [ ] M4违规提案被拦截
- [ ] M7速率限制生效
- [ ] 所有拒绝都有Witness记录

### TE29: 端到端测试——自动回滚

**文件**：
- 新建：`crates/axiom-evolution/tests/auto_rollback.rs`

**具体操作**：
1. 沙盒阶段构造一个通过沙盒但在金丝雀/采纳后导致问题的提案
   - 方法：sandbox重放用历史数据通过，但采纳后注入新的恶化模式
2. 采纳后24h监控（测试中用虚拟时间加速）发现熵升高>20%
3. RollbackManager自动触发回滚
4. 验证状态恢复到采纳前
5. 产生EvolutionWitness(adopted→auto_rolled_back)

**验收标准**：
- [ ] 自动回滚触发（不需要人工）
- [ ] 回滚后状态正确
- [ ] 回滚有Witness记录
- [ ] 回滚后该提案类型进入冷却期

### TE30: 单元测试与覆盖率

**文件**：所有模块

**具体操作**：
1. 确保所有public函数有单元测试
2. 核心模块（compliance/rollback/fitness/rate_limiter/witness）覆盖率≥80%
3. 运行`cargo tarpaulin`或`cargo llvm-cov`验证

**验收标准**：
- [ ] `cargo test -p axiom-evolution` 全部通过
- [ ] 核心模块覆盖率≥80%

---

## 最终验证

### TE31: 集成到workspace并通过全局检查

**文件**：workspace根

**具体操作**：
1. `cargo build --workspace` 零警告
2. `cargo fmt --all -- --check`
3. `cargo clippy --workspace -- -D warnings` 零警告
4. `cargo test --workspace` 全部通过
5. `axm check`（如果P0.5已完成则用此命令，否则用手动步骤）
6. `cargo doc --no-deps --workspace` 无警告

**验收标准**：
- [ ] 全部6项检查通过
- [ ] 无新增unsafe代码
- [ ] 无新增未审计的第三方依赖

### TE32: axiom-evolution性能基准

**文件**：
- 新建：`crates/axiom-evolution/benches/evolution_bench.rs`

**具体操作**：
1. 基准测试：
   - Observe阶段分析10000条Witness<100ms
   - Compliance检查1000个提案<10ms
   - Fitness评估<1ms
2. 确认进化引擎tick不阻塞runtime主循环

**验收标准**：
- [ ] 基准测试性能达标
- [ ] EvolutionGovernor.tick()在正常负载下<500ms

### TE33: 提交

**具体操作**：
1. `git add`所有文件
2. commit message: `feat(evolution): implement auto-evolution engine with M1-M7 meta-axioms (P4.5)`
3. 如果有remote则push

**验收标准**：
- [ ] 所有文件已commit
- [ ] commit message符合规范

---

## Phase 4.5 整体验收清单

P4.5完成时，必须满足：

- [ ] **参数自调优工作**：注入性能退化→引擎自动检测→提案→沙盒→金丝雀→采纳→性能恢复（TE27）
- [ ] **Axiom自动提案**：注入重复违规→引擎提案新Axiom→采纳后该违规被拦截（L2a流程）
- [ ] **元公理M3保护**：减弱门禁的提案→被拒绝不进沙盒（TE28）
- [ ] **元公理M4保护**：新增非法方向→拒绝（TE28）
- [ ] **元公理M7限速**：超出速率→排队/拒绝（TE28）
- [ ] **自动回滚**：采纳后恶化→自动回滚恢复（TE29）
- [ ] **EvolutionWitness链完整**：每次操作有Witness，hash链验证通过
- [ ] **沙盒隔离**：沙盒变更不影响生产（TE14）
- [ ] **EvolutionCell受监督**：panic被Supervisor重启（TE24）
- [ ] **CLI命令可用**：status/proposals/log/rollback/pause/resume功能正常（TE26）
- [ ] **元公理启动校验**：篡改元公理→启动失败（TE25）
- [ ] **全局检查通过**：编译/fmt/clippy/test/doc全部通过（TE31）
