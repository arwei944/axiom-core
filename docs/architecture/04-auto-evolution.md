# Axiom Core 自动进化体系设计

> **目标**：系统在运行中自动观察、假设、验证、采纳改进，实现架构级自我迭代——进化本身受不可变元公理约束。
>
> Spec参考：[03-automation-gates.md](./03-automation-gates.md)

---

## 一、核心问题与设计哲学

### 1.1 问题

传统系统的进化有两个极端：
- **完全手动**：人观察问题→人改代码→人测试→人部署——慢、人会犯错、无法24小时响应
- **无约束自动**：系统可以自我修改但没有安全边界——必然产生"架构癌变"，约束被进化掉，最终崩溃

Axiom Core需要第三条路：**自由进化 + 不可变元公理 + 多级安全验证 + 可回滚**。

### 1.2 生物进化类比

| 生物机制 | Axiom Core对应 |
|---------|---------------|
| DNA可以变异 | 代码/规则/参数/架构可以进化 |
| DNA复制机制极稳定 | 元公理层不可变 |
| 免疫系统清除癌变 | EvolutionGuardian拦截有害变更 |
| 适者生存 | 沙盒测试→金丝雀验证→适应度淘汰 |
| 有性生殖/基因重组 | 多个Hypothesis组合生成新提案 |
| 化石记录 | EvolutionWitness链，每次进化留痕 |
| 种群隔离（物种形成） | 沙盒环境隔离运行 |

### 1.3 核心原则

1. **元公理锁定**：7条Meta-Axioms写死在编译期，进化引擎自己不能修改
2. **进化可观测**：每次进化操作（提案/测试/采纳/回滚）产生不可篡改的EvolutionWitness
3. **进化可逆**：每次采纳必须有回滚Snapshot，延迟回滚保护（24h内恶化自动回滚）
4. **先验证后上线**：沙盒重放→金丝雀影子流量→正式采纳，三级验证
5. **小步快跑**：每次进化变更量受限（速率限制M7），不允许大爆炸式重构
6. **适应度可度量**：每次进化必须有量化指标（熵降/错误率降/延迟改善），不是"感觉更好"

---

## 二、架构分层（扩展）

在原四层+三层门禁基础上，新增Layer M和Layer E：

```
┌───────────────────────────────────────────────────────────┐
│  Layer M: 元公理层 (Meta-Axioms)                           │
│  ⚠️ 不可进化！编译期常量 + 启动时hash校验                   │
│  7条元公理定义进化的规则和边界                              │
├───────────────────────────────────────────────────────────┤
│  Layer E: 进化引擎层 (Evolution Engine)                    │
│  EvolutionGovernor                                       │
│  ├─ Observer: 从Witness链/熵值/性能数据中检测改进机会       │
│  ├─ HypothesisGenerator: 生成EvolutionProposal            │
│  ├─ SandboxTester: 沙盒重放验证                           │
│  ├─ CanaryDeployer: 金丝雀影子流量验证                    │
│  ├─ FitnessEvaluator: 量化评估进化效果                    │
│  ├─ RollbackManager: 回滚管理+延迟保护                    │
│  └─ EvolutionWitness: 进化审计链                          │
├───────────────────────────────────────────────────────────┤
│  Layer 0: 监督层 (Oversight) ← 可被进化（受M约束）         │
│  ArchitectureGuardian / EntropyGovernor / Supervisor     │
├───────────────────────────────────────────────────────────┤
│  Layer 3: Agent / Layer 2: Validate / Layer 1: Exec       │
│  ← 可被进化（受M约束）                                     │
├───────────────────────────────────────────────────────────┤
│  L0+L1+L2 三层门禁 ← M3: 只能加强不能减弱                  │
└───────────────────────────────────────────────────────────┘
```

---

## 三、七大元公理（Meta-Axioms）

元公理定义在`axiom-evolution` crate的`meta_axioms.rs`中，作为编译期常量存在。启动时EvolutionGovernor计算元公理的SHA-256 hash并与硬编码值比对，不一致则abort。

### M1: 五原语封闭性

> Cell、Signal、Lens、Axiom、Witness是系统中唯一的五个原语。任何进化操作不得引入第六个原语概念。

- 编译期：proc macro检查所有类型是否属于五个原语范畴
- 运行时：EvolutionGuardian扫描提案内容，检测是否引入新概念类别
- 违反：提案拒绝+Witness记录

### M2: 元公理不可变性

> 本列表的七条元公理（M1-M7）不得被任何自动化进化操作修改、删除、绕过或等价规避。

