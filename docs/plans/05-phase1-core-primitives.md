# Phase 1: 核心原语完善 Implementation Plan

> **Goal:** 完善 axiom-core 五大原语的集成：让 `#[derive(SignalPayload)]` 宏完全可用、CellContext 正确处理类型擦除的 SignalEnvelope、WitnessBuilder 自动注入 VersionInfo、Axiom/Migration 自动注册链路验证、Schema 校验自动调用。验收标准：hello_cell 示例能完整收发消息、产生 Witness、通过层约束和 Schema 校验。

> **Baseline:** 核心 trait（Cell/Signal/Schema/Axiom/Witness/Lens/Versioned/Migration）已定义；CellContext 已有层特化包装；5个 proc macro 骨架已实现；linkme 分布式注册已实现。但存在集成缺口：`#[derive(SignalPayload)]` 生成的代码与 Signal trait 不完全匹配；CellContext.send 需要正确处理类型擦除；Witness 未自动关联 VersionInfo；Axiom 自动注册后未自动加入 AxiomChain。

---

## Global Constraints

- Rust edition 2021，MSRV 1.75，禁止 async-trait
- unsafe 仅限 `unsafe_impl` 模块，必须有 SAFETY 注释
- 所有 public API 必须有 `///` rustdoc 注释和 `#[derive(Debug)]`
- cargo build/clippy/test 零警告，cargo fmt 通过
- axiom-core 不依赖其他 workspace crate（仅 axiom-macros 为可选依赖）
- 所有 Signal 必须 `#[derive(Clone, Serialize, Deserialize)]`
- 错误类型使用 thiserror，应用边界用 anyhow
- 每个 Task 结束后 `cargo test -p axiom-core` 通过

---

## Task 1: 修复 Signal trait 与 SignalPayload 宏的对齐

**Files:**
- Modify: `crates/axiom-core/src/signal.rs`
- Modify: `crates/axiom-macros/src/lib.rs`（SignalPayload derive 部分）

**Problem:** Signal trait 当前要求所有方法手动实现，但 `#[derive(SignalPayload)]` 宏生成的代码与 Signal trait 的方法签名可能不匹配。需要统一。

- [ ] **Step 1: 审查并对齐 Signal trait 方法签名**
  - 确认 `Signal` trait 的所有方法都有明确约束
  - `msg_id()` 返回 `&MsgId`（不是 &str）
  - `correlation_id()` 返回 `&CorrelationId`（不是 &str）
  - `trace_id()` 返回 `Option<&TraceId>`
  - `vector_clock()` 返回 `&VectorClock`
  - `schema_version()` 返回 `SchemaVersion`（从 #[schema_version(N)] 宏自动获取）
  - 确保 `Signal: Schema + Serialize + Clone` 约束成立

- [ ] **Step 2: 完善 SignalPayload derive 宏**
  - 宏解析结构体字段，找到 `msg_id`, `correlation_id`, `vector_clock` 字段自动生成对应方法
  - 解析 `#[signal(kind = "command", layer = "exec")]` 属性生成 `kind()` 和 `layer()` 返回值
  - 如果结构体有 `#[schema_version(N)]` 属性，自动使用该版本作为 `schema_version()` 返回值
  - 如果没有 trace_id 字段，自动返回 `None`
  - `timestamp_ns()` 使用 `now_ns()` 默认实现
  - `sender()` 默认返回 `None`
  - 自动派生 `Clone, Serialize, Deserialize, Debug`
  - 如果缺少必需字段（msg_id/correlation_id/vector_clock），给出清晰编译错误

- [ ] **Step 3: 移除 SignalDyn trait，统一使用 Signal trait 对象**
  - 当前 signal.rs 有 `SignalDyn` trait 用于类型擦除，与 `Signal` + `SignalClone` 重复
  - 将 `Signal` 本身改造为对象安全（不使用泛型方法），删除 `SignalDyn`
  - SignalEnvelope.inner 使用 `Box<dyn Signal>`

