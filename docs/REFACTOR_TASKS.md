# Axiom Core v0.4.0 — 重构任务清单

> **Branch**: `feat/wasm-plugin-core`  
> **Version Target**: v0.4.0  
> **Date**: 2026-07-05  
> **Status**: 待执行  
> ** granularity**: 最小可执行单元

---

## Phase 0：基础设施（1 天）

### T0.1 创建新 crate 骨架
- [ ] 创建 `crates/axiom-kernel/` 目录
- [ ] 创建 `crates/axiom-kernel/Cargo.toml`
  - 依赖：`serde`, `toml`, `thiserror`, `tracing`, `uuid`, `glob`
- [ ] 创建 `crates/axiom-kernel/src/lib.rs`（空 crate，导出 `pub mod` 占位）
- [ ] 创建模块目录结构：
  - `src/cell.rs`
  - `src/signal.rs`
  - `src/lens.rs`
  - `src/axiom.rs`
  - `src/witness.rs`
  - `src/plugin/mod.rs`
  - `src/plugin/abi.rs`
  - `src/plugin/loader/mod.rs`
  - `src/plugin/loader/native.rs`
  - `src/plugin/loader/wasm.rs`
  - `src/plugin/registry.rs`
  - `src/plugin/composer.rs`
  - `src/heatmap/mod.rs`
  - `src/heatmap/collector.rs`
  - `src/heatmap/exporter.rs`

**验收标准**：
- `cargo check -p axiom-kernel` 通过
- 所有模块文件存在且可编译

---

## Phase 1：Native 插件原型（3 天）

### T1.1 定义 Plugin ABI
- [ ] 在 `src/plugin/abi.rs` 中定义：
  - `AxiomPlugin` trait（id, version, dependencies, capabilities, init, start, stop, handle_message）
  - `PluginContext` struct（cells, signals, lens, axioms, witness, plugins, logger, metrics）
  - `PluginMessage` enum
  - `PluginReply` enum
  - `PluginError` enum
  - `PluginKind` enum（llm, memory, tool, mcp, planner, alert, viz, governance）
  - `CapabilityDescriptor` struct
  - `PluginStatus` enum（Loaded, Initialized, Running, Stopped, Error）

**验收标准**：
- `cargo check -p axiom-kernel` 通过
- 所有 trait/struct 有完整文档注释
- `AxiomPlugin` 可以在测试中手动实现

### T1.2 实现 Plugin Registry
- [ ] 在 `src/plugin/registry.rs` 中实现：
  - `PluginRegistry` struct
  - `register(plugin: Box<dyn AxiomPlugin>)`
  - `get(id: &str) -> Option<&dyn AxiomPlugin>`
  - `get_all_by_kind(kind: PluginKind) -> Vec<&dyn AxiomPlugin>`
  - `dependencies_resolved(&self, id: &str) -> bool`
  - `resolve_dependencies(&mut self) -> Result<(), PluginError>`
  - `list_all() -> Vec<&dyn AxiomPlugin>`

**验收标准**：
- 单元测试覆盖注册、查询、依赖解析
- 循环依赖检测测试通过

### T1.3 实现 Native Plugin Loader
- [ ] 在 `src/plugin/loader/native.rs` 中实现：
  - `NativePluginLoader` struct
  - `load(path: &Path) -> Result<Box<dyn AxiomPlugin>, PluginError>`
    - 使用 `libloading` crate 加载 `.so`/`.dll`/`.dylib`
    - 调用导出的 `axiom_plugin_create` 函数
  - `unload(plugin: &dyn AxiomPlugin) -> Result<(), PluginError>`
  - 错误处理：符号缺失、ABI 不匹配、加载失败

**验收标准**：
- 单元测试：加载一个测试 Native 插件
- 错误路径测试：缺失符号、错误路径

### T1.4 创建测试 Native 插件
- [ ] 创建 `crates/axiom-plugin-test/` 目录
- [ ] 创建 `crates/axiom-plugin-test/Cargo.toml`
  - `crate-type = ["cdylib"]`
  - 依赖：`axiom-kernel`
- [ ] 创建 `crates/axiom-plugin-test/src/lib.rs`
  - 导出 `#[no_mangle] pub extern "C" fn axiom_plugin_create() -> *mut dyn AxiomPlugin`
  - 实现 `EchoPlugin`（回显消息）
  - 实现 `CounterPlugin`（计数）
