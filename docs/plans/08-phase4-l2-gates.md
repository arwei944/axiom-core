# Phase 4: L2运行时门禁 Implementation Plan

> **Goal:** 完成 axiom-oversight 监督层的所有监督 Cells，实现三层门禁全开。验收标准：运行时拦截违规消息（L2）、熵超标自动治理、崩溃自动恢复、启动验证链完整、health endpoint暴露系统状态。三层门禁（L0 CLI + L1编译期 + L2运行时）全部生效。

> **Baseline:** axiom-oversight 已有 ArchitectureGuardian（层检查+跳数+schema版本）和 EntropyGovernor（熵值统计）的基本实现，但只有静态检查函数，未实现为可注册到Runtime的Oversight Cells。文档中列出但未实现的：IntentAuditor（意图漂移检测）、ResourceManager（Token预算/限流）、LoopDetector（全局消息环检测）、ComplianceGuard（PII检测）、OversightOversight（元监督）。

---

## Global Constraints

- axiom-oversight 依赖 axiom-core 和 axiom-runtime
- 所有 Oversight Cell 本身也是 Cell，受 Cell 生命周期和 Supervisor 监督
- Oversight Cell 崩溃同样被重启（不能因为监督层崩溃导致整个系统崩溃）
- L2 门禁是"深度防御"：即使L1编译期约束被绕过（如unsafe/动态类型），L2仍要拦截
- 监督动作必须产生 Witness（审计要求）
- cargo build/clippy/test 零警告

---

## Task 1: 将 ArchitectureGuardian 实现为 Oversight Cell

**Files:**
- Modify: `crates/axiom-oversight/src/architecture_guardian.rs`
- Modify: `crates/axiom-oversight/src/lib.rs`

**Problem:** 当前ArchitectureGuardian是一个普通struct，实现了BusInterceptor，但不是一个Cell，不能接收信号、不能产生Witness。需要将其改造为Oversight层Cell。

- [ ] **Step 1: 定义 GuardCell 结构体**
  ```rust
  pub struct ArchitectureGuardianCell {
      id: CellId,
      stats: Arc<Mutex<GuardianStats>>,
  }
  ```

- [ ] **Step 2: 实现 Cell trait**
  - layer() → Layer::Oversight
  - Message 类型：GuardCommand（CheckSignal/ReportViolation/QueryStats）
  - handle():
    - CheckSignal: 执行架构检查，返回检查结果
    - ReportViolation: 记录违规统计，产生Witness
    - QueryStats: 返回GuardianStats

- [ ] **Step 3: 保留 BusInterceptor 实现**
  - ArchitectureGuardian 仍然作为 BusInterceptor 运行在 MessageBus 中
  - 拦截到违规时，向 ArchitectureGuardianCell 发送 ReportViolation 信号
  - 拦截器本身不持有统计数据，通过消息传递给Guardian Cell（避免锁竞争）

- [ ] **Step 4: 添加违规类型统计**
  - GuardianStats 包含：
    - layer_violations: HashMap<(Layer,Layer), u64> — 各方向违规计数
    - hop_limit_exceeded: u64
    - schema_version_mismatch: u64
    - unknown_signal_type: u64
    - total_intercepted: u64
    - total_allowed: u64

- [ ] **Step 5: 测试**
  - 测试Guardian Cell注册到Runtime后能处理消息
  - 测试拦截器拦截违规时通知Guardian Cell
  - 测试统计数据正确累加
  - 测试Guardian Cell崩溃可被Supervisor重启

- [ ] **Step 6: Commit**
  - `feat(axiom-oversight): ArchitectureGuardianCell as Oversight Cell, violation stats tracking, Witness generation`

---

## Task 2: 将 EntropyGovernor 实现为 Oversight Cell

**Files:**
- Modify: `crates/axiom-oversight/src/entropy_governor.rs`
- Modify: `crates/axiom-oversight/src/lib.rs`

- [ ] **Step 1: 定义 EntropyGovernorCell**
  - 持有全局熵值状态
  - 持有 per-cell 熵值map
  - 配置阈值（GREEN/YELLOW/RED/CRITICAL）