- [ ] **Step 4: 为 Signal 提供 blanket Serialize 实现**
  - 确保 `Box<dyn Signal>` 可以通过 serde_json 序列化/反序列化
  - 使用 `typemap` 模式或 ` erased_serde` 实现类型擦除的序列化
  - 或者更简单：在 SignalEnvelope 中存储 `payload_json: Value`（序列化后的JSON），downcast时反序列化

- [ ] **Step 5: 编译验证**
  - `cargo build -p axiom-core -p axiom-macros` 零警告
  - 修复所有编译错误

- [ ] **Step 6: 测试**
  - 添加测试：使用 `#[derive(SignalPayload)]` 定义一个 Command 类型，验证所有方法返回正确值
  - 添加测试：SignalEnvelope 序列化/反序列化往返
  - `cargo test -p axiom-macros -p axiom-core` 通过

- [ ] **Step 7: Commit**
  - `feat(axiom-core): align Signal trait with SignalPayload derive macro, add typed envelope serialization`

---

## Task 2: 完善 CellContext 类型安全的消息发送

**Files:**
- Modify: `crates/axiom-core/src/context.rs`
- Modify: `crates/axiom-core/src/signal.rs`（SignalEnvelope）

**Problem:** CellContext.send() 需要在发送时自动：(1) 设置 sender 为当前 cell_id；(2) 检查层方向；(3) 合并 vector clock；(4) 自动调用 Schema::validate()；(5) 序列化payload到JSON；(6) 生成OutgoingEnvelope。当前层特化的 Context wrapper（ExecCellContext等）需要正确限制发送目标层。

- [ ] **Step 1: 重构 CellContext.send 方法**
  - `send<S: Signal>(&mut self, target: CellId, signal: S) -> Result<()>`
  - 内部执行：
    1. 调用 `signal.validate()`，如果 ValidationResult 有 errors，返回 `AxiomError::SignalValidation`
    2. 将 signal 序列化为 JSON Value
    3. 合并当前 clock 与 signal.vector_clock()
    4. 创建 OutgoingEnvelope：sender=当前cell_id, source_layer=当前layer, target=target, payload=signal, payload_json=json
    5. 检查 `self.layer.can_send_to(target_layer_hint)` — 但此时不知道target_layer，需要Runtime通过RoutingTable查询
    6. 层方向检查由 Runtime/ArchitectureGuardian 在投递时二次检查，Context只检查信号自身的layer标记
    7. 将 envelope 推入 outbox

- [ ] **Step 2: 添加 send_event（广播事件）方法**
  - `emit_event<S: Signal>(&mut self, signal: S) -> Result<()>`
  - 不指定 target（广播），target=None
  - 同样执行validate+序列化+入outbox

- [ ] **Step 3: 完善层特化 Context 的 send 约束**
  - ExecCellContext 只能发送到 Exec 和 Validate 层的 Cell
  - ValidateCellContext 可以发送到 Validate/Exec/Agent
  - AgentCellContext 可以发送到 Agent/Validate
  - OversightCellContext 可以发送到任意层
  - 通过在层特化 wrapper 上只暴露合法的 send 方法，在编译期防止违规
  - 例如 `ExecCellContext` 不提供 `send_to_agent()` 方法

- [ ] **Step 4: 添加 reply 方法**
  - `reply<S: Signal>(&mut self, incoming: &SignalEnvelope, response: S) -> Result<()>`
  - 自动设置 correlation_id 为 incoming 的 correlation_id
  - target 为 incoming 的 sender
  - 自动合并 vector clock

- [ ] **Step 5: 完善 Witness 发射方法**
  - `emit_success(summary: &str)` — 自动使用当前 correlation_id
  - `emit_failure(summary: &str, reason: &str)`
  - `emit_axiom_violation(axiom_name: &str, message: &str)`
  - 这些方法自动设置 WitnessBuilder 的 correlation_id 和 clock

- [ ] **Step 6: 添加 spawn_child 方法（为监督树准备）**
  - `spawn(&mut self, cell: Box<dyn Cell>) -> Result<CellId>` — 后续Task在Runtime中实现实际spawn逻辑
  - 此阶段只在Context中记录spawn请求，Runtime调度时处理

