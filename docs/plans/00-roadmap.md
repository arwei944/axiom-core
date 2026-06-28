# Axiom Core 开发路线图（Master Roadmap）

> **所有阶段严格按依赖顺序执行，每个阶段结束时必须有可编译、可测试、可运行的交付物。**

---

## 阶段总览

| Phase | 名称 | Crates | 关键交付物 | 验收里程碑 |
|-------|------|--------|-----------|-----------|
| **P0** | 基础设施 | workspace | 依赖配置、测试框架、移除async-trait、版本管理 | ✅ 已完成：`cargo build` 0 warnings，`cargo test` 全部通过 |
| **P0.5** | **L0开发门禁** | axiom-cli + .github/ | axm CLI(preflight/check/verify/version) + cargo-axiom + CI/CD + constraints.lock + git hooks | `axm check`一键跑完全部质量门，CI自动拦截不合规代码 |
| **P0.6** | **L1编译期门禁** | axiom-macros + axiom-core | 5个proc宏(SignalPayload/cell/axiom/schema_version/migration) + Sealed CanSendTo + build.rs + linkme自动注册 | 架构违规编译不过，boilerplate宏自动生成，迁移自动发现 |
| **P1** | 核心原语完善 | axiom-core | SignalPayload/SignalEnvelope重构 + CellContext适配宏 + WitnessBuilder自动注入VersionInfo | CellContext可发SignalEnvelope到outbox、产生Witness、必须使用宏定义Signal |
| **P2** | 事件存储 | axiom-store | Snapshot + Replay + EventStore完善 + auto_collect迁移验证 | 可以从EventLog重建任意状态，旧版本数据自动迁移，迁移链gap启动时abort |
| **P3** | 运行时 | axiom-runtime | Mailbox + Bus+ArchitectureGuardian拦截点 + Dispatcher + Supervisor自愈 + circuit breaker | 多Cell通信、panic自动重启、超时熔断、Bus拦截器就绪 |
| **P4** | **L2运行时门禁** | axiom-oversight | ArchitectureGuardian + EntropyGovernor自动触发 + 启动验证链 + health endpoint | 运行时拦截违规消息、熵超标自动治理、崩溃自动恢复，三层门禁全开 |
| **P4.5** | **自动进化引擎** | axiom-evolution | 元公理M1-M7 + Observe→Hypothesize→Sandbox→Canary→Adopt闭环 + EvolutionWitness + axm evolution CLI | 系统自动检测改进机会、沙盒验证、金丝雀部署、自动回滚；进化受7条不可变元公理约束 |
| **P5** | 可视化导出 | axiom-viz | topology/timeline/entropy/trace/metrics + evolution视图 | 可以导出完整的系统状态快照+进化历史 |
| **P6** | 身份系统 | axiom-agent | Identity/Persona/PermissionSet + Identity版本化 | Agent Cell可以挂载身份、受权限约束 |
| **P7** | 技能系统 | axiom-agent | Skill/SKILL.md解析/渐进式披露/激活/触发 | Skill可以自动触发激活/挂载Tools/Lenses/Axioms |
| **P8** | 规则引擎 | axiom-agent | Ruleset/Validator/Prompt注入/三层执行 | 规则违规可检测、可重试、可升级为Axiom |
| **P9** | LLM+工具 | axiom-llm, axiom-tool | LLM抽象/重试/缓存/结构化输出/Tool注册 | 可以调用LLM+工具完成简单任务 |
| **P10** | MCP桥接 | axiom-mcp | MCP Client/Server/Security | 可以连接MCP Server使用外部工具（依赖P9 ToolRegistry） |
| **P11** | CLI脚手架完善 | axiom-cli | axm init/new/doctor/top/trace/why | 可以创建项目、健康诊断、TUI监控、Trace查询、Witness根因分析 |
| **P12** | 记忆系统 | axiom-memory | 四层记忆+自动摘要+Token预算 | Agent可以记住历史、按需检索 |
| **P13** | 规划器 | axiom-planner | ReAct/Plan-Execute | Agent可以规划和执行多步任务 |
| **P14** | 提示词+RAG | axiom-prompt, axiom-rag | 类型安全模板/RAG检索 | 提示词可组合、文档可检索 |
| **P15** | 测试+评估 | axiom-test, axiom-eval | Mock LLM/故障注入/录制重放/Golden Set | 可以写确定性Agent测试 |
| **P16** | CLI完善 | axiom-cli | shell/replay/test/cell管理 | 完整CLI功能+REPL |
| **P17** | 示例+文档 | examples/ | 完整多Agent示例项目 | end-to-end示例跑通 |

