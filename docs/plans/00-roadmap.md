# Axiom Core 开发路线图（Master Roadmap）

> **所有阶段严格按依赖顺序执行，每个阶段结束时必须有可编译、可测试、可运行的交付物。**

---

## 阶段总览

| Phase | 名称 | Crates | 关键交付物 | 验收里程碑 |
|-------|------|--------|-----------|-----------|
| **P0** | 基础设施 | workspace | 依赖配置、测试框架、移除async-trait、版本管理 | ✅ 已完成 |
| **P0.5** | **L0开发门禁** | axiom-cli + .github/ | axm CLI(preflight/check/verify/version) + cargo-axiom + CI/CD + constraints.lock + git hooks | ✅ 已完成：`axm check`一键跑完全部质量门，CI自动拦截不合规代码 |
| **P0.6** | **L1编译期门禁** | axiom-macros + axiom-core | 5个proc宏(SignalPayload/cell/axiom/schema_version/migration) + Sealed CanSendTo + build.rs + linkme自动注册 | ✅ 已完成：架构违规编译不过，boilerplate宏自动生成，迁移自动发现 |
| **P1** | 核心原语完善 | axiom-core | SignalPayload/SignalEnvelope重构 + CellContext适配宏 + WitnessBuilder自动注入VersionInfo | 📋 任务书就绪（05-phase1） |
| **P2** | 事件存储 | axiom-store | Snapshot + Replay + EventStore完善 + auto_collect迁移验证 | 📋 任务书就绪（06-phase2） |
| **P3** | 运行时 | axiom-runtime | Mailbox + Bus+ArchitectureGuardian拦截点 + Dispatcher + Supervisor自愈 + circuit breaker | 📋 任务书就绪（07-phase3） |
| **P4** | **L2运行时门禁** | axiom-oversight | ArchitectureGuardian + EntropyGovernor自动触发 + 启动验证链 + health endpoint | 📋 任务书就绪（08-phase4） |
| **P4.5** | **自动进化引擎** | axiom-evolution | 元公理M1-M7 + Observe→Hypothesize→Sandbox→Canary→Adopt闭环 + EvolutionWitness + axm evolution CLI | 📋 任务书就绪（09-phase4.5） |
| **P5** | 可视化导出 | axiom-viz | topology/timeline/entropy/trace/metrics + evolution视图 | 📋 任务书就绪（10-phase5） |
| **P6** | 身份系统 | axiom-agent | Identity/Persona/PermissionSet + Identity版本化 | 📋 任务书就绪（11-phase6-8） |
| **P7** | 技能系统 | axiom-agent | Skill/SKILL.md解析/渐进式披露/激活/触发 | 📋 任务书就绪（11-phase6-8） |
| **P8** | 规则引擎 | axiom-agent | Ruleset/Validator/Prompt注入/三层执行 | 📋 任务书就绪（11-phase6-8） |
| **P9** | LLM+工具 | axiom-llm, axiom-tool | LLM抽象/重试/缓存/结构化输出/Tool注册 | 📋 任务书就绪（12-phase9-11） |
| **P10** | MCP桥接 | axiom-mcp | MCP Client/Server/Security | 📋 任务书就绪（12-phase9-11） |
| **P11** | CLI脚手架完善 | axiom-cli | axm init/new/doctor/top/trace/why | 📋 任务书就绪（12-phase9-11） |
| **P12** | 记忆系统 | axiom-memory | 四层记忆+自动摘要+Token预算 | 📋 任务书就绪（13-phase12-17） |
| **P13** | 规划器 | axiom-planner | ReAct/Plan-Execute | 📋 任务书就绪（13-phase12-17） |
| **P14** | 提示词+RAG | axiom-prompt, axiom-rag | 类型安全模板/RAG检索 | 📋 任务书就绪（13-phase12-17） |
| **P15** | 测试+评估 | axiom-test, axiom-eval | Mock LLM/故障注入/录制重放/Golden Set | 📋 任务书就绪（13-phase12-17） |
| **P16** | CLI完善 | axiom-cli | shell/replay/test/cell管理 | 📋 任务书就绪（13-phase12-17） |
| **P17** | 示例+文档 | examples/ | 完整多Agent示例项目 | 📋 任务书就绪（13-phase12-17） |

---

## 各阶段依赖关系

```
P0 基础设施 (+ 版本管理层) ✅
  ↓
P0.5 L0开发门禁 ✅ (axiom-cli + CI/CD + hooks + constraints.lock)
  ↓────── L0生效：代码提交/合并受axm check+CI门禁保护
P0.6 L1编译期门禁 ✅ (axiom-macros: 5个proc宏 + Sealed CanSendTo + build.rs + linkme)
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
→ Phase 1 核心原语完善：见 [05-phase1-core-primitives.md](./05-phase1-core-primitives.md)
→ Phase 2 事件存储：见 [06-phase2-event-store.md](./06-phase2-event-store.md)
→ Phase 3 运行时完善：见 [07-phase3-runtime.md](./07-phase3-runtime.md)
→ Phase 4 L2运行时门禁：见 [08-phase4-l2-gates.md](./08-phase4-l2-gates.md)
→ Phase 4.5 **自动进化引擎**：见 [09-phase4.5-auto-evolution.md](./09-phase4.5-auto-evolution.md)
→ Phase 5 可视化：见 [10-phase5-visualization.md](./10-phase5-visualization.md)
→ Phase 6-8 身份/技能/规则（axiom-agent）：见 [11-phase6-8-identity-skills-rules.md](./11-phase6-8-identity-skills-rules.md)
→ Phase 9-11 LLM+工具/MCP桥接/CLI脚手架：见 [12-phase9-11-llm-tools-mcp-cli.md](./12-phase9-11-llm-tools-mcp-cli.md)
→ Phase 12-17 记忆/规划/提示词+RAG/测试评估/CLI完善/示例文档：见 [13-phase12-17-memory-planner-prompt-test-cli-examples.md](./13-phase12-17-memory-planner-prompt-test-cli-examples.md)
