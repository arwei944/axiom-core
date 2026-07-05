# Axiom Core v0.4.0 — 重构验收文档

> **Branch**: `feat/wasm-plugin-core`  
> **Version Target**: v0.4.0  
> **Date**: 2026-07-05  
> **Status**: 待执行  
> **Purpose**: 定义每个 Phase / 任务的验收标准

---

## 1. 验收原则

### 1.1 Definition of Done

每个任务完成必须满足：
1. **代码完成**：实现代码 + 单元测试 + 集成测试
2. **测试通过**：`cargo test -p axiom-kernel` 全部通过
3. **文档完整**：公共 API 有文档注释
4. **CI 通过**：`cargo fmt`, `cargo clippy`, `cargo test` 无警告
5. **无破坏**：现有 `axiom-core` / `axiom-runtime` 等 crate 不受影响

### 1.2 验收层级

| 层级 | 描述 | 验证方法 |
|------|------|----------|
| **单元测试** | 单个函数/struct 的正确性 | `cargo test -p axiom-kernel` |
| **集成测试** | 模块间协作 | `cargo test --test integration` |
| **系统测试** | 端到端场景 | 手动运行示例程序 |
| **性能测试** | 延迟/吞吐量 | `cargo bench -p axiom-kernel` |
| **兼容性测试** | 与 v0.3.0 共存 | 同时运行两个版本 |

---

## 2. Phase 0：基础设施验收

### T0.1 创建新 crate 骨架

**功能验收**：
- [ ] `cargo check -p axiom-kernel` 通过
- [ ] 所有模块文件（`cell.rs`, `signal.rs`, `plugin/abi.rs` 等）存在且可编译
- [ ] `cargo doc -p axiom-kernel` 生成文档无错误

**质量验收**：
- [ ] `cargo fmt --check -p axiom-kernel` 通过
- [ ] `cargo clippy -p axiom-kernel -- -D warnings` 通过
- [ ] 无未使用导入、无 dead code

---

## 3. Phase 1：Native 插件原型验收

### T1.1 定义 Plugin ABI

**功能验收**：
- [ ] `AxiomPlugin` trait 包含 8 个方法：`id`, `version`, `dependencies`, `capabilities`, `init`, `start`, `stop`, `handle_message`
- [ ] `PluginContext` 包含 8 个字段：`cells`, `signals`, `lens`, `axioms`, `witness`, `plugins`, `logger`, `metrics`
- [ ] `PluginMessage` 和 `PluginReply` 支持至少 5 种消息类型
- [ ] `PluginError` 覆盖所有错误场景：加载失败、初始化失败、消息处理失败、依赖缺失

**测试验收**：
- [ ] 单元测试：手动实现 `AxiomPlugin` 并验证所有方法可调用
- [ ] 单元测试：`PluginContext` 字段可正常访问
- [ ] 文档测试：`AxiomPlugin` trait 的文档示例可编译

### T1.2 实现 Plugin Registry

**功能验收**：
- [ ] 注册 3 个不同 `PluginKind` 的插件，验证 `get_all_by_kind` 正确过滤
- [ ] 注册有循环依赖的插件，验证 `resolve_dependencies` 返回错误
- [ ] 注册依赖缺失的插件，验证错误信息包含缺失的依赖 ID
- [ ] `list_all` 返回所有已注册插件，按注册顺序

**测试验收**：
```rust
// 必须通过的测试用例
#[test]
fn test_register_and_get() { }
#[test]
fn test_get_all_by_kind_filters_correctly() { }
#[test]
fn test_resolve_dependencies_detects_cycles() { }
#[test]
fn test_resolve_dependencies_detects_missing() { }
#[test]
fn test_list_all_returns_all_plugins() { }
```

### T1.3 实现 Native Plugin Loader

