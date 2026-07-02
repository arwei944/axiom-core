# Phase 2: Witness持久化

> **预估工期**: 2周
> **前置条件**: Phase 1 完成（API稳定性）
> **后续阶段**: Phase 3 - CLI工具

---

## 阶段目标

将 Witness 审计记录持久化到事件存储，实现完整的事件重放和状态恢复能力。

---

## 任务清单

### Task 2.1: Witness → Event 序列化完善

**描述**: 完善 `witness_to_event()` 函数，确保所有 Witness 字段正确映射到 Event。

**涉及文件**:
- `crates/axiom-core/src/witness.rs`
- `crates/axiom-store/src/event.rs`

**步骤**:
1. 审查 `Witness` 结构体所有字段
2. 审查 `Event` 结构体所有字段
3. 实现完整的字段映射：
   - `witness_id` → `event_id`
   - `cell_id` → `source`
   - `correlation_id` → `correlation_id`
   - `timestamp_ns` → `timestamp`
   - `signal_type` → `event_type`
   - `outcome` → `status`
   - `before_state_hash` / `after_state_hash` → `metadata`
   - `parent_hash` → `parent_event_id`
4. 确保序列化/反序列化往返测试通过

**验收标准**:
- Witness所有字段可序列化/反序列化
- 往返测试通过

---

### Task 2.2: 运行时持久化接线

**描述**: 在 dispatch loop 中，将 `handle_dyn` 返回的 witnesses 写入 event store。

**涉及文件**:
- `crates/axiom-runtime/src/runtime.rs`
- `crates/axiom-runtime/src/supervisor.rs`

**步骤**:
1. 修改 `handle_dyn` 返回类型，包含 `Vec<OutgoingWitness>`
2. 在 dispatch loop 中收集所有 witnesses
3. 批量写入 event store
4. 确保写入失败时不丢失数据（写入DLQ或重试）

**验收标准**:
- 运行时产生的 Witness 可通过 event store 查询
- 写入失败时有适当的错误处理

---

### Task 2.3: 事件重放 API

**描述**: 实现按关联ID、时间范围、Cell等维度查询和重放事件。

**涉及文件**:
- `crates/axiom-store/src/store.rs`
- `crates/axiom-store/src/replay.rs`

**API设计**:
```rust
trait EventStore {
    fn replay(&self, correlation_id: &CorrelationId) -> Result<Vec<Event>, StoreError>;
    fn replay_from(&self, cell_id: &CellId, timestamp: u64) -> Result<Vec<Event>, StoreError>;
    fn replay_all(&self) -> Result<Vec<Event>, StoreError>;
    fn replay_range(&self, start: u64, end: u64) -> Result<Vec<Event>, StoreError>;
}
```

**验收标准**:
- 重放测试通过

---

### Task 2.4: 状态快照/恢复

**描述**: 实现 Cell 状态的快照和恢复功能。

**涉及文件**:
- `crates/axiom-store/src/snapshot.rs`
- `crates/axiom-runtime/src/runtime.rs`

**API设计**:
```rust
trait SnapshotManager {
    fn snapshot(&self, cell_id: &CellId) -> Result<SnapshotId, StoreError>;
    fn restore(&self, cell_id: &CellId, snapshot_id: &SnapshotId) -> Result<(), StoreError>;
    fn list_snapshots(&self, cell_id: &CellId) -> Result<Vec<SnapshotInfo>, StoreError>;
}
```

**步骤**:
1. 在 Cell trait 中添加 `snapshot()` 和 `restore()` 方法
2. 实现 SnapshotManager
3. 实现快照策略（定时/手动/事件触发）
4. 实现恢复流程

**验收标准**:
- 快照/恢复测试通过
- 崩溃恢复后状态一致

---

### Task 2.5: 集成测试

**描述**: 编写完整的集成测试，验证持久化和恢复流程。

**涉及文件**:
- `crates/axiom-store/tests/integration_tests.rs`

**测试场景**:
1. **正常流程**: 创建Cell → 发送消息 → 验证Witness持久化
2. **崩溃恢复**: 创建Cell → 发送消息 → 模拟崩溃 → 重启 → 验证状态恢复
3. **事件重放**: 创建Cell → 发送消息 → 重放事件 → 验证状态一致
4. **快照恢复**: 创建Cell → 发送消息 → 创建快照 → 修改状态 → 恢复快照 → 验证状态回滚

**验收标准**:
- 集成测试全部通过

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

- [ ] Witness → Event 序列化完善
- [ ] 运行时持久化接线完成
- [ ] 事件重放 API 实现
- [ ] 状态快照/恢复实现
- [ ] 集成测试全部通过
- [ ] 崩溃恢复后状态一致
- [ ] `cargo test --workspace` 全部通过

---

## 关键文件索引

| 文件 | 说明 |
|------|------|
| [crates/axiom-core/src/witness.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/witness.rs) | Witness定义 |
| [crates/axiom-store/src/event.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-store/src/event.rs) | Event定义 |
| [crates/axiom-store/src/replay.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-store/src/replay.rs) | 重放逻辑 |
| [crates/axiom-store/src/snapshot.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-store/src/snapshot.rs) | 快照逻辑 |
| [crates/axiom-runtime/src/runtime.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-runtime/src/runtime.rs) | 持久化接线 |
