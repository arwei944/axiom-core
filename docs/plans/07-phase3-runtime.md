# Phase 3: 运行时完善 Implementation Plan

> **Goal:** 完善 axiom-runtime 的消息调度循环，使多Cell通信、消息路由、Witness持久化、Supervisor自愈、CircuitBreaker熔断完整可用。验收标准：注册多个Cell后可以互相发送消息，panic自动重启，超时自动熔断，Witness自动写入Store，消息经过ArchitectureGuardian层检查。

> **Baseline:** MessageBus 已有（拦截器链+RoutingTable+原子统计），Mailbox 已有（Semaphore容量控制），Supervisor+CircuitBreaker已有（catch_unwind+指数退避+熔断器状态机），ArchitectureGuardian已有（层检查+跳数限制+schema版本检查），EntropyGovernor已有（统计drops/rejections/restarts），AxiomRuntime骨架已有（启动preflight+健康监控）。**关键缺口：调度循环只pop不处理消息——缺少实际调用Cell::handle、消息路由分发、OutgoingEnvelope/Witness持久化、CellHandle管理。**

---

## Global Constraints

- axiom-runtime 依赖 axiom-core 和 axiom-store
- 所有跨Cell通信必须经过 MessageBus（不能直接调用）
- Cell 内的状态是私有的（Rust所有权保证），消息处理是串行的（per-cell 单任务）
- panic 必须被 catch_unwind 捕获，不能杀死整个runtime
- 所有消息必须经过所有注册的 BusInterceptor
- Witness 在每次 handle 后自动写入 EventStore
- cargo build/clippy/test 零警告

---

## Task 1: 完善 CellRegistration 和 CellHandle 管理

**Files:**
- Modify: `crates/axiom-runtime/src/runtime.rs`

- [ ] **Step 1: 定义 CellRegistration 内部状态**
  - CellRegistration 包含：
    - id: CellId
    - layer: Layer
    - mailbox: Arc<Mailbox>（该Cell的消息队列）
    - cell: Mutex<Box<dyn DynCell>>（Cell实例，Mutex保护串行访问）
    - supervisor: Supervisor（该Cell的监督器）
    - meta: CellMeta
    - state: AtomicU8（CellState编码）

- [ ] **Step 2: 实现 register_cell 方法**
  - `async fn register_cell(&self, cell: impl Cell) -> Result<CellId, AxiomError>`
  - 创建 CellRegistration，插入 RwLock<HashMap<CellId, CellRegistration>>
  - 创建 Mailbox（容量从 CellMeta 获取）
  - 创建 Supervisor（策略从 Cell::supervision_strategy 获取）
  - 为 Cell 分配调度任务（Tokio task）
  - 调用 Cell::on_start（带 CellContext），如果失败根据supervision策略处理

- [ ] **Step 3: 实现 unregister_cell 方法**
  - `async fn unregister_cell(&self, id: &CellId) -> Result<(), AxiomError>`
  - 调用 Cell::on_stop
  - 停止调度任务
  - 从 HashMap 中移除

- [ ] **Step 4: 添加 RoutingTable 自动注册**
  - register_cell 时自动在 MessageBus.RoutingTable 中注册该Cell的id→layer映射
  - unregister_cell 时移除

- [ ] **Step 5: 测试**
  - 测试注册Cell后可通过id查询
  - 测试注册重复id返回错误
  - 测试注销后不再接收消息

- [ ] **Step 6: Commit**
  - `feat(axiom-runtime): CellRegistration with mailbox/supervisor binding, register/unregister lifecycle`

---

## Task 2: 实现消息调度循环（核心）

**Files:**
- Modify: `crates/axiom-runtime/src/runtime.rs`

**Problem:** 当前runtime.rs的调度循环只pop消息但不调用Cell::handle。需要实现完整的"接收消息→调用handle→收集Witness和Outgoing→持久化→路由分发"循环。

