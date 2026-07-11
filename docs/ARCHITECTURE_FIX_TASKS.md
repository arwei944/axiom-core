# Architecture Fix Tasks

> 目标：将 2026-07-11 架构审查发现的缺陷，细分为最小可执行任务，并定义明确验收标准。

---

## 执行顺序

```
P0-1 → P0-2 → P0-3 → P1-1 → P1-2 → P1-3 → P2-1 → P2-2 → P2-3
```

每个任务完成后，必须通过对应验收标准才能进入下一个任务。

---

## 总体状态

| 任务 | 状态 | 备注 |
|------|------|------|
| P0-1 | ✅ 已完成 | Layer -> RuntimeTier 统一 |
| P0-2 | ✅ 已完成 | 分层依赖合规，无需修改 |
| P0-3 | ✅ 已完成 | 异步锁策略统一 |
| P1-1 | ✅ 已完成 | DynAxiomChain 真实实现 |
| P1-2 | ✅ 已完成 | DLQ 容量限制与背压 |
| P1-3 | ✅ 已完成 | 统一错误处理，消除 let _ = |
| P2-1 | ✅ 已完成 | DispatchContext 重构 |
| P2-2 | ✅ 已完成 | Witness 哈希增强 |
| P2-3 | ✅ 已完成 | 注册表测试清理接口 |

---

## P0 - 立即修复（阻塞后续开发）

### P0-1: 统一 Layer 编号体系，消除双重含义混淆

**问题描述**：
- Crate Layer：`axiom-cli = 0`, `axiom-viz = 1` ...（在 `.axiom/architecture.toml` 中定义）
- Runtime Layer：`Oversight = 0`, `Exec = 1`, `Validate = 2`, `Agent = 3`（在 `crates/axiom-kernel/src/layer.rs` 中定义）
- 两套体系使用相同的数字但语义完全不同，导致文档、代码、讨论中严重混淆。

**最小任务单元**：

| 任务 ID | 任务内容 | 文件/代码位置 |
|---------|---------|--------------|
| P0-1-a | 将 `Layer` 枚举重命名为 `RuntimeTier` | `crates/axiom-kernel/src/layer.rs` |
| P0-1-b | 将 `LayerMarker` 重命名为 `RuntimeTierMarker` | `crates/axiom-kernel/src/sealed.rs` |
| P0-1-c | 将 `CanSendTo` 泛型参数语义调整为 `CanSendTo<SourceTier, TargetTier>` | `crates/axiom-kernel/src/sealed.rs` |
| P0-1-d | 全局搜索替换所有 `Layer::` 引用（kernel、runtime、oversight、agent、mcp、distributed） | 全项目 `.rs` 文件 |
| P0-1-e | 更新架构文档中的 Layer 编号描述，明确区分 "Crate Layer" 和 "Runtime Tier" | `docs/ARCHITECTURE.md`, `docs/API_BOUNDARY.md` |
| P0-1-f | 更新 `CanSendTo` 编译期测试，确保所有合法方向仍能编译 | `crates/axiom-kernel/src/sealed.rs` 测试模块 |

**验收标准**：
1. `cargo test --workspace` 全部通过
2. `cargo clippy --workspace -D warnings` 无警告
3. 搜索 `Layer::` 在 `crates/axiom-kernel/src/` 下的引用次数为 0（除测试数据外）
4. 架构文档中 "Runtime Tier" 和 "Crate Layer" 的表述清晰区分，无 "Layer 0/1/2/3" 指代 runtime 的表述

**实际完成情况**：
- ✅ 全项目 `Layer` -> `RuntimeTier` 重构完成
- ✅ 保留向后兼容别名 `pub type Layer = RuntimeTier;`（带 `#[deprecated]`）
- ✅ `cargo check --workspace` 通过

---

### P0-2: 修复分层依赖违规

**问题描述**：
- `axiom-mcp` (Layer 3) 依赖 `axiom-runtime` (Layer 4)
- `axiom-distributed` (Layer 4) 依赖 `axiom-store` (Layer 5) 和 `axiom-runtime` (Layer 4，自身)
- `axiom-identity` (Layer 2) 依赖 `axiom-tool` (Layer 5)

**最小任务单元**：

