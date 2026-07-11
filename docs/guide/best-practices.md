# 最佳实践

本文档总结 Axiom Core 的生产级最佳实践，帮助开发者构建可靠、可维护的 Agent 系统。

---

## 1. Cell 设计

### 1.1 单一职责

每个 Cell 应该只负责一个明确的业务领域：

```rust
// ✅ 好的设计：单一职责
pub struct GreetingCell { /* 只处理问候 */ }
pub struct OrderCell { /* 只处理订单 */ }

// ❌ 避免：上帝对象
pub struct MegaCell { /* 处理所有事情 */ }
```

### 1.2 状态最小化

Cell 状态应该尽可能小，只保留必要信息：

```rust
// ✅ 好的设计：最小状态
pub struct GreetingCell {
    greetings: Vec<String>,
}

// ❌ 避免：冗余状态
pub struct GreetingCell {
    greetings: Vec<String>,
    all_greetings_ever: Vec<String>,  // 重复存储
    greeting_count: usize,            // 可从 greetings 推导
}
```

### 1.3 监督策略选择

根据业务重要性选择合适的监督策略：

| 策略 | 适用场景 | 示例 |
|------|----------|------|
| `Restart` | 无状态或可重建状态 | 计算单元、纯函数 Cell |
| `Resume` | 有状态但可跳过错误 | 日志收集、统计 Cell |
| `Stop` | 关键状态，停止更安全 | 支付处理、账户管理 |
| `Escalate` | 需要人工介入 | 异常处理、告警 Cell |

---

## 2. Signal 设计

### 2.1 明确 Signal 类型

每个 Signal 应该有明确的语义和用途：

```rust
// ✅ 好的设计：明确的命令类型
#[signal(kind = "command", layer = "exec")]
pub struct CreateOrderCommand {
    pub order_id: OrderId,
    pub items: Vec<OrderItem>,
}

// ❌ 避免：万能消息
#[signal(kind = "command", layer = "exec")]
pub struct GenericMessage {
    pub data: serde_json::Value,
}
```

### 2.2 使用 Schema 验证

启用 `schema_version` 和自动验证：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, SignalPayload)]
#[signal(kind = "command", layer = "exec")]
#[schema_version(1)]
pub struct CreateOrderCommand {
    #[validate(range(min = 1))]
    pub order_id: u64,

    #[validate(length(min = 1, max = 100))]
    pub customer_name: String,

    #[validate(custom = "validate_items")]
    pub items: Vec<OrderItem>,
}
```

---

## 3. Axiom 设计

### 3.1 命名规范

Axiom 命名应该清晰表达约束含义：

```rust
// ✅ 好的命名
pub struct NonNegativeBalanceAxiom;
pub struct OrderIdempotencyAxiom;
pub struct MaxConcurrentOrdersAxiom;

// ❌ 避免模糊命名
pub struct CheckAxiom;
pub struct RuleAxiom;
```

### 3.2 分层过滤

合理使用 `applies_to_layer` 控制 Axiom 生效范围：

```rust
impl Axiom<OrderState, OrderCommand> for NonNegativeBalanceAxiom {
    fn applies_to_layer(&self, layer: RuntimeTier) -> bool {
        // 余额检查只在 Exec 层执行
        matches!(layer, RuntimeTier::Exec)
    }
}

impl Axiom<OrderState, OrderCommand> for OrderIdempotencyAxiom {
    fn applies_to_layer(&self, layer: RuntimeTier) -> bool {
        // 幂等性检查在 Validate 和 Exec 层都执行
        matches!(layer, RuntimeTier::Validate | RuntimeTier::Exec)
    }
}
```

---

## 4. Witness 审计

### 4.1 重要操作必须记录 Witness

```rust
async fn handle(
    &mut self,
    signal: PaymentCommand,
    ctx: LayeredCellContext<'_, Self::Layer>,
) -> (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>) {
    // 执行操作
    let result = self.process_payment(&signal);

    // 记录 Witness
    let witness = ctx.emit_witness(match &result {
        Ok(_) => TransitionOutcome::Success,
        Err(e) => TransitionOutcome::Failure(e.to_string()),
    }).await;

    (result, Vec::new(), vec![witness])
}
```

### 4.2 定期验证 Witness 链

```rust
#[tokio::main]
async fn main() {
    let witnesses = witness_store.get_all().await;
    if let Err(e) = WitnessKernel::verify_chain(&witnesses) {
        tracing::error!("Witness chain verification failed: {}", e);
        // 触发告警或修复流程
    }
}
```

---

## 5. 测试策略

### 5.1 单元测试

直接构造 `CellContext` 测试单个 Cell：

```rust
#[tokio::test]
async fn test_hello_cell() {
    let mut cell = HelloCell::new();
    let cell_id = CellId::new("hello-cell");
    let mut ctx = CellContext::new(&cell_id, RuntimeTier::Exec);

    let signal = HelloCommand::new("test");
    assert!(Schema::validate(&signal).is_valid());

    let layered = ctx.as_layered::<ExecTier>();
    let (result, _outgoing, witnesses) = cell.handle(signal, layered).await;
    result.unwrap();

    assert_eq!(cell.greetings, vec!["test"]);
    assert_eq!(witnesses.len(), 1);
    assert!(matches!(witnesses[0].0.outcome, TransitionOutcome::Success));
}
```

### 5.2 Axiom 链测试

测试 Axiom 是否正确注册并按层过滤：

```rust
#[test]
fn test_axiom_registry() {
    let chain = DynAxiomChain::from_registry_for_layer(RuntimeTier::Exec);
    assert!(chain.count() > 0, "Exec 层应至少有一个 Axiom");

    let violations = chain.check_all(
        &Vec::<String>::new() as &dyn Any,
        &vec!["".to_string()] as &dyn Any, // 包含空字符串
        &HelloCommand::new("x") as &dyn Any,
    );
    assert!(violations.iter().any(|v| v.axiom_name == "non-empty-greeting"));
}
```

---

## 6. 错误处理

### 6.1 使用 Result 而不是 panic

```rust
// ✅ 好的做法：返回 Result
async fn handle(&mut self, signal: Self::Message, ctx: &mut CellContext) -> Result<()> {
    if self.greetings.len() >= 1000 {
        return Err(KernelError::ResourceExhausted {
            resource: "greetings".to_string(),
        }.into());
    }
    // ...
}