- [ ] **Step 1: 定义 per-cell 消息处理任务**
  - 每个Cell注册时，spawn一个Tokio task运行消息循环：
    ```
    loop {
        let envelope = mailbox.pop().await;
        if envelope.is_none() { break; } // cell stopped
        let envelope = envelope.unwrap();

        // 1. hop limit检查
        if envelope.hop_count >= MAX_HOPS {
            // 记录LoopDetected错误，继续下一条
            continue;
        }

        // 2. 获取cell锁
        let mut cell_guard = cell.lock().await;

        // 3. 创建CellContext
        let mut ctx = CellContext::new(cell_id.clone(), layer);
        ctx.clock.merge(&envelope.vector_clock);
        ctx.prev_hash = last_witness_hash.clone();
        ctx.current_correlation_id = Some(envelope.correlation_id.clone());

        // 4. 反序列化消息到具体类型（需要类型注册）
        let signal = match deserialize_signal(&envelope) {
            Ok(s) => s,
            Err(e) => { /* 记录反序列化错误，可能需要版本迁移 */ continue; }
        };

        // 5. 调用Schema验证
        if let Err(validation_err) = signal.validate() {
            ctx.emit_failure("validation_failed", &validation_err.to_string());
            // 持久化witness后继续
            persist_witnesses(&ctx.witnesses).await;
            continue;
        }

        // 6. catch_unwind调用handle
        let handle_result = AssertUnwindSafe(
            cell_guard.handle(signal, &mut ctx)
        ).catch_unwind().await;

        // 7. 根据结果处理
        match handle_result {
            Ok(Ok(())) => {
                supervisor.record_success();
                ctx.emit_success("message_handled");
            }
            Ok(Err(e)) => {
                supervisor.record_failure();
                ctx.emit_failure("handle_error", &e.to_string());
            }
            Err(panic_info) => {
                let msg = panic_to_string(&panic_info);
                supervisor.record_failure();
                ctx.emit_failure("panic", &msg);
            }
        }

        // 8. 持久化Witnesses
        for witness in ctx.witnesses.drain(..) {
            last_witness_hash = Some(witness.hash.clone());
            event_store.append(witness_to_event(&witness)).await?;
        }

        // 9. 路由outgoing messages
        for out in ctx.outgoing.drain(..) {
            let next_env = SignalEnvelope {
                hop_count: envelope.hop_count + 1,
                ..build_next_envelope(out, cell_id, layer)
            };
            // 经过拦截器链...然后路由到目标mailbox
            dispatch_envelope(next_env).await;
        }

        // 10. 检查Supervisor决策（重启/熔断/停止）
        match supervisor.decision() {
            SupervisionDecision::Restart => {
                // 重置cell状态，调用on_start
            }
            SupervisionDecision::CircuitBreak(duration) => {
                // 熔断：sleep duration后进入half-open
            }
            SupervisionDecision::Stop => {
                break;
            }
            SupervisionDecision::Escalate => {
                // 通知Oversight层
            }
            SupervisionDecision::Continue => {}
        }
    }
    ```

- [ ] **Step 2: 实现消息反序列化和类型路由**
  - 需要一个 SignalTypeRegistry 映射 signal_type → 反序列化函数
  - Cell注册时，其Message类型自动注册到registry（通过proc macro辅助）
  - 当收到envelope时，根据signal_type查找反序列化函数
  - 如果版本不匹配，使用SchemaMigrator迁移到当前版本

- [ ] **Step 3: 实现 dispatch_envelope 路由逻辑**
  - 对每个 outgoing envelope，运行所有 BusInterceptor
  - 如果有 Reject 决策，产生 Witness(AxiomViolated)
  - 如果有 Redirect，修改target
  - 如果是 Allow：
    - 如果有target，查找对应Cell的Mailbox并push
    - 如果没有target（事件广播），发送到所有订阅该event_type的Cell
  - push到Mailbox时如果MailboxFull：
    - 记录到EntropyGovernor
    - 发送给DeadLetterQueue（ Oversight层处理）

- [ ] **Step 4: 定义 SignalTypeRegistry**
  ```rust
  pub struct SignalTypeRegistry {
      deserializers: HashMap<&'static str, DeserializeFn>,
  }
  type DeserializeFn = Arc<dyn Fn(&Value, SchemaVersion) -> Result<Box<dyn SignalDyn>, AxiomError> + Send + Sync>;
  ```
  - register_signal_type::<S: Signal + DeserializeOwned>() 方法
  - 由 `#[derive(SignalPayload)]` 宏自动注册（通过linkme）

- [ ] **Step 5: 测试**
  - 测试两个Cell之间发Command和Event
  - 测试Cell处理消息后产生Witness
  - 测试outgoing message正确路由到目标Cell
  - 测试hop_count递增
  - 测试MailboxFull时消息被reject（不panic）

- [ ] **Step 6: Commit**
  - `feat(axiom-runtime): core message dispatch loop with handle/catch_unwind/witness persistence/routing`

---

## Task 3: 完善 MessageBus 拦截器链集成

**Files:**
- Modify: `crates/axiom-runtime/src/bus.rs`
- Modify: `crates/axiom-runtime/src/guardian.rs`

