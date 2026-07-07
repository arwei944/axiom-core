# 最佳实践

本指南总结使用 Axiom Core 构建生产级智能体系统时的架构设计原则、性能优化建议、安全实践、错误处理模式与测试策略。这些建议源自 Axiom Core 的设计哲学——**架构就是一切，模型会犯错，架构不能**。

---

## 目录

- [架构设计原则](#架构设计原则)
- [性能优化建议](#性能优化建议)
- [安全实践](#安全实践)
- [错误处理模式](#错误处理模式)
- [测试策略](#测试策略)
- [生产部署检查清单](#生产部署检查清单)

---

## 架构设计原则

### 原则 1：确定性分层，不让 LLM 越权

Axiom Core 的四层架构不是装饰，而是确定性的边界。核心准则：

- **能确定的事不放给 LLM**：DB 操作、数值计算、权限判断放在 Exec/Validate 层，用确定性代码完成。
- **LLM 的输出必经校验**：Agent 层（Layer 3）产出的 Signal 必须经过 Validate 层（Layer 2）的 Schema 与 Axiom 校验，才能下发到 Exec 层执行。
- **监督层不碰业务**：Oversight 层只看熵值、合规、健康，不执行业务逻辑。

```
✅ 推荐：Agent 产出 Command → Validate 校验 → Exec 幂等执行
❌ 反模式：Agent 直接调用 DB 写入，跳过校验
```

### 原则 2：让 Cell 小而专注

每个 Cell 应只负责一个明确职责，状态尽量小。参考 Erlang 进程哲学：成千上万个轻量 Cell，而非少数几个巨型 Cell。

| ✅ 推荐 | ❌ 反模式 |
|--------|---------|
| `UserCell` 只管用户状态 | 一个 `GodCell` 管用户+订单+支付 |
| `OrderCell` 只管订单 | Cell 内部直接调用外部 HTTP |
| 每个 Cell 单一 `Message` 类型 | 一个 Cell 接收 5 种信号 |

### 原则 3：用 Axiom 守住业务不变量

`Schema` 校验单条信号的字段，`Axiom` 校验跨信号的状态不变量。两者互补，不可替代。

```rust
// Schema：校验单条命令字段（数据层）
impl Schema for TransferCommand {
    fn validate(&self) -> ValidationResult {
        let mut r = ValidationResult::ok();
        r += validators::require_non_empty("account", &self.account);
        r += validators::require_in_range("amount", self.amount, 0.01, 1_000_000.0);
        r
    }
}

// Axiom：校验状态不变量（业务层）
#[axiom]
struct BalanceNonNegativeAxiom;

impl Axiom for BalanceNonNegativeAxiom {
    type State = AccountState;
    type Message = TransferCommand;

    fn name(&self) -> &'static str { "balance-non-negative" }

    fn check(&self, _current: &Self::State, new: &Self::State, _msg: &Self::Message) -> Result<()> {
        if new.balance < 0.0 {
            return Err(AxiomError::InvariantViolated {
                message: format!("balance {} must not be negative", new.balance),
            });
        }
        Ok(())
    }

    fn violation_action(&self) -> ViolationAction {
        ViolationAction::Reject // 余额不足直接拒绝
    }
}
```

### 原则 4：按需选择 ViolationAction

`ViolationAction` 决定违反 Axiom 后的处置，按场景选择：

| Action | 适用场景 | 风险 |
|--------|---------|------|
| `Reject` | 硬性业务约束（余额、权限） | 低，最安全 |
| `Warn` | 软性提示（长度、风格） | 中，可能被忽略 |
| `CircuitBreak` | 反复失败时熔断保护 | 中，需配 reset 时间 |
| `Rollback` | 可回滚的状态变更 | 高，需确保回滚幂等 |

> **建议**：默认用 `Reject`，只有当你明确需要"放行但告警"时才用 `Warn`。`CircuitBreak` 与 `Rollback` 要配套测试。

### 原则 5：善用 Witness 链做可观测性

每次状态转换都会产出 Witness。生产中应把 Witness 链接入：

- **日志聚合**：按 `correlation_id` 串联一次完整调用。
- **告警**：`TransitionOutcome::AxiomViolated` 直接触发告警。
- **回放**：Witness + 事件日志可重建任意时刻状态。
- **熵值监控**：违规次数喂给 `EntropyScore`。

```rust
// 在 Cell 的 handle 内务必产出 Witness
ctx.emit_witness(
    ctx.witness()
        .summary("transfer executed")
        .outcome(TransitionOutcome::Success)
        .processing_time_us(elapsed)
)?;
```

---

## 性能优化建议

### 建议 1：合理设置 Witness 采样率

Witness 记录有开销（哈希计算、序列化）。在高频 Cell 中可降低采样率：

```rust
// 仅 10% 的转换记录详细 Witness
ctx.set_sample_rate(0.1);
```

> **权衡**：采样率越低，可观测性越弱。建议生产中关键 Cell（涉及资金、权限）保持 1.0，辅助 Cell 可降到 0.1。

### 建议 2：控制工作记忆 token 预算

`AgentCell` 的工作记忆按 token 预算裁剪。预算过大会拖慢 LLM，过小会丢失上下文：

```rust
// 经验值：简单问答 2000-4000，复杂推理 6000-8000
.with_memory_budget(4000)
.with_auto_summarize(true) // 超预算时自动摘要，而非截断
```

### 建议 3：共享 LlmClient 与 ToolRegistry

多个 Agent 共享同一 LLM provider 时，用 `Arc` 共享客户端，避免重复连接池：

```rust
use std::sync::Arc;

let llm = Arc::new(LlmClient::mock());

let agent1 = AgentBuilder::new("a1").with_llm_arc(llm.clone()).build()?;
let agent2 = AgentBuilder::new("a2").with_llm_arc(llm.clone()).build()?;
```

`ToolRegistry` 同理可用 `with_tools_arc` 共享。

### 建议 4：限制规划器迭代次数

ReAct 规划器可能陷入循环。务必设置 `max_iterations` 上限：

```rust
.with_max_iterations(5) // 超过 5 次强制停止
```

或在自定义规划器上：

```rust
let planner = Arc::new(ReActPlanner::new().with_max_iterations(5));
```

### 建议 5：用 Lens 替代全量历史

避免把全部历史塞进 LLM 上下文。用 Lens 从事件日志按需投影：

```
❌ 把 1000 条历史消息全部渲染进 prompt
✅ 用 Lens 投影"最近 5 条 + 相关摘要"，token 减少 90%
```

### 建议 6：限制信号载荷大小

`SignalEnvelope` 支持 payload 大小校验，防止大对象打爆消息总线：

```rust
env.validate_payload_size(64 * 1024)?; // 最大 64KB
```

在 Schema 中也可定义 `max_size_bytes`：

```rust
impl Schema for BigPayload {
    fn max_size_bytes() -> usize { 1024 * 1024 } // 1MB
}
```

### 建议 7：VectorClock 增量而非全量合并

`VectorClock::merge` 是 O(n) 操作。高频 Cell 应避免频繁合并巨型 clock，可在批处理边界统一合并。

---

## 安全实践

### 实践 1：永远校验 Schema，不信任输入

所有外部输入（用户、LLM 输出、外部 API）必须经过 `Schema::validate`。Axiom Core 在 `emit_internal` 中会自动调用校验，但你也应在边界显式校验：

```rust
let signal = HelloCommand::new(user_input);
let validation = axiom_kernel::Schema::validate(&signal);
if !validation.is_valid() {
    return Err(MyError::InvalidInput(validation.to_string()));
}
```

### 实践 2：工具调用设置权限

`ToolInfo::required_permission` 字段用于权限控制。生产中应把工具与权限绑定：

```rust
let info = ToolInfo {
    name: "delete_user".to_string(),
    // ...
    required_permission: Some("admin".to_string()), // 需要 admin 权限
    // ...
};
```

执行前由 `ToolRegistry` 校验调用方权限。

### 实践 3：LLM 输出当不可信数据

LLM 输出是非确定性的，必须当作不可信数据：

```
LLM 输出 JSON
   │
   ▼ 反序列化为强类型 Signal
Validate 层 Schema 校验（字段类型、范围、必填）
   │
   ▼ 校验通过
Axiom 校验（业务不变量）
   │
   ▼ 校验通过
Exec 层执行（幂等 + 重试）
```

绝不要把 LLM 原始输出直接传给 DB 或 API。

### 实践 4：限制信号跳数

`SignalEnvelope` 默认限制 8 跳，防止消息无限转发导致死循环。若需调整，确保有更严格的超时与熔断配套：

```rust
// 默认 MAX_HOPS = 8，超过返回 HandoffLimitExceeded
env.increment_hop()?;
```

### 实践 5：DisclosureLevel 渐进披露

对外暴露的 Agent 应使用 `DisclosureLevel::Minimal`，避免泄露内部身份信息：

```rust
// 对外 API：只暴露名字
.with_disclosure_level(DisclosureLevel::Minimal)

// 内部调试：暴露全部
.with_disclosure_level(DisclosureLevel::Transparent)
```

### 实践 6：Witness 哈希链防篡改

Witness 的 SHA-256 哈希链让审计记录不可篡改。生产中应：

- 定期用 `Witness::verify_chain_integrity` 校验完整性。
- 把 Witness 持久化到只追加存储（`axiom-store`）。
- 任何 `prev_hash` 不匹配都应触发告警。

```rust
if !Witness::verify_chain_integrity(&witnesses) {
    alert!("Witness chain integrity broken!");
}
```

### 实践 7：熵值熔断兜底

当系统熵值进入 Red/Critical，应主动降级或停机：

```rust
let entropy = EntropyScore::new();
// ... 累计违规 ...
if entropy.is_critical() {
    // 紧急停机，等待人工介入
    supervisor.shutdown()?;
}
if entropy.is_red() {
    // 触发熔断，暂停非关键路径
    circuit_breaker.trip()?;
}
```

| 熵值级别 | 阈值 | 建议动作 |
|---------|------|---------|
| Green | < 0.4 | 正常运行 |
| Yellow | 0.4–0.8 | 告警，加强监控 |
| Red | ≥ 0.8 | 熔断非关键路径 |
| Critical | ≥ 3.0 | 紧急停机 |

---

## 错误处理模式

### 模式 1：用 Result 显式传播，不 panic

Axiom Core 全程用 `Result<T, AxiomError>` 传播错误。Cell 内部不要 panic，而是返回错误让监督树处理：

```rust
fn handle<'a>(&'a mut self, signal: ..., ctx: LayeredCellContext<'a, Self::Layer>) -> ... {
    async move {
        let mut ctx = ctx;
        let result: Result<()> = (|| {
            // 业务逻辑
            let data = self.fetch(&signal.key).map_err(|e| {
                AxiomError::InvariantViolated { message: e.to_string() }
            })?;
            ctx.emit_to::<ExecLayer, _>(ProcessEvent { data })?;
            ctx.emit_success("done")?;
            Ok(())
        })();
        let (outgoing, witnesses) = ctx.end_processing();
        (result, outgoing, witnesses)
    }
}
```

### 模式 2：区分可恢复与不可恢复错误

| 错误类型 | 例子 | 处理方式 |
|---------|------|---------|
| 可恢复 | 网络超时、临时锁冲突 | 指数退避重试（Exec 层） |
| 业务违规 | 余额不足、权限不足 | `Reject` + 返回错误给上游 |
| 数据错误 | Schema 校验失败 | `Reject`，记录 Witness |
| 系统错误 | 内存不足、磁盘满 | 让 Cell 崩溃，监督树重启 |

### 模式 3：监督策略匹配故障性质

```rust
fn supervision_strategy(&self) -> SupervisionStrategy {
    match self.role {
        // 临时性故障：重启有限次
        Role::Worker => SupervisionStrategy::Restart { max_retries: 3 },
        // 持续性故障：直接停止
        Role::Critical => SupervisionStrategy::Stop,
        // 反复失败：熔断
        Role::External => SupervisionStrategy::CircuitBreak {
            failure_threshold: 5,
            reset_after_ms: 60_000,
        },
        // 无法本地恢复：上抛给 Oversight
        Role::Unknown => SupervisionStrategy::Escalate,
    }
}
```

### 模式 4：Witness 记录失败原因

失败时务必产出带 `reason` 的 Witness，便于回溯：

```rust
let result: Result<()> = do_work().await;
match &result {
    Ok(_) => ctx.emit_success("operation succeeded")?,
    Err(e) => ctx.emit_failure("operation failed", &e.to_string())?,
}
```

Axiom 违规时用专用方法：

```rust
ctx.emit_axiom_violation("balance-non-negative", "balance went negative")?;
```

### 模式 5：AgentCell 的 Planner fallback

`AgentCell::query` 在 Planner 失败时会自动 fallback 到直接 LLM 调用，并记录错误统计。你也可以显式处理：

```rust
match agent.query("complex task").await {
    Ok(resp) => { /* ... */ },
    Err(AgentError::Llm(_)) => {
        // LLM 调用失败，可降级返回缓存或默认回复
    },
    Err(AgentError::NotStarted) => {
        // 生命周期错误，应修复启动流程
    },
    Err(e) => {
        // 其他错误，记录并告警
        tracing::error!(error = %e, "agent query failed");
    }
}
```

### 模式 6：错误类型一览

`AxiomError` 涵盖所有核心错误，便于精确匹配：

```rust
pub enum AxiomError {
    LayerViolation { from, to, signal_type, source_cell }, // 层间越界
    SignalValidation { signal_type, message },             // Schema 校验失败
    SignalSerialization { signal_type, message },          // 序列化失败
    InvariantViolated { message },                         // Axiom 违反
    WitnessSerialization { cell_id, message },             // Witness 序列化失败
    CorrelationBroken { message, correlation_id },         // 链路追踪断裂
    HandoffLimitExceeded { msg_id, hops, correlation_id }, // 跳数超限
    TypeMismatch { expected, actual },                     // DynAxiom 类型不匹配
    // ...
}
```

---

## 测试策略

### 策略 1：用 LlmClient::mock() 做无网络测试

`axiom-llm` 提供 `LlmClient::mock()`，返回确定性回复，适合单元/集成测试：

```rust
#[tokio::test]
async fn test_agent_query() {
    let agent = AgentBuilder::new("test")
        .with_llm(LlmClient::mock())
        .build_and_start()
        .unwrap();

    let resp = agent.query("Hello").await.unwrap();
    assert!(!resp.is_empty());
}
```

### 策略 2：分层测试——原语层、Cell 层、集成层

| 层次 | 测试对象 | 工具 |
|------|---------|------|
| 原语层 | Signal 校验、VectorClock、Witness 哈希 | `#[test]` 纯函数 |
| Cell 层 | 单个 Cell 的 handle 行为 | `#[tokio::test]` + 手动构造 ctx |
| 集成层 | 多 Cell 协作、Agent 全流程 | `#[tokio::test]` + mock LLM |
| 架构层 | 层间约束、编译期检查 | `trybuild` 编译失败测试 |

### 策略 3：编译失败测试（trybuild）

Axiom Core 用 `trybuild` 验证"非法层间调用根本无法编译"。仓库 `crates/axiom-macros/tests/compile-fail/` 包含此类测试：

```rust
// cf-cross-layer-call.rs —— 期望编译失败
fn main() {
    // Exec 层尝试向 Agent 层发消息，应编译失败
    let _ = ctx.emit_to::<AgentLayer, _>(signal);
}
```

配套 `.stderr` 文件断言错误信息。这是验证架构约束的最强手段。

### 策略 4：Cell 单元测试范式

直接构造 `CellContext` 与 `LayeredCellContext` 测试单个 Cell：

```rust
#[tokio::test]
async fn test_hello_cell() {
    let mut cell = HelloCell::new();
    let cell_id = CellId::new("hello-cell");
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);

    let signal = HelloCommand::new("test");
    assert!(Schema::validate(&signal).is_valid());

    let layered = ctx.as_layered::<ExecLayer>();
    let (result, _outgoing, witnesses) = cell.handle(signal, layered).await;
    result.unwrap();

    assert_eq!(cell.greetings, vec!["test"]);
    assert_eq!(witnesses.len(), 1);
    assert!(matches!(witnesses[0].0.outcome, TransitionOutcome::Success));
}
```

### 策略 5：Axiom 链测试

测试 Axiom 是否正确注册并按层过滤：

```rust
#[test]
fn test_axiom_registry() {
    let chain = DynAxiomChain::from_registry_for_layer(Layer::Exec);
    assert!(chain.count() > 0, "Exec 层应至少有一个 Axiom");

    let violations = chain.check_all(
        &Vec::<String>::new() as &dyn Any,
        &vec!["".to_string()] as &dyn Any, // 包含空字符串
        &HelloCommand::new("x") as &dyn Any,
    );
    assert!(violations.iter().any(|v| v.axiom_name == "non-empty-greeting"));
}
```

### 策略 6：Witness 链完整性测试

```rust
#[test]
fn test_witness_chain() {
    let witnesses = collect_witnesses_from_run();
    assert!(Witness::verify_chain_integrity(&witnesses),
        "Witness 链必须完整");
}
```

### 策略 7：熵值模型测试

测试违规是否正确推高熵值，reset 是否生效：

```rust
#[test]
fn test_entropy_escalation() {
    let mut e = EntropyScore::new();
    assert!(e.is_green());

    for _ in 0..3 {
        e.record_axiom_violation();
        e.record_cell_restart();
        e.record_circuit_break();
    }
    assert!(e.is_red() || e.is_critical());

    e.reset();
    assert!(e.is_green());
}
```

### 策略 8：并发与重启测试

Axiom Core 提供并发与持久化测试模板（见 `crates/axiom-kernel/tests/` 与 `crates/axiom-runtime/tests/`）。关键场景：

- **并发消息**：多个信号同时到达同一 Cell，验证状态一致性。
- **Cell 崩溃重启**：模拟 panic，验证监督树重启后状态恢复。
- **事件回放**：从 Witness + 事件日志重建状态，与原始状态比对。

### 策略 9：性能基准测试

`axiom-bench` crate 提供基准测试，用于回归性能：

```bash
# 运行消息总线、信箱、Witness 链的基准测试
cargo bench -p axiom-bench
```

关键指标：消息派发延迟、信箱吞吐、Witness 链验证耗时。

---

## 生产部署检查清单

部署 Axiom Core 系统前，逐项确认：

### 架构合规

- [ ] 所有 Cell 都用 `#[cell("...")]` 声明了正确层级。
- [ ] 层间调用方向符合 `CanSendTo` 矩阵（编译期已保证）。
- [ ] LLM 输出经过 Validate 层校验后才进入 Exec 层。
- [ ] 关键业务不变量都实现了 Axiom 并注册。

### 可观测性

- [ ] `tracing` 日志已配置，关键路径有 `info`/`debug` 日志。
- [ ] Witness 链持久化到只追加存储。
- [ ] 熵值监控接入告警（Yellow 告警、Red 熔断、Critical 停机）。
- [ ] `correlation_id` 贯穿日志，便于链路追踪。

### 安全

- [ ] 所有外部输入经过 `Schema::validate`。
- [ ] LLM 输出当作不可信数据，反序列化 + 校验后才使用。
- [ ] 危险工具设置了 `required_permission`。
- [ ] 对外 Agent 使用 `DisclosureLevel::Minimal`。
- [ ] Witness 哈希链定期校验完整性。

### 性能

- [ ] LLM/Tool 客户端用 `Arc` 共享。
- [ ] `max_iterations` 已设置上限。
- [ ] 高频 Cell 的 Witness 采样率合理调整。
- [ ] 信号 payload 大小有限制。
- [ ] 工作记忆 token 预算符合 LLM 上下文窗口。

### 错误处理

- [ ] Cell 内部不 panic，全部用 `Result` 传播。
- [ ] 监督策略匹配故障性质（Restart/Stop/Escalate/CircuitBreak）。
- [ ] 失败时产出带 reason 的 Witness。
- [ ] Exec 层关键操作幂等，支持重试。

### 测试

- [ ] 原语层单元测试覆盖 Schema/Axiom/VectorClock/Witness。
- [ ] Cell 层测试覆盖 handle 正常与异常路径。
- [ ] 集成测试用 `LlmClient::mock()` 覆盖全流程。
- [ ] `trybuild` 编译失败测试覆盖层间约束。
- [ ] 性能基准测试无回归。

### 生命周期

- [ ] `AgentCell` 显式 `start()`/`stop()`，避免资源泄漏。
- [ ] 优雅停止时打印统计日志。
- [ ] 重启策略经过测试（崩溃后状态可恢复）。

---

## 小结

Axiom Core 的最佳实践可以浓缩为五条：

1. **确定性分层**——能确定的别交给 LLM，LLM 输出必经校验。
2. **Cell 小而专注**——单一职责，状态隔离，故障不传染。
3. **Axiom 守底线**——硬约束用 `Reject`，软提示用 `Warn`，熔断用 `CircuitBreak`。
4. **Witness 全记录**——每次转换留痕，哈希链防篡改，按 correlation 追踪。
5. **熵值做兜底**——Green 放行、Yellow 告警、Red 熔断、Critical 停机。

遵循这些实践，你就能构建出低熵、可观测、可自愈的生产级智能体系统。更多设计细节参见 `docs/architecture/` 目录下的需求文档与配套设计文档。
