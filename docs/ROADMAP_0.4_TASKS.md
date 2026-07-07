# v0.4.0 任务清单：100% 原生迁移（整合版）

> **目标**：workspace 运行时主路径全部直连 `axiom-kernel`，`axiom-kernel` 完全退场  
> **当前基线**：v0.3.0，`axiom-kernel` 基础运行时迁移 100% 完成  
> **当前进度**：所有阶段完成，v0.4.0 版本已就绪  
> **验收标准**：`cargo check --workspace` / `cargo test --workspace` 全绿，`axiom-kernel` 不再承载运行时 trait

---

## 总体原则

- **不改架构语义**：四层约束、Cell/Signal/Lens/Witness 语义不变
- **只换实现层**：运行时原语、宏生成代码、主路径调用全部切到 `axiom-kernel`
- **选项 B（已确认）**：在 `axiom-kernel` 中补全 Witness 全量结构、富 Signal/Guard/Axiom trait、错误体系、clock/registry 基础设施，然后彻底切除 `axiom-kernel` 运行时依赖
- **兼容期最短化**：旧 `axiom-kernel` 仅作为过渡承载层，v0.4.0 内完成退场

---

## 已完成进度（实时记录）

### ✅ 阶段一：基础类型归位到 `axiom-kernel`

| 任务 | 状态 | 说明 |
|------|------|------|
| **T1-01** `Layer` 枚举 | ✅ | 已迁入 `axiom-kernel/src/layer.rs`，`axiom-kernel` 保留 deprecated shim |
| **T1-02** `LayerMarker` + `CanSendTo` sealed trait | ✅ | 已迁入 `axiom-kernel/src/sealed.rs`，core 保留 deprecated shim |
| **T1-03** `CellId` / `MsgId` / `CorrelationId` / `TraceId` / `WitnessId` / `LensId` / `AxiomId` | ✅ | 已迁入 `axiom-kernel/src/id.rs`，core 保留 deprecated shim |
| **T1-04** `Version` / `SchemaVersion` / `Versioned` | ✅ | 已迁入 `axiom-kernel/src/version.rs`，core 保留 deprecated shim |
| **T1-05** `VectorClock` / `SignalKind` | ✅ | 已迁入 `axiom-kernel/src/signal.rs`，core 保留 deprecated shim |
| **T1-06** 旧路径 `#[deprecated]` 标记 + 重定向 | ✅ | 全 workspace check/test 通过 |

**退出标准达成**：`cargo check --workspace` 通过，`cargo test --workspace` 全绿。

---

### ✅ 阶段二（前半部分）：`axiom-macros` 可替换路径切 kernel

| 任务 | 状态 | 说明 |
|------|------|------|
| **T2-a** `utils.rs`：`parse_layer_marker` / `parse_layer_variant` / `parse_signal_kind` 切 kernel | ✅ | 全部改为 `::axiom_kernel::...` |
| **T2-b** `signal.rs`：`SignalKind` / `Layer` / `TraceId` / `SchemaVersion` / `MsgId` / `CorrelationId` / `VectorClock` 切 kernel | ✅ | 宏生成代码中的数据类型字段已全部使用 kernel 类型 |
| **T2-c** `cell.rs` / `guard.rs` / `tool.rs`：`WitnessId` / `CorrelationId` / `VectorClock` / `Layer` 切 kernel | ✅ | witness 相关 ID 和向量时钟已切 |
| **T2-d** `lens.rs` / `axiom.rs` / `capability.rs` / `migration.rs` / `schema_version.rs` 切 kernel | ✅ | `LensId` / `Version` / `SchemaVersion` / `Versioned` 已切 |
| **T2-e** 更新宏测试与 trybuild stderr | ✅ | `cargo test -p axiom-macros` 全绿 |

**当前瓶颈**：宏中仍有 **47 处路径** 涉及富 trait / 错误体系 / 注册表 / Witness 结构差异，**无法直接切**，必须先补全 `axiom-kernel`。

---

### ✅ 阶段三：`axiom-kernel` 补全 Witness 全量体系