- [ ] 在 `axiom-kernel/tests/` 中编写集成测试：
  - 加载 `axiom-plugin-test`
  - 发送消息，验证回显
  - 验证计数递增

**验收标准**：
- `cargo test -p axiom-kernel` 通过
- 测试 Native 插件加载、消息收发、卸载

### T1.5 实现 Composer（TOML 配置解析）
- [ ] 在 `src/plugin/composer.rs` 中实现：
  - `Composer` struct
  - `from_file(path: &Path) -> Result<Self, ComposerError>`
  - `from_str(toml: &str) -> Result<Self, ComposerError>`
  - `compose(&self, registry: &mut PluginRegistry) -> Result<(), ComposerError>`
    - 解析 `[system]` 段
    - 解析 `[[plugins]]` 数组
    - 解析 `[[connections]]` 数组
    - 按 `kind` 和 `instance` 加载插件
    - 建立连接关系
  - `SystemComposition` struct
  - `PluginSpec` struct
  - `ConnectionSpec` struct

**验收标准**：
- 单元测试：解析有效 TOML，验证插件列表
- 单元测试：解析无效 TOML，验证错误处理
- 集成测试：从 TOML 文件加载 2 个插件并建立连接

### T1.6 实现核心原语 Kernel（最小实现）
- [ ] 在 `src/cell.rs` 中实现：
  - `CellKernel` struct（管理 Cell 注册表）
  - `create(id: CellId, kind: CellKind) -> CellHandle`
  - `send(handle: &CellHandle, msg: SignalEnvelope) -> Result<(), KernelError>`
  - `receive(handle: &CellHandle) -> Result<SignalEnvelope, KernelError>`
- [ ] 在 `src/signal.rs` 中实现：
  - `SignalKernel` struct
  - `send(envelope: SignalEnvelope) -> Result<(), KernelError>`
  - `register_handler(handler: Box<dyn SignalHandler>)`
- [ ] 在 `src/lens.rs` 中实现：
  - `LensKernel` struct
  - `register(lens: Box<dyn Lens>)`
  - `query(id: &str, state: &State) -> Result<Projection, KernelError>`
- [ ] 在 `src/axiom.rs` 中实现：
  - `AxiomKernel` struct
  - `register(axiom: Box<dyn Axiom>)`
  - `check(state: &State, msg: &Message) -> Result<(), AxiomViolation>`
- [ ] 在 `src/witness.rs` 中实现：
  - `WitnessKernel` struct
  - `record(witness: Witness)`
  - `verify_chain() -> Result<(), WitnessError>`

**验收标准**：
- 单元测试覆盖每个 Kernel 的基本 CRUD
- 集成测试：创建 Cell → 发送 Signal → 验证 Witness

### T1.7 实现 Plugin Context
- [ ] 在 `src/plugin/abi.rs` 中实现：
  - `PluginContext` struct，包含：
    - `cells: CellKernel`
    - `signals: SignalKernel`
    - `lens: LensKernel`
    - `axioms: AxiomKernel`
    - `witness: WitnessKernel`
    - `plugins: PluginRegistry`
    - `logger: PluginLogger`
    - `metrics: PluginMetrics`
  - 所有字段为 `Arc<RwLock<...>>` 以支持并发访问

**验收标准**：
- 单元测试：插件通过 `PluginContext` 访问内核
- 线程安全测试：多插件并发访问内核

---

## Phase 2：WASM 运行时（3 天）

### T2.1 集成 wasmtime
- [ ] 在 `Cargo.toml` 中添加依赖：
  - `wasmtime = "21"`
  - `wasmtime-wasi = "21"`
  - `wasmtime-component-util = "21"`
- [ ] 创建 `src/plugin/loader/wasm.rs`
- [ ] 实现 `WasmPluginLoader` struct：
  - `load(path: &Path) -> Result<Box<dyn AxiomPlugin>, PluginError>`
  - `unload(plugin: &dyn AxiomPlugin) -> Result<(), PluginError>`

**验收标准**：
- `cargo check -p axiom-kernel` 通过
- 加载一个简单的 WASM 插件（编译为 `.wasm`）
- 调用 WASM 插件导出的函数

