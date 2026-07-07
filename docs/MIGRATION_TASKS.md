# Axiom Core — 新架构完全迁移任务清单

> **Branch**: `feat/wasm-plugin-core`
> **Target**: 完全迁移到以 `axiom-kernel` 为核心的新架构
> **Granularity**: 最小可执行单元 + 可验证验收标准
> **Convention**: 每项任务必须满足“完成定义”才能标记完成

---

## 执行规则

1. **逐项验证**：每完成一个小任务，立即运行对应验证命令
2. **不跨步合并**：未达验收标准不得标记完成
3. **失败回退**：若验证失败，立即回退到上一个通过状态
4. **文档同步**：每完成一个大阶段，更新本清单状态

---

## Phase 0：新架构基础（已完成）

| 任务 | 状态 | 验收命令 |
|------|------|----------|
| T0.1 `axiom-kernel` crate 骨架 | ✅ | `cargo check -p axiom-kernel` |
| T0.2 模块结构创建 | ✅ | `cargo check -p axiom-kernel` |
| T0.3 Workspace 注册 | ✅ | `cargo check --workspace` |

**Phase 0 验证**：`cargo check --workspace` 通过

---

## Phase 1：Native 插件原型（已完成）

| 任务 | 状态 | 验收标准 |
|------|------|----------|
| T1.1 Plugin ABI 定义 | ✅ | trait/struct 可编译，测试可手动实现 |
| T1.2 Plugin Registry | ✅ | `register/get/get_all_by_kind/resolve_dependencies` 测试通过 |
| T1.3 Native Plugin Loader | ✅ | `libloading` 加载测试插件成功 |
| T1.4 测试 Native 插件 | ✅ | `cargo test -p axiom-plugin-test` 通过 |
| T1.5 Composer TOML 解析 | ✅ | 解析测试 + 集成测试通过 |
| T1.6 核心原语 Kernel | ✅ | 5 个 Kernel 基本 CRUD 测试通过 |
| T1.7 Plugin Context | ✅ | 插件通过 Context 访问内核，线程安全测试通过 |

**Phase 1 验证**：`cargo test -p axiom-kernel -p axiom-plugin-test` 通过

---

## Phase 2：WASM 运行时（已完成）

| 任务 | 状态 | 验收标准 |
|------|------|----------|
| T2.1 集成 wasmtime | ✅ | `cargo check -p axiom-kernel --features wasm-loader` 通过 |
| T2.2 WASM 插件 ABI + SDK | ✅ | `axiom-plugin-wasm-sdk` 编译通过 |
| T2.3 WASM 实例管理 | ✅ | `postcard` 序列化 + 内存读写实现 |
| T2.4 插件间通信 | ✅ | `PluginContext::send_to_plugin` 实现 |
| T2.5 WASM 示例 + 测试 | ✅ | 3 个插件，3 个集成测试通过 |

**Phase 2 验证**：`cargo test -p axiom-plugin-example-wasm` 通过

---

## Phase 3：热力统计与可视化（已完成）

| 任务 | 状态 | 验收标准 |
|------|------|----------|
| T3.1 HeatmapCollector | ✅ | `record/snapshot/top/timeline` 方法可用 |
| T3.2 内核埋点 | ✅ | 5 个 Kernel 均有埋点调用 |
| T3.3 HeatmapExporter | ✅ | JSON/Prometheus/Viz 三种格式可导出 |
| T3.4 CLI `axm heatmap` | ✅ | 支持 `--export/--top/--module/--since/--format` |

**Phase 3 验证**：`cargo check -p axiom-cli` 通过，`axm heatmap --format prometheus` 可运行

---

## Phase 4：插件管理（已完成）

| 任务 | 状态 | 验收标准 |
|------|------|----------|
| T4.1 打包格式 `.axm-plugin` | ✅ | `pack/unpack/pack_to_file/unpack_from_file` 可用 |
| T4.2 插件管理 CLI | ✅ | `axm plugin list/install/uninstall/info` 命令存在 |
| T4.3 版本管理 | ✅ | `PluginVersion/Dependency/RepositoryIndex/load_index` 可用 |