- [ ] **Step 7: 测试**
  - 测试CellContext.send正确验证Schema（无效Signal返回错误）
  - 测试ExecCellContext编译期限制（trybuild测试：尝试send到Agent层应编译失败）
  - 测试reply自动设置correlation_id
  - 测试emit_success/emit_failure产生正确的Witness

- [ ] **Step 8: Commit**
  - `feat(axiom-core): complete CellContext with typed send/emit/reply, auto schema validation, layer-specific constraints`

---

## Task 3: Witness 自动注入 VersionInfo 和 Hash 链完善

**Files:**
- Modify: `crates/axiom-core/src/witness.rs`
- Modify: `crates/axiom-core/src/version.rs`
- Modify: `crates/axiom-core/src/context.rs`

**Problem:** Witness 当前没有记录触发它的 Signal 的版本信息，无法在重放时进行版本迁移。WitnessBuilder 需要自动从 CellContext 和 Signal 中提取 VersionInfo。

- [ ] **Step 1: 为 Witness 添加 version_info 字段**
  - 在 Witness 结构体中添加 `pub version_info: VersionInfo` 字段
  - VersionInfo 包含 schema_version/protocol_version 等信息（已有定义）

- [ ] **Step 2: WitnessBuilder 自动注入 VersionInfo**
  - CellContext 在调用 emit_* 时，从当前处理的 Signal 和 Cell 自身版本自动填充 VersionInfo
  - WitnessBuilder.build() 时将 VersionInfo 纳入 hash 计算（确保版本变化也导致 hash 不同）

- [ ] **Step 3: 添加信号指纹（signal_fingerprint）字段**
  - Witness 中添加 `pub signal_fingerprint: [u8; 32]`
  - 计算方式：SHA-256(signal_type + schema_version.to_string() + payload_json)
  - 用于快速判断两个Witness是否由相同信号触发

- [ ] **Step 4: 完善 Witness 序列化大小限制**
  - Witness 结构体添加 `pub payload_size_bytes: usize` 字段
  - summary 字段限制最大长度（如 512 bytes），超过自动截断
  - reason/message 字段同样限制

- [ ] **Step 5: 添加 WitnessBatch 类型**
  - 用于 CellContext 一次 handle 调用可能产生多个 Witness
  - WitnessBatch 包含 Vec<Witness> + 自动 hash 链验证
  - `fn verify_chain(&self) -> bool` 验证 batch 内所有 Witness 的 hash 链接正确

- [ ] **Step 6: 测试**
  - 测试Witness包含VersionInfo
  - 测试Witness hash 计算包含version_info（篡改version_info导致hash不匹配）
  - 测试WitnessBatch链式验证
  - 测试长summary自动截断

- [ ] **Step 7: Commit**
  - `feat(axiom-core): Witness auto-injects VersionInfo, adds signal fingerprint and WitnessBatch chain verification`

---

## Task 4: AxiomChain 与自动注册 Axiom 的集成

**Files:**
- Modify: `crates/axiom-core/src/axiom.rs`
- Modify: `crates/axiom-core/src/registry.rs`

**Problem:** `#[axiom]` 宏将 Axiom 注册到 `AXIOM_REGISTRY` 分布式切片，但 AxiomChain 需要在运行时从注册表构建。需要提供从注册表构建 AxiomChain 的功能，并支持按层过滤。

- [ ] **Step 1: 重构 Axiom trait 使其对象安全**
  - 当前 Axiom 有泛型关联类型 `State` 和 `Message`，使其不是对象安全的
  - 改为使用类型擦除：`fn check(&self, current: &dyn std::any::Any, new: &dyn std::any::Any, msg: &dyn std::any::Any) -> Result<()>`
  - 或者保留泛型 Axiom trait，提供 `DynAxiom` 对象安全 trait 作为包装
  - 选择更Rust-idiomatic的方案：保留泛型 Axiom 用于业务代码，增加 `DynAxiom` trait 用于运行时分发