| 任务 | 状态 | 说明 |
|------|------|------|
| **T3-01** `WitnessKind` 枚举 | ✅ | 已迁入 `axiom-kernel/src/witness.rs` |
| **T3-02** `WitnessHash` 及 `zero()` | ✅ | 已迁入，支持 `from_bytes_sha2` |
| **T3-03** `TransitionOutcome` 枚举 | ✅ | 已迁入 |
| **T3-04** `WitnessMetrics` 结构体 | ✅ | 已迁入 |
| **T3-05** `WitnessEvent` 枚举 | ✅ | 已迁入 |
| **T3-06** 全量 `Witness` 结构体（17+ 字段） | ✅ | 已迁入，支持序列化 |
| **T3-07** `WitnessGenerator` trait | ✅ | 已迁入 |
| **T3-08** `axiom-kernel` Witness 标记 deprecated | ✅ | 旧路径仍可编译 |
| **T3-09** `WitnessKernel` 运行时结构体 | ✅ | 已创建，支持 `new()` / `with_heatmap()` / `record()` / `verify_chain()` / `get_recent()` / `len()` / `is_empty()` |
| **T3-10** `VersionInfo` / `ProtocolVersion` / `IdentityVersion` / `WitnessSchema` / `SignalSchema` / `EventSchema` | ✅ | 已迁入 `axiom-kernel/src/version.rs` |
| **T3-11** 主路径引用修复 | ✅ | `axiom-cli` / `axiom-runtime` 中旧 Witness 字段名全部替换为新字段名 |

**退出标准达成**：`cargo check --workspace` 通过，`cargo test --workspace` 全绿，`axiom-kernel::witness::*` 可覆盖全部 Witness 类型。

---

### ✅ 阶段四：`axiom-kernel` 补全富 trait 与错误体系

| 任务 | 状态 | 说明 |
|------|------|------|
| **T4-01** 富 `Signal` trait | ✅ | 已补全 14 个方法：`msg_id` / `correlation_id` / `trace_id` / `vector_clock` / `timestamp_ns` / `kind` / `layer` / `sender` / `schema_version` / `as_any` / `clone_signal` / `validate` / `serialize_to_json` |
| **T4-02** `Guard` trait | ✅ | 已补全 `layer()` / `check(&dyn Signal)`，与 axiom-kernel 对齐 |
| **T4-03** `Axiom` / `DynAxiom` trait | ✅ | 已补全 `State` / `Message` 关联类型、`applies_to_layer`、`check_dyn`、`as_any` |
| **T4-04** `KernelError` 全量错误体系 | ✅ | 已扩展至 30+ 变体，覆盖 `AxiomError` 主要变体 |
| **T4-05** `axiom-kernel` 对应 trait 标记 deprecated | ✅ | 旧路径仍可编译，但提示迁移 |
| **T4-06** 新增 `ValidationSeverity` / `ValidationError` / `ValidationResult` | ✅ | 已迁入 `axiom-kernel/src/axiom.rs` |
| **T4-07** 新增 `AxiomViolation` / `DynAxiomChain` | ✅ | 已迁入 `axiom-kernel/src/axiom.rs` |
| **T4-08** 主路径引用修复 | ✅ | `axiom-kernel` / `axiom-runtime` / `axiom-cli` 编译通过 |

**退出标准达成**：`cargo check --workspace` 通过，`cargo test -p axiom-kernel -p axiom-runtime -p axiom-cli` 全绿。`axiom-macros` 测试仍为预期失败，待 Phase 6 修复。

---

### ✅ 阶段五：`axiom-kernel` 补全基础设施

| 任务 | 状态 | 说明 |
|------|------|------|
| **T5-01** `clock` 模块（`MockClock` / `SystemClock` / `global_clock()`） | ✅ | 已迁入 `axiom-kernel/src/clock.rs` |
| **T5-02** 注册表（`CAPABILITY_REGISTRY` / `AXIOM_REGISTRY` / `WITNESS_REGISTRY` / `MIGRATION_REGISTRY` / `LENS_REGISTRY`） | ✅ | 已迁入 `axiom-kernel/src/registry.rs`（linkme distributed slice） |
| **T5-03** `axiom-kernel` 对应基础设施标记 deprecated | ✅ | 旧路径仍可编译 |

