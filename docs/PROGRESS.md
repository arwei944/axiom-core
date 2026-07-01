# Axiom Core 项目进度总览

> **最后更新:** 2026-07-01
> **当前阶段:** P2 架构债务修复 (进行中)
> **代码基线:** master @ f955051
> **测试总数:** 175 (全部通过)
> **质量门禁:** ✅ build / ✅ test / ✅ fmt / ⚠️ clippy (type_complexity + manual_async_fn 警告)

---

## 项目里程碑

| 阶段 | 状态 | 描述 | 测试数 |
|------|------|------|--------|
| P0 基础设施 | ✅ 完成 | 三层门禁系统 (L0 开发/L1 编译期/L2 运行时) | - |
| P1 核心原语 | ✅ 完成 | Cell / Signal / Axiom / Witness / Schema / Entropy 核心抽象 | 179 → 175 |
| P2 架构债务修复 | 🚧 进行中 | 修复 P0 Bug + 死代码清理 + 去重复 + 测试补齐 | 175 |
| P3 运行时增强 | ⏳ 未开始 | 调度器优化 / 持久化 / 集群 | - |
| P4 生产就绪 | ⏳ 未开始 | 可观测性 / 压力测试 / 文档 | - |

---

## P2 架构债务修复 — 任务进度

详细计划见 [15-architecture-debt-fix.md](plans/15-architecture-debt-fix.md)

### Phase 0: 基础设施 — ✅ 100%
- [x] **Task 0.1** 新增 AxiomError 变体 (TypeMismatch, WitnessSerialization, SignalSerialization)
- [x] **Task 0.2** Signal::serialize_to_json 返回 Result
- [x] **Task 0.3** SignalPayload 宏修复 (validate + serialize_to_json)
- [x] **Task 0.4** #[schema(skip)] 属性支持

### Phase 1: 核心正确性修复 — ✅ 100%
- [x] **Task 1.1** 修复 DynAxiomChain::check_all 类型误报
- [x] **Task 1.2** 修复 axiom 宏 check_dyn 返回 TypeMismatch
- [x] **Task 1.3** 修复 EntropyEvent::Custom 丢弃 weight
- [x] **Task 1.4** 修复 Witness 序列化错误被静默吞掉
- [x] **Task 1.5** 修复 Witness 哈希链断裂
- [x] **Task 1.6** 修复 context.rs hop_count 不继承

### Phase 2: 架构强制约束 — ✅ 100%
- [x] **Task 2.1** 移除 ExecCellContext::send_to_validate
- [x] **Task 2.2** 移除 ValidateCellContext::send_to_agent
- [x] **Task 2.3** 修复 emit_internal 忽略 warnings

### Phase 3: Runtime 集成 — 🚧 67%
- [x] **Task 3.1** 统一 EntropyGovernor — 删除 runtime 副本
- [x] **Task 3.2** 修复 dispatch loop — 实际调用 Cell::handle
- [ ] **Task 3.3** 接线 EntropyGovernor 到派发路径 (部分完成)

### Phase 4: 死代码清理 — ✅ 100%
- [x] **Task 4.1** 删除 MigrationRegistry
- [x] **Task 4.2** 删除其他死代码 (DynamicSchema / typed AxiomChain / Lens / dead variants)

### Phase 5: 代码重复消除 — ⏳ 0%
- [ ] **Task 5.1** 统一 EntropyLevel (core ↔ oversight)
- [ ] **Task 5.2** 统一 now_ns (5 处重复定义)

### Phase 6: 测试补齐 — ⏳ 0%
- [ ] **Task 6.1** 错误路径测试 (5 个场景)
- [ ] **Task 6.2** 并发测试 (3 个场景)

### Phase 7: 全量验证 — ⚠️ 80%
- [x] cargo fmt --all -- --check
- [ ] cargo clippy --workspace -- -D warnings (2 类警告: type_complexity, manual_async_fn)
- [x] cargo build --workspace --all-targets
- [x] cargo test --workspace (175 passed)
- [x] 测试数量不回退 (基线 179 → 175, 减少 4 个死代码测试)
- [ ] 零 unwrap() / expect() 在非测试代码中

---

## 当前已知问题

### Clippy 警告 (非阻塞)
1. **type_complexity** (2 处): `handle_dyn` 返回类型 `Pin<Box<dyn Future<Output = ...>>` 太复杂
   - 位置: [cell.rs:153](file:///d:/work/trae/axiom-core/crates/axiom-core/src/cell.rs#L153-L153), [cell.rs:224](file:///d:/work/trae/axiom-core/crates/axiom-core/src/cell.rs#L224-L224)
   - 建议: 引入 type alias 如 `type BoxHandleFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>`

2. **manual_async_fn** (4 处): Cell impl 的 `handle` 方法可以用 `async fn` 语法
   - 位置: cell.rs 测试, hello_cell.rs, integration_tests.rs, macros/integration.rs
   - 原因: 使用 RPITIT (`impl Future`) 而非 `async fn`，这是为了 trait 签名一致性
   - 建议: 在 trait 中使用 `async fn` (Rust 1.75+ 支持)，或添加 `#[allow(clippy::manual_async_fn)]`

### 剩余未完成任务
详见上面 Phase 5 / Phase 6 / Phase 7 的未完成项。

---

## 代码质量指标

| 指标 | 当前值 | 目标 |
|------|--------|------|
| 测试总数 | 175 | ≥ 200 |
| 单元测试 | 147 | - |
| 集成测试 | 18 | ≥ 30 |
| Clippy 警告 | 6 (2 类) | 0 |
| 死代码率 | ~5% (已清理) | 0% |
| 文档覆盖率 | 中 | 高 |