**功能验收**：
- [ ] 加载有效的 `.so`/`.dll`/`.dylib` 文件，返回 `Box<dyn AxiomPlugin>`
- [ ] 加载不存在的文件，返回 `PluginError::LoadFailed`
- [ ] 加载缺少 `axiom_plugin_create` 符号的库，返回 `PluginError::MissingSymbol`
- [ ] 加载 ABI 版本不匹配的库，返回 `PluginError::AbiMismatch`
- [ ] 卸载插件后，资源被正确释放（无内存泄漏）

**测试验收**：
```rust
#[test]
fn test_load_valid_native_plugin() { }
#[test]
fn test_load_missing_file_returns_error() { }
#[test]
fn test_load_missing_symbol_returns_error() { }
#[test]
fn test_unload_releases_resources() { }
```

**跨平台验收**：
- [ ] Linux (`libplugin.so`) 加载成功
- [ ] Windows (`plugin.dll`) 加载成功（如有可能）
- [ ] macOS (`libplugin.dylib`) 加载成功（如有可能）

### T1.4 创建测试 Native 插件

**功能验收**：
- [ ] `EchoPlugin` 接收任意消息，返回相同内容
- [ ] `CounterPlugin` 每次调用 `handle_message` 计数 +1，返回当前计数
- [ ] 插件正确报告 `id`, `version`, `dependencies`, `capabilities`
- [ ] 插件可以正常 `init`, `start`, `stop` 生命周期

**测试验收**：
```rust
#[test]
fn test_echo_plugin_returns_same_message() { }
#[test]
fn test_counter_plugin_increments() { }
#[test]
fn test_plugin_lifecycle_init_start_stop() { }
```

**集成验收**：
- [ ] `axiom-kernel` 成功加载 `axiom-plugin-test`
- [ ] 向 `EchoPlugin` 发送 100 条消息，全部正确回显
- [ ] `CounterPlugin` 计数从 0 到 99

### T1.5 实现 Composer

**功能验收**：
- [ ] 解析有效 TOML，正确提取 `system.name`, `plugins`, `connections`
- [ ] 支持 `plugin.instance > 1`，创建多个插件实例
- [ ] 支持 `config` 字段传递给插件 `init`
- [ ] 解析 `[[connections]]`，建立 `from -> to` 的消息路由
- [ ] 缺少必需字段时返回明确的 `ComposerError`

**测试验收**：
```rust
#[test]
fn test_parse_valid_composition() { }
#[test]
fn test_parse_multiple_instances() { }
#[test]
fn test_parse_connections() { }
#[test]
fn test_missing_required_field_returns_error() { }
#[test]
fn test_invalid_toml_returns_error() { }
```

**集成验收**：
- [ ] 提供 `examples/composition.toml` 示例文件
- [ ] `Composer::from_file("examples/composition.toml")` 成功解析
- [ ] 加载后，插件间可以通过连接发送消息

### T1.6 实现核心原语 Kernel

**功能验收**：

**CellKernel**：
- [ ] 创建 Cell 时分配唯一 `CellId`
- [ ] 向 Cell 发送 Signal，`receive` 可以按 FIFO 顺序接收
- [ ] 重复发送相同 `msg_id` 的 Signal 被拒绝（幂等性）
- [ ] Cell 崩溃后，`Supervisor` 自动重启（如已集成）

**SignalKernel**：
- [ ] 发送 Signal 时自动附加 `VectorClock`
- [ ] `register_handler` 注册的 handler 可以接收所有 Signal
- [ ] Signal 包含正确的 `source`, `target`, `correlation_id`

**LensKernel**：
- [ ] 注册 Lens 后，可以通过 `query` 查询
- [ ] 查询不存在的 Lens 返回 `KernelError::LensNotFound`
- [ ] Lens 缓存机制生效（相同查询第二次更快）

**AxiomKernel**：
- [ ] 注册 Axiom 后，所有状态转换自动检查
- [ ] Axiom 违反时返回 `AxiomViolation`
- [ ] 多个 Axiom 按注册顺序检查，任一违反即停止

