# Migration Guide

本文档帮助用户从旧版本迁移到 Axiom Core v0.4.0。

---

## 1. 版本概览

| 版本 | 主要变更 |
|------|----------|
| v0.3.0 | 基础架构，`axiom-core` crate |
| v0.4.0 | 核心原语迁移至 `axiom-kernel`，新增 WASM 插件系统，统一 `RuntimeTier` |

---

## 2. 迁移清单

### 2.1 更新 Cargo.toml

```toml
# v0.3.0
[dependencies]
axiom-core = "0.3"

# v0.4.0
[dependencies]
axiom-kernel = { path = "../crates/axiom-kernel", features = ["sha2-id"] }
axiom-runtime = { path = "../crates/axiom-runtime" }
```

### 2.2 更新导入路径

```rust
// v0.3.0
use axiom_core::*;

// v0.4.0
use axiom_kernel::*;
use axiom_runtime::*;
```

### 2.3 更新 Cell trait

```rust
// v0.3.0
impl Cell for MyCell {
    type Message = MyMessage;
    type Layer = Layer;

    fn layer(&self) -> Layer { Layer::Exec }
}

// v0.4.0
impl Cell for MyCell {
    type Message = MyMessage;
    type Layer = ExecTier;

    fn layer(&self) -> RuntimeTier { RuntimeTier::Exec }
}
```

### 2.4 更新 Signal trait

```rust
// v0.3.0
pub trait Signal {
    fn layer(&self) -> Layer;
}

// v0.4.0
pub trait Signal {
    fn layer(&self) -> RuntimeTier;
}
```

### 2.5 更新 Axiom trait

```rust
// v0.3.0
impl Axiom<MyState, MyMessage> for MyAxiom {
    fn applies_to_layer(&self, layer: Layer) -> bool { ... }
}

// v0.4.0
impl Axiom<MyState, MyMessage> for MyAxiom {
    fn applies_to_layer(&self, layer: RuntimeTier) -> bool { ... }
}
```

### 2.6 更新 Runtime 初始化

```rust
// v0.3.0
let runtime = Runtime::new(Config::default()).await?;

// v0.4.0
let runtime = AxiomRuntime::new(RuntimeConfig::default()).await?;
```

### 2.7 更新 Cell 注册

```rust
// v0.3.0
runtime.register_cell(CellRegistration {
    id: CellId::new("my-cell"),
    layer: Layer::Exec,
    version: Version::new(1, 0, 0),
    supervision_strategy: SupervisionStrategy::Restart,
    cell: Some(Arc::new(Mutex::new(MyCell::new()))),
    factory: None,
}).await;

// v0.4.0 - 新增 factory 字段用于重启机制
runtime.register_cell(CellRegistration {
    id: CellId::new("my-cell"),
    layer: RuntimeTier::Exec,
    version: Version::new(1, 0, 0),
    supervision_strategy: SupervisionStrategy::Restart,
    cell: Some(Arc::new(Mutex::new(MyCell::new()))),
    factory: None,
}).await;
```

### 2.8 更新 Axiom 注册

```rust
// v0.3.0
let chain = AxiomChain::from_registry();

// v0.4.0
let chain = DynAxiomChain::from_registry_for_layer(RuntimeTier::Exec);
```

### 2.9 更新 Layer 相关代码

```rust
// v0.3.0
use axiom_core::layer::Layer;

// v0.4.0 - 新增编译期层标记
use axiom_kernel::layer::RuntimeTier;
use axiom_kernel::sealed::ExecTier;
```

### 2.10 更新 Runtime 提交

```rust
// v0.3.0
runtime.submit_signal(&task, None, Layer::Agent).await?;

// v0.4.0
runtime.submit_signal(&task, None, RuntimeTier::Agent).await?;
```

---

## 3. 主要变更详解

### 3.1 Crate 结构变更

| 旧版 | 新版 | 说明 |
|------|------|------|
| `axiom-core` | `axiom-kernel` + `axiom-runtime` | 拆分核心原语和运行时 |
| `Layer` 枚举 | `RuntimeTier` 枚举 | 消除与 Crate Layer 的混淆 |
| `LayerMarker` trait | `RuntimeTierMarker` trait | 同步重命名 |
| `CanSendTo<L1, L2>` | `CanSendTo<T1, T2>` | 语义更清晰 |

### 3.2 新增特性

- **WASM 插件系统**：支持运行时动态加载插件
- **热图系统**：信号流量可视化
- **SHA-256 Witness 哈希**：更强的审计链安全性（需启用 `sha2-id` feature）
- **DLQ 容量限制**：死信队列支持容量配置和背压
- **DispatchContext**：简化 dispatch loop 参数

### 3.3 废弃 API

| 废弃 API | 替代 API | 移除版本 |
|----------|----------|----------|
| `Layer` | `RuntimeTier` | v0.5.0 |
| `LayerMarker` | `RuntimeTierMarker` | v0.5.0 |
| `OversightLayer` | `OversightTier` | v0.5.0 |
| `AgentLayer` | `AgentTier` | v0.5.0 |
| `ValidateLayer` | `ValidateTier` | v0.5.0 |
| `ExecLayer` | `ExecTier` | v0.5.0 |

---

## 4. 常见问题

### Q: `Layer::Exec` 变成了什么？

A: `RuntimeTier::Exec`。旧别名 `Layer` 仍然可用但已废弃，将在 v0.5.0 移除。

### Q: `AxiomChain` 变成了 `DynAxiomChain`，如何适配？

A: 使用 `DynAxiomChain::from_registry_for_layer(RuntimeTier::Exec)` 按层查询注册的 Axiom。

### Q: Witness 哈希算法有变化吗？

A: 默认仍使用 `DefaultHasher`，但推荐启用 `sha2-id` feature 使用 SHA-256。

```toml
[dependencies]
axiom-kernel = { path = "../crates/axiom-kernel", features = ["sha2-id"] }
```

### Q: Cell 注册的 `factory` 字段是什么？

A: `factory` 用于监督重启时重建 Cell，建议提供 `Option<fn() -> Box<dyn Cell>>`。

---

## 5. 验证迁移

运行以下命令验证迁移是否成功：

```bash
# 编译检查
cargo check --workspace

# 运行测试
cargo test --workspace

# 检查废弃警告
cargo clippy --workspace -D warnings
```

---

## 6. 获取帮助

- [架构设计](../docs/ARCHITECTURE.md)
- [API 边界](../docs/API_BOUNDARY.md)
- [核心概念](../docs/guide/core-concepts.md)
- [最佳实践](../docs/guide/best-practices.md)