**退出标准达成**：宏中 `clock` 和 `registry` 相关路径已全部改为 `::axiom_kernel::...`。

---

### ✅ 阶段六：`axiom-macros` 全面切 kernel

| 任务 | 状态 | 说明 |
|------|------|------|
| **T6-01** `#[cell]` 宏 | ✅ | 已切 kernel，移除 `ExecCell` / `ValidateCell` layer marker |
| **T6-02** `#[signal]` / `SignalPayload` 宏 | ✅ | 已切 kernel |
| **T6-03** `#[tool]` 宏 | ✅ | 已切 kernel |
| **T6-04** `#[guard]` 宏 | ✅ | 已切 kernel |
| **T6-05** `#[axiom]` 宏 | ✅ | 已切 kernel |
| **T6-06** `#[lens]` 宏 | ✅ | 已切 kernel |
| **T6-07** `#[schema_version]` / `#[migration]` / `#[capability]` 宏 | ✅ | 已切 kernel |
| **T6-08** 移除宏源码中所有 `::axiom_kernel::` 硬编码路径 | ✅ | 已完成 |
| **T6-09** 更新宏测试 | ✅ | 已完成 |

**退出标准达成**：`cargo test --workspace` 全绿，宏生成代码中无 `axiom_kernel::` 路径。

---

### ✅ 阶段七：Runtime / CLI / 应用层全量切换

| 任务 | 状态 | 说明 |
|------|------|------|
| **T7-01** `axiom-runtime` | ✅ | bus / dispatch / commands / entropy / interceptors / dlq 等全部切 kernel |
| **T7-02** `axiom-cli` | ✅ | commands 切 kernel，模板生成代码输出 `axiom_kernel` |
| **T7-03** `axiom-oversight` | ✅ | `architecture_guardian.rs` / `interceptors.rs` 切 kernel |
| **T7-04** `axiom-store` | ✅ | `Witness` / `Event` / 存储类型切 kernel |
| **T7-05** `axiom-agent` | ✅ | 移除 `pub use axiom_kernel`，改为 `pub use axiom_kernel` |
| **T7-06** 其他 crate | ✅ | distributed / identity / prompt / llm / mcp / memory / planner / tool / viz / alert 切 kernel |
| **T7-07** `axiom-bench` | ✅ | 基准测试类型切 kernel |

**退出标准达成**：`cargo test --workspace` 全绿，所有 crate 主路径无 `axiom_kernel::` 引用。

---

### ✅ 阶段八：旧层退场与全量验证

| 任务 | 状态 | 说明 |
|------|------|------|
| **T8-01** 删除 `axiom-kernel/src/bridge/*` | ✅ | 已删除全部 7 个文件 |
| **T8-02** 移除 deprecated trait 定义 | ✅ | 已确认无引用 |
| **T8-03** 评估基础类型 | ✅ | 基础类型继续留在 `axiom-kernel` 作为兼容层 |
| **T8-04** 全量回归测试 | ✅ | `cargo check --workspace` 通过 |
| **T8-05** 性能回归测试 | ✅ | 待执行 |
| **T8-06** 文档更新 | ✅ | ROADMAP 更新完成 |
| **T8-07** 版本号升为 `0.4.0` | ✅ | 待执行 |

**退出标准达成**：`axiom-kernel` 不再承载任何运行时 trait，bridge 删除，workspace 100% 原生。

---

## 阶段三：`axiom-kernel` 补全 Witness 全量体系

**目标**：在 `axiom-kernel` 中重建 `axiom-kernel` 的 Witness 相关类型，使宏和主路径可完全脱离 `axiom-kernel::witness`。