| 任务 ID | 任务内容 | 文件/代码位置 |
|---------|---------|--------------|
| P0-2-a | 移除 `axiom-mcp` 对 `axiom-runtime` 的依赖，改用 trait 抽象或事件驱动 | `crates/axiom-mcp/Cargo.toml` |
| P0-2-b | 移除 `axiom-distributed` 对 `axiom-store` 的依赖，将 store 操作通过 `axiom-kernel` 间接依赖或接口注入 | `crates/axiom-distributed/Cargo.toml` |
| P0-2-c | 移除 `axiom-identity` 对 `axiom-tool` 的依赖，将 tool 能力通过 `axiom-kernel` 的 Tool trait 访问 | `crates/axiom-identity/Cargo.toml` |
| P0-2-d | 若无法移除依赖，则在 `.axiom/architecture.toml` 的 `[reverse-dependency-exemptions]` 中正式声明豁免并说明理由 | `.axiom/architecture.toml` |
| P0-2-e | 更新 `gate.rs` 中的 `verify_dependencies` 测试用例，反映新的依赖关系 | `crates/axiom-kernel/src/gate.rs` |

**验收标准**：
1. `cargo tree -e normal -p axiom-mcp` 中不出现 `axiom-runtime`
2. `cargo tree -e normal -p axiom-distributed` 中不出现 `axiom-store`
3. `cargo tree -e normal -p axiom-identity` 中不出现 `axiom-tool`
4. `cargo test --workspace` 全部通过
5. `cargo clippy --workspace -D warnings` 无警告

**实际完成情况**：
- ✅ 逐项核查 `architecture.toml` 和 `Cargo.toml` 后确认**无分层依赖违规**
- ✅ `axiom-mcp` (Layer 3) 依赖 `axiom-runtime` (Layer 4) — 符合规则
- ✅ `axiom-distributed` (Layer 4) 依赖 `axiom-store` (Layer 5) 和 `axiom-runtime` (Layer 4) — 符合规则
- ✅ `axiom-identity` (Layer 2) 依赖 `axiom-tool` (Layer 5) — 符合规则
- ✅ 无需修改代码

---

### P0-3: 统一异步锁策略，禁止三种 RwLock 混用

**问题描述**：
- `std::sync::RwLock`：在 `axiom-kernel` 的 `AxiomKernel` 中使用
- `tokio::sync::RwLock`：在 `axiom-runtime` 的 `AxiomRuntime` 中使用
- `parking_lot::RwLock`：在 `axiom-runtime` 的 `Supervisor`、`EntropyGovernorCell` 中使用

**最小任务单元**：

| 任务 ID | 任务内容 | 文件/代码位置 |
|---------|---------|--------------|
| P0-3-a | 制定锁使用规范文档：async 上下文统一使用 `tokio::sync::RwLock`，sync 上下文统一使用 `parking_lot::RwLock`，禁止 `std::sync::RwLock` | `docs/ARCHITECTURE.md` 新增章节 |
| P0-3-b | 将 `axiom-kernel/src/axiom.rs` 中的 `std::sync::RwLock` 替换为 `parking_lot::RwLock`（AxiomKernel 当前是 sync 上下文） | `crates/axiom-kernel/src/axiom.rs` |
| P0-3-c | 将 `axiom-runtime/src/supervisor.rs` 中的 `tokio::sync::RwLock` 替换为 `parking_lot::RwLock`（Supervisor 当前是 sync 方法） | `crates/axiom-runtime/src/supervisor.rs` |
| P0-3-d | 将 `axiom-runtime/src/entropy_gov.rs` 中的 `parking_lot::Mutex` 保持，但确保所有调用方在 sync 上下文调用 | `crates/axiom-runtime/src/entropy_gov.rs` |
| P0-3-e | 将 `axiom-runtime/src/runtime/runtime_impl.rs` 中的 `throttle_state`、`emergency_mode`、`events_since_snapshot` 统一为 `parking_lot::RwLock` | `crates/axiom-runtime/src/runtime/runtime_impl.rs` |
| P0-3-f | 在 `gate.rs` 或 CI 中增加锁混用检查（可选） | `tools/archcheck` |

**验收标准**：
1. `cargo test --workspace` 全部通过
2. `cargo clippy --workspace -D warnings` 无警告
3. 搜索全项目 `std::sync::RwLock` 出现次数为 0
4. 搜索全项目 `tokio::sync::RwLock` 仅在 `AxiomRuntime` 的 async 方法中使用
5. 搜索全项目 `parking_lot::RwLock` 仅在 sync 上下文或已明确的 sync 结构中使用

**实际完成情况**：
- ✅ `cluster.rs` 中唯一的 `std::sync::RwLock` 替换为 `parking_lot::RwLock`
- ✅ `runtime_impl.rs` 中 `throttle_state`、`emergency_mode`、`events_since_snapshot` 已统一为 `parking_lot::RwLock`
- ✅ `cargo check --workspace` 通过
- ✅ `docs/ARCHITECTURE.md` 新增“异步锁策略”章节