- [ ] **Step 2: 实现 Cell trait**
  - Message类型：EntropyCommand（RecordEvent/DecayTick/GetSnapshot/TakeAction）
  - handle():
    - RecordEvent(drop/rejection/restart/circuit_break/timeout)：增加对应权重的熵值
    - DecayTick：执行时间衰减（由runtime定时任务每秒发送）
    - GetSnapshot：返回EntropySnapshot
    - TakeAction：根据当前熵值水平返回治理建议（治理动作由Runtime执行，Cell只返回决策）

- [ ] **Step 3: 治理决策逻辑**
  - GREEN: 无动作
  - YELLOW: 返回 Warn 建议（增加日志采样率）
  - RED: 返回 Throttle 建议（非关键消息限流+通知热点Cell的Supervisor考虑重启）
  - CRITICAL: 返回 Emergency 建议（只允许Oversight层消息，触发熔断）
  - 冷却期：上次治理后30s内不重复治理

- [ ] **Step 4: 添加定时 DecayTick**
  - Runtime启动时spawn一个task，每秒发送DecayTick
  - 每5分钟输出一次熵值snapshot到tracing

- [ ] **Step 5: 测试**
  - 测试熵值累加和衰减
  - 测试阈值触发治理决策
  - 测试冷却期防止正反馈
  - 测试per-cell熵值追踪

- [ ] **Step 6: Commit**
  - `feat(axiom-oversight): EntropyGovernorCell with governance decisions, per-cell entropy, decay tick, Witness generation`

---

## Task 3: 实现 ResourceManager（资源管理 Cell）

**Files:**
- Create: `crates/axiom-oversight/src/resource_manager.rs`
- Modify: `crates/axiom-oversight/src/lib.rs`

- [ ] **Step 1: 定义 TokenBucket 限流**
  ```rust
  pub struct TokenBucket {
      capacity: u64,
      tokens: AtomicU64,
      refill_rate_per_sec: f64,
      last_refill: Mutex<Instant>,
  }
  ```
  - try_acquire(tokens: u64) -> bool
  - refill() — 按时间补充token

- [ ] **Step 2: 定义 ResourceManagerCell**
  - 管理：
    - 全局 Token 预算（LLM调用/外部API调用的token消耗）
    - 全局并发限制（同时处理的消息数上限）
    - 每个Cell的CPU时间配额
    - 内存使用监控
  - Message类型：ResourceCommand（AcquireTokens/ReleaseTokens/CheckQuota/SetLimit）
  - handle():
    - AcquireTokens: 检查预算，获取token，不足则Reject
    - ReleaseTokens: 归还token
    - CheckQuota: 查询剩余预算
    - SetLimit: 动态调整限制（由EvolutionGovernor或人工触发）

- [ ] **Step 3: 添加 ResourceExhausted 错误处理**
  - 返回 AxiomError::TokenBudgetExceeded 或 ResourceExhausted
  - 资源耗尽时产生Witness
  - 严重耗尽时通知EntropyGovernor

- [ ] **Step 4: 测试**
  - 测试TokenBucket获取/释放
  - 测试容量耗尽时拒绝
  - 测试按时间补充token
  - 测试全局并发限制

- [ ] **Step 5: Commit**
  - `feat(axiom-oversight): ResourceManagerCell with token bucket rate limiting, concurrency control, budget enforcement`

---

## Task 4: 实现 IntentAuditor（意图漂移检测 Cell）

**Files:**
- Create: `crates/axiom-oversight/src/intent_auditor.rs`
- Modify: `crates/axiom-oversight/src/lib.rs`

**IntentAuditor**检测Agent行为是否偏离声明意图。

- [ ] **Step 1: 定义 IntentProfile**
  ```rust
  pub struct IntentProfile {
      pub agent_id: String,
      pub declared_intent: String,       // 声明的目标
      pub expected_signal_types: Vec<String>,  // 预期会产生的信号类型
      pub expected_targets: Vec<String>,       // 预期会访问的目标Cell
      pub forbidden_actions: Vec<String>,      // 禁止的动作
      pub confidence: f64,                     // 意图匹配置信度
  }
  ```