### T2.2 定义 WASM 插件 ABI
- [ ] 在 `src/plugin/abi.rs` 中定义 WASM 导出函数签名：
  - `axiom_plugin_create() -> *mut PluginInstance`
  - `axiom_plugin_destroy(ptr: *mut PluginInstance)`
  - `axiom_plugin_handle_message(ptr: *mut PluginInstance, msg_ptr: *const u8, msg_len: usize) -> *mut PluginReply`
- [ ] 创建 `crates/axiom-plugin-wasm-sdk/`：
  - 提供 Rust 宏 `#[axiom_wasm_plugin]` 自动实现 ABI
  - 提供 `WasmPluginHost` 辅助 trait

**验收标准**：
- 使用 SDK 创建一个测试 WASM 插件
- 编译为 `.wasm` 并成功加载

### T2.3 实现 WASM 插件实例管理
- [ ] 实现 `WasmPluginInstance` struct：
  - 包含 `wasmtime::Instance`
  - 包含 `wasmtime::Linker`
  - 包含 `wasmtime::Store`
- [ ] 实现内存管理：
  - 在 WASM 线性内存中分配/释放 `PluginMessage`
  - 序列化/反序列化使用 `postcard` 或 `bincode`

**验收标准**：
- WASM 插件可以接收内核消息
- WASM 插件可以发送回复
- 内存不泄漏（使用 `wasmtime` 的 GC 或手动释放）

### T2.4 实现插件间通信
- [ ] 在 `PluginContext` 中添加：
  - `send_to_plugin(from: &str, to: &str, msg: PluginMessage) -> Result<PluginReply, PluginError>`
- [ ] 实现消息路由：
  - 内核根据 `to` 字段查找目标插件
  - 调用目标插件的 `handle_message`
  - 返回 `PluginReply`

**验收标准**：
- 两个 WASM 插件可以互相发送消息
- Native 插件和 WASM 插件可以互相通信

### T2.5 WASM 插件示例
- [ ] 创建 `crates/axiom-plugin-example-wasm/`：
  - `EchoPlugin`（回显）
  - `CounterPlugin`（计数）
  - `TransformerPlugin`（转换消息格式）
- [ ] 编写集成测试：
  - 加载 WASM 插件
  - 发送消息，验证回显
  - 验证插件间通信

**验收标准**：
- `cargo test -p axiom-kernel` 全部通过
- WASM 插件示例可以编译、加载、运行

---

## Phase 3：热力系统（2 天）

### T3.1 实现 HeatmapCollector
- [ ] 在 `src/heatmap/collector.rs` 中实现：
  - `HeatmapCollector` struct
  - `record_cell_message(cell_id: CellId)`
  - `record_signal_send(signal_type: &str)`
  - `record_tool_invoke(tool_id: &str)`
  - `record_axiom_check(axiom_id: &str)`
  - `record_lens_query(lens_id: &str)`
  - `snapshot() -> UsageSnapshot`
  - `timeline(range: TimeRange) -> Vec<UsageSnapshot>`
  - `top_cells(n: usize) -> Vec<(CellId, u64)>`
  - `top_tools(n: usize) -> Vec<(String, u64)>`

**验收标准**：
- 单元测试：记录 1000 次消息，验证计数器正确
- 单元测试：验证 `top_cells(5)` 返回前 5 名

### T3.2 内核埋点
- [ ] 在 `CellKernel::send` 中调用 `heatmap.record_cell_message`
- [ ] 在 `SignalKernel::send` 中调用 `heatmap.record_signal_send`
- [ ] 在 `AxiomKernel::check` 中调用 `heatmap.record_axiom_check`
- [ ] 在 `LensKernel::query` 中调用 `heatmap.record_lens_query`
- [ ] 在 `PluginContext::handle_message` 中调用 `heatmap.record_tool_invoke`

**验收标准**：
- 端到端测试：发送 100 条消息，验证热力数据正确记录

### T3.3 实现 HeatmapExporter
- [ ] 在 `src/heatmap/exporter.rs` 中实现：
  - `HeatmapExporter` trait
  - `JsonExporter`（导出 JSON）
  - `PrometheusExporter`（导出 Prometheus 指标）
  - `VizExporter`（导出 `axiom-viz` 格式）

**验收标准**：
- 单元测试：导出 JSON，验证格式
- 单元测试：导出 Prometheus 指标，验证指标名称

