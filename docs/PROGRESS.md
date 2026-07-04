# Axiom Core 项目进度总览

> **最后更新:** 2026-07-05
> **当前阶段:** v0.3.0 开发中
> **代码基线:** master @ latest
> **测试总数:** 47+ (全部通过)
> **质量门禁:** ✅ build / ✅ test / ✅ archcheck / ✅ clippy / ✅ fmt

---

## v0.3.0 完成情况

| 阶段 | 状态 | 描述 | 测试数 |
|------|------|------|--------|
| 文件拆分 | ✅ 完成 | sqlite.rs / replay.rs / file_store.rs 拆分为子模块 | 47 |
| 架构优化 | ✅ 完成 | 收敛过度暴露的 pub 接口、重构超 80 行函数 | 47 |
| 错误统一 | ✅ 完成 | StoreError -> AxiomError 统一转换 | 47 |
| 属性测试 | ✅ 完成 | 引入 proptest，5 个属性测试 | 47 |
| 并发测试 | ✅ 完成 | 引入 loom，1 个并发测试 | 47 |
| CI 覆盖率门禁 | ✅ 完成 | 建立 80% 覆盖率阈值 | - |
| 文档更新 | ✅ 完成 | README / API_BOUNDARY / architecture-diagram / STATE_TRANSITION | - |

---

## 已完成功能

### Task 1: 文件拆分
- [x] `sqlite.rs` → `sqlite/mod.rs` + `sqlite/store.rs` + `sqlite/queries.rs` + `sqlite/config.rs`
- [x] `replay.rs` → `replay/mod.rs` + `replay/engine.rs` + `replay/validation.rs` + `replay/witness.rs`
- [x] `file_store.rs` → `file_store/mod.rs` + `file_store/config.rs` + `file_store/store.rs`

### Task 2: 架构优化
- [x] 收敛过度暴露的 pub 接口
- [x] 重构超 80 行函数（拆分 file_store read 逻辑）
- [x] 统一错误类型：`StoreError -> AxiomError`

### Task 3: 测试增强
- [x] 引入 proptest：5 个属性测试（roundtrip、batch 顺序、read_after_sequence、重复拒绝、序列号单调性）
- [x] 引入 loom：1 个并发测试（序列号生成唯一性）
- [x] 所有测试通过（47 passed）

### Task 4: CI/CD 增强
- [x] 建立 CI 覆盖率门禁（80% 阈值）
- [x] 更新 .github/workflows/ci.yml

### Task 5: 文档更新
- [x] 更新 README.md（项目结构、快速开始）
- [x] 更新 docs/API_BOUNDARY.md（v0.3.0、sqlite 默认启用）
- [x] 更新 docs/architecture-diagram.md（子模块结构）
- [x] 新建 docs/STATE_TRANSITION.md（状态转换图）

---

## v0.2.0 开发计划

### 阶段目标：生产深度 > 功能广度

| Phase | 周期 | 目标 |
|-------|------|------|
| Phase 1 | 1周 | Lens 原语实现 — 完成5原语故事 |
| Phase 2 | 2周 | Store 持久化 — SQLite + 文件系统后端 |
| Phase 3 | 1周 | 约束运行时统一 — 编译期约束在运行时总线层强制执行 |
| Phase 4 | 1周 | 现有 crate 深化 — identity/prompt/planner/memory |
| Phase 5 | 1周 | API 稳定与发布 — v0.2.0 发布 |

### v0.2.0 关键里程碑

| 里程碑 | 日期 | 完成标准 |
|--------|------|----------|
| M1: Phase 1 完成 | Week 1 结束 | Lens 原语可用 |
| M2: Phase 2 完成 | Week 3 结束 | SQLite + FileStore 可用 |
| M3: Phase 3 完成 | Week 4 结束 | 约束运行时闭环 |
| M4: Phase 4 完成 | Week 5 结束 | 配套 crate 深化 |
| M5: v0.2.0 发布 | Week 5 结束 | 所有检查通过 |

---

## 质量指标

| 指标 | 目标 | 当前 |
|------|------|------|
| 测试通过率 | 100% | 100% (47/47) |
| Clippy 警告 | 0 | 0 |
| 代码格式合规 | 100% | 100% |
| 架构违规 | 0 | 0 |
| 覆盖率 | >= 80% | 建立门禁 |