**WitnessKernel**：
- [ ] 每次状态转换自动记录 Witness
- [ ] `verify_chain` 验证 Witness 链完整性
- [ ] Witness 链断裂时返回 `WitnessError::ChainBroken`

**测试验收**：
```rust
// CellKernel
#[test]
fn test_cell_create_assigns_unique_id() { }
#[test]
fn test_cell_send_receive_fifo() { }
#[test]
fn test_cell_duplicate_msg_id_rejected() { }

// SignalKernel
#[test]
fn test_signal_auto_attaches_vector_clock() { }
#[test]
fn test_handler_receives_all_signals() { }

// LensKernel
#[test]
fn test_lens_query_returns_projection() { }
#[test]
fn test_lens_cache_improves_performance() { }

// AxiomKernel
#[test]
fn test_axiom_violation_returns_error() { }
#[test]
fn test_multiple_axioms_checked_in_order() { }

// WitnessKernel
#[test]
fn test_witness_recorded_on_state_transition() { }
#[test]
fn test_verify_chain_valid_chain() { }
#[test]
fn test_verify_chain_broken_chain() { }
```

### T1.7 实现 Plugin Context

**功能验收**：
- [ ] 插件可以通过 `ctx.cells.send(...)` 发送消息
- [ ] 插件可以通过 `ctx.signals.send(...)` 发送 Signal
- [ ] 插件可以通过 `ctx.lens.query(...)` 查询 Lens
- [ ] 插件可以通过 `ctx.axioms.check(...)` 检查 Axiom
- [ ] 插件可以通过 `ctx.witness.record(...)` 记录 Witness
- [ ] 插件可以通过 `ctx.plugins.get("other-plugin")` 访问其他插件
- [ ] `ctx.logger` 支持 `info`, `warn`, `error` 级别
- [ ] `ctx.metrics` 支持计数器、仪表盘、直方图

**测试验收**：
```rust
#[test]
fn test_plugin_can_send_via_context() { }
#[test]
fn test_plugin_can_query_lens_via_context() { }
#[test]
fn test_plugin_can_access_other_plugin() { }
#[test]
fn test_plugin_logger_works() { }
#[test]
fn test_plugin_metrics_record_counters() { }
```

**并发验收**：
- [ ] 10 个插件同时通过 `PluginContext` 发送消息，无数据竞争
- [ ] 使用 `cargo test --test concurrency` 验证

---

## 4. Phase 2：WASM 运行时验收

### T2.1 集成 wasmtime

**功能验收**：
- [ ] `wasmtime` 21.x 版本成功编译
- [ ] 加载一个简单的 WASM 模块（返回 `42`）
- [ ] WASM 模块可以访问导入的函数（如 `logger.info`）

**测试验收**：
```rust
#[test]
fn test_load_wasm_module() { }
#[test]
fn test_wasm_can_call_imported_function() { }
```

### T2.2 定义 WASM 插件 ABI

**功能验收**：
- [ ] `axiom_plugin_create` 返回有效的 `PluginInstance` 指针
- [ ] `axiom_plugin_destroy` 正确释放资源
- [ ] `axiom_plugin_handle_message` 可以接收序列化消息并返回回复
- [ ] ABI 版本检查：不匹配时返回错误

**测试验收**：
```rust
#[test]
fn test_wasm_abi_create_destroy() { }
#[test]
fn test_wasm_abi_handle_message() { }
#[test]
fn test_wasm_abi_version_mismatch() { }
```

### T2.3 实现 WASM 插件实例管理

**功能验收**：
- [ ] WASM 插件可以分配线性内存存储 `PluginMessage`
- [ ] WASM 插件可以读取内核传入的消息
- [ ] WASM 插件可以写入回复到线性内存
- [ ] 内核可以正确读取 WASM 插件的回复
- [ ] 内存泄漏检测：加载/卸载 1000 次，内存稳定

**测试验收**：
```rust
#[test]
fn test_wasm_memory_allocation() { }
#[test]
fn test_wasm_memory_no_leak() { }
```