### T3.4 CLI 命令 `axm heatmap`
- [ ] 在 `crates/axiom-cli/src/commands/` 中创建 `heatmap.rs`
- [ ] 实现子命令：
  - `axm heatmap --live`（实时显示）
  - `axm heatmap --export <path>`（导出 JSON）
  - `axm heatmap --module <name>`（按模块过滤）
  - `axm heatmap --since <duration>`（时间范围）
- [ ] 在 `crates/axiom-cli/src/main.rs` 中注册命令

**验收标准**：
- 手动测试：`axm heatmap --live` 实时显示热力图
- 手动测试：`axm heatmap --export heatmap.json` 导出文件
- 导出文件格式验证

---

## Phase 4：插件市场与工具链（持续）

### T4.1 插件打包格式
- [ ] 定义 `.axm-plugin` 包格式：
  - 文件头：`AXM_PLUGIN` magic bytes
  - 版本号：u32
  - Manifest：JSON（id, version, dependencies, capabilities, entry）
  - WASM 字节码：zlib 压缩
  - 签名：Ed25519
- [ ] 实现打包工具 `axm plugin pack`
- [ ] 实现解包工具 `axm plugin unpack`

**验收标准**：
- 打包测试插件，验证 `.axm-plugin` 格式
- 解包验证完整性

### T4.2 插件管理 CLI
- [ ] 实现 `axm plugin list`（列出已安装插件）
- [ ] 实现 `axm plugin install <path>`（安装插件）
- [ ] 实现 `axm plugin uninstall <id>`（卸载插件）
- [ ] 实现 `axm plugin info <id>`（插件详情）

**验收标准**：
- 手动测试：安装、列出、卸载插件
- 错误处理：重复安装、依赖缺失

### T4.3 插件版本管理
- [ ] 实现 `PluginVersion` struct
- [ ] 实现版本解析和比较
- [ ] 实现依赖版本约束（semver）
- [ ] 实现插件仓库索引

**验收标准**：
- 版本比较测试
- 依赖解析测试（包括冲突检测）

---

## 最小可行产品（MVP）任务集

如果资源有限，优先完成以下任务：

### Week 1：核心 + Native 插件
- [ ] T0.1：创建 crate 骨架
- [ ] T1.1：定义 Plugin ABI
- [ ] T1.2：实现 Plugin Registry
- [ ] T1.3：实现 Native Plugin Loader
- [ ] T1.4：创建测试 Native 插件
- [ ] T1.6：实现核心原语 Kernel（最小实现）

### Week 2：Composer + 热力
- [ ] T1.5：实现 Composer
- [ ] T1.7：实现 Plugin Context
- [ ] T3.1：实现 HeatmapCollector
- [ ] T3.2：内核埋点
- [ ] T3.4：CLI `axm heatmap` 基础版

---

## 任务依赖图

```
T0.1 (crate 骨架)
  │
  ├── T1.1 (Plugin ABI)
  │     │
  │     ├── T1.2 (Plugin Registry)
  │     │     │
  │     │     ├── T1.3 (Native Loader)
  │     │     │     │
  │     │     │     └── T1.4 (测试 Native 插件)
  │     │     │           │
  │     │     │           └── T1.6 (核心原语 Kernel)
  │     │     │
  │     │     ├── T1.5 (Composer)
  │     │     │     │
  │     │     │     └── T1.7 (Plugin Context)
  │     │     │           │
  │     │     │           └── T2.x (WASM 运行时)
  │     │     │
  │     │     └── T3.x (热力系统)
  │     │
  │     └── T4.x (插件市场)
  │
  └── T3.x (热力系统)
        │
        └── T3.4 (CLI axm heatmap)
```

---

## 估算

| Phase | 任务数 | 估算时间 | 优先级 |
|-------|--------|----------|--------|
| Phase 0 | 1 | 0.5 天 | P0 |
| Phase 1 | 7 | 3 天 | P0 |
| Phase 2 | 5 | 3 天 | P1 |
| Phase 3 | 4 | 2 天 | P1 |
| Phase 4 | 3 | 持续 | P2 |
| **MVP** | **7** | **1 周** | - |

---

## 执行建议

1. **严格按 Phase 执行**，每个 Phase 完成后验证验收标准再进入下一个
2. **每个任务独立 commit**，便于回滚和 review
3. **测试驱动**：先写验收测试，再写实现
4. **每日验证**：每天结束前运行 `cargo test -p axiom-kernel` 确保不break
