# v0.5.0 工程硬化完成说明

**状态**：`docs/TASK_CHECKLIST.md` 全量关闭（open = 0）  
**主题矩阵**：T1–T15 **完全满足** — [`../../unified/FEATURE_THEME_MATRIX.md`](../../unified/FEATURE_THEME_MATRIX.md)  
**日期**：2026-07-19  
**主语言**：Rust  
**关联**：ULE 商用交付 [`../../unified/COMMERCIAL_DELIVERY.md`](../../unified/COMMERCIAL_DELIVERY.md)

---

## 1. 本文档目的

说明 **Architecture Upgrade Task Checklist（约 58 项 v0.5.0 升级任务）** 落地后的：

- 能力映射（做了什么、代码在哪）
- **生产调用链**（禁止「只写 helper、不接 start/dispatch」）
- 验证方式
- 已知偏差与后续非目标

---

## 2. 清单状态

| 指标 | 值 |
|------|-----|
| 未勾 `[ ]` | **0** |
| 已勾 `[x]` | **91**（含早期 Phase 1–4 + v0.5.0 P0–P3） |
| 清单文件 | `docs/TASK_CHECKLIST.md` |

完成标准不是勾选框，而是：**非 test 生产路径有调用者 + 路径驱动测试失败会暴露断线**。

---

## 3. 分优先级落地摘要

### P0 — 正确性

| 项 | 行为 | 主要位置 |
|----|------|----------|
| Witness 哈希链 | `compute_hash` 非零；`verify_chain_integrity` 校验 prev 链接 **且** 重算哈希；篡改失败 | `crates/axiom-kernel/src/witness.rs` |
| 层校验统一 | `SignalEnvelope::validate_layer_transition` → `RuntimeTier::can_send_to` | `signal.rs` |
| CellKernel O(1) | `HashMap` 查找，1000 cell 往返测试 | `cell.rs` |
| 按类型 Schema | `TypeSchemaRegistry` | `version.rs` |
| 熔断默认策略 + 持久化 | 默认 CB 作用于所有 strategy；`persist_circuits_to_store` / `restore_circuits_from_store` | `crates/axiom-runtime/src/supervisor.rs` |

### P1 — 韧性与恢复

| 项 | 行为 | 主要位置 |
|----|------|----------|
| DLQ 持久化 | `DeadLetterStore`：`peek` / `ack` / `retry`；内存 + 文件适配器；崩溃后未 ack 可重开 | `dlq_store.rs` |
| Backoff 抖动 | full-jitter；`RuntimeConfig.backoff_*` 注入 `Supervisor::with_backoff` | `config` + `supervisor` + `runtime_impl` |
| Replay aggregate_id | `replay_with_events(aggregate_id, …)`，禁止空字符串查 snapshot | `crates/axiom-store/src/replay/engine.rs` |
| Guard 层绑定 | `Guard::layer() -> RuntimeTier`；`GuardRegistry::register` 校验层 | `guard.rs` |
| 插件 ABI | `abi_version` + `check_abi_compatible`；WASM 可选导出 `axiom_abi_version` | `plugin/package.rs`、`loader/wasm.rs` |

### P2 — 平台

| 项 | 行为 | 主要位置 |
|----|------|----------|
| Mailbox | `parking_lot::Mutex` 短临界区队列 | `mailbox.rs` |
| Signal LRU | 有界 + TTL 缓存 | `signal.rs` `SignalKernel` |
| Metrics 默认 | **启动时读取** `metrics_enabled` → `RuntimeHealth.metrics_active` | `runtime/start.rs` |
| 心跳 / 降级 | dispatch **每 tick 写** `last_heartbeat_ms`；health poller 超时设 `degraded` | `dispatch/loop.rs`、`start.rs` |
| Composer 接线 | `ConnectionSpec` 校验并记录拓扑 | `plugin/composer.rs` |

### P3 — 工程化

| 项 | 行为 | 主要位置 |
|----|------|----------|
| 宏错误 UX | `axiom-macros/src/error.rs` 统一文案 | `crates/axiom-macros` |
| 插件热更语义 | `PluginRegistry::upgrade` + refcount | `plugin/registry.rs` |
| CI 覆盖率 | `.github/workflows/coverage.yml`（core 80%） | 仓库根 |
| 动态版本 | `build.rs` 注入 `Version::CURRENT` | `version.rs` + `build.rs` |