**Phase 4 验证**：`cargo check --workspace` 通过

---

## Phase 5：旧 `axiom-core` 桥接到新内核（待执行）

> **目标**：让旧 `axiom-core` 成为 `axiom-kernel` 的薄封装，保留对外 API 不变，内部转发到新内核

### T5.1 分析旧 `axiom-core` 公共接口
- [ ] 读取 `crates/axiom-core/src/lib.rs`
- [ ] 读取 `crates/axiom-core/src/context.rs`
- [ ] 读取 `crates/axiom-core/src/registry.rs`
- [ ] 读取 `crates/axiom-core/src/cell.rs`
- [ ] 读取 `crates/axiom-core/src/signal.rs`
- [ ] 读取 `crates/axiom-core/src/lens.rs`
- [ ] 读取 `crates/axiom-core/src/axiom.rs`
- [ ] 读取 `crates/axiom-core/src/witness.rs`
- [ ] 输出《旧 core 公共接口清单》

**验收标准**：
- 输出文档列出所有 `pub` 类型、trait、函数签名
- 标注哪些是“稳定 API”，哪些是“内部 API”

**验证命令**：
```bash
cargo doc --open # 人工检查文档
```

### T5.2 设计桥接层结构
- [ ] 在 `crates/axiom-core/src/` 下创建 `bridge/` 目录
- [ ] 设计桥接策略：
  - 类型别名：`pub type Cell = axiom_kernel::CellKernel`
  - 包装器：`pub struct CellHandle(axiom_kernel::CellHandle)`
  - 转发 impl：`impl Cell for Wrapper { ... }`
- [ ] 输出《桥接层设计文档》

**验收标准**：
- 设计文档通过评审
- 无循环依赖：`axiom-core -> axiom-kernel`，`axiom-kernel` 不依赖 `axiom-core`

**验证命令**：
```bash
cargo check -p axiom-kernel # 确保无循环依赖
```

### T5.3 实现桥接层：Cell/Signal/Lens
- [ ] 在 `bridge/cell.rs` 中实现旧 Cell API 转发
- [ ] 在 `bridge/signal.rs` 中实现旧 Signal API 转发
- [ ] 在 `bridge/lens.rs` 中实现旧 Lens API 转发
- [ ] 更新 `axiom-core/src/lib.rs` 导出桥接类型

**验收标准**：
- `cargo check -p axiom-core` 通过
- 旧 API 可调用，内部转发到 `axiom-kernel`

**验证命令**：
```bash
cargo check -p axiom-core
cargo test -p axiom-core
```

### T5.4 实现桥接层：Axiom/Witness/Context
- [ ] 在 `bridge/axiom.rs` 中实现旧 Axiom API 转发
- [ ] 在 `bridge/witness.rs` 中实现旧 Witness API 转发
- [ ] 在 `bridge/context.rs` 中实现旧 Context API 转发
- [ ] 更新 `axiom-core/src/lib.rs` 导出

**验收标准**：
- `cargo check -p axiom-core` 通过
- 旧 Context 可创建，内部包含新内核实例

**验证命令**：
```bash
cargo check -p axiom-core
```

### T5.5 桥接层集成测试
- [ ] 创建 `crates/axiom-core/tests/bridge_integration.rs`
- [ ] 测试：通过旧 API 创建 Cell → 发送 Signal → 验证 Witness
- [ ] 测试：通过旧 API 注册 Lens → 查询 Lens
- [ ] 测试：通过旧 API 注册 Axiom → 检查 Axiom

**验收标准**：
- 所有桥接集成测试通过
- 测试覆盖率 >= 80%

**验证命令**：
```bash
cargo test -p axiom-core --test bridge_integration
tarpaulin --workspace --fail-under 80 # 或等效覆盖率工具
```

### T5.6 旧 `axiom-core` 测试迁移
- [ ] 迁移 `architecture_conformance.rs`
- [ ] 迁移 `concurrency_tests.rs`
- [ ] 迁移 `error_path_tests.rs`
- [ ] 迁移 `integration_tests.rs`
- [ ] 迁移 `lens_tests.rs`

