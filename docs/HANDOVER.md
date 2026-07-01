# Axiom Core 项目交接文档

> **版本:** v0.1.0 (pre-release)
> **交接日期:** 2026-07-01
> **代码基线:** master @ f955051
> **仓库:** https://github.com/arwei944/axiom-core

---

## 1. 项目概述

### 1.1 项目定位
Axiom Core 是一个基于 **低熵架构哲学** 的 Actor 风格运行时框架，核心目标是：
- **功能增加时熵不增长** — 通过严格的层级隔离和不变式约束
- **问题一秒定位** — 通过 Witness 哈希链 + CorrelationId + VectorClock 可追溯
- **功能方便增删** — 通过宏 + 分布式注册表 (linkme) 实现插件化

### 1.2 核心设计理念
| 设计原则 | 实现机制 |
|---------|---------|
| 层级隔离 | 4 层架构: Oversight → Agent → Validate → Exec，每层只能向下发消息 |
| 私有状态 | Cell 状态完全私有，只能通过 `handle()` 修改 |
| 不变式保障 | Axiom trait + 分布式注册表，状态变更前自动检查 |
| 可追溯性 | Witness 哈希链 + CorrelationId + VectorClock |
| 熵监控 | EntropyScore 8 因子模型 + EntropyGovernor 冷却降级 |