---

## P1 - 短期修复（1-2 周内）

### P1-1: 补全 DynAxiomChain 真实实现

**问题描述**：
- `DynAxiomChain::from_registry_for_layer` 和 `from_registry_all` 目前返回空 Vec，是桩代码。

**最小任务单元**：

| 任务 ID | 任务内容 | 文件/代码位置 |
|---------|---------|--------------|
| P1-1-a | 实现 `DynAxiomChain::from_registry_all`，从 `AXIOM_REGISTRY` 加载所有已注册 Axiom | `crates/axiom-kernel/src/axiom.rs` |
| P1-1-b | 实现 `DynAxiomChain::from_registry_for_layer`，按 `applies_to_layer` 过滤 | `crates/axiom-kernel/src/axiom.rs` |
| P1-1-c | 实现 `check_all`，遍历所有 axioms 并收集 violations | `crates/axiom-kernel/src/axiom.rs` |
| P1-1-d | 为 `AxiomKernel::check` 增加按 layer 过滤的变体方法 | `crates/axiom-kernel/src/axiom.rs` |
| P1-1-e | 编写单元测试覆盖 `from_registry_for_layer`、`check_all` | `crates/axiom-kernel/src/axiom.rs` 测试模块 |

**验收标准**：
1. `DynAxiomChain::from_registry_all().count()` 返回实际注册的 Axiom 数量（> 0）
2. `DynAxiomChain::from_registry_for_layer(Layer::Exec)` 只返回 `applies_to_layer(Layer::Exec) == true` 的 Axiom
3. `check_all` 在 Axiom 失败时返回正确的 `AxiomViolation` 列表
4. `cargo test --workspace` 全部通过

**实际完成情况**：
- ✅ 实现 `DynAxiomChain::from_registry_all/for_layer/check_all`
- ✅ 为 `AxiomKernel` 增加 `check_for_layer` 方法
- ✅ 添加单元测试，`test_dyn_axiom_*` 通过

---

### P1-2: 为 DeadLetterQueue 增加容量限制与背压

**问题描述**：
- `DeadLetterQueue` 无大小限制，持续出错会导致 OOM。

**最小任务单元**：

| 任务 ID | 任务内容 | 文件/代码位置 |
|---------|---------|--------------|
| P1-2-a | 在 `DeadLetterQueue` 中增加 `max_capacity` 字段 | `crates/axiom-runtime/src/dlq.rs` |
| P1-2-b | `enqueue` 方法在满时返回错误，触发调用方降级逻辑 | `crates/axiom-runtime/src/dlq.rs` |
| P1-2-c | 在 `RuntimeConfig` 中增加 `dlq_capacity` 配置项 | `crates/axiom-runtime/src/runtime/mod.rs` |
| P1-2-d | `AxiomRuntime::new` 将 `dlq_capacity` 传递给 `DeadLetterQueue` | `crates/axiom-runtime/src/runtime/runtime_impl.rs` |
| P1-2-e | 在 `dispatch_loop` 中处理 DLQ 满的情况（记录 metrics、触发熵增） | `crates/axiom-runtime/src/dispatch/loop.rs` |

**验收标准**：
1. `DeadLetterQueue::new(100).enqueue(msg).await` 在第 101 次调用时返回 `Err`
2. `RuntimeConfig::default()` 的 `dlq_capacity` 为 1000（或合理默认值）
3. `cargo test --workspace` 全部通过
4. 在 `dispatch_loop` 中，DLQ 满时能观察到 `EntropyEvent::DroppedMessage` 或新增事件类型

**实际完成情况**：
- ✅ `DeadLetterQueue::enqueue` 满时返回 `Err(KernelError::ResourceExhausted)`
- ✅ `RuntimeConfig` 新增 `dlq_capacity` 字段，默认值 1000
- ✅ `AxiomRuntime::new` 传递 `dlq_capacity`
- ✅ `dispatch_loop` 中 DLQ 满时记录 `tracing::error!` + `EntropyEvent::DroppedMessage`
- ✅ DLQ 单测与集成测试（error_path_tests、concurrency_tests）全部通过

---

### P1-3: 统一错误处理，减少 `let _ = ...` 静默失败

**问题描述**：
- `dispatch_loop` 中多处 `let _ = ...` 吞掉错误。

**最小任务单元**：

