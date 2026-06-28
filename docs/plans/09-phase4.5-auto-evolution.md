# Phase 4.5: 自动进化引擎 Implementation Plan

> **Goal:** 实现 axiom-evolution crate：元公理M1-M7编译期锁定 + Observe→Hypothesize→Sandbox→Canary→Adopt→Monitor六步进化闭环 + EvolutionWitness审计链 + axm evolution CLI。验收标准：系统能自动检测改进机会（高频违规/性能退化/热点Cell）、沙盒验证、金丝雀部署、自动回滚；所有进化操作受7条元公理约束，产生不可篡改的EvolutionWitness。

> **Spec参考:** [04-auto-evolution.md](../architecture/04-auto-evolution.md) 包含完整设计，本任务书是其实现分解。

> **Baseline:** 无 axiom-evolution crate（需要新建）。P4已实现Runtime/EventStore/Oversight基础设施，进化引擎作为Layer E的Cell运行在Oversight层之上。

---

## Global Constraints

- axiom-evolution 依赖 axiom-core、axiom-runtime、axiom-store、axiom-oversight
- 7条元公理编译期硬编码，不可被进化引擎自身修改
- 进化引擎自身是Cell（EvolutionGovernorCell），受所有Cell规则约束
- 每次进化操作必须产生EvolutionWitness
- 沙盒环境与生产环境完全隔离
- 金丝雀只复制影子流量，不写回生产
- 采纳必须原子性（全成功或全回滚）
- 同时在线金丝雀数≤1
- L1/L2/L3/L4各级进化有严格速率限制（M7）
- cargo build/clippy/test 零警告

---

## Task 1: 创建 axiom-evolution crate 骨架和元公理

**Files:**
- Create: `crates/axiom-evolution/Cargo.toml`
- Create: `crates/axiom-evolution/src/lib.rs`
- Create: `crates/axiom-evolution/src/meta_axioms.rs`

- [ ] **Step 1: 创建 Cargo.toml**
  - package name = "axiom-evolution", version = "0.1.0", edition = "2021"
  - dependencies: axiom-core (path), axiom-runtime (path), axiom-store (path), axiom-oversight (path), sha2, serde, serde_json, tokio (rt, sync, time, macros), tracing, thiserror, chrono (可选, 用于时间处理)
  - dev-dependencies: tokio/test-util, tempfile

- [ ] **Step 2: 实现元公理 M1-M7 编译期定义**
  - meta_axioms.rs 中：
  - `META_AXIOMS_TEXT: &str` — 7条元公理的完整文本
  - `META_AXIOMS_HASH: [u8; 32]` — const 计算的SHA-256 hash（使用const-sha1或build.rs计算，或运行时启动时计算并与硬编码值比较）
  - `fn verify_meta_axioms_integrity() -> Result<(), EvolutionError>` — 运行时验证hash与硬编码一致

- [ ] **Step 3: 实现元公理合规检查函数**
  ```rust
  pub fn check_m1_five_primitives(proposal: &EvolutionProposal) -> ComplianceResult;
  pub fn check_m2_meta_axioms_immutable(proposal: &EvolutionProposal) -> ComplianceResult;
  pub fn check_m3_gates_not_weakened(proposal: &EvolutionProposal) -> ComplianceResult;
  pub fn check_m4_direction_iron_law(proposal: &EvolutionProposal) -> ComplianceResult;
  pub fn check_m5_witness_chain_unbroken(proposal: &EvolutionProposal) -> ComplianceResult;
  pub fn check_m6_rollback_available(proposal: &EvolutionProposal) -> ComplianceResult;
  pub fn check_m7_rate_limits(proposal: &EvolutionProposal, state: &RateLimitState) -> ComplianceResult;
  ```
  - 每个检查函数返回 ComplianceResult { compliant: bool, violations: Vec<String> }
  - 初期（P4.5初版）M1/M2/M3/M4/M7做字符串级别和结构级别检查；M5/M6在沙盒阶段验证