### T2.4 实现插件间通信

**功能验收**：
- [ ] WASM 插件 A 可以向 WASM 插件 B 发送消息
- [ ] WASM 插件可以向 Native 插件发送消息
- [ ] Native 插件可以向 WASM 插件发送消息
- [ ] 消息路由正确，无丢失

**测试验收**：
```rust
#[test]
fn test_wasm_to_wasm_communication() { }
#[test]
fn test_wasm_to_native_communication() { }
#[test]
fn test_native_to_wasm_communication() { }
```

### T2.5 WASM 插件示例

**功能验收**：
- [ ] `EchoPlugin`（WASM 版）正确回显
- [ ] `CounterPlugin`（WASM 版）正确计数
- [ ] `TransformerPlugin` 将消息转换为大写
- [ ] 所有示例可以编译为 `.wasm` 并成功加载

**测试验收**：
```rust
#[test]
fn test_wasm_echo_plugin() { }
#[test]
fn test_wasm_counter_plugin() { }
#[test]
fn test_wasm_transformer_plugin() { }
```

**性能验收**：
- [ ] WASM 插件消息延迟 < 1ms（本地测试）
- [ ] 吞吐量 > 1000 msg/s

---

## 5. Phase 3：热力系统验收

### T3.1 实现 HeatmapCollector

**功能验收**：
- [ ] 记录 1000 次 `cell_message`，`top_cells(5)` 返回前 5 名
- [ ] 记录 1000 次 `signal_send`，`top_signals(5)` 返回前 5 名
- [ ] 记录 1000 次 `tool_invoke`，`top_tools(5)` 返回前 5 名
- [ ] `snapshot` 返回当前时刻的快照
- [ ] `timeline` 返回时间范围内的所有快照

**测试验收**：
```rust
#[test]
fn test_record_cell_messages_and_top_cells() { }
#[test]
fn test_record_signals_and_top_signals() { }
#[test]
fn test_snapshot_captures_current_state() { }
#[test]
fn test_timeline_returns_range() { }
```

**并发验收**：
- [ ] 10 个线程同时记录消息，计数器准确
- [ ] 无数据竞争（`cargo test --test concurrency`）

### T3.2 内核埋点

**功能验收**：
- [ ] 发送 Signal 时，`signal_send` 计数 +1
- [ ] Cell 接收消息时，`cell_message` 计数 +1
- [ ] 执行 Tool 时，`tool_invoke` 计数 +1
- [ ] 检查 Axiom 时，`axiom_check` 计数 +1
- [ ] 查询 Lens 时，`lens_query` 计数 +1

**测试验收**：
```rust
#[test]
fn test_signal_send_increments_counter() { }
#[test]
fn test_cell_receive_increments_counter() { }
#[test]
fn test_tool_invoke_increments_counter() { }
#[test]
fn test_axiom_check_increments_counter() { }
#[test]
fn test_lens_query_increments_counter() { }
```

### T3.3 实现 HeatmapExporter

**功能验收**：

**JSON 导出**：
- [ ] 导出格式包含 `timestamp`, `hot_cells`, `hot_signals`, `hot_tools`
- [ ] JSON 格式可通过 `serde_json` 反序列化

**Prometheus 导出**：
- [ ] 导出指标名称符合 Prometheus 命名规范
- [ ] 指标类型正确（Counter, Gauge, Histogram）
- [ ] 标签（label）正确设置

**Viz 导出**：
- [ ] 导出格式与 `axiom-viz` 的 `VizSnapshot` 兼容
- [ ] 拓扑图节点颜色按使用频率映射

**测试验收**：
```rust
#[test]
fn test_json_export_format() { }
#[test]
fn test_prometheus_export_format() { }
#[test]
fn test_viz_export_format() { }
```

### T3.4 CLI 命令 `axm heatmap`