- 编译期：元公理文本作为&'static str硬编码，hash值作为const断言
- 运行时：启动时hash校验+每小时一次完整性检查
- 唯一例外：人类通过`axm evolution approve-meta`显式审批，需7天冷却期
- 违反：进程abort（启动时）或紧急熔断（运行时检测到篡改）

### M3: 三层门禁不可降级

> L0开发门禁、L1编译期门禁、L2运行时门禁的检查规则只能加强（新增检查项），不能减弱（删除/放宽检查项）。

- 检测方式：提案的代码diff与现有门禁检查对比
  - 不允许删除checks/模块中的检查逻辑
  - 不允许在CanSendTo中新增非法方向
  - 不允许在ArchitectureGuardian中移除审查项
  - CI workflow的step只能增加不能减少
- 违反：提案直接拒绝

### M4: 方向铁律不可违反

> Signal合法方向矩阵（11个合法方向）不可增加新的非法→合法路径。

- 当前合法方向（11个）：
  - Oversight→Oversight, Oversight→Exec, Oversight→Validate, Oversight→Agent
  - Exec→Exec, Exec→Validate
  - Validate→Exec, Validate→Validate, Validate→Agent
  - Agent→Validate, Agent→Agent
- 不允许任何进化增加新的CanSendTo impl
- 例外：人类审批+7天冷却期（同M2）
- 违反：编译错误（proc macro）+运行时拦截（EvolutionGuardian）

### M5: Witness不可断链

> 任何进化操作不得导致Witness链断裂；进化操作自身必须产生EvolutionWitness。

- 采纳变更前必须验证：新旧Witness链兼容（hash链不断）
- 每个进化步骤产生EvolutionWitness
- 违反：沙盒阶段拒绝

### M6: 进化可逆性

> 每一次进化采纳必须有对应的回滚点（Snapshot + 反向Migration）。无回滚点的变更不得进入金丝雀阶段。

- 采纳前检查：
  1. 变更前系统Snapshot存在且可恢复
  2. 如果是数据格式变更，有正向+反向Migration
  3. RollbackManager能在30s内完成回滚
- 违反：不得进入Canary阶段

### M7: 进化速率限制

> 单位时间内进化变更量受上限约束，防止大爆炸式变更导致系统不稳定。

- 速率限制：
  - 参数调整（L1）：每小时≤5个
  - Axiom/Lens/路由变更（L2）：每天≤3个
  - Cell生成（L3）：每周≤1个
  - 架构调整（L4）：每月≤1个
  - 元公理修正（需人审）：每季度≤1个
- 同时在线金丝雀数≤1（一次只验证一个变更，避免交互效应）
- 违反：速率限制器排队等待

---