**验收标准**：
- 所有旧测试在新架构下通过
- 无测试被跳过或标记为 `ignore`

**验证命令**：
```bash
cargo test -p axiom-core
```

**Phase 5 验证**：`cargo test -p axiom-core` 全绿，旧 API 行为不变

---

## Phase 6：`axiom-runtime` 切换到新内核（待执行）

> **目标**：让 `axiom-runtime` 使用 `axiom-kernel` 作为底层内核

### T6.1 分析 Runtime 架构
- [ ] 读取 `crates/axiom-runtime/src/runtime/builder.rs`
- [ ] 读取 `crates/axiom-runtime/src/runtime/runtime_impl.rs`
- [ ] 读取 `crates/axiom-runtime/src/bus.rs`
- [ ] 读取 `crates/axiom-runtime/src/mailbox.rs`
- [ ] 读取 `crates/axiom-runtime/src/dispatch/loop.rs`
- [ ] 输出《Runtime 组件清单及依赖关系》

**验收标准**：
- 输出文档明确每个组件与旧 core 的依赖点

**验证命令**：
```bash
cargo doc -p axiom-runtime --open
```

### T6.2 Runtime Builder 切换到新内核
- [ ] 修改 `RuntimeBuilder` 装配 `CellKernel/SignalKernel/LensKernel/AxiomKernel/WitnessKernel`
- [ ] 移除旧 `axiom-core` 直接依赖（保留通过桥接层的间接依赖）
- [ ] 更新 `runtime_impl.rs` 使用新内核

**验收标准**：
- `cargo check -p axiom-runtime` 通过
- Runtime 可启动，无 panic

**验证命令**：
```bash
cargo check -p axiom-runtime
cargo test -p axiom-runtime --lib
```

### T6.3 Bus/Mailbox 适配新内核
- [ ] 修改 `Bus` 使用 `SignalKernel` 替代旧 Signal 实现
- [ ] 修改 `Mailbox` 使用新消息格式
- [ ] 更新 `dispatch/loop.rs` 使用新 dispatch 机制

**验收标准**：
- `cargo check -p axiom-runtime` 通过
- 消息通过 Bus 发送/接收正常

**验证命令**：
```bash
cargo test -p axiom-runtime --lib
```

### T6.4 EntropyGov/Guardian 适配新内核
- [ ] 修改 `EntropyGov` 使用新 `AxiomKernel`
- [ ] 修改 `Guardian` 使用新监控机制
- [ ] 更新 `entropy_interceptors.rs` 使用新拦截器接口

**验收标准**：
- `cargo check -p axiom-runtime` 通过
- 约束检查正常触发

**验证命令**：
```bash
cargo test -p axiom-runtime --lib
```

### T6.5 Runtime 集成测试
- [ ] 迁移 `runtime/tests.rs`
- [ ] 迁移 `concurrency_tests.rs`
- [ ] 迁移 `constraint_tests.rs`
- [ ] 迁移 `e2e_tests.rs`
- [ ] 迁移 `error_path_tests.rs`
- [ ] 迁移 `governance_integration.rs`
- [ ] 迁移 `persistence_tests.rs`
- [ ] 迁移 `restart_tests.rs`

**验收标准**：
- 所有 Runtime 测试通过
- 无测试被跳过或标记为 `ignore`

**验证命令**：
```bash
cargo test -p axiom-runtime
```

**Phase 6 验证**：`cargo test -p axiom-runtime` 全绿

---

## Phase 7：CLI 全量迁移（待执行）

> **目标**：所有 `axm` 命令内部使用新架构

### T7.1 迁移 `axm cell` 命令
- [ ] 读取 `crates/axiom-cli/src/commands/cell.rs`
- [ ] 修改命令实现调用 `axiom-kernel` API
- [ ] 保留旧命令参数和输出格式

**验收标准**：
- `cargo check -p axiom-cli` 通过
- `axm cell list` 等子命令可正常运行

**验证命令**：
```bash
cargo check -p axiom-cli
cargo run -p axiom-cli -- cell list
```