**功能验收**：
- [ ] `axm heatmap --live` 实时显示热力图（刷新间隔 1s）
- [ ] `axm heatmap --export heatmap.json` 导出 JSON 文件
- [ ] `axm heatmap --module llm` 只显示 LLM 模块的热力
- [ ] `axm heatmap --since 1h` 显示最近 1 小时的数据
- [ ] `axm heatmap --top 10` 显示前 10 个热点

**测试验收**：
- [ ] 手动运行 `axm heatmap --export test.json`，验证文件格式
- [ ] 手动运行 `axm heatmap --live`，验证实时更新
- [ ] 错误处理：无效时间范围、无效模块名

---

## 6. Phase 4：插件市场与工具链验收

### T4.1 插件打包格式

**功能验收**：
- [ ] `.axm-plugin` 文件包含正确的 magic bytes
- [ ] 文件头包含版本号
- [ ] Manifest 包含 `id`, `version`, `dependencies`, `capabilities`, `entry`
- [ ] WASM 字节码经过 zlib 压缩
- [ ] 签名验证通过 Ed25519

**测试验收**：
```rust
#[test]
fn test_pack_creates_valid_axm_plugin() { }
#[test]
fn test_unpack_restores_original() { }
#[test]
fn test_signature_verification() { }
```

### T4.2 插件管理 CLI

**功能验收**：
- [ ] `axm plugin list` 列出所有已安装插件
- [ ] `axm plugin install plugin.axm-plugin` 安装成功
- [ ] `axm plugin uninstall my-plugin` 卸载成功
- [ ] `axm plugin info my-plugin` 显示插件详情
- [ ] 重复安装返回错误
- [ ] 卸载不存在的插件返回错误

**测试验收**：
- [ ] 手动测试完整安装-列出-卸载流程
- [ ] 错误场景测试

### T4.3 插件版本管理

**功能验收**：
- [ ] 支持 semver 版本号解析
- [ ] 支持版本约束（`^1.0.0`, `~1.2.0`, `>=1.0.0`）
- [ ] 版本冲突检测：两个插件依赖同一库的不同版本
- [ ] 插件仓库索引可以按 `kind` 过滤

**测试验收**：
```rust
#[test]
fn test_semver_parsing() { }
#[test]
fn test_version_constraint_satisfies() { }
#[test]
fn test_version_conflict_detection() { }
```

---

## 7. MVP 验收（Week 1 结束）

### 7.1 功能清单

| 功能 | 状态 | 验收方法 |
|------|------|----------|
| `axiom-kernel` crate 可编译 | ☐ | `cargo check -p axiom-kernel` |
| Plugin ABI 定义完整 | ☐ | 文档 + 单元测试 |
| Plugin Registry 可注册/查询插件 | ☐ | 单元测试 |
| Native Plugin Loader 可加载 `.so`/`.dll` | ☐ | 集成测试 |
| 测试 Native 插件（Echo + Counter） | ☐ | 集成测试 |
| 核心原语 Kernel（Cell/Signal/Lens/Axiom/Witness）最小实现 | ☐ | 单元测试 + 集成测试 |
| Plugin Context 可访问内核 | ☐ | 单元测试 |
| 无内存泄漏（加载/卸载 1000 次） | ☐ | `cargo test --test memory` |

### 7.2 性能基准

| 指标 | 目标 | 测试方法 |
|------|------|----------|
| 插件加载时间 | < 100ms | 计时测试 |
| 消息延迟（Native 插件） | < 1ms | 1000 次平均 |
| 消息吞吐量（Native 插件） | > 10,000 msg/s | 压力测试 |
| 内存增长（1000 次加载/卸载） | < 1MB | 内存 profiling |

### 7.3 兼容性

- [ ] 现有 `axiom-core` v0.3.0 API 不受影响
- [ ] `cargo check --workspace` 通过
- [ ] `cargo test --workspace` 通过
- [ ] `foxguard` 安全审计通过（0 findings）

---

## 8. 非功能性验收