- [ ] **Step 2: 定义 DynAxiom 对象安全 trait**
  ```rust
  pub trait DynAxiom: Send + Sync {
      fn name(&self) -> &'static str;
      fn applies_to_layer(&self, layer: Layer) -> bool;
      fn violation_action(&self) -> ViolationAction;
      fn check_dyn(&self, current: &dyn std::any::Any, new: &dyn std::any::Any, msg: &dyn std::any::Any) -> Result<()>;
  }
  ```
  - 为所有实现了 `Axiom<State=S, Message=M>` 的类型自动实现 `DynAxiom`

- [ ] **Step 3: 增加 RegistryAxiom 包装**
  - `#[axiom]` 宏除了将 Axiom 注册到 AXIOM_REGISTRY，还自动生成 DynAxiom 实现
  - 注册表中存储 `&'static dyn DynAxiom`

- [ ] **Step 4: 提供全局 AxiomChain 构建函数**
  - `fn build_axiom_chain_for_layer(layer: Layer) -> Vec<&'static dyn DynAxiom>`
  - 从 AXIOM_REGISTRY 收集所有 applies_to_layer(layer) 的 axioms

- [ ] **Step 5: 测试**
  - 测试 `#[axiom]` 标记的 Axiom 能通过 registry 被发现
  - 测试 build_axiom_chain_for_layer 正确按层过滤
  - 测试 DynAxiom 对具体类型的 check 正确工作

- [ ] **Step 6: Commit**
  - `feat(axiom-core): DynAxiom object-safe trait, axiom macro auto-generates DynAxiom impl, runtime axiom chain builder`

---

## Task 5: Migration 链验证与 Schema 自动迁移集成

**Files:**
- Modify: `crates/axiom-core/src/version.rs`
- Modify: `crates/axiom-core/src/registry.rs`

**Problem:** Migration 已可自动注册，但缺少：(1) 运行时自动检测迁移链完整性并在缺失时拒绝启动；(2) 从旧版本JSON自动迁移到最新版本的功能；(3) Versioned 的 min_supported_version 正确设置。

- [ ] **Step 1: 完善 MigrationChain 验证**
  - `verify_migration_chain_completeness()` 已有，增加对每个schema类型的链验证
  - 返回详细错误信息：哪个类型在哪个版本有gap，哪些migration缺失
  - 验证每个migration的source_version = 前一个migration的target_version
  - 验证第一个migration的source_version >= 1
  - 验证所有migration的target_version = source_version + 1（线性递增）

- [ ] **Step 2: 实现 SchemaMigrator**
  ```rust
  pub struct SchemaMigrator {
      migrations: HashMap<&'static str, Vec<&'static dyn Migration>>,
  }
  impl SchemaMigrator {
      pub fn from_registry() -> Self; // 从MIGRATION_REGISTRY构建
      pub fn migrate_to_latest(&self, signal_type: &str, from_version: SchemaVersion, json: Value) -> Result<(Value, SchemaVersion)>;
      pub fn migrate_to(&self, signal_type: &str, from: SchemaVersion, to: SchemaVersion, json: Value) -> Result<Value>;
  }
  ```
  - migrate_to_latest 按顺序执行migration链中的所有migration
  - 如果无法迁移（gap），返回 AxiomError::MigrationChainGap

- [ ] **Step 3: 为 Versioned trait 添加 migrations() 关联函数**
  - `#[migration(from = N)]` 宏自动将migration注册到对应signal_type的链
  - signal_type 通过migration处理的target类型确定（或者通过宏属性指定）
  - 实际上：`#[migration(from = N, for = "MySignal")]` 指定migration目标类型名称

- [ ] **Step 4: 添加 proc macro 编译期检查**
  - `#[migration(from = N)]` 如果N+1 != 当前类型schema_version，编译错误
  - 例如：`#[schema_version(3)] struct MySignal;` 对应的migration必须 `from = 2`，target=3

- [ ] **Step 5: 测试**
  - 测试SchemaMigrator迁移单步
  - 测试SchemaMigrator迁移多步（v1→v2→v3）
  - 测试gap检测（缺少v2的migration时返回错误）
  - 测试trybuild：migration from版本不对时编译失败

- [ ] **Step 6: Commit**
  - `feat(axiom-core): SchemaMigrator with automatic migration chain application, chain gap detection, proc macro compile-time version check`