---

## 各阶段依赖关系

```
P0 基础设施 (+ 版本管理层) ✅
  ↓
P0.5 L0开发门禁 (axiom-cli + CI/CD + hooks + constraints.lock)
  ↓────── L0生效：代码提交/合并受axm check+CI门禁保护
P0.6 L1编译期门禁 (axiom-macros: 5个proc宏 + Sealed CanSendTo + build.rs + linkme)
  ↓────── L1生效：架构违规编译不过，boilerplate宏自动生成
P1 核心原语 (axiom-core: SignalPayload/SignalEnvelope重构 + CellContext适配宏 + VersionInfo自动注入)
  ↓
P2 事件存储 (axiom-store: Snapshot/Replay/Migration auto_collect验证)
  ↓
P3 运行时 (axiom-runtime: Bus拦截点 + Mailbox/Dispatcher + Supervisor自愈/circuit breaker)
  ↓
P4 L2运行时门禁 (axiom-oversight: ArchitectureGuardian + EntropyGovernor + 启动验证链)
  ↓────── L2生效：运行时拦截+自愈+自动去熵，三层门禁全开
P4.5 自动进化引擎 (axiom-evolution: 元公理M1-M7 + 观察→假设→沙盒→金丝雀→采纳闭环 + EvolutionWitness)
  ↓────── Layer E生效：系统可自动进化，进化受元公理约束
P5 可视化 (axiom-viz，含evolution视图)
  ↓
P6 身份(+版本化) → P7 技能 → P8 规则 (axiom-agent)
  ↓
P9 LLM+工具 (axiom-llm, axiom-tool)
  ↓
P10 MCP (axiom-mcp) — 依赖 P9 ToolRegistry
  ↓
P11 CLI脚手架完善 (axm init/new/doctor/top/trace/why)
  ↓
P12 记忆 → P13 规划 → P14 提示词+RAG → P15 测试+评估
  ↓
P16 CLI完善 → P17 示例+文档
```

---

## 每个阶段的通用验收标准

每个 Phase 结束时必须满足（P0.5起由`axm check`+CI自动强制执行）：

1. **编译通过**：`cargo build --workspace` 零警告（`RUSTFLAGS="-D warnings"`）
2. **格式正确**：`cargo fmt --all -- --check` 通过
3. **Clippy零警告**：`cargo clippy --workspace -- -D warnings` 零警告
4. **测试通过**：`cargo test --workspace` 全部通过
5. **架构验证通过**：`axm verify`（依赖方向/unsafe审计/层间trait使用）通过
6. **文档编译无警告**：`cargo doc --no-deps -p <crate>` 通过
7. **无unsafe泄露**：unsafe代码全部有SAFETY注释，unsafe_audit检查通过
8. **依赖审计通过**：无新增第三方依赖未经R-022审计，deps_audit通过
9. **无占位符**：非测试代码无`todo!()`/`unimplemented!()`/`FIXME!`
10. **版本兼容**：Schema版本号正确，旧版本数据可迁移或明确拒绝
11. **文档完整**：所有public API有rustdoc注释
12. **CI绿灯**：push/PR触发CI workflow全部通过
13. **可提交**：每个Phase作为一个或多个commit，commit message格式 `feat(scope): description`

---

## 详细计划索引

→ Phase 0 基础设施（已完成）：见 [01-phase0-1-foundation.md](./01-phase0-1-foundation.md)
→ Phase 0.5-0.6 **三层自动化门禁**：见 [03-phase0-5-0-6-automation-gates.md](./03-phase0-5-0-6-automation-gates.md)
→ Phase 4.5 **自动进化引擎**：见 [04-phase4-5-auto-evolution.md](./04-phase4-5-auto-evolution.md)