- [ ] **Step 4: 定义 EvolutionError**
  - 变体：MetaAxiomViolation(String), SandboxFailed(String), CanaryFailed(String), RollbackFailed(String), RateLimitExceeded, InvalidProposal(String)

- [ ] **Step 5: 创建 lib.rs 模块声明和基本导出**

- [ ] **Step 6: 测试**
  - 测试元公理hash验证通过
  - 测试各个check函数对合规proposal返回compliant=true
  - 测试明显违反M2/M3/M4的proposal被检测

- [ ] **Step 7: Commit**
  - `feat(axiom-evolution): create crate skeleton, M1-M7 meta axioms with compile-time integrity hash, compliance checker functions`

---

## Task 2: 实现 EvolutionProposal 数据结构

**Files:**
- Create: `crates/axiom-evolution/src/proposal.rs`

- [ ] **Step 1: 定义 ProposalId 强类型ID**
  - 使用 UUID v4

- [ ] **Step 2: 定义 ProposalType 枚举**
  ```rust
  pub enum ProposalType {
      ParamTuning,       // L1: 参数自调优
      AxiomAddition,     // L2a: 新增Axiom
      LensAddition,      // L2b: 新增Lens
      RouteOptimization, // L2c: 路由优化
      CellGeneration,    // L3: 生成新Cell（仅预定义模板）
      ArchitectureChange, // L4: 架构调整（初期不实现）
  }
  ```

- [ ] **Step 3: 定义 EvolutionLayer 级别常量**
  - L1_PARAM, L2a_AXIOM, L2b_LENS, L2c_ROUTE, L3_CELL, L4_ARCH

- [ ] **Step 4: 定义 Change 枚举**
  ```rust
  pub enum Change {
      ParamChange { key: String, old_value: Value, new_value: Value },
      AddAxiom { axiom_def: AxiomDefinition },
      AddLens { lens_def: LensDefinition },
      RouteChange { from: String, to: String, operation: RouteOp },
      AddCell { cell_template: CellTemplate, config: Value },
  }
  ```

- [ ] **Step 5: 定义 RollbackPlan**
  ```rust
  pub struct RollbackPlan {
      pub before_snapshot_id: String,
      pub reverse_migration: Option<ReverseMigration>,
      pub estimated_rollback_ms: u64,
  }
  ```

- [ ] **Step 6: 定义 FitnessDelta 和 FitnessMetric**
  ```rust
  pub struct FitnessDelta {
      pub entropy_delta: f64,
      pub error_rate_delta: f64,
      pub latency_p99_delta_ms: f64,
  }
  pub enum FitnessMetric {
      EntropyReduction,
      ErrorRateReduction { target_cell: String },
      LatencyImprovement { target_cell: String, percentile: u8 },
      ViolationElimination { axiom_name: String },
  }
  ```

- [ ] **Step 7: 定义 EvolutionProposal 完整结构体**
  - id: ProposalId
  - proposal_type: ProposalType
  - layer: EvolutionLayer
  - target_components: Vec<ComponentId>
  - changes: Vec<Change>
  - rollback_plan: RollbackPlan
  - expected_fitness: FitnessDelta
  - fitness_metrics: Vec<FitnessMetric>
  - created_at: u64 (timestamp_ns)
  - meta_axiom_compliance: ComplianceCheck

- [ ] **Step 8: 实现 ComplianceCheck**
  - m1_compliant: bool, m2_compliant: bool, ..., m7_compliant: bool
  - violations: Vec<String>
  - `fn is_fully_compliant(&self) -> bool`

- [ ] **Step 9: 测试**
  - 测试Proposal序列化/反序列化
  - 测试ComplianceCheck全true时is_fully_compliant返回true
  - 测试各种Change类型构造

- [ ] **Step 10: Commit**
  - `feat(axiom-evolution): EvolutionProposal with all change types, rollback plans, fitness metrics, compliance check results`

---

## Task 3: 实现 EvolutionWitness 审计链

**Files:**
- Create: `crates/axiom-evolution/src/witness.rs`