---

## Task 6: Schema 验证集成和 Validator 工具

**Files:**
- Modify: `crates/axiom-core/src/schema.rs`

**Problem:** Schema trait 已定义，validators 模块有工具函数，但缺少：(1) Schema 与 Signal 的自动关联（Signal: Schema 已约束，但validate需要在发送时自动调用）；(2) 常用validator更丰富；(3) ValidationResult的serde支持。

- [ ] **Step 1: 完善 ValidationResult**
  - 添加 `fn merge(&mut self, other: ValidationResult)` 方法
  - 添加 `fn ok() -> Self`（已有）
  - 添加 `fn from_errors(errors: Vec<ValidationError>) -> Self`
  - 实现 `std::ops::AddAssign` 用于方便合并
  - 添加 `fn is_ok(&self) -> bool`
  - 添加 `fn into_result(self) -> Result<()>` 将ValidationError转为AxiomError::SignalValidation

- [ ] **Step 2: 添加常用 validators**
  - `require_non_empty(value: &str, field: &str) -> ValidationResult`
  - `require_max_length(value: &str, max: usize, field: &str) -> ValidationResult`
  - `require_min_length(value: &str, min: usize, field: &str) -> ValidationResult`
  - `require_range<T: PartialOrd + Display>(value: T, min: T, max: T, field: &str) -> ValidationResult`
  - `require_pattern(value: &str, regex: &str, field: &str) -> ValidationResult`
  - `require_non_negative(value: i64, field: &str) -> ValidationResult`
  - `require_array_max_length<T>(value: &[T], max: usize, field: &str) -> ValidationResult`

- [ ] **Step 3: 添加 DynamicSchema JSON Schema 支持**
  - DynamicSchema 可以从 JSON Schema 定义创建
  - 提供 `fn validate_against_schema(value: &Value, schema: &Value) -> ValidationResult`
  - 使用 serde_json 进行JSON Schema校验（不需要额外依赖，自己实现简单校验）

- [ ] **Step 4: 测试**
  - 所有validator单元测试
  - ValidationResult合并测试
  - DynamicSchema校验测试

- [ ] **Step 5: Commit**
  - `feat(axiom-core): complete Schema validation with common validators, ValidationResult merge, DynamicSchema JSON validation`

---

## Task 7: EntropyScore 完善和指标定义

**Files:**
- Modify: `crates/axiom-core/src/entropy.rs`

**Problem:** EntropyScore 已有基本结构，但需要：(1) 明确定义熵贡献因子；(2) 添加熵值序列化；(3) 添加单元测试验证熵值计算的正确性。

- [ ] **Step 1: 定义熵贡献因子权重**
  - dropped_messages: 权重 1.0（被Mailbox丢弃的消息）
  - rejected_by_guardian: 权重 2.0（被架构守卫拦截的消息）
  - axiom_violations: 权重 3.0（公理违规）
  - cell_restarts: 权重 5.0（Cell崩溃重启）
  - circuit_breaks: 权重 4.0（熔断触发）
  - timeouts: 权重 1.5（超时）
  - duplicate_messages: 权重 0.5（重复消息）
  - stale_state_violations: 权重 2.0（过时状态）

- [ ] **Step 2: 添加时间衰减**
  - 熵值随时间自然衰减（半衰期：5分钟）
  - `fn decay(&mut self, elapsed_secs: f64)`
  - 使用指数衰减：`score = score * 0.5^(elapsed/half_life)`

- [ ] **Step 3: 添加阈值常量**
  - GREEN_THRESHOLD: 0.4
  - YELLOW_THRESHOLD: 0.8
  - RED_THRESHOLD: 1.5（超过则触发治理）
  - CRITICAL_THRESHOLD: 3.0（超过则紧急熔断）

- [ ] **Step 4: 添加 EntropySnapshot 可序列化**
  - 添加 Serialize/Deserialize derive
  - 添加 per-cell entropy breakdown

- [ ] **Step 5: 测试**
  - 测试熵值累加正确
  - 测试时间衰减
  - 测试阈值判断
  - 测试高贡献因子的熵值增长更快