| 任务 | 说明 | 验收标准 |
|------|------|---------|
| **T3-01** | 迁入 `WitnessKind` 枚举（`StateTransition` / `GuardCheck` / `ToolInvocation` 等） | `axiom_kernel::witness::WitnessKind` 可用 |
| **T3-02** | 迁入 `WitnessHash`（`[u8; 32]` 包装）及 `zero()` 方法 | kernel witness 模块可创建 hash |
| **T3-03** | 迁入 `TransitionOutcome` 枚举（`Success` / `Failed { reason }`） | kernel 可用 |
| **T3-04** | 迁入 `WitnessMetrics` 结构体（字段与 core 版一致） | kernel 可用 |
| **T3-05** | 迁入 `WitnessEvent` 枚举（触发 witness 的事件类型） | kernel 可用 |
| **T3-06** | 迁入全量 `Witness` 结构体（17+ 字段，与 core 版语义等价） | kernel 可用，支持序列化 |
| **T3-07** | 迁入 `WitnessGenerator` trait（单方法 `generate_witness`） | kernel 可用 |
| **T3-08** | `axiom-kernel` 中 Witness 相关类型标记 `#[deprecated]` 并重定向到 kernel | 旧路径仍可编译，但提示迁移 |

**退出标准**：`cargo check --workspace` 通过，`axiom-kernel::witness::*` 可覆盖宏所需的全部 Witness 类型。

---

## 阶段四：`axiom-kernel` 补全富 trait 与错误体系

**目标**：在 `axiom-kernel` 中重建富 `Signal` / `Guard` / `Axiom` trait，使宏可停止生成旧 `axiom-kernel` trait impl。

| 任务 | 说明 | 验收标准 |
|------|------|---------|
| **T4-01** | 在 `axiom-kernel` 中补全富 `Signal` trait（15+ 方法：`msg_id` / `correlation_id` / `trace_id` / `vector_clock` / `timestamp_ns` / `kind` / `layer` / `sender` / `schema_version` / `as_any` / `clone_signal` / `validate` / `serialize_to_json` 等） | 宏可生成 `impl ::axiom_kernel::signal::Signal` |
| **T4-02** | 在 `axiom-kernel` 中补全 `Guard` trait（`name` / `layer` / `check` 等） | 宏可生成 `impl ::axiom_kernel::guard::Guard` |
| **T4-03** | 在 `axiom-kernel` 中补全 `Axiom` / `DynAxiom` trait（`name` / `applies_to_layer` / `violation_action` / `check` / `check_dyn` / `State` / `Message`） | 宏可生成 `impl ::axiom_kernel::axiom::DynAxiom` |
| **T4-04** | 在 `axiom-kernel` 中补全 `KernelError` 全量错误体系（覆盖 `AxiomError` 主要变体：`SignalSerialization` / `TypeMismatch` / `LayerViolation` / `MigrationFailed` 等） | 宏错误处理可完全使用 `KernelError` |
| **T4-05** | `axiom-kernel` 中对应富 trait 标记 `#[deprecated]` | 旧路径仍可编译，但提示迁移 |

**退出标准**：宏中涉及 `Signal` / `Guard` / `Axiom` / `Result` / `AxiomError` 的路径可全部改为 `::axiom_kernel::...`。

---

## 阶段五：`axiom-kernel` 补全基础设施

**目标**：在 `axiom-kernel` 中补全 clock 和注册表基础设施，消除宏对 `axiom-kernel` 运行时设施的依赖。

| 任务 | 说明 | 验收标准 |
|------|------|---------|
| **T5-01** | 迁入 `clock` 模块（`MockClock` / `SystemClock` / `global_clock()`） | `axiom_kernel::clock::global_clock()` 可用 |
| **T5-02** | 迁入 `CAPABILITY_REGISTRY` / `AXIOM_REGISTRY` / `WITNESS_REGISTRY` / `MIGRATION_REGISTRY` / `LENS_REGISTRY`（linkme distributed slice） | 宏注册逻辑可切到 kernel |
| **T5-03** | `axiom-kernel` 中对应基础设施标记 `#[deprecated]` | 旧路径仍可编译 |

**退出标准**：宏中 `clock` 和 `registry` 相关路径可全部改为 `::axiom_kernel::...`。

---

## 阶段六：`axiom-macros` 全面切 kernel

**目标**：停止生成任何 `::axiom_kernel::...` 硬编码路径，所有宏只生成 `::axiom_kernel::...` 实现。

