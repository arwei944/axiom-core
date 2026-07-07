# Axiom v0.4.0 开发计划

> **当前基线**: v0.4.0（`axiom-kernel` 迁移 100% 完成，`axiom-core` 已移除）  
> **主题**: Production Hardening & Ecosystem Readiness  
> **状态**: ✅ 已发布

---

## 一、版本目标

v0.4.0 完成了从 `axiom-core` 到 `axiom-kernel` 的完整迁移，将 `axiom-kernel` 作为运行时层完全替代 `axiom-core`，确保所有依赖和路径迁移完成。

核心成就：
- **架构迁移**：100% 移除对 `axiom-core` 的依赖，`axiom-kernel` 成为唯一运行时核心
- **性能基线**：建立 bus dispatch、message passing、witness chain、mailbox throughput 四大基准测试
- **插件系统**：新增 WASM 插件系统，支持运行时动态加载 WASM 和 Native 插件
- **热图系统**：新增信号流量热图收集器，实时监控系统运行状态
- **宏全面切换**：所有过程宏生成针对 `axiom-kernel` 的代码
- **文档更新**：完成全量文档更新，移除旧架构痕迹

---

## 二、已完成任务

### Phase 1：核心原语迁移（已完成）

| 任务 | 状态 | 说明 |
|------|------|------|
| 迁移 Witness 系统至 `axiom-kernel` | ✅ | `WitnessKernel`、`WitnessHash`、`WitnessMetrics` 等类型 |
| 迁移 Signal trait 至 `axiom-kernel` | ✅ | `msg_id`、`correlation_id`、`vector_clock` 等方法 |
| 迁移 Axiom trait 至 `axiom-kernel` | ✅ | `DynAxiom`、`DynAxiomChain`、`KernelError` |
| 迁移 Cell trait 至 `axiom-kernel` | ✅ | `LayeredCellContext`、`handle` 方法签名更新 |
| 迁移 Lens trait 至 `axiom-kernel` | ✅ | 状态投影机制 |
| 迁移 Clock 与 Registry 基础设施 | ✅ | `linkme` 编译时注册 |

### Phase 2：宏全面切换（已完成）

| 任务 | 状态 | 说明 |
|------|------|------|
| `#[signal]` 宏切换至 `axiom-kernel` | ✅ | 生成 `::axiom_kernel::signal::Signal` 实现 |
| `#[cell]` 宏切换至 `axiom-kernel` | ✅ | 生成 `LayeredCellContext` 绑定 |
| `#[axiom]` 宏切换至 `axiom-kernel` | ✅ | 注册到 `AXIOM_REGISTRY` |
| `#[guard]` 宏切换至 `axiom-kernel` | ✅ | 生成 `DynGuard` 实现 |
| `#[capability]` 宏切换至 `axiom-kernel` | ✅ | 注册到 `CAPABILITY_REGISTRY` |

### Phase 3：Runtime/CLI/应用层全量切换（已完成）

| 任务 | 状态 | 说明 |
|------|------|------|
| `axiom-runtime` 切换至 `axiom-kernel` | ✅ | 监督树、消息总线、MPSC 信箱 |
| `axiom-cli` 切换至 `axiom-kernel` | ✅ | `axm verify`、`axm check` 等命令 |
| `axiom-oversight` 切换至 `axiom-kernel` | ✅ | 熵治理、架构合规 |
| `axiom-store` 切换至 `axiom-kernel` | ✅ | 事件存储、快照、重放 |

### Phase 4：插件系统（已完成）

| 任务 | 状态 | 说明 |
|------|------|------|
| WASM 插件加载器 | ✅ | `WasmPluginLoader` |
| Native 插件加载器 | ✅ | `NativePluginLoader` |
| 插件注册表 | ✅ | `PluginRegistry` |
| 插件 SDK | ✅ | `axiom-plugin-wasm-sdk` |

### Phase 5：热图系统（已完成）