- [ ] **Step 1: 定义 EvolutionWitness**
  ```rust
  pub struct EvolutionWitness {
      pub witness_id: WitnessId,
      pub proposal_id: ProposalId,
      pub step: EvolutionStep,  // Observed/Hypothesized/SandboxPassed/SandboxFailed/CanaryPassed/CanaryFailed/Adopted/AutoRolledBack/ManuallyRolledBack/Rejected
      pub timestamp_ns: u64,
      pub summary: String,
      pub metrics: Option<FitnessDelta>,
      pub reason: Option<String>,
      pub hash: WitnessHash,
      pub prev_hash: Option<WitnessHash>,
  }
  pub enum EvolutionStep {
      Observed, Hypothesized, SandboxPassed, SandboxFailed,
      CanaryPassed, CanaryFailed, Adopted, AutoRolledBack,
      ManuallyRolledBack, Rejected, Paused, Resumed,
  }
  ```

- [ ] **Step 2: 实现 EvolutionWitnessBuilder**
  - 自动计算SHA-256 hash（链接prev_hash）
  - 自动设置timestamp_ns
  - 与axiom-core的Witness共享hash算法

- [ ] **Step 3: 实现 EvolutionWitness 链写入EventStore**
  - 每次进化步骤产生一个EvolutionWitness
  - 作为特殊event_type "_evolution.witness"写入EventStore
  - 提供 `async fn append_evolution_witness(store: &dyn EventStore, witness: EvolutionWitness) -> Result<()>`

- [ ] **Step 4: 实现 verify_evolution_chain**
  - 从EventStore读取所有EvolutionWitness
  - 验证hash链完整性
  - 验证每个proposal的生命周期是合法的状态转换

- [ ] **Step 5: 测试**
  - 测试EvolutionWitness hash链接
  - 测试写入EventStore和读取验证
  - 测试链篡改检测

- [ ] **Step 6: Commit**
  - `feat(axiom-evolution): EvolutionWitness with hash-chained audit log, writes to EventStore, chain integrity verification`

---

## Task 4: 实现 Observer（观察引擎）

**Files:**
- Create: `crates/axiom-evolution/src/observer.rs`

- [ ] **Step 1: 定义 ImprovementSignal**
  ```rust
  pub struct ImprovementSignal {
      pub signal_type: ImprovementType,
      pub evidence: serde_json::Value,
      pub affected_components: Vec<String>,
      pub confidence: f64,
      pub detected_at: u64,
  }
  pub enum ImprovementType {
      HotCell,
      RepeatedViolation { axiom_name: String },
      PerformanceDegradation { cell_id: String, metric: String, ratio: f64 },
      MissingAxiom { violation_pattern: String },
      HighEntropy { subsystem: String },
      RepeatedFailure { cell_id: String, error_pattern: String },
      MailboxBackpressure { cell_id: String },
  }
  ```

- [ ] **Step 2: 实现 ObserverCell**
  - 运行定时采样循环（每10秒）
  - 数据源：
    1. EntropyGovernor 的熵值热力图（每10秒）
    2. Witness链分析（每100条Witness或每5分钟）
    3. Runtime性能指标（延迟/队列深度）
    4. ArchitectureGuardian违规统计

- [ ] **Step 3: 实现分析规则**
  - HotCell检测：cell熵值 > 全局平均 * 2
  - RepeatedViolation检测：同类型violation在5分钟内出现 > 10次
  - PerformanceDegradation检测：P99延迟 > 历史基线 * 2 持续2个采样周期
  - HighEntropy检测：全局熵值 > YELLOW阈值
  - RepeatedFailure检测：同cell在5分钟内panic > 3次
  - MailboxBackpressure检测：mailbox深度持续 > 容量80%

- [ ] **Step 4: 输出 ImprovementSignal**
  - 产生的ImprovementSignal发送到HypothesisGenerator（通过内部channel）
  - 每个signal产生EvolutionWitness(Observed)

- [ ] **Step 5: 去重和冷却**
  - 同一(component, signal_type)的信号在冷却期内（1小时）不重复产生
  - 防止风暴