### 1.3 架构约束 (Hard Constraints)
详见 [project_memory.md](file:///c:/Users/Administrator/.trae-cn/memory/projects/-d-work-trae-axiom-core/project_memory.md)

- 禁止 async-trait (R-004)
- Crate 依赖必须遵循 N → ≥ N 方向
- 外部依赖必须在白名单 (R-022)
- Axiom `check()` 必须纯函数 (无 async, 无 IO)
- Migration `migrate()` 必须纯函数
- Schema `validate()` 必须同步执行
- 层间通信必须遵循 can_send_to 规则
- 信号方向严格自上而下: Oversight → Agent → Validate → Exec

---

## 2. 代码结构

### 2.1 Crate 分层
```
crates/
├── axiom-core/         # L2 Core — 核心原语 (Cell/Signal/Axiom/Witness/Entropy)
├── axiom-macros/       # L2 Core — 过程宏 (cell/axiom/migration/SignalPayload)
├── axiom-runtime/      # L3 Runtime — 消息总线/调度/监管/熵治理
├── axiom-oversight/    # L4 Oversight — 架构监护/健康检查/资源管理
├── axiom-store/        # L3 Store — 事件存储/重放/快照
├── axiom-agent/        # L4 Agent — 智能体层 (骨架)
├── axiom-cli/          # L3 CLI — 开发工具/门禁检查
└── axiom-viz/          # L3 Viz — 可视化 (骨架)
```

### 2.2 关键文件索引

| 模块 | 文件 | 核心类型/函数 |
|------|------|-------------|
| Cell | [cell.rs](file:///d:/work/trae/axiom-core/crates/axiom-core/src/cell.rs) | `Cell`, `DynHandleCell`, `CellHandle` |
| 信号 | [signal.rs](file:///d:/work/trae/axiom-core/crates/axiom-core/src/signal.rs) | `Signal`, `SignalEnvelope`, `VectorClock` |
| 不变式 | [axiom.rs](file:///d:/work/trae/axiom-core/crates/axiom-core/src/axiom.rs) | `Axiom`, `DynAxiom`, `DynAxiomChain` |
| 见证 | [witness.rs](file:///d:/work/trae/axiom-core/crates/axiom-core/src/witness.rs) | `Witness`, `WitnessBuilder`, `WitnessBatch` |
| 熵 | [entropy.rs](file:///d:/work/trae/axiom-core/crates/axiom-core/src/entropy.rs) | `EntropyScore`, `EntropyLevel`, `EntropySnapshot` |
| 上下文 | [context.rs](file:///d:/work/trae/axiom-core/crates/axiom-core/src/context.rs) | `CellContext`, `OutgoingEnvelope`, `OutgoingWitness` |
| 版本 | [version.rs](file:///d:/work/trae/axiom-core/crates/axiom-core/src/version.rs) | `SchemaVersion`, `Migration`, `SchemaMigrator` |
| 运行时 | [runtime.rs](file:///d:/work/trae/axiom-core/crates/axiom-runtime/src/runtime.rs) | `AxiomRuntime`, `RuntimeBuilder`, `CellRegistration` |
| 消息总线 | [bus.rs](file:///d:/work/trae/axiom-core/crates/axiom-runtime/src/bus.rs) | `MessageBus` |
| 监管 | [supervisor.rs](file:///d:/work/trae/axiom-core/crates/axiom-runtime/src/supervisor.rs) | `Supervisor` |
| 架构监护 | [architecture_guardian.rs](file:///d:/work/trae/axiom-core/crates/axiom-oversight/src/architecture_guardian.rs) | `ArchitectureGuardian` |
| 熵治理 | [entropy_governor.rs](file:///d:/work/trae/axiom-core/crates/axiom-oversight/src/entropy_governor.rs) | `EntropyGovernorCell`, `EntropyEvent` |
| 宏 | [lib.rs](file:///d:/work/trae/axiom-core/crates/axiom-macros/src/lib.rs) | `#[cell]`, `#[axiom]`, `#[migration]`, `SignalPayload` |

---

## 3. 核心设计模式

### 3.1 Cell::handle — "Drain Inside, Return Everything"

**问题:** RPITIT (Return Position Impl Trait In Traits) 的不透明 future 会将 `&mut ctx` 借用绑定到 `'a` 生命周期，编译器无法证明 `.await` 后借用结束。

**解决方案:**
```rust
fn handle<'a>(
    &'a mut self,
    signal: Self::Message,
    ctx: &'a mut CellContext<'a>,
) -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a;
```

- 实现内部调用 `ctx.end_processing()` 排空所有数据
- 返回三元组，调用方**永远不要**在 `handle().await` 后访问 `ctx`
- 完全消除第二次可变借用的需求

### 3.2 循环中多次调用 handle

**问题:** 同上，循环中 RPITIT 借用会扩展到所有迭代。

**解决方案:** 使用 `Arc<Mutex<Cell>>` 包装，每次循环获取本地 guard：
```rust
for i in 0..5 {
    let mut guard = cell.lock().await;
    let mut ctx = CellContext::new(&cell_id, Layer::Exec);
    let (r, outgoing, witnesses) = guard.handle(signal, &mut ctx).await;
    // guard 和 ctx 在本次迭代结束时销毁，借用释放
}
```

Runtime 的 dispatch loop 就是这个模式。

### 3.3 闭包模式处理 `?` 运算符

**问题:** async block 返回 tuple 时不能直接用 `?`。

**解决方案:**
```rust
async move {
    let result: Result<()> = (|| {
        ctx.emit_event(event, Layer::Exec)?;
        ctx.witness().summary("...").emit(ctx)?;
        Ok(())
    })();
    let (outgoing, witnesses) = ctx.end_processing();
    (result, outgoing, witnesses)
}
```

### 3.4 DynHandleCell — 类型擦除调度

```rust
pub trait DynHandleCell: DynCell {
    fn handle_dyn<'a>(
        &'a mut self,
        env: SignalEnvelope,
        ctx: &'a mut CellContext<'a>,
    ) -> Pin<Box<dyn Future<Output = (Result<()>, Vec<OutgoingEnvelope>)> + Send + 'a>>;
}
```

- 从 JSON 反序列化为具体类型，调用 `Cell::handle`
- 返回二元组 (丢弃 witnesses，后续版本会接线)
- 通过 `CellHandle` 包装后可放入 `Vec<CellHandle>` 统一调度

### 3.5 分布式注册表 (linkme)

- `#[axiom]` 宏 → `AXIOM_REGISTRY` slice
- `#[migration]` 宏 → `MIGRATION_REGISTRY` slice
- 零成本静态注册，运行时直接遍历

---

## 4. 当前状态与已知问题

### 4.1 已完成
- ✅ P1 核心原语全部可用
- ✅ P2 Phase 0-4 (基础/Bug 修复/架构强制/死代码清理)
- ✅ 175 个测试全部通过
- ✅ Runtime dispatch loop 实际调用 `Cell::handle`
- ✅ EntropyGovernor 接线到派发路径 (部分)

### 4.2 已知问题 / 技术债务

#### Clippy 警告 (非阻塞)
| 警告类型 | 数量 | 位置 | 建议修复 |
|---------|------|------|---------|
| type_complexity | 2 | cell.rs:153, cell.rs:224 | 引入 type alias |
| manual_async_fn | 4 | 测试/示例代码 | `#[allow]` 或改用 `async fn` |

#### 未完成任务 (P2 Phase 5-7)
详见 [PROGRESS.md](PROGRESS.md)

1. **Task 5.1: 统一 EntropyLevel** — core 和 oversight 各有一份定义
2. **Task 5.2: 统一 now_ns** — 5 处重复定义
3. **Task 6.1: 错误路径测试** — 5 个场景未覆盖
4. **Task 6.2: 并发测试** — 3 个场景未覆盖
5. **Task 7.1: clippy 零警告** — 需消除 type_complexity 和 manual_async_fn

### 4.3 设计待决策
1. **async fn in trait** — 目前用 RPITIT，是否升级到 `async fn` 语法？
2. **Witness 接线** — `handle_dyn` 丢弃了 witnesses，何时接入持久化？
3. **EntropyGovernor 统一** — runtime 和 oversight 各有一份，已删除 runtime 的 simple 版，但 oversight 的 `EntropyGovernorCell` API 与 runtime 的使用方式需要对齐

---

## 5. 开发工作流

### 5.1 质量门禁
```bash
# 格式化
cargo fmt --all

# 静态检查
cargo clippy --workspace --all-targets --all-features

# 构建
cargo build --workspace --all-targets

# 测试
cargo test --workspace
```

### 5.2 新增功能步骤
1. 定义 Signal (用 `SignalPayload` 宏)
2. 定义 Axiom (用 `#[axiom]` 宏) — 自动注册
3. 定义 Cell (用 `#[cell]` 宏) — 实现 `handle`
4. 编写测试 (单元 + 集成)
5. 通过所有质量门禁

### 5.3 常见坑

| 坑 | 现象 | 解决方案 |
|----|------|---------|
| RPITIT 借用冲突 | E0499 二次借用 | 使用 drain-inside 模式，不要在 handle 后访问 ctx |
| 循环中调用 handle | E0499 循环借用 | 用 Arc<Mutex<Cell>> 包装，每次循环取 guard |
| async block 中用 `?` | 编译错误，返回类型不匹配 | 用闭包模式 `(|| { ...?; Ok(()) })()` |
| 宏展开后调试困难 | 无法看到宏生成的代码 | `cargo expand` 或 trybuild 测试 |
| Windows 增量编译 ICE | rustc 内部错误 | `cargo clean -p <crate>` 后重编 |

---

## 6. 后续工作建议

### 高优先级 (P0)
1. **完成 P2 剩余任务** — 约 1-2 天工作量 (去重复 + 测试补齐)
2. **修复 clippy 警告** — type_complexity 引入 alias，manual_async_fn 加 allow

### 中优先级 (P1)
1. **Witness 持久化接线** — 将 handle 返回的 witnesses 写入存储
2. **EntropyGovernor 治理动作** — should_reduce 触发后实际执行限流/熔断
3. **Cell 重启机制** — supervisor 检测到崩溃后实际重启 cell

### 低优先级 (P2)
1. 完善文档和示例
2. 性能基准测试
3. 增加更多集成测试场景

---

## 7. 相关文档

- [架构文档](architecture/) — 架构设计说明
- [开发计划](plans/) — 各阶段详细开发计划
- [进度总览](PROGRESS.md) — 当前进度仪表盘
- [项目约束](../.axiom/rules/) — Axiom 约束检查规则

---

## 8. 联系人

如有疑问，优先查阅：
1. 本文档的 **设计模式** 章节
2. [project_memory](file:///c:/Users/Administrator/.trae-cn/memory/projects/-d-work-trae-axiom-core/project_memory.md) 中的 Hard Constraints
3. 对应模块的单元测试 (最准确的使用示例)
