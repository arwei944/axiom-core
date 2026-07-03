# Axiom Core 架构设计图

> 基于实际源代码的全面架构可视化

---

## 1. 整体分层架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Layer 0: 顶层应用                                                       │
│   ├── axiom-cli      — axm 命令行工具                                    │
│   └── axiom-bench    — 基准测试套件                                     │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 1: 可视化                                                         │
│   └── axiom-viz      — 拓扑/时间轴/熵值导出                              │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 2: Agent 门面                                                     │
│   ├── axiom-identity — 身份、Persona、Skill 激活条件                     │
│   └── axiom-prompt   — Prompt 模板注册表                                 │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 3: 监督与集成                                                     │
│   ├── axiom-mcp      — MCP 协议桥接（Client + Server）                  │
│   ├── axiom-alert    — 告警系统（阈值、路由、静默）                      │
│   ├── axiom-agent    — Agent 开发配套（Identity + Skill + Rules + LLM） │
│   └── axiom-oversight— Layer 0 监督层（架构合规 + 熵治理 + 资源管理）    │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 4: 运行时与协调                                                   │
│   ├── axiom-distributed — 集群、节点发现、同步                           │
│   ├── axiom-planner     — 规划器（ReAct / PlanAndExecute）              │
│   └── axiom-runtime     — Tokio 运行时（消息总线 + 监督树 + 熵治理）    │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 5: 存储与工具                                                     │
│   ├── axiom-llm        — LLM 客户端抽象                                  │
│   ├── axiom-tool       — Tool trait + ToolRegistry + 权限控制            │
│   ├── axiom-memory     — 工作记忆 + 上下文预算                           │
│   └── axiom-store      — 事件存储（Append-Only Log + 快照 + 重放）      │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 6: （预留）                                                       │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 7: 核心原语                                                       │
│   └── axiom-core       — Cell / Signal / Lens / Axiom / Witness         │
│                          + Layer / Entropy / Capability / Version       │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 8: Proc-macro（豁免）                                             │
│   └── axiom-macros     — #[signal] #[cell] #[tool] #[guard] #[capability]│
└─────────────────────────────────────────────────────────────────────────┘