- [ ] **Step 6: 测试**
  - 测试各规则正确触发ImprovementSignal
  - 测试冷却期去重
  - 使用模拟数据测试检测逻辑

- [ ] **Step 7: Commit**
  - `feat(axiom-evolution): ObserverCell with entropy/witness/performance analysis, improvement signal detection, deduplication/cooldown`

---

## Task 5: 实现 HypothesisGenerator（假设生成）

**Files:**
- Create: `crates/axiom-evolution/src/hypothesis.rs`

- [ ] **Step 1: 实现规则模板库**
  - 对每种ImprovementType定义对应的Proposal生成模板：
    - HotCell + PerformanceDegradation → ParamTuning（增加timeout/capacity）
    - RepeatedViolation + MissingAxiom → AxiomAddition（生成新Axiom约束）
    - HighEntropy → RouteOptimization（调整路由）
    - RepeatedFailure → ParamTuning（调整circuit breaker阈值）
    - MailboxBackpressure → ParamTuning（增加mailbox容量或添加batcher cell——L3初版不支持，只做L1）

- [ ] **Step 2: 实现参数自调优模板（L1）**
  - timeout调整：如果P99 > timeout*0.8，建议timeout *= 1.5
  - mailbox容量调整：如果深度持续 > 80%，建议capacity *= 2
  - circuit breaker阈值：如果频繁熔断，建议错误阈值*=2或timeout*=0.8
  - 所有参数调整有上下限（不能无限增大）

- [ ] **Step 3: 实现历史类比**
  - 搜索历史EvolutionWitness中相同ImprovementType的提案
  - 查看哪些提案Adopted后有正效果（fitness改善）
  - 优先使用历史成功的提案模板

- [ ] **Step 4: 实现元公理预检查**
  - 生成Proposal后，立即运行M1-M7 compliance检查
  - 不通过的提案直接Rejected，不进入Sandbox
  - 每个拒绝产生EvolutionWitness(Rejected, reason="meta-axiom-violation: Mx")

- [ ] **Step 5: 实现速率限制（M7）**
  - 跟踪各级别的提案数量（L1每小时≤5，L2每天≤3，L3每周≤1，L4每月≤1）
  - 超限时排队等待

- [ ] **Step 6: 测试**
  - 测试每种ImprovementSignal生成对应类型的Proposal
  - 测试参数调整在上下限内
  - 测试元公理不合规Proposal被拒绝
  - 测试速率限制排队

- [ ] **Step 7: Commit**
  - `feat(axiom-evolution): HypothesisGenerator with rule templates, parameter tuning, history analogy, M1-M7 pre-check, M7 rate limiting`

---

## Task 6: 实现 SandboxTester（沙盒验证）

**Files:**
- Create: `crates/axiom-evolution/src/sandbox.rs`

- [ ] **Step 1: 定义 SandboxRuntime**
  - 从生产环境创建隔离的Runtime实例
  - 不从EventStore加载全量数据，只加载最近1小时（可配置）的Event作为测试输入
  - 沙盒有独立的MemoryStore（不写回生产EventStore）

- [ ] **Step 2: 实现 apply_proposal 方法**
  - 将EvolutionProposal的changes应用到SandboxRuntime：
    - ParamChange: 修改Cell的配置参数
    - AddAxiom: 将新Axiom注册到沙盒Runtime的AxiomChain
    - AddLens: 注册新Lens
    - RouteChange: 修改路由表
    - AddCell: 注册新Cell实例（从模板创建）

- [ ] **Step 3: 实现确定性重放测试**
  - 从生产EventStore读取最近1小时的Event
  - 确定性重放到SandboxRuntime
  - 验证：
    1. 无panic/crash
    2. Witness链完整性（M5检查）
    3. 无AxiomViolation（或violation不增加）
    4. 所有消息在timeout内处理
  - 结果对比：
    - 熵值是否降低（fitness.entropy_delta < 0）
    - 目标指标是否改善
    - 是否产生新的violation类型

