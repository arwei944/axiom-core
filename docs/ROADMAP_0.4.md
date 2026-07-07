# Axiom v0.4.0 开发计划

> **当前基线**: v0.3.0（`axiom-kernel` 迁移 100% 完成）  
> **下一个版本**: v0.4.0  
> **主题**: Production Hardening & Ecosystem Readiness  
> **预估工期**: 4-6 周

---

## 一、版本目标

v0.4.0 不做架构大改，而是把 v0.3.0 已迁移完成的新架构“做成真正可生产用的版本”。

核心目标：
- **稳定性**：把当前 104 个 deprecated warning 压到可接受范围，完成旧 API 到 `axiom-kernel` 的迁移收尾
- **性能**：补齐 benchmark 回归红线，确保 dispatch / bus / witness / lens 关键路径有可量化基线
- **插件成熟度**：让 WASM 插件从“能跑”变成“可发布、可隔离、可观测”
- **可观测性**：统一 tracing / metrics / logging，满足生产运维基本要求
- **文档与示例**：补齐用户视角的入门文档和端到端示例
- **安全与合规**：依赖审计、插件边界检查、安全策略落地

---

## 二、v0.4.0 任务清单

### Phase 1：弃用清理与 API 收敛（1.5 周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P1-01** | 移除 `axiom-core` 中已 deprecated trait 的桥接冗余路径 | `cargo check` 非 deprecated warning ≤ 10 |
| **P1-02** | `axiom-runtime` / `axiom-agent` / `axiom-cli` 切换为 `axiom-kernel` API | 关键调用点无 deprecated 警告 |
| **P1-03** | 统一 `SignalKind` / `Layer` / `CellId` 等基础类型来源 | 全 workspace `grep` 无混用 |
| **P1-04** | 更新宏测试与示例，全部指向 `axiom-kernel` | `cargo test --workspace` 通过 |

**收益**：降低后续维护成本，让 `axiom-core` 真正退居“兼容层”。

---

### Phase 2：性能基线建设（1 周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P2-01** | 扩展 `axiom-bench`：bus dispatch、witness append、lens projection、signal validation | 至少 4 个基准场景 |
| **P2-02** | 建立 p50/p95/p99 基线，防止后续回归 | 输出 `bench-results.md` |
| **P2-03** | 关键路径热点分析（tokio / lock / clone / json） | 给出优化清单 |
| **P2-04** | 优化 Top-3 热点（如信号 envelope 分配、lens cache miss） | 关键指标提升 ≥ 20% |

**收益**：v0.4.0 具备可量化的性能承诺，而不是“感觉上还行”。

---

### Phase 3：插件生产化（1.5 周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P3-01** | WASM 插件加载失败隔离与错误上报 | 崩溃不影响宿主 |
| **P3-02** | 插件能力声明 + 运行时校验（dim / version / layer） | 非法插件拒绝加载 |
| **P3-03** | 插件热更新与版本回滚 | 支持 reload / rollback |
| **P3-04** | 插件可观测性：独立的 tracing span + metrics | 可区分宿主与插件耗时 |

**收益**：插件从“演示能力”升级为“可交付给外部开发者使用”。

---

### Phase 4：可观测性与运维（1 周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P4-01** | 统一 tracing 分层命名（cell / signal / lens / witness / plugin） | Jaeger/OTLP 可读 |
| **P4-02** | `HeatmapCollector` 增加导出接口（prometheus / json） | `axm heatmap --format prometheus` |
| **P4-03** | 关键错误增加结构化日志字段 | 日志可检索、可聚合 |
| **P4-04** | 运行时健康检查暴露 HTTP / CLI | `axm health` / `/health` |

**收益**：满足生产环境“能监控、能排障、能告警”的基本要求。

---

### Phase 5：安全与依赖治理（0.5 周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P5-01** | `cargo audit` + `cargo deny` 集成到 CI | CI 自动阻断高危漏洞 |
| **P5-02** | 插件 API 最小权限边界检查 | 无越权访问宿主内存可能 |
| **P5-03** | 更新 `SECURITY.md` 与安全策略 | 漏洞响应流程明确 |

**收益**：把安全从“文档说很重要”变成“代码和流程里真的 enforced”。

---

### Phase 6：文档、示例与发布（0.5 周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P6-01** | 新增 `examples/`：最小 cell、signal、lens、plugin | 每个示例可单独运行 |
| **P6-02** | 更新 `README.md`、`MIGRATION.md`、`API_BOUNDARY.md` | 与 v0.4.0 实际状态一致 |
| **P6-03** | 更新 `CHANGELOG.md` 与 `RELEASE_CHECKLIST.md` | 发布检查清单可执行 |
| **P6-04** | `cargo doc --workspace --no-deps` 无警告 | 文档发布可用 |

---

## 三、不纳入 v0.4.0 的范围

为避免范围膨胀，以下工作**不纳入 v0.4.0**：

- 新 crate 创建
- 架构层大改（如删除四层架构、替换 tokio 等）
- Lens / Witness / Store 等原语的重大语义变更
- v1.0  Breaking Change 大扫除（保留在 v1.0.0 处理）

---

## 四、质量门禁

v0.4.0 发布前必须满足：

```bash
cargo fmt --all --check
cargo clippy --workspace -D warnings
cargo check --workspace
cargo test --workspace
cargo doc --workspace --no-deps
cargo audit
cargo deny check
```

- 非 deprecated warning：**0**
- deprecated warning：仅限已登记兼容层，且数量明确下降
- 测试：全量通过
- 文档：公共 API 有文档

---

## 五、里程碑建议

| 里程碑 | 内容 | 预计时间 |
|--------|------|---------|
| **M1** | Phase 1 弃用清理完成 | Week 1.5 |
| **M2** | Phase 2 性能基线建立 | Week 2.5 |
| **M3** | Phase 3 插件生产化完成 | Week 4 |
| **M4** | Phase 4-5 可观测性与安全完成 | Week 4.5 |
| **M5** | Phase 6 文档与发布就绪 | Week 5-6 |

---

## 六、与 v1.0.0 的关系

v0.4.0 是为 v1.0.0 打基础的关键版本：

- v0.4.0 解决“能不能用”
- v1.0.0 解决“好不好用、稳不稳定、能不能商业化”

建议 v0.4.0 发布后，再评估是否还需要一个 v0.5.0 过渡版，还是直接进入 v1.0.0 API 冻结。

---

**文档创建时间**：2026-07-06  
**当前状态**：v0.3.0 已交付，`axiom-kernel` 全量迁移完成，`cargo check/test` 全绿  
**下一步**：进入 v0.4.0 Phase 1 弃用清理