**Problem:** BusInterceptor trait和ArchitectureGuardian已有，但需要确保：(1) 拦截器在dispatch时被正确调用；(2) 可以动态添加/移除拦截器；(3) 拦截器返回Reject时产生正确的Witness。

- [ ] **Step 1: 修复 intercept 签名和决策**
  - 当前 InterceptDecision 是 Allow/Reject/Redirect
  - Reject 应携带 reason 字段用于 Witness
  - Redirect 应携带 new_target 字段
  - Allow 应可携带修改后的envelope（如添加trace信息）

- [ ] **Step 2: 实现 intercept_chain 函数**
  - `fn run_interceptors(interceptors: &[Box<dyn BusInterceptor>], env: &mut SignalEnvelope) -> InterceptDecision`
  - 按顺序执行每个拦截器
  - 第一个Reject/Redirect短路返回
  - Allow继续执行下一个
  - 记录每个拦截器的决策到tracing

- [ ] **Step 3: 添加多个内置拦截器**
  - **HopLimitInterceptor**: hop_count >= 8 → Reject（已有）
  - **SchemaVersionInterceptor**: 检查接收方是否支持该schema版本
  - **EntropyInterceptor**: 熵值>=CRITICAL时Reject非关键消息
  - **IdempotencyInterceptor**: 基于msg_id去重（维护LRU缓存最近处理的msg_id）

- [ ] **Step 4: 完善 ArchitectureGuardian**
  - 当前已有 check_cross_layer_signal，补充：
    - 检查sender是否在RoutingTable中存在
    - 检查target是否存在（定向发送）
    - 记录详细的拒绝原因（用于Witness）

- [ ] **Step 5: 添加拦截器指标统计**
  - 每个拦截器统计：allow_count/reject_count/redirect_count
  - 可通过health()查询

- [ ] **Step 6: 测试**
  - 测试ArchitectureGuardian拦截跨层消息
  - 测试HopLimitInterceptor拦截8跳消息
  - 测试拦截器链短路（首个Reject短路）
  - 测试IdempotencyInterceptor去重

- [ ] **Step 7: Commit**
  - `feat(axiom-runtime): complete interceptor chain with HopLimit/SchemaVersion/Entropy/Idempotency interceptors, metrics`

---

## Task 4: 完善 Supervisor 和 CircuitBreaker

**Files:**
- Modify: `crates/axiom-runtime/src/supervisor.rs`

**Problem:** Supervisor和CircuitBreaker已有基本实现，需要确保：(1) 重启时正确重置Cell状态；(2) 熔断器half-open状态探测；(3) 指数退避有上限；(4) 重启次数耗尽后Stop/Escalate。

- [ ] **Step 1: 完善重启逻辑**
  - Restart策略时：
    1. 等待backoff_duration（指数退避：100ms, 200ms, 400ms...max 30s）
    2. 创建新的Cell实例（需要Factory模式，因为Cell被消费了）
      - **问题**: Cell实例在第一次handle时被move了，重启需要新建实例
      - **解决方案**: CellRegistration存储CellFactory（Fn() -> Box<dyn DynCell>），而非直接存储Box<dyn DynCell>
    3. 调用on_start
    4. 重置连续失败计数

- [ ] **Step 2: 添加 CellFactory**
  ```rust
  pub type CellFactory = Arc<dyn Fn() -> Box<dyn DynCell> + Send + Sync>;
  ```
  - 修改register_cell接收factory而非直接接收cell实例
  - 首次启动和重启都调用factory()创建新实例

- [ ] **Step 3: 完善 CircuitBreaker half-open 探测**
  - Open状态持续timeout后进入HalfOpen
  - HalfOpen状态：允许1条消息通过（探测）
    - 如果处理成功 → 转为Closed，重置统计
    - 如果处理失败 → 回到Open，timeout翻倍
  - 连续成功N次（默认3）完全关闭

- [ ] **Step 4: 重启次数限制**
  - Restart { max_retries } 策略：连续重启超过max_retries后
    - 如果有Escalate策略，Escalate给Oversight
    - 否则Stop
  - 成功处理一条消息后重置连续重启计数

- [ ] **Step 5: 添加 SupervisionDecision 扩展**
  - 当前已有 Restart/Stop/Escalate/CircuitBreak
  - 添加 Delay(duration)：等待一段时间后重试（不创建新实例）
  - 添加 Resume：熔断恢复后继续

- [ ] **Step 6: 测试**
  - 测试Cell panic后被自动重启
  - 测试指数退避延迟递增
  - 测试熔断打开后消息被拒绝
  - 测试half-open探测成功恢复
  - 测试max_retries耗尽后Stop
  - 测试成功消息重置失败计数