- [ ] **Step 4: 实现回滚验证**
  - 应用proposal后，执行RollbackPlan
  - 验证30秒内恢复到before_snapshot状态
  - 回滚后状态hash与snapshot一致

- [ ] **Step 5: 沙盒通过标准**
  - 无编译错误（L3/L4涉及代码变更时）
  - 所有现有测试通过（单元+集成）
  - 重放1小时数据无panic
  - Witness链完整
  - 至少一个fitness指标改善 >5%
  - 无关键指标恶化 >5%
  - 回滚验证成功

- [ ] **Step 6: 测试**
  - 测试ParamTuning提案沙盒验证通过
  - 测试导致panic的提案被沙盒拒绝
  - 测试回滚验证成功/失败
  - 测试确定性重放

- [ ] **Step 7: Commit**
  - `feat(axiom-evolution): SandboxRuntime with deterministic replay, proposal application, fitness evaluation, rollback verification`

---

## Task 7: 实现 CanaryDeployer（金丝雀验证）

**Files:**
- Create: `crates/axiom-evolution/src/canary.rs`

- [ ] **Step 1: 定义 CanarySession**
  ```rust
  pub struct CanarySession {
      pub proposal_id: ProposalId,
      pub started_at: u64,
      pub duration_secs: u64,
      pub min_messages: u64,
      pub messages_processed: AtomicU64,
      pub errors_old: AtomicU64,
      pub errors_new: AtomicU64,
      pub latency_sum_old: AtomicU64,
      pub latency_sum_new: AtomicU64,
      pub state: CanaryState,
  }
  pub enum CanaryState { Running, Passed, Failed, Stopped }
  ```

- [ ] **Step 2: 实现影子流量复制**
  - 金丝雀部署后，MessageBus 复制目标Cell的消息到CanaryCell
  - CanaryCell应用proposal变更处理消息
  - 处理结果不写回系统（不产生outgoing消息，不写Witness到生产Store）
  - 对比新旧版本的：
    - 成功/失败率
    - 延迟分布
    - Axiom拦截率（新版本只能更严格，不能宽松）
    - 是否panic

- [ ] **Step 3: 实现持续时间和消息量要求**
  - L1参数变更：10分钟，≥1000条消息
  - L2规则/Lens变更：30分钟，≥5000条消息
  - L3新Cell：2小时，≥10000条消息
  - L4架构调整：24小时，≥100000条消息（P4.5初版不支持L4）

- [ ] **Step 4: 实现金丝雀评估**
  - 通过标准：
    - 错误率：新版本 ≤ 旧版本 × 1.05
    - P99延迟：新版本 ≤ 旧版本 × 1.1
    - 熵增量：新版本 ≤ 旧版本
    - Axiom拦截率：新版本 ≥ 旧版本 × 0.95
    - 无panic/crash
  - 满足所有条件 → Passed
  - 任何条件不满足 → Failed + 停止金丝雀

- [ ] **Step 5: 测试**
  - 测试影子流量复制不影响生产
  - 测试通过/失败条件判断
  - 测试金丝雀超时时自动停止
  - 测试同时只有一个金丝雀运行（M7）

- [ ] **Step 6: Commit**
  - `feat(axiom-evolution): CanaryDeployer with shadow traffic, comparative metrics (error rate/latency/entropy), duration thresholds, pass/fail criteria`

---

## Task 8: 实现 Deployer（采纳部署）和 RollbackManager

**Files:**
- Create: `crates/axiom-evolution/src/deployer.rs`
- Create: `crates/axiom-evolution/src/rollback.rs`

- [ ] **Step 1: 实现原子部署（Deployer）**
  - 采纳流程：
    1. 暂停金丝雀
    2. 创建before_snapshot（如果RollbackPlan中没有）
    3. 原子应用变更：
       - ParamChange: 更新Cell配置，发送ConfigUpdated信号
       - AddAxiom: 注册到AxiomChain
       - AddLens: 注册到LensRegistry
       - RouteChange: 更新RoutingTable
       - AddCell: 正式注册Cell到Runtime
    4. 创建after_snapshot
    5. 更新注册表/路由表
    6. 产生EvolutionWitness(Adopted)
  - 如果任何步骤失败，自动执行回滚