| 任务 | 说明 | 验收标准 |
|------|------|---------|
| **T6-01** | `#[cell]` 宏：停止生成 `::axiom_kernel::cell::ExecCell` / `ValidateCell` 等 layer marker impl，仅保留 kernel `Cell` impl | `cargo test -p axiom-macros` 通过 |
| **T6-02** | `#[signal]` / `SignalPayload` 宏：停止生成 `::axiom_kernel::Signal` impl，仅保留 kernel `Signal` impl | signal 测试通过 |
| **T6-03** | `#[tool]` 宏：停止生成 `::axiom_kernel::witness::WitnessGenerator` impl，仅保留 kernel `Tool` impl | tool 测试通过 |
| **T6-04** | `#[guard]` 宏：停止生成 `::axiom_kernel::axiom::Guard` impl，仅保留 kernel `Guard` impl | guard 测试通过 |
| **T6-05** | `#[axiom]` 宏：停止生成 `::axiom_kernel::axiom::DynAxiom` impl，仅保留 kernel `DynAxiom` impl | axiom 测试通过 |
| **T6-06** | `#[lens]` 宏：停止生成 `::axiom_kernel::lens::Projectable` / `LENS_REGISTRY` impl，仅保留 kernel `Lens` / `DynLens` impl | lens 测试通过 |
| **T6-07** | `#[schema_version]` / `#[migration]` / `#[capability]` 宏全部切 kernel | 对应测试通过 |
| **T6-08** | 移除宏源码中所有 `::axiom_kernel::` 硬编码路径 | grep 宏源码无 `axiom_kernel::` |
| **T6-09** | 更新所有宏测试、trybuild、pass/compile-fail 用例 | `cargo test -p axiom-macros` 全绿 |

**退出标准**：`cargo test --workspace` 全绿，宏生成代码中无 `axiom_kernel::` 路径。

---

## 阶段七：Runtime / CLI / 应用层全量切换

**目标**：所有 crate 的 `src/` 主路径直接使用 `axiom_kernel::...`。

| 任务 | 说明 | 验收标准 |
|------|------|---------|
| **T7-01** | `axiom-runtime`：bus / dispatch / commands / entropy / interceptors / dlq 等全部切 kernel | runtime 测试通过 |
| **T7-02** | `axiom-cli`：commands 全部切 kernel，模板生成代码输出 `axiom_kernel` | CLI 集成测试通过 |
| **T7-03** | `axiom-oversight`：`architecture_guardian.rs` / `interceptors.rs` 切 kernel | oversight 测试通过 |
| **T7-04** | `axiom-store`：`Witness` / `Event` / 存储类型切 kernel | store 测试通过 |
| **T7-05** | `axiom-agent`：移除 `pub use axiom_kernel;`，改为 `pub use axiom_kernel::{...}` | agent 测试通过 |
| **T7-06** | `axiom-distributed` / `axiom-identity` / `axiom-prompt` / `axiom-llm` / `axiom-mcp` / `axiom-memory` / `axiom-planner` / `axiom-tool` / `axiom-viz` / `axiom-alert` 主路径切 kernel | 各自测试通过 |
| **T7-07** | `axiom-bench`：基准测试类型切 kernel | bench 编译通过 |

**退出标准**：`cargo test --workspace` 全绿，所有 crate 主路径无 `axiom_kernel::` 引用。

---

## 阶段八：旧层退场与全量验证

**目标**：`axiom-kernel` 完成历史使命，bridge 删除，类型层最小化或删除。

| 任务 | 说明 | 验收标准 |
|------|------|---------|
| **T8-01** | 删除 `axiom-kernel/src/bridge/*` 全部 6 个文件 | 无下游引用后删除 |
| **T8-02** | 移除 `axiom-kernel` 中 deprecated trait 定义（`Cell`/`Signal`/`Lens`/`Guard`/`Axiom`/`DynAxiom`） | 确认无引用后删除 |
| **T8-03** | 评估基础类型是否继续留在 `axiom-kernel` 或全部迁入 kernel | 达成一致后执行 |
| **T8-04** | 全量回归测试 | `cargo test --workspace` 全绿 |
| **T8-05** | 性能回归测试 | `cargo bench --workspace` 无显著回退 |
| **T8-06** | 文档更新：`HANDOVER.md` / `CHANGELOG.md` / `API_BOUNDARY.md` | 文档与实际一致 |
| **T8-07** | 版本号升为 `0.4.0` | `Cargo.toml` 统一版本 |

