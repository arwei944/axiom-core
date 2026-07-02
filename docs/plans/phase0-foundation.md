# Phase 0: 基础完备

> **预估工期**: 2周
> **前置条件**: P2 Phase 0-4 完成（基础设施/Bug修复/架构强制/死代码清理）
> **后续阶段**: Phase 1 - API稳定性

---

## 阶段目标

完成架构债务修复的剩余工作，确保代码库达到商用级别的基础质量标准：
- 消除所有代码重复
- 补齐测试覆盖
- 消除 Clippy 警告（已完成）
- 消除非测试代码中的 unwrap/expect

---

## 任务清单

### Task 0.1: 统一 EntropyLevel 定义

**描述**: `EntropyLevel` 在 `axiom-core` 和 `axiom-oversight` 各有一份定义，需要统一到 `axiom-core`，`axiom-oversight` 通过 re-export 使用。

**涉及文件**:
- `crates/axiom-core/src/entropy.rs` - 主定义位置
- `crates/axiom-oversight/src/entropy_governor.rs` - 删除重复定义

**步骤**:
1. 在 `axiom-core/src/entropy.rs` 中确认 `EntropyLevel` 定义完整
2. 在 `axiom-oversight/src/lib.rs` 中添加 re-export: `pub use axiom_core::entropy::EntropyLevel;`
3. 删除 `axiom-oversight/src/entropy_governor.rs` 中的 `EntropyLevel` 定义
4. 更新所有引用位置

**验收标准**:
- `axiom-oversight` 不再定义 `EntropyLevel`
- `cargo build --workspace` 通过
- 所有测试通过

---

### Task 0.2: 统一 now_ns() 函数

**描述**: 目前有5处重复定义的 `now_ns()` 函数，需要统一到 `axiom-core::signal` 模块。

**涉及文件**:
- `crates/axiom-core/src/signal.rs` - 主定义位置
- `crates/axiom-runtime/src/runtime.rs` - 删除重复
- `crates/axiom-oversight/src/entropy_governor.rs` - 删除重复
- `crates/axiom-oversight/src/intent_auditor.rs` - 删除重复
- `crates/axiom-oversight/src/health.rs` - 删除重复

**步骤**:
1. 在 `axiom-core/src/signal.rs` 中确认 `now_ns()` 定义完整且为 `pub`
2. 在 `axiom-core/src/lib.rs` 中添加 re-export: `pub use signal::now_ns;`
3. 删除其他 crate 中的 `now_ns()` 定义
4. 更新所有引用为 `axiom_core::now_ns()`

**验收标准**:
- 全局只有一处 `now_ns()` 定义
- `cargo build --workspace` 通过
- 所有测试通过

---

### Task 0.3: 消除 type_complexity 警告

**描述**: `handle_dyn` 返回类型 `Pin<Box<dyn Future<Output = ...>>>` 太复杂，需要引入 type alias。

**涉及文件**:
- `crates/axiom-core/src/cell.rs`

**步骤**:
1. 在 `cell.rs` 顶部添加 type alias:
   ```rust
   type BoxHandleFuture<'a> = Pin<Box<dyn Future<Output = (crate::Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a>>;
   ```
2. 使用 `BoxHandleFuture` 替换 `handle_dyn` 的返回类型

**验收标准**:
- `cargo clippy --workspace` 零警告

---

### Task 0.4: 消除 manual_async_fn 警告

**描述**: Cell impl 的 `handle` 方法使用 RPITIT 而非 `async fn`，需要添加 `#[allow(clippy::manual_async_fn)]`。

**涉及文件**:
- `crates/axiom-core/examples/hello_cell.rs`
- `crates/axiom-core/tests/integration_tests.rs`
- `crates/axiom-macros/tests/integration.rs`
- `crates/axiom-core/tests/cell_tests.rs`

**步骤**:
1. 在 `#[cell]` 宏生成的 impl 块上添加 `#[allow(clippy::manual_async_fn)]`
2. 或在 `Cell` trait 定义上添加 `#[allow(clippy::manual_async_fn)]`

**验收标准**:
- `cargo clippy --workspace` 零警告

---

### Task 0.5: 错误路径测试补齐（5场景）