- [ ] **Step 2: 实现延迟监控（RollbackManager）**
  - 采纳后24小时持续监控
  - 监控指标：
    - 全局熵值（比采纳前上升>20% → 自动回滚）
    - 目标组件错误率（上升>50% → 自动回滚）
    - P99延迟（翻倍 → 自动回滚）
    - 新crash/panic（采纳前不存在的模式 → 自动回滚）
    - 门禁被绕过（应拦截但未拦截 → 自动回滚）

- [ ] **Step 3: 实现自动回滚执行**
  - 触发条件满足时：
    1. 立即执行RollbackPlan（从before_snapshot恢复）
    2. 产生EvolutionWitness(AutoRolledBack, reason)
    3. 该proposal类型进入7天冷却期
    4. 通知Oversight层分析失败原因

- [ ] **Step 4: 测试**
  - 测试原子部署成功和失败回滚
  - 测试延迟监控触发自动回滚
  - 测试回滚后系统恢复正常

- [ ] **Step 5: Commit**
  - `feat(axiom-evolution): Deployer with atomic adoption, RollbackManager with 24h delayed monitoring and auto-rollback triggers`

---

## Task 9: 实现 EvolutionGovernorCell 和六步闭环协调

**Files:**
- Create: `crates/axiom-evolution/src/evolution_guardian.rs`
- Modify: `crates/axiom-evolution/src/lib.rs`

- [ ] **Step 1: 定义 EvolutionGovernorCell**
  - 作为Orchestrator，协调Observer→Hypothesis→Sandbox→Canary→Deploy→Monitor
  - 持有：
    - observer: Sender<ImprovementSignal>
    - hypothesis: Receiver<EvolutionProposal>
    - sandbox: SandboxTester
    - canary: CanaryDeployer
    - deployer: Deployer
    - rollback: RollbackManager
    - rate_limiter: RateLimiter
    - active_canary: Option<ProposalId>（同时只有一个）
    - state: Mutex<GovernorState>

- [ ] **Step 2: 实现主循环**
  - 监听来自Observer的ImprovementSignal
  - 发送给HypothesisGenerator生成Proposal
  - 对合规Proposal执行Sandbox
  - Sandbox通过后启动Canary
  - Canary通过后执行Deploy
  - 启动24h Monitor
  - 每个阶段产生对应EvolutionWitness

- [ ] **Step 3: 实现 pause/resume**
  - 支持暂停自动进化（停止生成新Proposal，但继续监控已采纳变更）
  - 恢复时从头开始循环

- [ ] **Step 4: 集成到Runtime启动**
  - RuntimeBuilder 添加 `with_auto_evolution(config: EvolutionConfig)` 方法
  - 自动注册EvolutionGovernorCell到Oversight层
  - 启动时验证元公理hash（失败则abort）

- [ ] **Step 5: 测试**
  - 测试端到端闭环（使用模拟信号触发ParamTuning提案，沙盒通过→金丝雀通过→采纳）
  - 测试沙盒失败不进入金丝雀
  - 测试金丝雀失败不采纳
  - 测试暂停/恢复

- [ ] **Step 6: Commit**
  - `feat(axiom-evolution): EvolutionGovernorCell orchestrating Observe→Hypothesize→Sandbox→Canary→Adopt→Monitor loop, pause/resume, runtime integration`

---

## Task 10: 实现 axm evolution CLI 命令

**Files:**
- Modify: `crates/axiom-cli/src/commands/` (新增 evolution 子命令模块)
- Modify: `crates/axiom-cli/src/lib.rs`

- [ ] **Step 1: 实现 `axm evolution status`**
  - 显示进化引擎状态：运行中/暂停、当前观察信号数、运行中的沙盒/金丝雀
  - 显示最近N条进化活动

- [ ] **Step 2: 实现 `axm evolution proposals`**
  - 列出待处理/历史提案（id/type/status/target_components/created_at）
  - 支持 --status pending/adopted/rejected/failed 过滤