- [ ] **Step 7: Commit**
  - `feat(axiom-runtime): Supervisor with CellFactory for restart, complete CircuitBreaker half-open probe, backoff limits`

---

## Task 5: 完善 EntropyGovernor 运行时集成

**Files:**
- Modify: `crates/axiom-runtime/src/entropy_gov.rs`

- [ ] **Step 1: 将 EntropyGovernor 连接到 Runtime 的各个事件源**
  - Mailbox reject → record_drop()
  - ArchitectureGuardian reject → record_rejection()
  - Supervisor restart → record_restart()
  - Circuit break → record_circuit_break()
  - 消息处理超时 → record_slow_handoff()
  - 拦截器链中自动调用对应record方法

- [ ] **Step 2: 实现熵值治理动作**
  - GREEN: 正常运行
  - YELLOW: 日志告警，开始采样（记录更多诊断信息）
  - RED: 触发治理动作：
    1. 拒绝非关键路径消息（如Query类）
    2. 通知EntropyGovernor Cell（Oversight层）分析原因
    3. 增加backpressure：Mailbox容量临时减小
  - CRITICAL: 紧急熔断：
    1. 拒绝所有非Oversight层消息
    2. 通知Supervisor对高熵Cell进行重启
    3. 产生CriticalEntropy Witness

- [ ] **Step 3: 添加冷却期机制**
  - 每次治理动作后进入冷却期（默认30s），冷却期内不再触发新的治理动作
  - 防止治理动作本身引起更多熵（正反馈环路）

- [ ] **Step 4: 添加 per-cell 熵值追踪**
  - 不仅全局熵值，每个Cell独立计算熵值
  - 热点Cell检测：per-cell熵值 > 全局平均 * 2

- [ ] **Step 5: 测试**
  - 测试熵值随drop/rejection/restart正确累加
  - 测试阈值触发治理动作
  - 测试冷却期防止正反馈
  - 测试时间衰减

- [ ] **Step 6: Commit**
  - `feat(axiom-runtime): EntropyGovernor integrated with all event sources, per-cell entropy, threshold-based governance actions`

---

## Task 6: 实现 Runtime 启动和关闭流程

**Files:**
- Modify: `crates/axiom-runtime/src/runtime.rs`

- [ ] **Step 1: 实现 RuntimeBuilder**
  ```rust
  pub struct RuntimeBuilder {
      config: RuntimeConfig,
      interceptors: Vec<Box<dyn BusInterceptor>>,
      event_store: Option<Arc<dyn EventStore>>,
      snapshot_store: Option<Arc<dyn SnapshotStore>>,
      auto_register_builtin_cells: bool,
  }
  ```
  - `fn new() -> Self`
  - `fn with_event_store(mut self, store: Arc<dyn EventStore>) -> Self`
  - `fn with_config(mut self, config: RuntimeConfig) -> Self`
  - `fn add_interceptor(mut self, i: Box<dyn BusInterceptor>) -> Self`
  - `fn with_auto_register_builtins(mut self, b: bool) -> Self`
  - `async fn build(self) -> Result<AxiomRuntime, AxiomError>`

- [ ] **Step 2: build() 中的启动preflight检查**
  1. 验证迁移链完整性（调用 axiom-store 的 validate_migration_chains_at_startup）
  2. 注册内置拦截器（ArchitectureGuardian, HopLimit, SchemaVersion等）
  3. 创建 MessageBus / EntropyGovernor
  4. 如果auto_register_builtins，注册Oversight层内置Cells（EntropyGovernorCell等）
  5. 启动健康监控后台任务
  6. 启动熵值衰减后台任务（每秒衰减一次）

- [ ] **Step 3: 实现优雅关闭**
  - `async fn shutdown(&self, timeout: Duration) -> Result<(), AxiomError>`
  - 停止接受新消息
  - 等待所有Cell处理完Mailbox中的已有消息（最多timeout时间）
  - 对每个Cell调用on_stop
  - 写入最终Witness到EventStore
  - 取消所有后台任务

- [ ] **Step 4: 实现 publish_command 便捷方法**
  - `async fn publish_command<S: Signal>(&self, target: CellId, signal: S) -> Result<(), AxiomError>`
  - 创建SignalEnvelope，通过MessageBus路由到目标Cell

- [ ] **Step 5: 实现 health() 方法**
  - 返回 RuntimeHealth：全局状态、各Cell状态、熵值、消息统计、存储健康

- [ ] **Step 6: 测试**
  - 测试启动preflight成功和失败场景
  - 测试优雅关闭：处理完队列中消息再退出
  - 测试超时强制关闭
  - 测试publish_command端到端