### T7.2 迁移 `axm entropy` 命令
- [ ] 修改实现调用新 `AxiomKernel`
- [ ] 保留旧输出格式

**验收标准**：
- `axm entropy` 命令正常运行

**验证命令**：
```bash
cargo run -p axiom-cli -- entropy
```

### T7.3 迁移 `axm new` / `axm init` 命令
- [ ] 修改 `new.rs`、`init.rs`、`new_cell.rs`、`new_signal.rs`、`new_crate.rs`、`new_tool.rs`
- [ ] 保留脚手架生成逻辑

**验收标准**：
- `axm new` 系列命令正常运行

**验证命令**：
```bash
cargo run -p axiom-cli -- new cell test-cell
```

### T7.4 迁移 `axm run` / `axm trace` / `axm witness` 命令
- [ ] 修改 `run.rs`、`trace.rs`、`witness.rs`
- [ ] 使用新 Runtime 和 Kernel

**验收标准**：
- 命令可正常运行

**验证命令**：
```bash
cargo run -p axiom-cli -- run
```

### T7.5 迁移 `axm top` / `axm why` 命令
- [ ] 修改 `top.rs`、`why.rs`
- [ ] 使用新 HeatmapCollector

**验收标准**：
- 命令可正常运行

**验证命令**：
```bash
cargo run -p axiom-cli -- top
```

### T7.6 CLI 集成测试
- [ ] 迁移 `axiom-cli/tests/` 下所有集成测试
- [ ] 确保所有 CLI 命令在测试环境下可调用

**验收标准**：
- `cargo test -p axiom-cli` 全绿

**验证命令**：
```bash
cargo test -p axiom-cli
```

**Phase 7 验证**：`cargo test -p axiom-cli` 全绿

---

## Phase 8：应用层迁移（待执行）

> **目标**：将 `agent/alert/llm/mcp/memory/tool/viz/distributed/identity/planner/prompt/oversight/store` 迁移为插件或适配新内核

### T8.1 `axiom-tool` 迁移为插件
- [ ] 分析 `axiom-tool/src/tool.rs` 公共接口
- [ ] 实现 `ToolPlugin` 包装器，将 `Tool` trait 适配为 `AxiomPlugin`
- [ ] 注册到 `PluginRegistry`
- [ ] 保留旧 `Tool` trait 作为桥接

**验收标准**：
- `cargo check -p axiom-tool` 通过
- 工具可通过插件机制加载

**验证命令**：
```bash
cargo test -p axiom-tool
```

### T8.2 `axiom-llm` 适配新内核
- [ ] 修改 `Client` 使用 `PluginContext::send_to_plugin` 与 LLM 插件通信
- [ ] 保留旧 `Client` API 作为桥接

**验收标准**：
- `cargo check -p axiom-llm` 通过

**验证命令**：
```bash
cargo test -p axiom-llm
```

### T8.3 `axiom-mcp` 适配新内核
- [ ] 修改 `McpClient` 使用新内核通信机制
- [ ] 保留旧 API 作为桥接

**验收标准**：
- `cargo check -p axiom-mcp` 通过

**验证命令**：
```bash
cargo test -p axiom-mcp
```

### T8.4 `axiom-memory` 通过 Lens Kernel 暴露
- [ ] 修改 `Memory` 实现为 `Lens` 投影
- [ ] 保留旧 `Memory` API 作为桥接

**验收标准**：
- `cargo check -p axiom-memory` 通过

**验证命令**：
```bash
cargo test -p axiom-memory
```

### T8.5 `axiom-store` 通过 Lens Kernel 暴露
- [ ] 修改 `Store` 实现为 `Lens` 投影
- [ ] 保留旧 `Store` API 作为桥接

**验收标准**：
- `cargo check -p axiom-store` 通过

**验证命令**：
```bash
cargo test -p axiom-store
```

### T8.6 `axiom-alert` 迁移为插件
- [ ] 实现 `AlertPlugin` 包装器
- [ ] 注册到 `PluginRegistry`
- [ ] 保留旧 `Alert` API 作为桥接

**验收标准**：
- `cargo check -p axiom-alert` 通过