- [ ] **Step 6: Commit**
  - `feat(axiom-core): complete EntropyScore with weighted factors, time decay, thresholds, and serializable snapshots`

---

## Task 8: 完善 CellHandle 类型擦除句柄

**Files:**
- Modify: `crates/axiom-core/src/cell.rs`（或新增cell_handle.rs）

**Problem:** Runtime 需要存储和调度任意类型的 Cell，需要类型擦除的 CellHandle。

- [ ] **Step 1: 定义 DynCell 对象安全 trait**
  ```rust
  pub trait DynCell: Send + 'static {
      fn id(&self) -> &CellId;
      fn layer(&self) -> Layer;
      fn supervision_strategy(&self) -> SupervisionStrategy;
      fn meta(&self) -> CellMeta;
      fn state_hash(&self) -> Option<[u8; 32]>;
      fn as_any(&self) -> &dyn std::any::Any;
      fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
  }
  ```
  - 为所有 `Cell` 类型提供 blanket impl

- [ ] **Step 2: 定义 CellHandle**
  ```rust
  pub struct CellHandle {
      inner: Box<dyn DynCell>,
  }
  ```
  - 提供 downcast_ref/downcast_mut 方法
  - 实现 Deref to DynCell

- [ ] **Step 3: 测试**
  - 测试CellHandle创建和downcast
  - 测试DynCell的blanket impl对任意Cell类型工作

- [ ] **Step 4: Commit**
  - `feat(axiom-core): DynCell object-safe trait and CellHandle type-erased wrapper for runtime scheduling`

---

## Task 9: 完善 hello_cell 示例和端到端集成测试

**Files:**
- Modify: `crates/axiom-core/examples/hello_cell.rs`
- Create: `crates/axiom-core/tests/integration_tests.rs`

- [ ] **Step 1: 更新 hello_cell 示例**
  - 使用 `#[derive(SignalPayload)]` 定义 GreetCommand 和 GreetedEvent
  - 使用 `#[cell("exec")]` 标记 GreeterCell
  - 使用 `#[axiom]` 定义 NoNegativeAmount 公理
  - 在 handle 中使用 ctx.emit_event() 发送事件
  - 使用 ctx.emit_success() 产生Witness

- [ ] **Step 2: 编写端到端集成测试**
  - 测试Signal从创建到发送到Witness产生的完整流程（不依赖runtime，纯core层测试）
  - 测试Schema验证在发送时自动触发
  - 测试Axiom check在状态变更时触发
  - 测试Migration链自动发现

- [ ] **Step 3: 运行完整验证**
  - `cargo build --workspace` 零警告
  - `cargo clippy --workspace --all-targets -- -D warnings` 零警告
  - `cargo fmt --all -- --check` 通过
  - `cargo test --workspace` 全部通过
  - `cargo doc --no-deps -p axiom-core` 无警告

- [ ] **Step 4: Commit**
  - `feat(axiom-core): updated hello_cell example with all macros, end-to-end integration tests`

---

## P1 阶段验收标准

| # | 验收项 | 验证方式 |
|---|--------|---------|
| 1 | cargo build --workspace 零警告 | 命令行验证 |
| 2 | cargo clippy --workspace -D warnings 零警告 | 命令行验证 |
| 3 | cargo test --workspace 全部通过（≥35个测试） | 命令行验证 |
| 4 | #[derive(SignalPayload)] 完整可用 | 集成测试验证 |
| 5 | CellContext.send/emit/reply 正确工作 | 单元测试验证 |
| 6 | Witness 自动包含VersionInfo和signal_fingerprint | 单元测试验证 |
| 7 | Axiom 通过linkme自动注册，按层构建AxiomChain | 单元测试验证 |
| 8 | SchemaMigrator 自动迁移+gap检测 | 单元测试验证 |
| 9 | 层特化Context编译期约束 | trybuild测试验证 |
| 10 | hello_cell示例编译运行（不依赖runtime的部分） | cargo run --example hello_cell |
| 11 | axm check通过（除了branch check） | 命令行验证 |
| 12 | cargo doc 编译无警告 | 命令行验证 |