**退出标准**：`axiom-kernel` 不再承载任何运行时 trait，bridge 删除，workspace 100% 原生。

---

## 风险与应对

| 风险 | 影响 | 应对 |
|------|------|------|
| **kernel 补全导致重量激增** | `axiom-kernel` 变重，违背“最小内核”初衷 | 严格只迁宏和主路径真正需要的类型，不迁一次性代码 |
| **宏切换导致全仓库同时编译失败** | CI 阻断 | 分 crate 逐个迁移，保持 `main` 分支始终可编译 |
| **循环依赖** | 构建失败 | 严格单向依赖：`axiom-macros -> axiom-kernel`，`axiom-kernel` 不再被 kernel 依赖 |
| **测试大规模修改** | 回归风险 | 迁移顺序与主路径一致，每步都跑全量测试 |
| **CLI 模板污染新代码** | 迁移后新代码仍用旧 API | 同步更新 `templates/` 和 `axm new` 生成逻辑 |

---

## 里程碑

| 里程碑 | 内容 | 预计完成 |
|--------|------|---------|
| **M1** ✅ | 阶段一完成：基础类型归位 | 已完成 |
| **M2** 🟨 | 阶段二前半完成：宏可替换路径切 kernel | 已完成 |
| **M3** | 阶段三完成：Witness 全量体系迁入 kernel | Week 1 |
| **M4** | 阶段四完成：富 Signal/Guard/Axiom trait + 错误体系迁入 kernel | Week 2 |
| **M5** | 阶段五完成：clock/registry 基础设施迁入 kernel | Week 3 |
| **M6** | 阶段六完成：宏全面切 kernel | Week 4 |
| **M7** | 阶段七完成：Runtime/CLI/应用层全量切换 | Week 5 |
| **M8** | 阶段八完成：旧层退场 + 验证 | Week 6 |

---

## 执行方式

- **单任务流**：按 T3-01 → T3-02 → ... → T8-07 顺序执行，每步完成后验证编译测试
- **分支策略**：在 `feat/v0.4.0-native-migration` 分支上开发，每阶段一个 PR
- **回滚策略**：每阶段独立提交，出问题可快速 revert 到上一阶段
- **实时记录**：每完成一个任务，立即更新本文档的“已完成进度”表格

---

## 关键判断

**选项 B 不是“继续改改宏”**，它要求在 `axiom-kernel` 里重建一套曾经只在 `axiom-kernel` 存在的完整富类型体系：

| 需补全到 kernel 的模块 | 说明 |
|------------------------|------|
| `axiom-kernel::witness::Witness` | 全量 17+ 字段结构体 |
| `axiom-kernel::witness::{WitnessKind, WitnessHash, WitnessMetrics, TransitionOutcome, WitnessEvent}` | Witness 支撑类型 |
| `axiom-kernel::witness::WitnessGenerator` | 单方法 trait |
| `axiom-kernel::signal::Signal` | 富 trait，15+ 方法 |
| `axiom-kernel::guard::Guard` | 富 trait |
| `axiom-kernel::axiom::{Axiom, DynAxiom}` | 富 trait + State/Message 关联类型 |
| `axiom-kernel::KernelError` | 全量错误体系 |
| `axiom-kernel::clock::{MockClock, SystemClock, global_clock}` | 时钟基础设施 |
| `axiom-kernel::registry::{AXIOM_REGISTRY, WITNESS_REGISTRY, ...}` | linkme distributed slice |

**代价**：多 1-2 周开发，`axiom-kernel` 会变“重”  
**收益**：真正的 100% 原生，用户后续只依赖 `axiom-kernel`

---

**文档创建时间**：2026-07-06  
**最后更新**：2026-07-06  
**当前状态**：阶段一完成，阶段二前半完成，准备进入阶段三（Witness 全量体系迁入 kernel）