铁律：Layer N 只能依赖 Layer >= N 的 crate（向下依赖）
```

---

## 2. 核心原语关系图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           axiom-core                                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐         │
│  │  Signal  │    │   Cell   │    │   Lens   │    │  Axiom   │         │
│  │ 消息类型 │    │ 状态单元  │    │ 状态投影  │    │ 不变量   │         │
│  └────┬─────┘    └────┬─────┘    └────┬─────┘    └────┬─────┘         │
│       │               │               │               │                │
│       │  msg_id       │  handle()     │  project()    │  validate()    │
│       │  corr_id      │  ctx          │  cache        │  chain()       │
│       │  vector_clock │  witnesses    │  depends_on   │  dimensions    │
│       │  timestamp    │  layer        │  token_est    │  registry      │
│       └──────────────►│               │               │                │
│                      │               │               │                │
│  ┌──────────┐        │               │               │                │
│  │ Witness  │◄───────┘               │               │                │
│  │ 审计链   │◄───────────────────────┘               │                │
│  └────┬─────┘                                         │                │
│       │                                               │                │
│       │  prev_hash                                    │                │
│       │  SHA-256 chain                                │                │
│       └──────────────────────────────────────────────►│                │
│                                                        │                │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐        │                │
│  │  Layer   │    │  Entropy │    │Capability│◄───────┘                │
│  │ 架构层   │    │ 熵治理   │    │ 版本管理  │                         │
│  └──────────┘    └──────────┘    └──────────┘                         │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. 运行时拓扑

```
                        ┌─────────────────────────┐
                        │     AxiomRuntime         │
                        │  ┌───────────────────┐  │
                        │  │    MessageBus     │  │
                        │  │  (路由 + 拦截器链) │  │
                        │  └─────────┬─────────┘  │
                        │            │             │
                        │  ┌─────────▼─────────┐  │
                        │  │   Supervisor      │  │
                        │  │ (监督树 + 熔断器)  │  │
                        │  └─────────┬─────────┘  │
                        │            │             │
                        │  ┌─────────▼─────────┐  │
                        │  │ EntropyGovernor   │  │
                        │  │ (熵值 + 治理动作)  │  │
                        │  └───────────────────┘  │
                        └───────────┬────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        │                           │                           │
        ▼                           ▼                           ▼
  ┌─────────────┐           ┌─────────────┐           ┌─────────────┐
  │   Mailbox    │           │   Mailbox    │           │   Mailbox    │
  │  ┌────────┐  │           │  ┌────────┐  │           │  ┌────────┐  │
  │  │ Cell A │  │           │  │ Cell B │  │           │  │ Cell C │  │
  │  │(Exec)  │  │           │  │(Validate│  │           │  │(Agent) │  │
  │  └────────┘  │           │  └────────┘  │           │  └────────┘  │
  └─────────────┘           └─────────────┘           └─────────────┘
        │                           │                           │
        └───────────────────────────┼───────────────────────────┘
                                    │
                        ┌───────────▼──────────┐
                        │     EventStore       │
                        │  (FileStore/SQLite)  │
                        │  + SnapshotStore     │
                        └──────────────────────┘
```

---

## 4. 消息流

```
外部输入 / CLI
    │
    ▼
Runtime.submit_signal() / publish_command()
    │
    ▼
MessageBus.publish()
    │
    ├─ validate_layer_transition()        ← 运行时层检查
    ├─ Interceptor Chain:
    │   ├─ ArchitectureGuardian           ← 层违规 + 跳数 + Schema
    │   ├─ ThrottleInterceptor            ← 熵节流
    │   ├─ EmergencyInterceptor           ← 紧急模式
    │   ├─ HopLimitInterceptor            ← 跳数限制 (max 8)
    │   ├─ IdempotencyInterceptor         ← 去重 (msg_id)
    │   ├─ SchemaVersionInterceptor       ← Schema 版本 (0 拒绝)
    │   ├─ LoopDetectInterceptor          ← 循环检测 (16 槽滑动窗口)
    │   ├─ CapabilityVersionInterceptor   ← 能力版本兼容性
    │   └─ GuardInterceptor               ← Guard 检查
    │
    ▼
RoutingTable.resolve() → target cell(s)
    │
    ▼
Mailbox.push()  [容量控制，Semaphore]
    │
    ▼
Runtime Dispatch Loop (poll interval 10ms)
    │
    ▼
Supervisor.before_handle()  [熔断器检查]
    │
    ▼
Cell.handle_dyn(env, ctx)
    │   ├─ deserialize SignalEnvelope → Cell::Message
    │   ├─ Cell::handle(msg, LayeredCellContext)
    │   │       ├─ ctx.send_to / emit_to (编译期层限制)
    │   │       ├─ ctx.witness().emit() → WitnessBuilder
    │   │       └─ return (Result, outgoing_envelopes, witnesses)
    │   └─ catch_unwind (panic 恢复)
    │
    ▼
on success:
    ├─ bus.publish(outgoing_envelopes)
    ├─ supervisor.record_success()
    └─ persist witnesses → EventStore
        persist snapshots (每 100 事件)
        verify witness chain integrity

on failure:
    ├─ supervisor.record_panic()
    │   └─ SupervisionDecision: Restart / Stop / Escalate / CircuitBreak
    ├─ governor.record(AxiomViolation / CellRestart / CircuitBreak)
    └─ on Restart: factory() → replace cell handle with backoff
```

---

## 5. Agent 系统架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            AgentCell                                     │
│                   (axiom-agent / Layer 3)                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                 │
│  │   LLM       │    │   Tool      │    │   Memory    │                 │
│  │   Client    │    │  Registry   │    │  (Working)  │                 │
│  │ (axiom-llm) │    │(axiom-tool) │    │(axiom-memory)                │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘                 │
│         │                  │                   │                        │
│         │  ┌───────────────┼───────────────┐   │                        │
│         │  │               │               │   │                        │
│         ▼  ▼               ▼               ▼   ▼                        │
│  ┌─────────────────────────────────────────────────────┐               │
│  │                    Planner                          │               │
│  │                (axiom-planner)                      │               │
│  │  ┌─────────────────┐    ┌─────────────────┐        │               │
│  │  │ ReAct Strategy  │    │ PlanAndExecute  │        │               │
│  │  └─────────────────┘    └─────────────────┘        │               │
│  └─────────────────────────────────────────────────────┘               │
│                                                                         │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                 │
│  │  Identity   │    │   Prompt    │    │   Rules     │                 │
│  │ (axiom-     │    │ (axiom-     │    │  (Guard)    │                 │
│  │  identity)  │    │  prompt)    │    │             │                 │
│  └─────────────┘    └─────────────┘    └─────────────┘                 │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘

Identity 组成：
  AgentIdentity (id, name, system_prompt, traits, capabilities)
  └── AgentPersona ( manages Identity + Skills )
      └── Skill (activation_condition, tools, prompt_fragments)
          └── ActivationCondition (Always / Keyword / Context / Schedule / And / Or / Not)

MCP 桥接：
  McpClient → HTTP → 外部 MCP Server
  McpServer ← Axum ← 内部 Axiom Tool
  安全检查：Permission → Rules → Axiom → Human-in-the-loop
```

---

## 6. 数据流 / 事件溯源

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Runtime / Cell                                  │
│                                                                         │
│  Cell.handle(signal, ctx)                                               │
│       │                                                                  │
│       ▼                                                                  │
│  ctx.witness().emit(TransitionOutcome::Success)                         │
│       │                                                                  │
│       ▼                                                                  │
│  WitnessBuilder::build()                                                 │
│       │                                                                  │
│       ├─ prev_hash ───────────────────┐                                  │
│       ├─ state_hash_before            │                                  │
│       ├─ state_hash_after             │                                  │
│       ├─ signal_ref                   │                                  │
│       ├─ vector_clock                 │                                  │
│       ├─ correlation_id / trace_id    │                                  │
│       └─ version_info                 │                                  │
│            │                          │                                  │
│            ▼                          │                                  │
│  ┌──────────────────┐                 │                                  │
│  │  Witness Chain   │                 │                                  │
│  │  (SHA-256链)     │                 │                                  │
│  └────────┬─────────┘                 │                                  │
│           │                          │                                  │
│           ▼                          │                                  │
│  ┌──────────────────┐    ┌──────────▼──────────┐                       │
│  │   EventStore     │    │   SnapshotStore     │                       │
│  │ (axiom-store)    │    │ (每100事件触发)      │                       │
│  ├──────────────────┤    ├─────────────────────┤                       │
│  │ append(event)    │    │ save(cell_id, state)│                       │
│  │ read(aggregate)  │    │ load(cell_id)       │                       │
│  │ read_by_corr()   │    │ retention: 5个      │                       │
│  │ read_by_cell()   │    │ compression: snappy │                       │
│  │ subscribe()      │    │                     │                       │
│  └──────────────────┘    └─────────────────────┘                       │
│           │                                                              │
│           ▼                                                              │
│  ┌──────────────────┐                                                   │
│  │   DLQ            │                                                   │
│  │ (Dead Letter     │                                                   │
│  │  Queue, 容量1000)│                                                   │
│  └──────────────────┘                                                   │
└─────────────────────────────────────────────────────────────────────────┘

Witness 链验证：
  Witness::verify_chain_integrity(events)
    └─ 检查 prev_hash 是否形成有效 SHA-256 链
```

---

## 7. 监督模型

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Runtime 监督 (axiom-runtime)                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐    │
│  │   Supervisor    │    │ CircuitBreaker  │    │  EntropyGovernor │    │
│  │ (per-cell)      │    │ (Closed/Open/   │    │ (global + per-   │    │
│  │                 │    │  HalfOpen)      │    │  cell tracking)  │    │
│  │ 决策:           │    │                 │    │                 │    │
│  │  Restart        │    │ threshold: 5    │    │ 8种熵增事件      │    │
│  │  Stop           │    │ reset_after:    │    │ 时间衰减(300s)   │    │
│  │  Escalate       │    │  30s            │    │ 四色阈值:        │    │
│  │  CircuitBreak   │    │                 │    │  Green→Yellow→   │    │
│  │  max_retries: 3 │    │                 │    │  Red→Critical    │    │
│  │  backoff: 30s   │    │                 │    │ 治理动作:        │    │
│  └─────────────────┘    └─────────────────┘    │  None→Warn→      │    │
│                                                  │  Throttle→       │    │
│                                                  │  Emergency       │    │
│                                                  └─────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                      Layer 0 监督 (axiom-oversight)                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────┐        │
│  │                   OversightSupervisor                       │        │
│  │                 (聚合 8 个监督组件)                         │        │
│  ├─────────────────────────────────────────────────────────────┤        │
│  │  1. ArchitectureGuardianCell    — 架构合规                  │        │
│  │  2. EntropyGovernorCell         — 熵治理                     │        │
│  │  3. ResourceManagerCell         — 资源限制 (并发/内存/FD)   │        │
│  │  4. IntentAuditorCell           — 意图审计                   │        │
│  │  5. ComplianceGuardCell         — 合规守卫 (PII 检测)       │        │
│  │  6. MetaOversightCell           — 元监督 (监督监督者)        │        │
│  │  7. HealthCollectorCell         — 健康检查聚合               │        │
│  │  8. LoopDetector               — 循环检测                   │        │
│  │  9. StartupVerification         — 启动验证                   │        │
│  └─────────────────────────────────────────────────────────────┘        │
│                                                                         │
│  元监督：MetaOversightCell 监控其他监督组件自身状态                      │
│  如果监督组件崩溃 → 触发 Restart / Escalate                              │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 8. 拦截器链

```
Signal Envelope 进入 MessageBus
    │
    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Interceptor Chain (按顺序执行)                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  1. ArchitectureGuardian                                                 │
│     └─ 检查层转换合法性、跳数限制、Schema 版本兼容性                     │
│                                                                         │
│  2. ThrottleInterceptor                                                  │
│     └─ 熵值超过阈值时， hottest cell 消息率减半                          │
│                                                                         │
│  3. EmergencyInterceptor                                                 │
│     └─ Critical 熵值时停止新消息                                          │
│                                                                         │
│  4. HopLimitInterceptor                                                  │
│     └─ 最大 8 跳，超过则拒绝                                              │
│                                                                         │
│  5. IdempotencyInterceptor                                               │
│     └─ 基于 msg_id 去重                                                  │
│                                                                         │
│  6. SchemaVersionInterceptor                                             │
│     └─ Schema 版本为 0 直接拒绝                                          │
│                                                                         │
│  7. LoopDetectInterceptor                                                │
│     └─ 16 槽滑动窗口检测循环                                              │
│                                                                         │
│  8. CapabilityVersionInterceptor                                         │
│     └─ 检查 Cell 能力版本兼容性                                           │
│                                                                         │
│  9. GuardInterceptor                                                     │
│     └─ 执行自定义 Guard 检查                                             │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
    │
    ▼
RoutingTable → Target Cell(s)
```

---

## 9. 架构治理系统

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      架构治理系统                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────┐      │
│  │              .axiom/architecture.toml                         │      │
│  │  ┌─────────────────────────────────────────────────────────┐ │      │
│  │  │ [crate-layers]         — 18 个 crate 的分层              │ │      │
│  │  │ [forbidden-deps]       — async-trait 等禁止项            │ │      │
│  │  │ [audited-deps]         — 30 个审计通过的依赖              │ │      │
│  │  │ [dev-dependencies-audit] — dev-deps 检查开关              │ │      │
│  │  │ [proc-macro-exemptions] — axiom-macros → axiom-core      │ │      │
│  │  │ [reverse-dependency-   — axiom-agent → identity/prompt   │ │      │
│  │  │   exemptions]                                        │ │      │
│  │  └─────────────────────────────────────────────────────────┘ │      │
│  └───────────────────────────────────────────────────────────────┘      │
│                              │                                           │
│              ┌───────────────┼───────────────┐                          │
│              │               │               │                          │
│              ▼               ▼               ▼                          │
│  ┌─────────────────┐ ┌───────────────┐ ┌─────────────────┐            │
│  │   archcheck      │ │   gate.rs     │ │   build.rs      │            │
│  │   (CLI + lib)    │ │ (runtime API) │ │ (compile-time)  │            │
│  │                   │ │               │ │                 │            │
│  │  loader.rs       │ │ 从 TOML 加载  │ │ 调用 archcheck   │            │
│  │  checker.rs      │ │ 公共 API      │ │ ::build_hook     │            │
│  │  reporter.rs     │ │ 向后兼容      │ │ ::check_current  │            │
│  │  build_hook.rs   │ │               │ │ _crate()         │            │
│  └─────────────────┘ └───────────────┘ └─────────────────┘            │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────┐      │
│  │                    xtask (统一入口)                            │      │
│  │  gatecheck — 运行 archcheck                                    │      │
│  │  state     — 生成状态快照                                       │      │
│  └───────────────────────────────────────────────────────────────┘      │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────┐      │
│  │              .github/workflows/architecture-observer.yml       │      │
│  │  — 非阻塞 CI，每次 push/PR 自动运行                            │      │
│  └───────────────────────────────────────────────────────────────┘      │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘

编译期执行流程：
  cargo build
    ├── 解析 Cargo.toml
    ├── 执行 build.rs
    │   └── archcheck::build_hook::check_current_crate(env!("CARGO_PKG_NAME"))
    │       ├── 加载 .axiom/architecture.toml（OnceLock 缓存）
    │       ├── 检查 [dependencies] 内部依赖方向 + 第三方依赖
    │       ├── 检查 [build-dependencies]
    │       ├── 检查 [dev-dependencies]（如果启用）
    │       └── 违规 → panic() 阻断编译
    └── 编译 crate 源码
```

---

## 10. 核心设计模式

| 模式 | 说明 | 示例 |
|------|------|------|
| **编译期架构 Enforcement** | `CanSendTo` trait + sealed trait + build.rs 门禁 | `LayeredCellContext<L>` |
| **类型擦除 + 动态分发** | `DynCell`、`DynHandleCell`、`DynAxiom` | `Box<dyn DynCell>` |
| **分布式 slice 注册** | `linkme` 用于 Axiom、Lens、Migration 的静态注册 | `#[axiom]` 宏 |
| **拦截器链** | BusInterceptor 模式，支持动态注册和运行时 enforcement | `MessageBus::register_interceptor()` |
| **Erlang 监督树** | let it crash + 自动重启 + 熔断器 | `Supervisor::restart()` |
| **事件溯源 + 快照** | Witness 链 + EventStore + SnapshotStore | `FileStore::append()` |
| **渐进式披露** | DisclosureLevel + Skill 激活条件 + Lens 投影 | `DisclosureLevel::Full` |

---

## 11. 扩展点

| 扩展点 | 方式 | 示例 |
|--------|------|------|
| **自定义 Cell** | 实现 `Cell` trait | `#[cell(layer = "exec")]` |
| **自定义 Signal** | 使用 `#[signal]` 宏 | `#[signal(kind = "command")]` |
| **自定义 Axiom** | 使用 `#[axiom]` 宏 | `#[axiom(dim = "witness")]` |
| **自定义 Lens** | 使用 `#[lens]` 宏 | `#[lens(id = "...", depends_on = [...])]` |
| **自定义 Tool** | 使用 `#[tool]` 宏 | `#[tool(permission = "read")]` |
| **自定义 Guard** | 使用 `#[guard]` 宏 | `#[guard(layer = "all")]` |
| **自定义 Interceptor** | 实现 `BusInterceptor` trait | `bus.register_interceptor()` |
| **自定义 EventStore** | 实现 `EventStore` trait | `runtime.set_witness_store()` |
| **自定义 SnapshotStore** | 实现 `SnapshotStore` trait | `runtime.set_snapshot_store()` |

---

## 12. 系统能力总结

```
能力维度          状态    说明
─────────────────────────────────────────────────────────
编译期架构强制      ✅    18 个 crate 全部覆盖，违规直接 panic
分层依赖规则        ✅    9 层架构，单向依赖，零循环
禁止依赖拦截        ✅    async-trait 等硬性禁止
审计依赖管理        ✅    30 个 audited deps，未审计阻止引入
dev-deps 检查       ✅    可选启用，当前已启用
build-deps 检查     ✅    CLI + 编译期双重覆盖
自约束              ✅    archcheck 自身也受规则约束
向后兼容 API        ✅    gate.rs 公共 API 稳定
CI 集成             ✅    非阻塞观察者模式
事件溯源            ✅    Witness 链 + EventStore + Snapshot
监督树              ✅    Erlang 风格 let it crash + 熔断器
熵治理              ✅    四色阈值 + 治理动作 + 冷却机制
类型安全消息        ✅    Signal 类型 + Vector Clock
按需投影            ✅    Lens 增量缓存 + token 预算
Agent 配套          ✅    Identity + Skill + Rules + MCP
可视化              ✅    拓扑/时间轴/熵值导出
分布式              ✅    集群、节点发现、同步
```
