# Axiom Core 开发路线图（Master Roadmap）

> **所有阶段严格按依赖顺序执行，每个阶段结束时必须有可编译、可测试、可运行的交付物。**

---

## 阶段总览

| Phase | 名称 | Crates | 关键交付物 | 验收里程碑 |
|-------|------|--------|-----------|-----------|
| **P0** | 基础设施 | workspace | 依赖配置、测试框架、移除async-trait、版本管理 | `cargo build` 0 warnings，`cargo test` 全部通过 |
| **P0.5** | 过程宏 | axiom-macros | #[derive(Signal)]/#[cell]/#[axiom]宏 | 派生宏可自动生成Signal实现，无需手写boilerplate |
| **P1** | 核心原语完善 | axiom-core | Cell/Signal/Lens/Axiom/Witness完整实现 + version/schema | CellContext可发SignalEnvelope到outbox、产生Witness、Schema验证 |
| **P2** | 事件存储 | axiom-store | Snapshot + Replay + EventStore完善 + schema迁移 | 可以从EventLog重建任意状态，旧版本数据可迁移 |
| **P3** | 运行时 | axiom-runtime | Mailbox + Bus + Dispatcher + Supervisor + Kernel | 多Cell可以通信、崩溃自动重启、hello_cell端到端收发 |
| **P4** | 监督层 | axiom-oversight | 7个Oversight Cell完整实现 | 架构违规/熵超标/循环检测自动触发治理 |
| **P5** | 可视化导出 | axiom-viz | topology/timeline/entropy/trace/metrics | 可以导出完整的系统状态快照 |
| **P6** | 身份系统 | axiom-agent | Identity/Persona/PermissionSet + Identity版本化 | Agent Cell可以挂载身份、受权限约束 |
| **P7** | 技能系统 | axiom-agent | Skill/SKILL.md解析/渐进式披露/激活/触发 | Skill可以自动触发激活/挂载Tools/Lenses/Axioms |
| **P8** | 规则引擎 | axiom-agent | Ruleset/Validator/Prompt注入/三层执行 | 规则违规可检测、可重试、可升级为Axiom |
| **P9** | LLM+工具 | axiom-llm, axiom-tool | LLM抽象/重试/缓存/结构化输出/Tool注册 | 可以调用LLM+工具完成简单任务 |
| **P10** | MCP桥接 | axiom-mcp | MCP Client/Server/Security | 可以连接MCP Server使用外部工具（依赖P9 ToolRegistry） |
| **P11** | CLI基础 | axiom-cli | axm new/run/top/trace/why/version | 可以创建项目、启动系统、TUI监控、版本检查 |
| **P12** | 记忆系统 | axiom-memory | 四层记忆+自动摘要+Token预算 | Agent可以记住历史、按需检索 |
| **P13** | 规划器 | axiom-planner | ReAct/Plan-Execute | Agent可以规划和执行多步任务 |
| **P14** | 提示词+RAG | axiom-prompt, axiom-rag | 类型安全模板/RAG检索 | 提示词可组合、文档可检索 |
| **P15** | 测试+评估 | axiom-test, axiom-eval | Mock LLM/故障注入/录制重放/Golden Set | 可以写确定性Agent测试 |
| **P16** | CLI完善 | axiom-cli | shell/replay/test/doctor/cell管理 | 完整CLI功能+REPL |
| **P17** | 示例+文档 | examples/ | 完整多Agent示例项目 | end-to-end示例跑通 |

---

## 各阶段依赖关系

```
P0 基础设施 (+ 版本管理层)
  ↓
P0.5 过程宏 (axiom-macros)
  ↓
P1 核心原语 (axiom-core: Cell/Signal/Lens/Axiom/Witness + version/schema)
  ↓
P2 事件存储 (axiom-store: Snapshot/Replay/Migration)
  ↓
P3 运行时 (axiom-runtime)
  ↓
P4 监督层 (axiom-oversight) ──→ P5 可视化 (axiom-viz)
  ↓
P6 身份(+版本化) → P7 技能 → P8 规则 (axiom-agent)
  ↓
P9 LLM+工具 (axiom-llm, axiom-tool)
  ↓
P10 MCP (axiom-mcp) — 依赖 P9 ToolRegistry
  ↓
P11 CLI基础 (axiom-cli: new/run/top/trace/why/version)
  ↓
P12 记忆 → P13 规划 → P14 提示词+RAG → P15 测试+评估
  ↓
P16 CLI完善 → P17 示例+文档
```

---

## 每个阶段的通用验收标准

每个 Phase 结束时必须满足：

1. **编译通过**：`cargo build --workspace` 零警告（`deny(warnings)` 启用）
2. **Clippy零警告**：`cargo clippy --workspace -- -D warnings` 零警告
3. **测试通过**：`cargo test --workspace` 全部通过
4. **格式正确**：`cargo fmt --check` 通过
5. **示例可运行**：如果是功能阶段，至少有一个 example 可运行
6. **无unsafe泄露**：unsafe代码全部在 `unsafe_impl` 模块中，且有SAFETY注释
7. **依赖方向正确**：`cargo tree` 验证无反向依赖
8. **版本兼容**：Schema版本号正确，旧版本数据可迁移或明确拒绝
9. **文档完整**：所有public API有rustdoc注释
10. **可提交**：每个Phase作为一个或多个commit，commit message格式 `feat(scope): description`

---

## Phase 0-1 详细计划

→ 见 [00-phase0-1-foundation.md](./00-phase0-1-foundation.md)