// ❌ 避免：直接 panic
async fn handle(&mut self, signal: Self::Message, ctx: &mut CellContext) {
    assert!(self.greetings.len() < 1000, "too many greetings");
}
```

### 6.2 记录错误上下文

```rust
match process_payment(&cmd).await {
    Ok(receipt) => {
        ctx.emit_witness(TransitionOutcome::Success).await;
    }
    Err(e) => {
        tracing::error!(cell = ?self.id(), order_id = ?cmd.order_id, error = ?e, "payment failed");
        ctx.emit_witness(TransitionOutcome::Failure(e.to_string())).await;
    }
}
```

---

## 7. 性能优化

### 7.1 使用 Arc 共享只读数据

```rust
// ✅ 好的做法：Arc 共享配置
pub struct ConfigCell {
    config: Arc<GlobalConfig>,
}

// ❌ 避免：每个 Cell 都复制配置
pub struct ConfigCell {
    config: GlobalConfig,  // 可能很大，复制开销高
}
```

### 7.2 批量处理

```rust
// ✅ 好的做法：批量发送信号
let signals = vec![signal1, signal2, signal3];
bus.publish_batch(&signals).await?;

// ❌ 避免：逐个发送
for signal in signals {
    bus.publish(&signal).await?;
}
```

---

## 8. 监控与可观测性

### 8.1 使用 tracing 记录关键事件

```rust
use tracing::{info, warn, error};

async fn handle(&mut self, signal: Self::Message, ctx: &mut CellContext) -> Result<()> {
    info!(cell = ?self.id(), signal_type = signal.signal_type(), "handling signal");

    let result = self.process(signal).await;

    match &result {
        Ok(_) => info!(cell = ?self.id(), "signal handled successfully"),
        Err(e) => error!(cell = ?self.id(), error = ?e, "signal handling failed"),
    }

    result
}
```

### 8.2 定期导出热图数据

```rust
let collector = HeatmapCollector::new();
// ... 运行一段时间后 ...
let data = collector.get_data();
let exporter = JsonExporter;
std::fs::write("heatmap.json", exporter.export(&data)?)?;
```

---

## 9. 部署建议

### 9.1 资源配置

| 资源 | 建议 | 说明 |
|------|------|------|
| CPU | 2+ 核 | Runtime 和 Cell 处理需要 CPU |
| 内存 | 512MB+ | 每个 Cell 约 1-10KB，加上 Witness 存储 |
| 磁盘 | 1GB+ | Witness 链和事件存储 |
| 网络 | 10Mbps+ | 分布式场景需要 |

### 9.2 监控指标

- **Cell 数量**：当前活跃 Cell 数
- **信号吞吐量**：每秒处理信号数
- ** Witness 增长率**：每秒新增 Witness 数
- **熵值分布**：各 Cell 熵值统计
- **错误率**：处理失败信号占比

---

## 10. 常见陷阱

### 10.1 避免在 Cell 中阻塞

```rust
// ❌ 避免：长时间阻塞
async fn handle(&mut self, signal: Self::Message, ctx: &mut CellContext) -> Result<()> {
    std::thread::sleep(Duration::from_secs(10));  // 阻塞整个 Runtime
    Ok(())
}

// ✅ 好的做法：异步等待
async fn handle(&mut self, signal: Self::Message, ctx: &mut CellContext) -> Result<()> {
    tokio::time::sleep(Duration::from_secs(10)).await;
    Ok(())
}
```

### 10.2 避免共享可变状态

```rust
// ❌ 避免：跨 Cell 共享
static mut GLOBAL_COUNTER: usize = 0;

// ✅ 好的做法：通过消息通信
struct CounterCell {
    count: usize,
}
```

### 10.3 避免过度使用 Axiom

Axiom 是硬约束，过多 Axiom 会导致性能下降：

```rust
// ✅ 合理的 Axiom：关键业务规则
pub struct NonNegativeBalanceAxiom;
pub struct OrderIdempotencyAxiom;

// ❌ 过度使用：每个字段都加 Axiom
pub struct NameNotEmptyAxiom;
pub struct EmailValidAxiom;
pub struct PhoneValidAxiom;
// ... 几十个 Axiom
```

---

## 总结

遵循这些最佳实践，你可以构建出：

1. **可靠**：通过监督树和 Axiom 保证系统稳定
2. **可维护**：清晰的职责分离和测试策略
3. **可观测**：完整的审计链和监控指标
4. **高性能**：合理的资源使用和批量处理
5. **可扩展**：易于添加新功能和 Cell