**描述**: 覆盖5个关键错误场景的集成测试。

**涉及文件**:
- `crates/axiom-core/tests/error_path_tests.rs`（新建）

**测试场景**:

#### Test 0.5.1: LayerViolation（Exec→Agent）
- 创建 Exec 层 Cell，尝试发送信号到 Agent 层
- 验证返回 `AxiomError::LayerViolation`
- 验证 Witness 记录违规事件

#### Test 0.5.2: Witness哈希链断裂
- 构造断裂的 Witness 链（parent_hash 指向不存在的 witness）
- 验证 `WitnessBatch::verify_chain()` 返回错误
- 验证熵值增加

#### Test 0.5.3: Signal序列化失败
- 创建包含无法序列化字段的 Signal
- 验证发送时返回 `AxiomError::SignalSerialization`
- 验证消息进入 DLQ

#### Test 0.5.4: Cell崩溃恢复
- 创建会 panic 的 Cell
- 发送信号触发 panic
- 验证 Supervisor 检测到崩溃并重启
- 验证状态恢复正常

#### Test 0.5.5: 信箱溢出
- 创建容量有限的信箱
- 并发发送超过容量的消息
- 验证超出部分被拒绝
- 验证熵值增加

**验收标准**:
- 5个场景测试全部通过
- 测试覆盖错误路径的完整处理流程

---

### Task 0.6: 并发测试补齐（3场景）

**描述**: 覆盖3个关键并发场景的集成测试。

**涉及文件**:
- `crates/axiom-core/tests/concurrency_tests.rs`（新建）

**测试场景**:

#### Test 0.6.1: 多Cell并发处理
- 创建多个 Cell，并发发送消息
- 验证所有消息正确投递
- 验证没有数据竞争

#### Test 0.6.2: 同一Cell多消息串行处理
- 向同一 Cell 并发发送多个消息
- 验证消息按顺序处理
- 验证状态一致性

#### Test 0.6.3: 信箱背压测试
- 创建低容量信箱
- 持续发送消息
- 验证背压机制生效
- 验证系统不会崩溃

**验收标准**:
- 3个场景测试全部通过
- 无数据竞争警告

---

### Task 0.7: 零 unwrap/expect（非测试代码）

**描述**: 将非测试代码中的所有 `unwrap()` 和 `expect()` 替换为 proper error handling。

**涉及文件**:
- 全局搜索 `unwrap()` / `expect()`

**步骤**:
1. 使用 `grep -r "unwrap\|expect" crates/ --include="*.rs" | grep -v test | grep -v "tests/"` 查找所有非测试代码中的 unwrap/expect
2. 逐一替换为 proper error handling：
   - 函数返回类型改为 `Result<T, E>`
   - 使用 `?` 传播错误
   - 或使用 `unwrap_or_else(|e| panic!(...))` 仅在不可恢复错误时使用

**验收标准**:
- `grep -r "unwrap\|expect" crates/ --include="*.rs" | grep -v test | grep -v "tests/"` 无结果
- `cargo build --workspace` 通过
- 所有测试通过

---

## 质量门禁

```bash
# 每次任务完成后必须通过
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -D warnings
cargo build --workspace --all-targets
cargo test --workspace
```

---

## 阶段验收标准

- [ ] `EntropyLevel` 统一到 `axiom-core`
- [ ] `now_ns()` 全局只有一处定义
- [ ] Clippy 零警告
- [ ] 错误路径测试（5场景）通过
- [ ] 并发测试（3场景）通过
- [ ] 非测试代码零 unwrap/expect
- [ ] 测试总数 ≥ 200
- [ ] `cargo test --workspace` 全部通过

---

## 关键文件索引

| 文件 | 说明 |
|------|------|
| [crates/axiom-core/src/entropy.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/entropy.rs) | EntropyLevel 定义 |
| [crates/axiom-core/src/signal.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/signal.rs) | now_ns() 定义 |
| [crates/axiom-core/src/cell.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/cell.rs) | handle_dyn 返回类型 |
| [crates/axiom-oversight/src/entropy_governor.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-oversight/src/entropy_governor.rs) | 重复定义位置 |