**验证命令**：
```bash
cargo test -p axiom-alert
```

### T8.7 `axiom-viz` 迁移为插件
- [ ] 实现 `VizPlugin` 包装器
- [ ] 注册到 `PluginRegistry`
- [ ] 保留旧 `Viz` API 作为桥接

**验收标准**：
- `cargo check -p axiom-viz` 通过

**验证命令**：
```bash
cargo test -p axiom-viz
```

### T8.8 `axiom-oversight` 适配新内核
- [ ] 修改 `Supervisor` 使用新内核监控机制
- [ ] 保留旧 API 作为桥接

**验收标准**：
- `cargo check -p axiom-oversight` 通过

**验证命令**：
```bash
cargo test -p axiom-oversight
```

### T8.9 `axiom-planner` 适配新内核
- [ ] 修改 `Planner` 使用新内核计划执行机制
- [ ] 保留旧 API 作为桥接

**验收标准**：
- `cargo check -p axiom-planner` 通过

**验证命令**：
```bash
cargo test -p axiom-planner
```

### T8.10 `axiom-distributed` / `axiom-identity` / `axiom-prompt` 评估
- [ ] 评估是否迁移为插件或保留为独立 crate
- [ ] 如需迁移，实现插件包装器
- [ ] 如保留，确保通过新内核接口通信

**验收标准**：
- `cargo check --workspace` 通过

**验证命令**：
```bash
cargo check --workspace
```

**Phase 8 验证**：`cargo test --workspace` 全绿

---

## Phase 9：宏适配（待执行）

> **目标**：让 `axiom-macros` 生成的代码指向新内核

### T9.1 分析宏生成代码
- [ ] 读取 `axiom-macros/src/cell.rs`
- [ ] 读取 `axiom-macros/src/signal.rs`
- [ ] 读取 `axiom-macros/src/lens.rs`
- [ ] 读取 `axiom-macros/src/tool.rs`
- [ ] 读取 `axiom-macros/src/guard.rs`
- [ ] 输出《宏生成代码分析报告》

**验收标准**：
- 报告列出所有宏生成的代码模式
- 标注需要修改的生成逻辑

**验证命令**：
```bash
cargo test -p axiom-macros
```

### T9.2 修改 `#[cell]` 宏
- [ ] 修改生成代码调用 `axiom-kernel` API
- [ ] 保留宏表面语法不变

**验收标准**：
- `cargo check -p axiom-macros` 通过
- 宏生成的代码可编译

**验证命令**：
```bash
cargo test -p axiom-macros --test pass_macros
```

### T9.3 修改 `#[signal]` 宏
- [ ] 修改生成代码调用 `axiom-kernel` API
- [ ] 保留宏表面语法不变

**验收标准**：
- `cargo test -p axiom-macros` 通过

**验证命令**：
```bash
cargo test -p axiom-macros --test pass_macros
```

### T9.4 修改 `#[lens]` / `#[tool]` / `#[guard]` 宏
- [ ] 逐一修改每个宏的生成逻辑
- [ ] 保留宏表面语法不变

**验收标准**：
- `cargo test -p axiom-macros` 全绿

**验证命令**：
```bash
cargo test -p axiom-macros
```

### T9.5 宏编译失败测试更新
- [ ] 更新 `tests/compile-fail/` 下所有 `.stderr` 文件
- [ ] 确保编译失败信息与新架构一致

**验收标准**：
- `cargo test -p axiom-macros --test trybuild` 通过

**验证命令**：
```bash
cargo test -p axiom-macros --test trybuild
```

**Phase 9 验证**：`cargo test -p axiom-macros` 全绿

---

## Phase 10：全量验证与清理（待执行）

### T10.1 全量编译检查
- [ ] 运行 `cargo check --workspace`
- [ ] 修复所有编译错误
- [ ] 运行 `cargo clippy --workspace -- -D warnings`
- [ ] 修复所有 Clippy 警告
- [ ] 运行 `cargo fmt --all -- --check`
- [ ] 格式化所有代码

**验收标准**：
- 零编译错误
- 零 Clippy 警告
- 代码格式化合规