- [ ] **Step 3: 实现 `axm evolution show <id>`**
  - 显示提案详情：变更内容、预期fitness、当前状态、验证结果
  - 显示该提案的EvolutionWitness时间线

- [ ] **Step 4: 实现 `axm evolution log [--limit N]`**
  - 显示最近N条EvolutionWitness
  - 包含step/summary/reason

- [ ] **Step 5: 实现 `axm evolution diff <id>`**
  - 显示提案的具体变更diff（参数old→new值等）

- [ ] **Step 6: 实现 `axm evolution rollback <id>`**
  - 手动触发回滚到某个采纳前状态
  - 需要二次确认

- [ ] **Step 7: 实现 `axm evolution pause/resume`**
  - 暂停/恢复自动进化

- [ ] **Step 8: (高级) `axm evolution approve <id>`**
  - 人工批准提案（跳过沙盒和金丝雀，仅限紧急情况）
  - 需要强制确认和详细警告

- [ ] **Step 9: 测试**
  - 所有命令可以正常运行（连接到running runtime或读取EventStore）
  - 输出格式清晰

- [ ] **Step 10: Commit**
  - `feat(axiom-cli): axm evolution subcommands - status/proposals/show/log/diff/rollback/pause/resume/approve`

---

## Task 11: 进化引擎速率限制和 EvolutionGuardian

**Files:**
- Create: `crates/axiom-evolution/src/rate_limiter.rs`
- Create: `crates/axiom-evolution/src/evolution_guardian.rs`

- [ ] **Step 1: 实现 RateLimiter**
  - 跟踪各级别的提案在时间窗口内的数量
  - 提供 `fn try_acquire(layer: EvolutionLayer) -> Result<(), RateLimitExceeded>`
  - 滑动窗口计数

- [ ] **Step 2: 实现 EvolutionGuardian（进化守门人）**
  - 检查每个提案是否满足M1-M7
  - 特别检查：
    - M3: 提案的代码diff中没有删除checks/模块中的检查逻辑
    - M4: 没有新增CanSendTo impl
  - 作为提案进入Sandbox前的最后一道关卡

- [ ] **Step 3: 测试**
  - 测试速率限制各级别阈值正确
  - 测试EvolutionGuardian拦截违反M3/M4的提案

- [ ] **Step 4: Commit**
  - `feat(axiom-evolution): RateLimiter with sliding window per evolution level, EvolutionGuardian final compliance gate`

---

## P4.5 阶段验收标准

| # | 验收项 | 验证方式 |
|---|--------|---------|
| 1 | cargo build -p axiom-evolution 零警告 | 命令行验证 |
| 2 | cargo test -p axiom-evolution 全部通过（≥40个测试） | 命令行验证 |
| 3 | M1-M7元公理编译期hash校验，启动时验证 | 单元测试 |
| 4 | Observer检测性能退化/热点Cell/重复违规 | 集成测试 |
| 5 | HypothesisGenerator生成L1参数调整提案 | 集成测试 |
| 6 | 元公理不合规提案被compliance拒绝 | 单元测试 |
| 7 | SandboxRuntime确定性重放+fitness评估 | 集成测试 |
| 8 | 金丝雀影子流量对比+通过/失败判断 | 集成测试 |
| 9 | 原子采纳+24h监控+自动回滚 | 集成测试 |
| 10 | EvolutionWitness链完整，可验证 | 单元测试 |
| 11 | 参数自调优端到端工作（注入退化→检测→沙盒→采纳→恢复） | 端到端测试 |
| 12 | M3门禁保护：尝试删除check项的提案被拒绝 | 单元测试 |
| 13 | M7速率限制：并发金丝雀≤1，级别频率受限 | 单元测试 |
| 14 | axm evolution CLI命令可用 | 手动测试 |
| 15 | 进化引擎本身是Cell，受Supervisor保护 | 集成测试 |
| 16 | cargo clippy/test/fmt全部通过 | axm check |