- [ ] **Step 2: 定义 BehaviorSample**
  - 记录一个时间窗口内（如10分钟）Agent实际产生的信号：
    - 信号类型分布
    - 目标Cell分布
    - 错误率
    - 消息频率
    - 数据访问模式

- [ ] **Step 3: 实现 IntentAuditorCell**
  - Message类型：AuditCommand（RegisterIntent/RecordBehavior/AuditCheck/QueryDeviation）
  - handle():
    - RegisterIntent: 注册Agent的声明意图
    - RecordBehavior: 记录一个Witness/信号样本
    - AuditCheck: 执行偏离检测：
      1. 对每个Agent，比较BehaviorSample与IntentProfile
      2. 如果出现未声明的信号类型/目标Cell，标记为Suspicious
      3. 如果错误率突增（2σ以上），标记为Anomalous
      4. 如果置信度低于0.5，产生Deviation Witness
    - QueryDeviation: 查询当前偏离状态

- [ ] **Step 4: 偏离度计算**
  - 使用简单的Jaccard相似度比较预期和实际集合
  - 使用Z-score检测错误率突变
  - 偏离度 > 阈值时产生IntentDrift Witness

- [ ] **Step 5: 测试**
  - 测试正常行为不触发告警
  - 测试未声明的信号类型触发偏离检测
  - 测试错误率突增触发告警
  - 测试IntentProfile注册和查询

- [ ] **Step 6: Commit**
  - `feat(axiom-oversight): IntentAuditorCell with intent profile registration, behavior sampling, deviation detection`

---

## Task 5: 实现 ComplianceGuard（合规检查 Cell）

**Files:**
- Create: `crates/axiom-oversight/src/compliance_guard.rs`
- Modify: `crates/axiom-oversight/src/lib.rs`

**ComplianceGuard**检查敏感数据泄露。

- [ ] **Step 1: 定义敏感数据模式**
  ```rust
  pub struct SensitivePattern {
      pub name: &'static str,
      pub regex: &'static str,
      pub severity: Severity,  // Low/Medium/Critical
      pub action: ComplianceAction,  // Log/Warn/Redact/Reject
  }
  ```
  - 内置模式：
    - Email地址
    - 电话号码（中国+通用格式）
    - API密钥/Token格式（如 ghp_, sk-, Bearer 开头的）
    - 身份证号
    - 银行卡号（Luhn校验）

- [ ] **Step 2: 实现 ComplianceGuardCell**
  - 作为 BusInterceptor 运行（在ArchitectureGuardian之后）
  - 检查SignalEnvelope.payload中的字符串字段
  - Message类型：ComplianceCommand（CheckPayload/AddPattern/SetPolicy/QueryViolations）
  - handle():
    - CheckPayload: 扫描payload中的敏感数据
    - AddPattern: 动态添加敏感模式（由EvolutionGovernor或人工触发）
    - SetPolicy: 设置策略（如哪些层需要扫描、是否redact）
    - QueryViolations: 查询违规统计

- [ ] **Step 3: 违规处理**
  - Log: 仅记录
  - Warn: 记录tracing::warn + Witness
  - Redact: 将敏感数据替换为 [REDACTED] 后继续传递
  - Reject: 直接拒绝消息

- [ ] **Step 4: 测试**
  - 测试Email/电话/API key检测
  - 测试Redact模式正确替换
  - 测试Reject模式正确拒绝
  - 测试误报率（正常数字不触发银行卡检测等）

- [ ] **Step 5: Commit**
  - `feat(axiom-oversight): ComplianceGuardCell with PII/sensitive data detection, redaction, and rejection policies`

---

## Task 6: 实现 OversightOversight（元监督 Cell）

**Files:**
- Create: `crates/axiom-oversight/src/meta_oversight.rs`
- Modify: `crates/axiom-oversight/src/lib.rs`

**OversightOversight**监督监督者——检查其他Oversight Cell是否正常工作。

- [ ] **Step 1: 实现 MetaOversightCell**
  - 职责：
    1. 定期heartbeat检查：每个Oversight Cell必须在N秒内响应ping
    2. 如果某个Oversight Cell无响应，触发其Supervisor重启
    3. 如果某个Oversight Cell违规（如尝试发送到非法层），产生Critical Witness
    4. 审计Oversight Cell产生的Witness，确保完整
  - Message类型：MetaCommand（Ping/CellCrashed/OversightViolation/HealthCheck）

