# Troubleshooting

本文档汇总 Axiom Core 的常见问题和解决方案。

---

## 1. 编译问题

### 1.1 `Layer` 已废弃警告

**现象**：
```
warning: use of deprecated type alias `layer::Layer`
```

**原因**：v0.4.0 将 `Layer` 重命名为 `RuntimeTier`。

**解决方案**：
```rust
// 旧代码
use axiom_kernel::layer::Layer;

// 新代码
use axiom_kernel::layer::RuntimeTier;
```

### 1.2 找不到 `ExecLayer` 类型

**现象**：
```
error[E0433]: failed to resolve: use of undeclared type `ExecLayer`
```

**原因**：v0.4.0 将 `ExecLayer` 重命名为 `ExecTier`。

**解决方案**：
```rust
// 旧代码
use axiom_kernel::sealed::ExecLayer;

// 新代码
use axiom_kernel::sealed::ExecTier;
```

### 1.3 `Cell::layer` 返回类型不匹配

**现象**：
```
error[E0308]: mismatched types
```

**原因**：`layer()` 现在返回 `RuntimeTier` 而不是 `Layer`。

**解决方案**：
```rust
// 旧代码
fn layer(&self) -> Layer { Layer::Exec }

// 新代码
fn layer(&self) -> RuntimeTier { RuntimeTier::Exec }
```

---

## 2. 运行时问题

### 2.1 Cell 持续崩溃

**现象**：Cell 反复重启，日志中看到 `SupervisionStrategy::Restart`。

**原因**：
1. Cell 处理逻辑有 bug
2. 外部依赖不可用
3. 资源不足

**解决方案**：
1. 检查 Cell 的 `handle` 方法，确保正确处理错误
2. 添加 `tracing` 日志定位崩溃原因
3. 考虑更换监督策略为 `Stop` 或 `Escalate`

### 2.2 信号处理延迟

**现象**：信号处理时间超过预期。

**原因**：
1. Cell 处理逻辑复杂
2. 锁竞争
3. 消息队列积压

**解决方案**：
1. 使用 `HeatmapCollector` 分析信号分布
2. 检查是否有 `std::sync::RwLock` 阻塞 async 上下文
3. 增加 Runtime 的工作线程数

### 2.3 Witness 链验证失败

**现象**：
```
Witness chain verification failed: hash mismatch
```

**原因**：
1. Witness 被篡改
2. 存储介质损坏
3. 哈希计算不一致

**解决方案**：
1. 检查 Witness 存储的完整性
2. 确保所有节点使用相同的 `sha2-id` feature 配置
3. 从最近的快照恢复

---

## 3. 架构问题

### 3.1 层间调用违规

**现象**：
```
error: LayerViolation: Exec cannot send to Oversight
```

**原因**：`CanSendTo` 编译期检查阻止了非法调用。

**解决方案**：
1. 检查调用方向，确保从高层向低层调用
2. 如果需要反向调用，使用 `Oversight` 作为中介
3. 或重新设计架构，避免跨层调用

### 3.2 依赖方向错误

**现象**：
```
error: crate `axiom-agent` depends on crate `axiom-runtime`, but axiom-agent is at layer 3 and axiom-runtime is at layer 4
```

**原因**：违反了 Crate Layer 依赖规则。

**解决方案**：
1. 检查 `Cargo.toml` 依赖关系
2. 使用 trait 抽象或事件驱动解耦
3. 如需豁免，在 `architecture.toml` 中声明

---

## 4. 性能问题

### 4.1 内存增长过快

**现象**：进程内存持续增长。

**原因**：
1. Witness 存储未限制大小
2. 消息队列积压
3. 内存泄漏

**解决方案**：
1. 配置 `WitnessRegistry` 的最大容量
2. 检查 `DeadLetterQueue` 的 `dlq_capacity`
3. 使用 `heaptrack` 或 `valgrind` 分析内存泄漏

### 4.2 CPU 使用率过高

**现象**：CPU 使用率持续高于 80%。

**原因**：
1. Cell 处理逻辑过于复杂
2. 热图采样率过高
3. 锁竞争激烈

**解决方案**：
1. 优化 Cell 的 `handle` 方法
2. 降低热图采样率
3. 检查并统一锁策略，避免 `std::sync::RwLock` 在 async 上下文使用

---

## 5. 测试问题

### 5.1 测试隔离失败

**现象**：测试 A 影响测试 B 的结果。

**原因**：全局注册表（`AXIOM_REGISTRY`、`WITNESS_REGISTRY`）未清理。

**解决方案**：
```rust
use axiom_kernel::registry::RegistryGuard;

#[test]
fn test_something() {
    let _guard = RegistryGuard::new();
    // 测试代码，注册表修改在 guard 析构后自动恢复
}
```

### 5.2 异步测试超时

**现象**：
```
thread 'test' panicked at 'timed out'
```

**原因**：async 测试中缺少 `.await`。

**解决方案**：
```rust
#[tokio::test]
async fn test_async_cell() {
    let result = cell.handle(signal, ctx).await;
    result.unwrap();
}
```

---

## 6. 常见错误码

| 错误码 | 含义 | 解决方案 |
|--------|------|----------|
| `LayerViolation` | 层间调用违规 | 检查 `CanSendTo` 约束 |
| `ResourceExhausted` | 资源耗尽 | 检查 DLQ 容量、内存限制 |
| `InvariantViolated` | Axiom 违反 | 检查业务逻辑 |
| `CellNotFound` | Cell 未找到 | 检查 Cell 注册 |
| `SignalValidationFailed` | 信号校验失败 | 检查 Signal 字段 |

---

## 7. 获取帮助

- [架构设计](../docs/ARCHITECTURE.md)
- [API 边界](../docs/API_BOUNDARY.md)
- [核心概念](../docs/guide/core-concepts.md)
- [最佳实践](../docs/guide/best-practices.md)
- [迁移指南](../MIGRATION.md)