**验证命令**：
```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

### T10.2 全量测试
- [ ] 运行 `cargo test --workspace`
- [ ] 修复所有测试失败
- [ ] 运行 `cargo test --workspace --all-features`
- [ ] 确保 feature 开关下测试也通过

**验收标准**：
- 所有测试通过（无 skip、无 ignore）
- 覆盖率 >= 80%

**验证命令**：
```bash
cargo test --workspace
cargo test --workspace --all-features
tarpaulin --workspace --fail-under 80
```

### T10.3 旧代码清理
- [ ] 移除 `axiom-core/src/` 下已桥接的旧实现
- [ ] 保留 `bridge/` 作为唯一实现
- [ ] 移除 `axiom-runtime/src/` 下已迁移的旧实现
- [ ] 移除各应用层 crate 中已桥接的旧实现

**验收标准**：
- `cargo check --workspace` 通过
- 无未使用的代码警告

**验证命令**：
```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
```

### T10.4 文档更新
- [ ] 更新 `docs/MIGRATION.md`，标注迁移完成
- [ ] 更新 `docs/PROGRESS.md`，记录迁移里程碑
- [ ] 更新 `docs/API_BOUNDARY.md`，标注新架构边界
- [ ] 更新 `README.md`，反映新架构

**验收标准**：
- 所有文档内容与代码一致
- 无过时信息

**验证命令**：
```bash
# 人工检查文档
cat docs/MIGRATION.md
cat docs/PROGRESS.md
cat README.md
```

### T10.5 发布准备
- [ ] 更新 `Cargo.toml` 版本号为 `0.4.0`
- [ ] 更新 `CHANGELOG.md`
- [ ] 创建 `v0.4.0` git tag
- [ ] 发布到 crates.io（如需要）

**验收标准**：
- `cargo publish --dry-run` 通过
- 所有 crate 可发布

**验证命令**：
```bash
cargo publish --dry-run -p axiom-kernel
cargo publish --dry-run -p axiom-runtime
# ... 对所有 crate 执行
```

**Phase 10 验证**：全量测试通过，文档更新，可发布

---

## 快速参考：验证命令速查

```bash
# Phase 0-4 验证
cargo check --workspace
cargo test --workspace

# Phase 5 验证
cargo test -p axiom-core
cargo test -p axiom-core --test bridge_integration

# Phase 6 验证
cargo test -p axiom-runtime

# Phase 7 验证
cargo test -p axiom-cli

# Phase 8 验证
cargo test -p axiom-tool
cargo test -p axiom-llm
cargo test -p axiom-mcp
cargo test -p axiom-memory
cargo test -p axiom-store
cargo test -p axiom-alert
cargo test -p axiom-viz
cargo test -p axiom-oversight
cargo test -p axiom-planner

# Phase 9 验证
cargo test -p axiom-macros

# Phase 10 验证
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo test --workspace --all-features
```

---

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 旧 API 行为差异 | 高 | 桥接层 + 集成测试覆盖所有旧 API |
| 循环依赖 | 高 | 严格分层：`axiom-kernel` 不依赖上层 |
| 宏生成代码复杂 | 中 | 逐步迁移，保留旧实现作为 fallback |
| 测试覆盖率下降 | 中 | 每阶段要求覆盖率 >= 80% |
| 性能回退 | 中 | 基准测试对比新旧实现 |

---

## 里程碑

| 里程碑 | 完成标准 | 预计周期 |
|--------|----------|----------|
| M1: Phase 5 完成 | 旧 core 桥接完成，测试全绿 | 1 周 |
| M2: Phase 6 完成 | Runtime 切换新内核，测试全绿 | 1 周 |
| M3: Phase 7 完成 | CLI 全量迁移，测试全绿 | 3 天 |
| M4: Phase 8 完成 | 应用层迁移完成，测试全绿 | 1 周 |
| M5: Phase 9 完成 | 宏适配完成，测试全绿 | 2 天 |
| M6: Phase 10 完成 | 全量验证通过，可发布 | 2 天 |

**总预计周期**：约 4-5 周