- [ ] **Step 7: Commit**
  - `feat(axiom-runtime): RuntimeBuilder with preflight checks, graceful shutdown, health endpoint, publish_command helper`

---

## Task 7: 实现 Dead Letter Queue 和消息循环检测

**Files:**
- Create: `crates/axiom-runtime/src/dlq.rs`
- Create: `crates/axiom-runtime/src/loop_detector.rs`

- [ ] **Step 1: 实现 DeadLetterQueue**
  - 存储无法投递的消息（MailboxFull/target not found/validation failed）
  - DLQ 有容量限制（默认1000），超过则丢弃最老的
  - DLQ中的消息可以被Oversight层消费和分析
  - 提供 `drain_dead_letters() -> Vec<DeadLetter>` 方法

- [ ] **Step 2: 实现 LoopDetector**
  - 基于correlation_id追踪消息链路
  - 如果同一个correlation_id的消息经过的Cell超过阈值（如10个不同Cell），触发LoopDetected错误
  - 使用LRU缓存追踪最近的correlation_id链路
  - 检测到循环时：Reject消息 + Witness记录 + 通知Oversight

- [ ] **Step 3: 将DLQ和LoopDetector集成为BusInterceptor**
  - DlqInterceptor：当MailboxFull时，将消息转移到DLQ而非直接丢弃
  - LoopDetectInterceptor：检查循环

- [ ] **Step 4: 测试**
  - 测试无法投递的消息进入DLQ
  - 测试DLQ容量限制
  - 测试循环消息被检测和拒绝
  - 测试正常的长链式消息不误报

- [ ] **Step 5: Commit**
  - `feat(axiom-runtime): DeadLetterQueue for undeliverable messages, LoopDetector with correlation tracking, DLQ/LoopDetect interceptors`

---

## Task 8: Timeout 处理和慢消息检测

**Files:**
- Modify: `crates/axiom-runtime/src/runtime.rs`
- Modify: `crates/axiom-runtime/src/mailbox.rs`

- [ ] **Step 1: 实现 handle 超时**
  - 调用 Cell::handle 时使用 tokio::time::timeout
  - 默认超时：30秒（可通过CellMeta配置）
  - 超时后：
    - 被视为失败（record_failure + Witness记录）
    - 注意：Rust中无法取消正在运行的Future，超时只是停止等待，Cell可能继续运行
    - 如果Cell在超时后返回结果，记录"late completion"但不重复处理

- [ ] **Step 2: 实现慢消息检测**
  - 处理时间超过 warning_threshold（默认1秒）时，记录 tracing::warn
  - 记录慢消息的signal_type/cell_id/duration_ms到EntropyGovernor
  - 慢消息不中断处理，仅记录

- [ ] **Step 3: 添加每个Cell的消息超时配置**
  - CellMeta 中添加 `handle_timeout: Option<Duration>` 字段
  - 默认使用RuntimeConfig的default_timeout

- [ ] **Step 4: 测试**
  - 测试超时消息被正确记录和处理
  - 测试慢消息产生warn日志
  - 测试超时后cell继续完成不导致崩溃

- [ ] **Step 5: Commit**
  - `feat(axiom-runtime): per-cell handle timeout with tokio::time::timeout, slow message detection and reporting`

---

## P3 阶段验收标准

| # | 验收项 | 验证方式 |
|---|--------|---------|
| 1 | cargo build -p axiom-runtime 零警告 | 命令行验证 |
| 2 | cargo test -p axiom-runtime 全部通过（≥35个测试） | 命令行验证 |
| 3 | 注册Cell后消息可从A Cell发送到B Cell | 集成测试 |
| 4 | 消息经过所有BusInterceptor，ArchitectureGuardian拦截跨层 | 单元+集成测试 |
| 5 | Cell panic被catch_unwind捕获，自动重启 | 单元测试 |
| 6 | CircuitBreaker Open/HalfOpen/Closed状态机完整 | 单元测试 |
| 7 | Witness在每次handle后自动写入EventStore | 集成测试 |
| 8 | 消息超时被检测和处理 | 单元测试 |
| 9 | EntropyGovernor连接到所有事件源 | 集成测试 |
| 10 | DLQ收集无法投递的消息 | 单元测试 |
| 11 | LoopDetector检测消息循环 | 单元测试 |
| 12 | 优雅关闭：处理完已有消息再退出 | 集成测试 |
| 13 | RuntimeBuilder preflight检查 | 单元测试 |
| 14 | cargo clippy/test/fmt全部通过 | axm check |