- [ ] **Step 2: Heartbeat机制**
  - Runtime启动后，MetaOversightCell启动定时heartbeat task
  - 每10秒向每个Oversight Cell发送Ping
  - 如果连续3次Ping无响应，触发重启
  - Ping/Pong 不产生Witness（健康检查不是业务操作）

- [ ] **Step 3: 监督完整性检查**
  - 检查所有BusInterceptor都在正常运行（通过拦截器统计）
  - 检查Witness链不断裂（从EventStore读取最新Witness验证hash）
  - 如果Witness链断裂，产生Critical Witness并暂停非Oversight消息处理

- [ ] **Step 4: 测试**
  - 测试心跳检测正常响应
  - 测试无响应的Cell被标记并触发重启
  - 测试Witness链断裂检测
  - 测试MetaOversight自己崩溃时的重启（它也是Cell，受Supervisor保护）

- [ ] **Step 5: Commit**
  - `feat(axiom-oversight): MetaOversightCell (OversightOversight) with heartbeat monitoring, supervision integrity checks, Witness chain verification`

---

## Task 7: 启动验证链（Startup Verification Chain）

**Files:**
- Modify: `crates/axiom-oversight/src/lib.rs`
- Create: `crates/axiom-oversight/src/startup.rs`

- [ ] **Step 1: 定义启动验证项**
  ```rust
  pub struct StartupVerification {
      pub checks: Vec<Box<dyn StartupCheck>>,
  }
  pub trait StartupCheck: Send + Sync {
      fn name(&self) -> &'static str;
      fn check(&self) -> Result<(), StartupError>;
  }
  pub enum StartupError {
      Blocking(String),  // 阻断启动
      Warning(String),   // 警告但继续
  }
  ```

- [ ] **Step 2: 实现内置启动检查**
  1. MigrationChainCheck: 验证迁移链完整性（调用axiom-store）
  2. AxiomRegistrationCheck: 验证Axiom注册表非空且无重复
  3. LayerCanSendToCheck: 验证合法方向矩阵（编译期保证，此处运行时二次验证）
  4. WitnessGenesisCheck: 如果EventStore非空，验证genesis Witness存在
  5. ConfigVersionCheck: 验证配置文件版本兼容
  6. MetaAxiomHashCheck: 验证元公理hash（P4.5阶段启用）

- [ ] **Step 3: 实现 run_startup_verification()**
  - 依次执行所有check
  - Blocking错误 → abort启动
  - Warning错误 → 打印警告，继续启动
  - 返回 VerificationReport（所有结果）

- [ ] **Step 4: 将启动验证集成到 RuntimeBuilder.build()**
  - 在Runtime启动前调用run_startup_verification
  - 验证报告作为第一个Witness写入EventStore

- [ ] **Step 5: 测试**
  - 测试所有检查通过时启动成功
  - 测试Blocking错误阻止启动
  - 测试Warning错误不阻止启动但记录
  - 测试验证报告被写入Witness链

- [ ] **Step 6: Commit**
  - `feat(axiom-oversight): startup verification chain with migration/axiom/layer/witness/config checks, integrated into RuntimeBuilder`

---

## Task 8: Health Endpoint 和系统状态导出

**Files:**
- Create: `crates/axiom-oversight/src/health.rs`
- Modify: `crates/axiom-oversight/src/lib.rs`