---

## 4. 生产调用链（必读）

以下路径必须在 **非 `#[cfg(test)]` 代码**中存在（断线即回归）：

```
AxiomRuntime::start()
  ├─ 读取 config.metrics_enabled → health.metrics_active / metrics_endpoint
  ├─ spawn dispatch_loop(DispatchContext { health, ... })
  │     └─ 每 tick: health.last_heartbeat_ms = now; degraded = false
  └─ spawn health poller
        └─ 若 now - last_heartbeat_ms > heartbeat_stale_ms → degraded = true

GuardInterceptor::intercept(env)
  └─ registry.check_all(envelope_as_signal, env.target_layer)
        （register_guard 为唯一注册入口，带层校验）
```

路径驱动测试（摘录）：

| 测试 | 证明 |
|------|------|
| `dispatch_loop_advances_heartbeat_without_helpers` | 不调用 `set_heartbeat_ms`，start 后心跳前进 |
| `health_poller_marks_degraded_when_dispatch_stops` | 停 dispatch 后 poller 标 degraded |
| `metrics_enabled_consumed_on_start` | true/false 启动结果分叉 |
| `registered_guard_rejects_via_intercept` | 注册拒绝 Guard 后普通信号被 Reject |
| `circuit_persist_restore_after_crash` | EventStore 熔断快照恢复 |
| `crash_recovery_reopens_nonempty_durable_store` | DLQ 未 ack 重开仍在 |
| `correlation_replay_uses_aggregate_id_not_empty_for_snapshot` | 回放不走空 aggregate_id |

运行：

```powershell
cd C:\work\architecture\axiom-core
cargo test -p axiom-kernel -p axiom-runtime -p axiom-store --lib
cargo test -p axiom-isa -p axiom-resilience -p axiom-demo-taskflow
cargo run -p axiom-demo-taskflow -- success
```

---

## 5. 运行时配置相关字段

| 字段 | 含义 | 默认 |
|------|------|------|
| `metrics_enabled` | 启动时是否激活 metrics 面 | `true` |
| `metrics_endpoint` | 指标端点；启用且未设时用 `internal://metrics` | `None` |
| `heartbeat_stale_ms` | 心跳超过该毫秒数 → `degraded` | `5000` |
| `dispatch_poll_interval_ms` | dispatch 轮询（同时驱动心跳） | `10` |
| `backoff_base_ms` / `backoff_cap_ms` / `backoff_multiplier` | 重启 full-jitter backoff | 100 / 30000 / 2.0 |

定义见：`crates/axiom-runtime/src/runtime/mod.rs`（`RuntimeConfig` / `RuntimeHealth`）。

---

## 6. 已知偏差（有意简化）

1. **Mailbox**：低延迟 `parking_lot` 队列，非完全无锁结构。  
2. **DLQ「sqlite」适配器**：文件 JSON 持久化，语义为 peek/ack/retry，非完整 sqlx 表结构。  
3. **Prometheus 导出**：默认开的是运行时 `metrics_active` 标志；完整 Prometheus 仍在 `axiom-viz`。  
4. **Workbench**：白名单沙箱，非无限 LLM 写代码。

---

## 7. 仍不在范围内（非目标）

- LE Go 全量机械迁移  
- 多租户计费 / 跨区 HA  
- 完整 MCP 产品化  
- `cargo test --workspace` 全仓强制全绿  

---

## 8. 文档索引

| 文档 | 内容 |
|------|------|
| `docs/TASK_CHECKLIST.md` | 勾选清单（已全绿） |
| `docs/COMMERCIAL_OPS.md` | 运维 / 健康 / 鉴权 / 部署 |
| `docs/deployment.md` | 部署指南 |
| `docs/PRODUCTION.md` | 生产部署总览 |
| `../../unified/COMMERCIAL_DELIVERY.md` | 商用交付说明（产品面） |
| 本文 | 工程硬化与生产接线说明 |

---

## 9. 变更记录

| 日期 | 说明 |
|------|------|
| 2026-07-19 | v0.5.0 清单关闭；补生产心跳/metrics/Guard 真路径与路径测试 |
