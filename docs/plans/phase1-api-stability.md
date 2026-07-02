# Phase 1: API稳定性

> **预估工期**: 1周
> **前置条件**: Phase 0 完成（基础完备）
> **后续阶段**: Phase 2 - Witness持久化

---

## 阶段目标

定义 v1 API 边界，制定版本策略，完善错误处理，确保项目达到商用级别的 API 稳定性。

---

## 任务清单

### Task 1.1: 定义 v1 API 边界

**描述**: 标记所有公共API为稳定或不稳定，不稳定API标记为 `#[cfg(feature = "unstable")]`。

**涉及文件**:
- `crates/axiom-core/src/lib.rs` - 顶层导出
- `crates/axiom-core/src/cell.rs` - Cell trait
- `crates/axiom-core/src/signal.rs` - Signal trait
- `crates/axiom-core/src/axiom.rs` - Axiom trait
- `crates/axiom-core/src/witness.rs` - Witness相关
- `crates/axiom-core/src/context.rs` - Context相关
- `crates/axiom-runtime/src/runtime.rs` - Runtime API

**步骤**:
1. 审查所有 `pub` 项，标记为稳定或不稳定
2. 将不稳定API包裹在 `#[cfg(feature = "unstable")]` 中
3. 更新 `Cargo.toml` 添加 `unstable` feature
4. 编写文档说明稳定/不稳定边界

**验收标准**:
- 文档清晰标注稳定/不稳定API
- `cargo build --features unstable` 通过
- `cargo build`（无unstable feature）只编译稳定API

---

### Task 1.2: 版本策略文档

**描述**: 制定语义化版本规则、弃用流程、breaking change通知机制。

**涉及文件**:
- `docs/VERSIONING.md`（新建）

**内容**:
- **语义化版本规则**:
  - 主版本号：breaking change
  - 次版本号：新功能（兼容）
  - 修订号：bug修复
- **弃用流程**:
  - 提前2个版本发布警告
  - 在文档中明确标记 `#[deprecated]`
  - 提供迁移指南
- **Breaking Change通知**:
  - CHANGELOG中明确标注
  - 发布说明中包含迁移步骤

**验收标准**:
- 版本策略文档完整

---

### Task 1.3: 错误类型完善

**描述**: 确保 `AxiomError` 覆盖所有错误场景，提供清晰的错误信息。

**涉及文件**:
- `crates/axiom-core/src/error.rs`

**步骤**:
1. 审查现有 `AxiomError` 变体
2. 补充缺失的错误变体：
   - `CellNotFound` - Cell不存在
   - `MailboxFull` - 信箱已满
   - `CorrelationError` - 关联ID错误
   - `PermissionDenied` - 权限拒绝
   - `ResourceExhausted` - 资源耗尽
3. 为每个错误提供详细的 `#[error]` 消息
4. 确保错误信息包含足够的调试信息（cell_id, signal_type, layer等）

**验收标准**:
- 所有错误场景有对应变体
- 错误信息清晰包含调试信息

---

### Task 1.4: 公共API文档完备

**描述**: 确保每个公开函数/类型有文档注释。

**涉及文件**:
- `crates/axiom-core/src/lib.rs`
- `crates/axiom-core/src/cell.rs`
- `crates/axiom-core/src/signal.rs`
- `crates/axiom-core/src/axiom.rs`
- `crates/axiom-core/src/witness.rs`
- `crates/axiom-core/src/context.rs`
- `crates/axiom-runtime/src/runtime.rs`

**步骤**:
1. 使用 `cargo doc --open` 检查文档覆盖率
2. 为缺失文档的公开项添加 `///` 文档注释
3. 确保文档包含：
   - 功能描述
   - 参数说明
   - 返回值说明
   - 错误场景
   - 使用示例（如适用）

**验收标准**:
- `cargo doc --no-deps` 无缺失文档警告

---

### Task 1.5: 错误处理一致性

**描述**: 确保错误处理风格一致，使用统一的错误传播模式。

**涉及文件**:
- 全局搜索错误处理代码

**步骤**:
1. 统一错误类型命名（使用 `Error` 后缀）
2. 统一错误传播方式（使用 `?` 而非手动匹配）
3. 统一错误构造方式（使用 `AxiomError::Variant { ... }` 而非 `format!`）
4. 确保错误链完整（使用 `thiserror` 的 `#[source]` 属性）

**验收标准**:
- 错误处理风格一致
- 错误链完整可追溯

---

## 质量门禁

```bash
# 每次任务完成后必须通过
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -D warnings
cargo build --workspace --all-targets
cargo test --workspace
cargo doc --no-deps
```

---

## 阶段验收标准

- [ ] v1 API 边界定义完成
- [ ] 不稳定API标记为 `#[cfg(feature = "unstable")]`
- [ ] 版本策略文档完成
- [ ] `AxiomError` 覆盖所有错误场景
- [ ] 公共API文档完备
- [ ] 错误处理一致性
- [ ] `cargo doc --no-deps` 无警告
- [ ] `cargo test --workspace` 全部通过

---

## 关键文件索引

| 文件 | 说明 |
|------|------|
| [crates/axiom-core/src/error.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/error.rs) | AxiomError 定义 |
| [crates/axiom-core/src/lib.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/lib.rs) | 顶层导出 |
| [crates/axiom-runtime/src/runtime.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-runtime/src/runtime.rs) | Runtime API |
| [docs/VERSIONING.md](file:///D:/work/trae/axiom-core-project/docs/VERSIONING.md) | 版本策略文档 |