## 四、进化六步闭环

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  ┌───────┐    ┌────────────┐    ┌────────┐    ┌─────────┐  │
│  │Observe│───→│Hypothesize │───→│Sandbox │───→│ Canary  │  │
│  │       │    │            │    │        │    │         │  │
│  └───↑───┘    └────────────┘    └───┬────┘    └────┬────┘  │
│      │      拒绝←失败          失败→│  失败→回滚  │        │
│      │                              │             ↓        │
│      │                        ┌─────┴─────┬───────┐        │
│      │                        │           ↓       │        │
│      │                      采纳      回滚    延迟监控      │
│      │                        │           ↑       │        │
│      │                        ↓           │       │        │
│      │                   ┌────────┐       │       │        │
│      └───────────────────│ Monitor│───────┘       │        │
│                          │ (24h)  │  恶化→自动回滚  │        │
│                          └────────┘───────────────┘        │
│                                                             │
│  每一步都产生 EvolutionWitness ──→ 不可篡改审计链             │
└─────────────────────────────────────────────────────────────┘
```

### 4.1 Step 1: Observe（观察）

进化引擎持续从以下数据源分析改进机会：

**数据源**：
1. **Witness链分析**：扫描最近N条Witness，识别
   - 高频AxiomViolation类型（某类违规反复出现但无对应Axiom拦截）
   - 高频Failed Witness（某Cell频繁panic/timeout）
   - 高频AxiomViolated后无治理动作（监督层盲区）
2. **熵值热力图**：
   - 每个Cell的实时熵值
   - 熵值持续高于平均2σ的"热点Cell"
   - 熵值上升速率最快的子系统
3. **性能指标**：
   - 各Cell消息处理P50/P95/P99延迟
   - Mailbox队列深度
   - 消息重放次数（错误重试频率）
4. **Schema演进模式**：
   - 被频繁读取但从未用作filter的字段（暗示需要新Lens）
   - 频繁被手动transform的Signal类型（暗示需要专门处理Cell）

**观察频率**：
- 实时指标：每10s采样
- Witness链批量分析：每100条Witness或每5min
- 全局熵值评估：每小时

**输出**：`ImprovementSignal`结构体列表，每个信号包含：
- signal_type: HotCell/RepeatedViolation/PerformanceDegradation/MissingAxiom/SchemaPattern/...
- evidence: 量化数据（失败率/延迟/熵值）
- affected_components: 涉及的Cell/Axiom/Lens
- confidence: 0.0-1.0 信号置信度

### 4.2 Step 2: Hypothesize（假设生成）

将ImprovementSignal转化为具体的EvolutionProposal。

**进化类型层级**：

| 层级 | 类型 | 触发ImprovementSignal | 提案内容 |
|------|------|----------------------|---------|
| L1 | 参数自调优 | PerformanceDegradation | 调整timeout/circuit-breaker阈值/Mailbox容量/并发度/TTL |
| L2a | Axiom提案 | RepeatedViolation/MissingAxiom | 新增Axiom约束（如MaxRetries<3/FieldRequired） |
| L2b | Lens投影 | SchemaPattern | 新增Lens定义（SQL-like投影规则） |
| L2c | 路由优化 | HotCell/PerformanceDegradation | 调整Cell拓扑位置/消息路由路径 |
| L3 | Cell生成 | 复杂重复模式 | 生成新Cell（去重/缓存/限流/batcher等模式） |
| L4 | 架构调整 | 持续高熵子系统 | 拆分/合并层或Cell组（受M7严格速率限制） |

**提案生成策略**：
1. **规则模板**：对L1/L2类，使用预定义的模板（如"如果P99延迟>阈值×2，建议增加timeout到1.5×"）
2. **历史类比**：搜索历史EvolutionWitness中类似场景及其采纳效果
3. **LLM辅助（P9后启用）**：对L3/L4复杂提案，由Agent层的LLM生成候选方案，但必须经过所有安全检查
4. **组合变异**：将两个已采纳的小变更组合，验证是否有叠加效果

**提案结构**：
```rust
pub struct EvolutionProposal {
    pub id: ProposalId,
    pub proposal_type: ProposalType,
    pub layer: EvolutionLayer,       // L1/L2a/L2b/L2c/L3/L4
    pub target_components: Vec<ComponentId>,
    pub changes: Vec<Change>,         // 具体变更（参数diff/Axiom代码/Lens定义/路由表/...）
    pub rollback_plan: RollbackPlan,  // 回滚方案（Snapshot ID+反向Migration）
    pub expected_fitness: FitnessDelta, // 预期改善
    pub fitness_metrics: Vec<FitnessMetric>, // 衡量指标
    pub created_at: Timestamp,
    pub meta_axiom_compliance: ComplianceCheck, // M1-M7预检查结果
}
```

**预检查（Hypothesis阶段必须通过）**：
- M1: 不引入新原语
- M2: 不修改元公理
- M3: 不减弱门禁
- M4: 不增加非法方向
- M7: 不超速率限制
- 不通过预检查的提案直接丢弃，产生EvolutionWitness(rejected, reason="meta-axiom-violation")

### 4.3 Step 3: Sandbox（沙盒验证）

提案通过预检查后，进入沙盒测试。

**沙盒环境**：
1. 从当前系统的最新Snapshot创建一个隔离的SandboxRuntime
2. 应用提案中的变更
3. 从EventStore加载最近1小时的真实Witness/Event作为测试输入
4. 确定性重放（相同输入必须产生相同输出）

**沙盒测试项**：
1. **编译测试**：变更后的代码能编译（L3/L4涉及代码生成时）
2. **clippy+fmt**：静态检查通过
3. **单元测试**：现有测试全部通过
4. **trybuild测试**：编译失败用例仍按预期失败（宏变更时）
5. **重放测试**：确定性重放1小时真实数据
   - 新变更不导致panic
   - 重放完成后Witness链完整性验证通过
   - 无数据丢失（对比Snapshot前后状态hash）
6. **适应度评估**：
   - 熵值是否降低？
   - 目标错误率是否降低？
   - P99延迟是否不恶化？
   - 不产生新的AxiomViolation类型？
7. **回滚验证**：RollbackPlan能在30s内恢复到变更前状态

**沙盒通过标准**：
- 全部编译/测试通过
- 适应度指标不劣化（statistical signficiance, p<0.1）
- 至少一个目标指标改善（>5%）
- 回滚验证成功

**失败处理**：
- 记录EvolutionWitness(sandbox_failed, reason, metrics)
- 失败的提案类型进入冷却期（同类提案1小时内不重复生成）

### 4.4 Step 4: Canary（金丝雀验证）

沙盒通过后，在生产环境进行影子流量验证。

**金丝雀部署**：
1. 新版本与旧版本并行运行
2. 真实Signal被**复制**到新版本（影子流量），新版本的处理结果不写回系统
3. 新旧版本输出对比：
   - 新版本产生的消息是否合法？（经过ArchitectureGuardian验证）
   - 新版本的Axiom检查结果是否与旧版本一致或更严格？
   - 新版本处理延迟是否可接受？
4. 持续时间：
   - L1参数变更：10分钟，≥1000条消息
   - L2规则/Lens变更：30分钟，≥5000条消息
   - L3新Cell：2小时，≥10000条消息
   - L4架构调整：24小时，≥100000条消息

**金丝雀评估指标**：
- 错误率：新版本 ≤ 旧版本×1.05（不超过5%恶化）
- 延迟P99：新版本 ≤ 旧版本×1.1（不超过10%恶化）
- 熵增量：新版本 ≤ 旧版本
- Axiom拦截率：新版本 ≥ 旧版本×0.95（不减少拦截）
- 无panic/crash

**失败处理**：
- 立即停止金丝雀
- 记录EvolutionWitness(canary_failed, metrics_comparison)
- 同类提案进入冷却期

### 4.5 Step 5: Adopt（采纳）

金丝雀通过后正式采纳变更。

**采纳流程**：
1. 暂停金丝雀环境
2. 执行正式部署（参数热更新/Axiom注册/Lens注册/代码版本切换）
3. 创建正式Snapshot作为回滚点
4. 更新路由表/注册表
5. 产生EvolutionWitness(adopted, before_snapshot, after_metrics)
6. 通知所有相关Cell（如参数变更发送ConfigUpdated信号）

**采纳保证**：
- 采纳过程原子性：要么完全成功，要么自动回滚
- 采纳不中断正在处理的消息（优雅切换）

### 4.6 Step 6: Monitor（延迟监控+自动回滚）

采纳后24小时持续监控。

**监控指标**：
- 全局熵值变化趋势
- 目标组件的错误率
- 系统整体P99延迟
- AxiomViolation率
- 是否出现新的crash/panic模式

**自动回滚触发条件**：
- 全局熵值比采纳前上升>20%
- 目标组件错误率上升>50%
- 系统P99延迟翻倍
- 出现采纳前不存在的crash
- 三层门禁被绕过（检测到应拦截但未拦截的消息）

自动回滚流程：
1. 立即执行RollbackPlan恢复
2. 产生EvolutionWitness(auto_rolled_back, reason, metrics)
3. 该提案类型进入7天冷却期
4. 通知Oversight层分析失败原因

---

## 五、Crate设计

新增crate：`axiom-evolution`

```
crates/axiom-evolution/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── meta_axioms.rs          # M1-M7编译期定义+hash校验
    ├── proposal.rs             # EvolutionProposal/Change/ProposalType
    ├── witness.rs              # EvolutionWitness（链接到主Witness链）
    ├── observer.rs             # Step1: 观察引擎（信号检测）
    ├── hypothesis.rs           # Step2: 假设生成（模板+规则+历史类比）
    ├── sandbox.rs              # Step3: 沙盒运行时+重放+验证
    ├── canary.rs               # Step4: 金丝雀部署+影子流量+对比
    ├── fitness.rs              # 适应度评估函数
    ├── deployer.rs             # Step5: 采纳+原子部署
    ├── rollback.rs             # 回滚管理器（含延迟监控）
    ├── rate_limiter.rs         # M7速率限制实现
    ├── compliance.rs           # 元公理合规检查器
    └── evolution_guardian.rs   # 进化守门人：拦截违反M的操作