- [ ] **Step 1: 定义 SystemHealth 结构体**
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct SystemHealth {
      pub status: HealthStatus,  // Healthy/Degraded/Critical
      pub uptime_secs: u64,
      pub version: String,
      pub cells: Vec<CellHealth>,
      pub entropy: EntropySnapshot,
      pub messages: MessageStats,
      pub resources: ResourceStats,
      pub oversight: OversightHealth,
      pub started_at: chrono::DateTime<chrono::Utc>,
  }
  pub struct CellHealth {
      pub id: String,
      pub layer: String,
      pub state: String,  // Running/Restarting/CircuitOpen/Stopped
      pub processed_messages: u64,
      pub failed_messages: u64,
      pub restart_count: u32,
      pub last_message_ns: Option<u64>,
      pub mailbox_depth: usize,
  }
  ```

- [ ] **Step 2: 实现 HealthCollectorCell**
  - 定时（每5秒）从所有监督组件收集健康数据
  - 汇总为SystemHealth
  - Message类型：HealthQuery（GetHealth/GetCellHealth）
  - 响应HTTP endpoint（P11阶段提供HTTP server，此阶段只提供数据查询接口）

- [ ] **Step 3: 健康状态判断逻辑**
  - Healthy: 所有Cell运行中，熵值GREEN，无CircuitOpen
  - Degraded: 有Cell在Restarting，熵值YELLOW，或有CircuitOpen但已恢复
  - Critical: 有Cell停止，熵值CRITICAL，或MetaOversight报告问题

- [ ] **Step 4: 添加 axm health CLI 命令（为P11准备接口）**
  - 在axiom-cli中预留 `axm doctor` 调用health query

- [ ] **Step 5: 测试**
  - 测试健康数据收集
  - 测试状态判断逻辑
  - 测试Cell崩溃后状态变化反映在health中

- [ ] **Step 6: Commit**
  - `feat(axiom-oversight): SystemHealth aggregation, HealthCollectorCell, health status determination (Healthy/Degraded/Critical)`

---

## Task 9: 注册所有 Oversight Cells 到 Runtime

**Files:**
- Modify: `crates/axiom-runtime/src/runtime.rs`
- Modify: `crates/axiom-oversight/src/lib.rs`

- [ ] **Step 1: 实现 register_oversight_cells() 函数**
  - RuntimeBuilder::build() 时自动注册：
    1. ArchitectureGuardianCell（同时注册为BusInterceptor）
    2. EntropyGovernorCell（同时注册为BusInterceptor）
    3. ResourceManagerCell
    4. IntentAuditorCell
    5. ComplianceGuardCell（同时注册为BusInterceptor）
    6. MetaOversightCell
    7. HealthCollectorCell
  - 这些Cell都在Layer::Oversight层
  - 使用固定的CellId（如 "oversight:architecture-guardian"）

- [ ] **Step 2: 确保 Oversight Cells 之间的通信合法**
  - Oversight→Oversight 是合法方向
  - 不允许Oversight Cell绕过ArchitectureGuardian直接发消息到Exec层（仍然需要通过Bus）

- [ ] **Step 3: 测试**
  - 测试Runtime启动后所有Oversight Cell已注册
  - 测试Oversight Cells可以互相发送消息
  - 测试拦截器链包含所有Oversight拦截器
  - 端到端测试：一个Exec Cell发送违规消息被Guardian拦截

- [ ] **Step 4: Commit**
  - `feat(axiom-runtime): auto-register all Oversight cells during build, interceptor chain integration, end-to-end L2 gate test`

---

## P4 阶段验收标准

| # | 验收项 | 验证方式 |
|---|--------|---------|
| 1 | cargo build -p axiom-oversight -p axiom-runtime 零警告 | 命令行验证 |
| 2 | cargo test -p axiom-oversight -p axiom-runtime 全部通过（≥50个测试） | 命令行验证 |
| 3 | ArchitectureGuardianCell 拦截违规消息并产生Witness | 集成测试 |
| 4 | EntropyGovernorCell 熵超标自动治理（Warn/Throttle/Emergency） | 集成测试 |
| 5 | ResourceManagerCell Token预算和限流 | 单元测试 |
| 6 | IntentAuditorCell 意图漂移检测 | 单元测试 |
| 7 | ComplianceGuardCell PII检测和脱敏 | 单元测试 |
| 8 | MetaOversightCell 心跳检测+Witness链完整性 | 单元测试 |
| 9 | 启动验证链：阻断错误阻止启动 | 集成测试 |
| 10 | SystemHealth聚合和状态判断 | 单元测试 |
| 11 | 所有Oversight Cell崩溃可被重启（不导致系统崩溃） | 集成测试 |
| 12 | 端到端三层门禁：L0 axm check + L1编译错误 + L2运行时拦截 | 手动+自动测试 |
| 13 | cargo clippy/test/fmt全部通过 | axm check |