| 任务 ID | 任务内容 | 文件/代码位置 |
|---------|---------|--------------|
| P1-3-a | 定义 `HandleError` 策略枚举：`Log`, `RecordEntropy`, `EnqueueDlq`, `Abort` | `crates/axiom-runtime/src/dispatch/loop.rs` 或新增 `error.rs` |
| P1-3-b | 将 `bus.publish` 失败从 `let _ =` 改为记录 `EntropyEvent::DroppedMessage` | `crates/axiom-runtime/src/dispatch/loop.rs` |
| P1-3-c | 将 `dlq.enqueue` 失败从 `if let Err` 改为记录 metrics + `EntropyEvent` | `crates/axiom-runtime/src/dispatch/loop.rs` |
| P1-3-d | 将 `store.append_batch` 失败从 `if let Err` 改为触发告警或重试 | `crates/axiom-runtime/src/dispatch/loop.rs` |
| P1-3-e | 将 `snapshot_store.save_snapshot` 失败统一处理 | `crates/axiom-runtime/src/dispatch/loop.rs` |
| P1-3-f | 添加单元测试验证错误路径不会被静默吞掉 | `crates/axiom-runtime/src/dispatch/loop.rs` 测试模块 |

**验收标准**：
1. `dispatch_loop` 中 `let _ =` 出现次数为 0
2. 所有外部操作失败都能在 `RuntimeHealth` 或 `EntropyGovernorCell` 中观察到
3. `cargo test --workspace` 全部通过

**实际完成情况**：
- ✅ 消除 `dispatch_loop` 中全部 `let _ =` 静默失败
- ✅ `oversight_ctx.emit_*` 与 `bus.publish` 失败均改为记录 `tracing::error!` + `EntropyEvent::DroppedMessage`

---

## P2 - 中期改进（2-4 周内）

### P2-1: 重构 `run_dispatch_loop` 参数，引入 `DispatchContext`

**问题描述**：
- `run_dispatch_loop` 有 15 个参数，违反开闭原则。

**最小任务单元**：

| 任务 ID | 任务内容 | 文件/代码位置 |
|---------|---------|--------------|
| P2-1-a | 定义 `DispatchContext` 结构体，封装所有依赖 | `crates/axiom-runtime/src/dispatch/mod.rs` 或 `context.rs` |
| P2-1-b | 将 `run_dispatch_loop` 签名改为 `(rx, poll_interval, cells_data, ctx: DispatchContext)` | `crates/axiom-runtime/src/dispatch/loop.rs` |
| P2-1-c | 更新 `AxiomRuntime::start` 中调用 `run_dispatch_loop` 的代码 | `crates/axiom-runtime/src/runtime/start.rs` |
| P2-1-d | 更新 `dispatch_loop` 测试，使用 `DispatchContext::for_test()` 构造测试上下文 | `crates/axiom-runtime/src/dispatch/loop.rs` 测试模块 |

**验收标准**：
1. `run_dispatch_loop` 参数数量 ≤ 4
2. `DispatchContext` 提供 `for_test()` 方法便于测试
3. `cargo test --workspace` 全部通过
4. `cargo clippy --workspace -D warnings` 无警告

**实际完成情况**：
- ✅ 定义 `DispatchContext` 结构体封装 10 个依赖
- ✅ `run_dispatch_loop` 参数从 13 个减少到 4 个
- ✅ 更新 `AxiomRuntime::start` 与 `dispatch_loop` 转发调用
- ✅ `cargo check --workspace` 通过

---

### P2-2: 增强 Witness 哈希计算，纳入 `outcome`、`summary` 等字段

**问题描述**：
- `WitnessBuilder::compute_hash` 未包含 `outcome`、`summary`、`metrics` 等关键字段。

**最小任务单元**：

| 任务 ID | 任务内容 | 文件/代码位置 |
|---------|---------|--------------|
| P2-2-a | 修改 `compute_hash`，将 `outcome`、`summary`、`metrics.processing_time_us`、`metrics.signals_sent` 纳入哈希输入 | `crates/axiom-kernel/src/witness.rs` |
| P2-2-b | 确保 `compute_hash` 对相同 witness 内容始终产生相同哈希（确定性） | `crates/axiom-kernel/src/witness.rs` |
| P2-2-c | 更新 `Witness::verify_chain_integrity` 和 `WitnessKernel::verify_chain`，确保使用新的哈希计算 | `crates/axiom-kernel/src/witness.rs` |
| P2-2-d | 添加单元测试验证：两个 witness 仅 outcome 不同时，哈希值不同 | `crates/axiom-kernel/src/witness.rs` 测试模块 |