### 8.1 性能

| 场景 | 目标 | 验证方法 |
|------|------|----------|
| 单 Cell 消息吞吐 | > 50,000 msg/s | `cargo bench` |
| 100 Cell 并发 | 无死锁，延迟 < 5ms | `loom` 测试 |
| Witness 链验证（1000 条） | < 10ms | 基准测试 |
| Lens 查询（缓存命中） | < 0.1ms | 基准测试 |
| Lens 查询（缓存未命中） | < 1ms | 基准测试 |

### 8.2 安全性

| 检查项 | 目标 | 验证方法 |
|--------|------|----------|
| `foxguard` 扫描 | 0 findings | CI job |
| `cargo audit` | 0 高危漏洞 | CI job |
| 不安全代码 | 0 行（除必要 FFI） | `cargo geiger` |
| 权限隔离 | 插件无法访问其他插件的私有数据 | 渗透测试 |

### 8.3 可靠性

| 场景 | 目标 | 验证方法 |
|------|------|----------|
| 插件崩溃 | 内核不崩溃，其他插件正常运行 | 混沌测试 |
| 插件热加载 | 无停机替换插件 | 集成测试 |
| 长时间运行 | 7x24 小时无内存泄漏 | 压力测试 |
| 网络分区（分布式场景） | 数据一致性保持 | 模拟测试 |

---

## 9. 验收流程

### 9.1 每日验收

每天结束前执行：
```bash
# 1. 编译检查
cargo check -p axiom-kernel

# 2. 格式化检查
cargo fmt --check -p axiom-kernel

# 3. Clippy 检查
cargo clippy -p axiom-kernel -- -D warnings

# 4. 单元测试
cargo test -p axiom-kernel

# 5. 集成测试
cargo test --test integration -p axiom-kernel

# 6. 安全扫描
cargo audit
```

### 9.2 Phase 验收

每个 Phase 完成后：
1. 运行全部测试套件
2. 更新 `REFACTOR_TASKS.md`，标记完成任务
3. 编写 Phase 总结报告
4. 提交代码并进行 code review

### 9.3 发布验收

v0.4.0-alpha 发布前：
1. 全部 Phase 验收通过
2. 性能基准测试通过
3. 安全扫描通过
4. 文档完整（README, API docs, 迁移指南）
5. 至少 3 个示例插件可用

---

## 10. 验收检查清单（Checklist）

### Phase 0
- [ ] T0.1：crate 骨架创建完成

### Phase 1
- [ ] T1.1：Plugin ABI 定义完成
- [ ] T1.2：Plugin Registry 实现完成
- [ ] T1.3：Native Plugin Loader 实现完成
- [ ] T1.4：测试 Native 插件创建完成
- [ ] T1.5：Composer 实现完成
- [ ] T1.6：核心原语 Kernel 实现完成
- [ ] T1.7：Plugin Context 实现完成

### Phase 2
- [ ] T2.1：wasmtime 集成完成
- [ ] T2.2：WASM 插件 ABI 定义完成
- [ ] T2.3：WASM 插件实例管理完成
- [ ] T2.4：插件间通信完成
- [ ] T2.5：WASM 插件示例完成

### Phase 3
- [ ] T3.1：HeatmapCollector 实现完成
- [ ] T3.2：内核埋点完成
- [ ] T3.3：HeatmapExporter 实现完成
- [ ] T3.4：CLI `axm heatmap` 完成

### Phase 4
- [ ] T4.1：插件打包格式完成
- [ ] T4.2：插件管理 CLI 完成
- [ ] T4.3：插件版本管理完成

---

## 11. 验收签署

| 角色 | 姓名 | 签名 | 日期 |
|------|------|------|------|
| 架构师 | | | |
| 开发负责人 | | | |
| 测试负责人 | | | |
| 产品负责人 | | | |

---

*本文档定义 v0.4.0 重构的严格验收标准，所有 Phase 必须通过验收才能进入下一阶段。*