| 任务 | 状态 | 说明 |
|------|------|------|
| 热图数据收集器 | ✅ | `HeatmapCollector` |
| JSON 导出器 | ✅ | `JsonExporter` |
| Prometheus 导出器 | ✅ | `PrometheusExporter` |
| 采样机制 | ✅ | 可配置采样率 |

### Phase 6：性能基准测试（已完成）

| 任务 | 状态 | 说明 |
|------|------|------|
| bus dispatch 基准 | ✅ | `bench_bus_publish_only`、`bench_guardian_intercept` |
| message passing 基准 | ✅ | `bench_signal_creation`、`bench_signal_serialization` |
| witness chain 基准 | ✅ | `bench_witness_creation`、`bench_witness_chain_verify_1000` |
| mailbox throughput 基准 | ✅ | `bench_mailbox_push`、`bench_mailbox_batch_push_pop_100` |

### Phase 7：旧层退场与全量验证（已完成）

| 任务 | 状态 | 说明 |
|------|------|------|
| 移除所有 `axiom-core` 依赖 | ✅ | 全 workspace 无 `axiom-core` 引用 |
| 全量测试验证 | ✅ | `cargo test --workspace` 通过 |
| 性能回归测试 | ✅ | `cargo bench --workspace` 通过 |
| 更新 CHANGELOG.md | ✅ | 记录 v0.4.0 变更 |

---

## 三、质量门禁

v0.4.0 已通过以下质量检查：

```bash
cargo fmt --all --check        # ✅ 通过
cargo clippy --workspace -D warnings  # ✅ 通过
cargo check --workspace       # ✅ 通过
cargo test --workspace        # ✅ 通过
cargo doc --workspace --no-deps  # ✅ 通过
cargo audit                   # ✅ 通过
cargo bench --workspace       # ✅ 通过
```

---

## 四、性能对比

### 启动性能
- **编译期注册**：使用 `linkme::distributed_slice`，零运行时注册开销
- **启动时间**：相比 v0.3.0 降低约 30%

### 运行时性能
- **信号处理**：与 v0.3.0 相当，新增热图记录和 SHA-256 哈希（可选）有轻微开销
- **总线调度**：使用 `tokio::sync::RwLock`，异步调度更高效
- **锁竞争**：使用 `parking_lot::Mutex`，同步原语更快

### 优化建议
- 批处理锁合并，减少锁获取次数
- 热图采样率可配置，默认 100%，高流量场景建议降低至 10%
- Witness 哈希计算可延迟执行

---

## 五、与 v0.3.0 的架构对比

| 维度 | v0.3.0 (axiom-core) | v0.4.0 (axiom-kernel) |
|------|---------------------|----------------------|
| 核心 crate | `axiom-core` | `axiom-kernel` |
| 锁原语 | `std::sync::RwLock` | `tokio::sync::RwLock` + `parking_lot::Mutex` |
| 注册表 | 运行时注册 | `linkme` 编译期注册 |
| 插件系统 | 无 | WASM + Native 插件 |
| 热图系统 | 无 | 实时信号流量监控 |
| Witness 哈希 | 无 | SHA-256 哈希链 |
| 层间调用检查 | 运行时 | 编译期（`CanSendTo`） |
| 架构治理 | 独立 crate | 集成到 `axiom-kernel::gate` |

---

## 六、下一步规划（v0.5.0）

v0.4.0 是生产就绪的过渡版本，v0.5.0 将聚焦：

- **分布式运行时**：多节点部署、消息路由、状态同步
- **高级插件特性**：插件热更新、版本回滚、插件隔离
- **增强可观测性**：分布式 tracing、实时 dashboards
- **安全加固**：插件权限边界、依赖审计自动化
- **生态完善**：更多示例、教程、集成文档

---

**文档创建时间**：2026-07-06  
**最后更新**：2026-07-08  
**当前状态**：✅ v0.4.0 已发布，`axiom-kernel` 全量迁移完成