```

### 关键依赖

- `axiom-core`: Cell/Signal/Axiom/Witness/Version基础类型
- `axiom-runtime`: SandboxRuntime复用运行时基础设施
- `axiom-store`: Snapshot/EventStore用于沙盒重放和回滚
- `axiom-oversight`: 复用EntropyGovernor/ArchitectureGuardian的接口
- `sha2`: 元公理hash校验
- `tokio`: 异步运行时（观察循环、金丝雀流量复制）
- `serde/serde_json`: 提案序列化/反序列化

---

## 六、CLI扩展（axm evolution）

在axiom-cli中新增`evolution`子命令组：

| 命令 | 功能 |
|------|------|
| `axm evolution status` | 显示进化引擎状态（当前观察到的信号、运行中的沙盒/金丝雀） |
| `axm evolution proposals` | 列出待处理/历史提案 |
| `axm evolution show <id>` | 显示提案详情+预期效果+验证状态 |
| `axm evolution log [--limit N]` | 显示最近N条EvolutionWitness（采纳/拒绝/回滚记录） |
| `axm evolution diff <id>` | 显示提案的具体变更diff |
| `axm evolution approve <id>` | 人工批准一个提案（跳过沙盒/金丝雀，需M2级权限） |
| `axm evolution approve-meta <id>` | 批准元公理修正案（触发7天冷却期） |
| `axm evolution rollback <id>` | 手动触发回滚到某个进化前状态 |
| `axm evolution pause` | 暂停自动进化（保留观察，不自动提案/部署） |
| `axm evolution resume` | 恢复自动进化 |

---

## 七、与三层门禁的关系

进化体系建立在三层门禁之上，不是替代：

| 层级 | 进化如何与之交互 |
|------|----------------|
| L0开发门禁 | 进化生成的代码变更必须通过`axm check`+CI才能进入沙盒；进化引擎自身的代码受L0保护 |
| L1编译期门禁 | 进化生成的proc macro使用、CanSendTo impl、trait实现必须通过编译期检查；M4由编译期强制 |
| L2运行时门禁 | 进化生成的Axiom/Cell/路由在运行时仍受ArchitectureGuardian监督；进化不能移除Guardian的检查项（M3） |

**关键**：进化引擎本身也是一个Cell（EvolutionCell），受所有Axiom约束、Witness审计、Supervisor监督。进化引擎不是"上帝模式"——它是系统中的一个公民，受同样的规则约束。

---

## 八、与原Roadmap的关系

自动进化（Layer E）作为P4.5阶段（P4 L2运行时门禁完成后、P5可视化前）：

```
P0.5 L0开发门禁
P0.6 L1编译期门禁
P1 核心原语
P2 事件存储
P3 运行时+自愈
P4 L2运行时门禁 ← 三层门禁全开，系统可稳定运行
P4.5 自动进化引擎 ← 新增（axiom-evolution crate）
P5 可视化导出
P6-P10 Agent体系（在进化引擎辅助下迭代）
P11 CLI脚手架完善（含axm evolution子命令）
P12-P17 高级功能
```

**P4.5阶段验收标准**（进化引擎自身的"可进化性"验收）：

| # | 验收项 | 验证方式 |
|---|--------|---------|
| 1 | 参数自调优工作 | 注入性能退化模式→引擎自动检测→沙盒验证→金丝雀→采纳参数调整→延迟恢复 |
| 2 | Axiom自动提案 | 注入重复违规模式（如空消息反复通过）→引擎提案新Axiom→沙盒验证→采纳后空消息被拦截 |
| 3 | 元公理M3保护 | 引擎尝试生成一个删除check项的提案→compliance检查阶段被拒绝 |
| 4 | 元公理M4保护 | 引擎尝试增加Exec→Oversight方向→编译期或compliance拒绝 |
| 5 | 自动回滚 | 采纳一个导致熵升高的变更→24h监控期内自动回滚 |
| 6 | 速率限制M7 | 同时生成多个架构级提案→速率限制器排队，不超过1个金丝雀 |
| 7 | EvolutionWitness链完整 | 每次进化操作都有Witness，链hash验证通过 |
| 8 | 沙盒隔离 | 沙盒中的变更不影响生产系统 |
| 9 | 进化引擎本身受监督 | EvolutionCell panic→Supervisor重启；有AxiomViolation→Guardian拦截 |
| 10 | axm evolution命令可用 | status/proposals/log/diff/rollback功能正常 |

---

## 九、不做的事情（YAGNI）

- 不做进化引擎的进化引擎（无限元回归turtle问题截止在Layer M）
- 不做跨系统的"基因交换"（不与其他Axiom实例交换进化经验，安全风险）
- 不做实时热代码patch（WASM/JIT）——代码变更走编译→沙盒→金丝雀的正式流程
- 不做基于强化学习的进化策略（P12-P13规划器阶段再考虑，初版用规则模板+历史类比）
- L3（Cell代码生成）初期只支持预定义模式（dedupe/cache/batcher/rate-limiter/circuit-breaker模板），不做任意代码生成
- L4（架构调整）初期只支持Cell在层内迁移，不支持层的拆分/合并（等P4稳定后再实现）
- 进化引擎不直接修改proc macro（编译期门禁的宏是基础设施，修改宏等同于修改编译器，风险极高，初期禁止）
