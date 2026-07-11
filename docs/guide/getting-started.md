# 快速开始

本指南将帮助你快速上手 Axiom Core，从零开始构建你的第一个 Agent。

---

## 1. 环境准备

### 1.1 安装 Rust

Axiom Core 需要 Rust 1.70+：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 1.2 创建项目

```bash
cargo new my-axiom-app
cd my-axiom-app
```

### 1.3 添加依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
axiom-kernel = { path = "../crates/axiom-kernel", features = ["sha2-id"] }
axiom-runtime = { path = "../crates/axiom-runtime" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

---

## 2. 定义 Signal

Signal 是类型化的不可变消息：

```rust
use axiom_kernel::{Signal, SignalKind, RuntimeTier};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, SignalPayload)]
#[signal(kind = "command", layer = "exec")]
#[schema_version(1)]
pub struct GreetingSignal {
    pub message: String,
}
```

---

## 3. 定义 Cell

Cell 是状态单元的基本抽象：

```rust
use axiom_kernel::*;

pub struct GreetingCell {
    greetings: Vec<String>,
}

impl GreetingCell {
    pub fn new() -> Self {
        Self {
            greetings: Vec::new(),
        }
    }
}

#[cell("exec")]
impl Cell for GreetingCell {
    type Message = GreetingSignal;
    type State = Vec<String>;

    fn id(&self) -> CellId {
        CellId::new("greeting-cell")
    }

    fn layer(&self) -> RuntimeTier {
        RuntimeTier::Exec
    }

    async fn handle(
        &mut self,
        signal: Self::Message,
        ctx: LayeredCellContext<'_, Self::Layer>,
    ) -> (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>) {
        self.greetings.push(signal.message.clone());
        println!("Received: {}", signal.message);

        let witness = ctx.emit_witness(TransitionOutcome::Success).await;
        (Ok(()), Vec::new(), vec![witness])
    }
}
```

---

## 4. 定义 Axiom

Axiom 是跨状态的不变量约束：

```rust
#[axiom]
pub struct NonEmptyGreetingAxiom;

impl Axiom<Vec<String>, GreetingSignal> for NonEmptyGreetingAxiom {
    fn name(&self) -> &'static str {
        "non-empty-greeting"
    }

    fn check(
        &self,
        _current: &Self::State,
        new: &Self::State,
        _msg: &Self::Message,
    ) -> Result<()> {
        if new.iter().any(|g| g.is_empty()) {
            return Err(KernelError::InvariantViolated {
                message: "greeting must not be empty".into(),
            });
        }
        Ok(())
    }

    fn applies_to_layer(&self, layer: RuntimeTier) -> bool {
        matches!(layer, RuntimeTier::Exec | RuntimeTier::Validate)
    }
}
```

---

## 5. 启动 Runtime

```rust
use axiom_runtime::*;

#[tokio::main]
async fn main() -> Result<()> {
    let runtime = AxiomRuntime::new(RuntimeConfig::default()).await?;

    runtime.register_cell(CellRegistration {
        id: CellId::new("greeting-cell"),
        layer: RuntimeTier::Exec,
        version: Version::new(1, 0, 0),
        supervision_strategy: SupervisionStrategy::Restart,
        cell: Some(Arc::new(Mutex::new(GreetingCell::new()))),
        factory: None,
    }).await?;

    runtime.start().await?;

    // 发送信号
    let signal = GreetingSignal {
        message: "Hello, Axiom!".to_string(),
    };
    runtime.submit_signal(&signal, None, RuntimeTier::Exec).await?;

    // 保持运行
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

---

## 6. 测试 Cell

直接构造 `CellContext` 与 `LayeredCellContext` 测试单个 Cell：

```rust
#[tokio::test]
async fn test_hello_cell() {
    let mut cell = GreetingCell::new();
    let cell_id = CellId::new("greeting-cell");
    let mut ctx = CellContext::new(&cell_id, RuntimeTier::Exec);

    let signal = GreetingSignal {
        message: "test".to_string(),
    };
    assert!(Schema::validate(&signal).is_valid());

    let layered = ctx.as_layered::<ExecTier>();
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
    let chain = DynAxiomChain::from_registry_for_layer(RuntimeTier::Exec);
    assert!(chain.count() > 0, "Exec 层应至少有一个 Axiom");

    let violations = chain.check_all(
        &Vec::<String>::new() as &dyn Any,
        &vec!["".to_string()] as &dyn Any, // 包含空字符串
        &GreetingSignal { message: "x".to_string() } as &dyn Any,
    );
    assert!(violations.iter().any(|v| v.axiom_name == "non-empty-greeting"));
}
```

---

## 7. 下一步

- 阅读 [核心概念](core-concepts.md) 深入理解五大原语
- 查看 [最佳实践](best-practices.md) 学习生产级用法
- 探索 [插件系统](../PLUGIN_SYSTEM.md) 扩展功能
- 阅读 [API 边界](../../docs/API_BOUNDARY.md) 了解稳定 API

---

## 常见问题

### Q: 如何调试 Cell？

A: 使用 `tracing`  crate 记录日志，或使用 `axm why` 命令查看 Witness 链。

### Q: 如何处理 Cell 崩溃？

A: 配置 `SupervisionStrategy`，Runtime 会自动重启失败的 Cell。

### Q: 如何扩展 Axiom？

A: 使用 `#[axiom]` 宏定义新的 Axiom，自动注册到全局注册表。