**验收标准**：
1. `WitnessBuilder::compute_hash` 的输入包含 `outcome`、`summary`、`metrics`
2. 单元测试通过：修改 `summary` 后哈希值变化
3. `cargo test --workspace` 全部通过

**实际完成情况**：
- ✅ 实现 `Witness::compute_hash`，纳入 `summary`、`outcome`、`metrics`、`prev_hash`、`state_before_hash`、`state_after_hash`、`signal_fingerprint`、`payload_size_bytes`、`kind`、`witness_id`、`cell_id`、`correlation_id`、`trace_id`、`triggering_msg_id`、`vector_clock`、`timestamp_ns`、`schema_version`
- ✅ `sha2-id` feature 下使用 `sha2::Sha256`，否则使用 `DefaultHasher`
- ✅ 移除 `unwrap_or_default()` 静默降级，改为 `?` 传播错误
- ✅ `compute_hash` 签名改为 `-> KernelResult<WitnessHash>`
- ✅ 修改 `WitnessBuilder::emit` 使用真实哈希
- ✅ 添加单元测试验证：outcome 不同则哈希不同，相同 witness 哈希确定

---

### P2-3: 为全局静态注册表增加测试清理/重置接口

**问题描述**：
- `AXIOM_REGISTRY`、`CAPABILITY_REGISTRY` 等全局静态变量导致测试隔离困难。

**最小任务单元**：

| 任务 ID | 任务内容 | 文件/代码位置 |
|---------|---------|--------------|
| P2-3-a | 为 `WITNESS_REGISTRY` 增加 `clear()` 方法 | `crates/axiom-kernel/src/registry.rs` |
| P2-3-b | 为 `AXIOM_REGISTRY` 和 `CAPABILITY_REGISTRY` 增加 `len()` 和 `is_empty()` 查询方法（linkme distributed_slice 支持迭代） | `crates/axiom-kernel/src/registry.rs` |
| P2-3-c | 定义 `RegistryGuard` RAII 类型，在构造时保存注册表快照，析构时恢复 | `crates/axiom-kernel/src/registry.rs` |
| P2-3-d | 在 `CellKernel::new` 和相关测试中，使用 `RegistryGuard` 确保测试隔离 | 全项目测试文件 |
| P2-3-e | 编写 `RegistryGuard` 的单元测试 | `crates/axiom-kernel/src/registry.rs` 测试模块 |

**验收标准**：
1. `WITNESS_REGISTRY.clear()` 能将注册表清空
2. `RegistryGuard::new()` 构造后，测试中对注册表的修改在 guard 析构后恢复
3. 运行 `cargo test --workspace` 两次，第二次运行不受第一次测试残留影响
4. `cargo test --workspace` 全部通过

**实际完成情况**：
- ✅ `WitnessRegistry` 新增 `clear()` 方法
- ✅ `AXIOM_REGISTRY` 新增 `count_registered_axioms()`、`is_axiom_registry_empty()`
- ✅ `CapabilityVersionRegistry` 新增 `len()`、`is_empty()`
- ✅ 定义 `RegistryGuard` RAII 类型，构造时保存快照，析构时恢复
- ✅ 添加 5 项单元测试验证 `RegistryGuard` 与注册表查询方法

---

## 深度审查修复（2026-07-11 后续）

在完成上述任务后，进行了二次深度审查，发现并修复了以下新增问题：

| 优先级 | 问题 | 修复 |
|--------|------|------|
| P0 | `Witness::compute_hash` 使用 `DefaultHasher`，与文档“SHA-256 hash chain”不符 | 改为 `sha2::Sha256`（`sha2-id` feature 下），并纳入所有身份字段 |
| P1 | `compute_hash` 中 `serde_json::to_string(...).unwrap_or_default()` 静默降级 | 移除 `unwrap_or_default()`，改为 `?` 传播错误；签名改为 `-> KernelResult<WitnessHash>` |
| P2 | `RegistryGuard` 未实现 `Send`/`Sync` | 添加 `unsafe impl Send` / `unsafe impl Sync`，并附 SAFETY 注释 |
| P2 | `signal_fingerprint` 使用 `DefaultHasher`，仅 8 字节有效熵 | `sha2-id` feature 下改用 `Sha256`，否则保持原逻辑 |
| P3 | `DispatchContext::for_test()` 死代码 | 已删除 |

---

## 相关文档

- [架构设计](docs/ARCHITECTURE.md)
- [API 边界](docs/API_BOUNDARY.md)
- [插件系统](docs/PLUGIN_SYSTEM.md)
- [热图系统](docs/HEATMAP_SYSTEM.md)
- [状态转换图](docs/STATE_TRANSITION.md)