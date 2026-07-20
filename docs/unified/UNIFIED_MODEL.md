# 统一内核宪法（UNIFIED MODEL）

**状态**：宪法有效 · **产品地板初心 100% 达成**（见 [INTENT_COMPLETE.md](./INTENT_COMPLETE.md)）  
**版本**：0.5.0（对齐 commercial floor；本文条款自 0.1 冻结起未破坏）  
**日期**：2026-07-20  
**宿主**：Axiom（Rust）  
**收编方**：low-entropy-core（业务 ISA / 标准库 / 流程）— 现为归档只读资产  
**目标产品名**：ULE — Unified Low-Entropy Kernel  

> 本文是大一统的**唯一词汇与边界真相源**。  
> 与本文冲突的实现、文档、桥接、双写，一律视为违规，进入 [DEPRECATION.md](./DEPRECATION.md)。  
> 产品地板严格验收：[INTENT_COMPLETE.md](./INTENT_COMPLETE.md) · MVP：[MVP_ACCEPTANCE.md](./MVP_ACCEPTANCE.md)。

---

## 0. 宪法一句话

> **一个内核、一套词、一种历史、一种熵、一份层图。**  
> Axiom 提供运行时与门禁；LE 的四原语与韧性成为其上的业务 ISA 与标准库。  
> 不存在对等双核，不存在长期联邦桥。

---

## 1. 不可谈判的铁律（MUST）

| ID | 铁律 | 违反即 |
|----|------|--------|
| L1 | **唯一宿主语言内核为 Rust（axiom-kernel + axiom-runtime）** | 架构违规 |
| L2 | **唯一执行历史为 Witness 哈希链**；ExecutionStep 不得独立权威 | 架构违规 |
| L3 | **唯一熵权威为一个 Governor**（合并 LE Guardian + Axiom Entropy/Oversight） | 架构违规 |
| L4 | **业务形态只能是四原语**：Atom / Port / Adapter / Composer | 架构违规 |
| L5 | **运行单元只能是 Cell**；跨单元通信只能是 Signal | 架构违规 |
| L6 | **Composer 只在 Cell 内同步执行**；不另立 Pipeline Runtime | 架构违规 |
| L7 | **依赖方向单向**：上层可调用下层，禁止反向与循环 | 编译期/CI 失败 |
| L8 | **副作用必须经 Port**；Atom 必须纯（同输入同输出、无 I/O） | 架构违规 |
| L9 | **SDK ≠ 第二内核**：Go/TS SDK 只能调用统一 API，不得自带 runtime 决策 | 架构违规 |
| L10 | **灭双优先于新功能**：迁移期禁止两边分叉加新能力 | 流程违规 |

---

## 2. 统一词汇表（唯一合法名词）

### 2.1 业务 ISA（来自 LE，升格为正式层）

| 名词 | 定义 | 非法近义词（废弃） |
|------|------|-------------------|
| **Atom** | 纯计算：`In → Out`，无 I/O、无时钟、无随机（除非注入） | Service、Helper、Util 业务逻辑 |
| **Port** | 外部边界：网络/DB/文件系统/MQ/LLM 调用出口 | 直接在 Atom 里写 HTTP |
| **Adapter** | 协议/形状转换：DTO↔领域、外部 schema↔内部 schema | 到处散落的 mapper |
| **Composer** | 编排：组合 Atom/Port/Adapter 及其他 Composer | 随意 service 串联、第二套 workflow 引擎 |

### 2.2 运行时内核（来自 Axiom，唯一）

| 名词 | 定义 | 非法近义词（废弃） |
|------|------|-------------------|
| **Cell** | 有邮箱、状态、监督关系的运行单元 | 独立 LE goroutine 业务运行时 |
| **Signal** | 带因果（VectorClock）的跨 Cell 消息 | 裸 channel、双写事件总线 |
| **Axiom** | 可编译/可运行校验的硬约束 | 仅文档约束、仅 lint 建议 |
| **Witness** | 每次状态变迁的不可变审计记录（哈希链） | 独立 ExecutionStep 主库 |
| **Guard** | 拦截器：入站/出站检查 | 散落 if 校验 |
| **Lens** | 状态投影/只读视图 | 直接读别的 Cell 内部状态 |
| **Entropy** | 可度量无序度；驱动告警与熔断 | 多套互不兼容的“健康分” |

### 2.3 治理与 Agent

| 名词 | 定义 |
|------|------|
| **Governor** | 唯一熵决策与熔断权威（吸收 Guardian + Oversight 决策面） |
| **Workbench** | Agent 写代码/改系统的受控工作台（挂在 Agent 层，产出仍回 Witness） |
| **Handoff** | 任务交接协议；实现为 **Signal 载荷标准**，不是第二消息系统 |
| **Skill / MCP / Tool** | Agent 能力面；Tool 调用必须经 Port 语义 + Witness |

### 2.4 禁止并存的“双词”

| 旧双词 | 统一后 |
|--------|--------|
| ExecutionStep **与** Witness | **Witness 唯一**；Step 字段降为 Witness 扩展/视图 |
| Guardian **与** Oversight/Entropy 双决策 | **Governor 唯一** |
| LE L0–L7 **与** Axiom Crate Layer 对外双叙事 | **对外一张楼（见 §3）** |
| LE EventStore **与** axiom-store 双主 | **统一 Store**，Witness 为执行史权威 |
| 联邦 Bridge 长期态 | **禁止**（迁移窗口除外，见灭双清单） |

---

## 3. 统一分层（对外只讲这一张）

对外产品与文档 **只使用** 下表。实现可映射到 crate/文件，但叙事不得再讲两套楼。

```
┌──────────────────────────────────────────────────────────┐
│ U7  应用 / CLI / Dashboard / 示例                         │
├──────────────────────────────────────────────────────────┤
│ U6  Agent 面（Identity / Workbench / MCP / Handoff）      │
├──────────────────────────────────────────────────────────┤
│ U5  治理（Governor / Alert / 策略表）                      │
├──────────────────────────────────────────────────────────┤
│ U4  编排与计划（Composer 标准库 / Planner）                │
├──────────────────────────────────────────────────────────┤
│ U3  运行时（Cell / Signal / Supervisor / 调度）            │
├──────────────────────────────────────────────────────────┤
│ U2  韧性标准库（Retry / Circuit / Bulkhead / …）           │
├──────────────────────────────────────────────────────────┤
│ U1  业务 ISA（Atom / Port / Adapter / Composer traits）    │
├──────────────────────────────────────────────────────────┤
│ U0  内核原语（Witness / Axiom / Guard / Lens / ID / 熵度量）│
└──────────────────────────────────────────────────────────┘
```

### 3.1 依赖铁律

- **U_n 只能依赖 U_m（m ≤ n 的下层能力）**；禁止上层实现被下层 import。  
- 编译期：延续 Axiom crate layer + `architecture.toml` 门禁。  
- 业务代码：新逻辑必须落在 U1–U2 原语组合，不得在 U7 堆业务。

### 3.2 与旧分层的映射（仅迁移用，不对外教学）

| 统一层 | LE 旧层 | Axiom 旧位置 |
|--------|---------|--------------|
| U0 | L0 错误/基础 | axiom-kernel（witness/axiom/id/entropy…） |
| U1 | L1 四原语 | **新建** `axiom-kernel` 或 `axiom-isa` 模块 |
| U2 | L2 单机韧性 | kernel/runtime 标准库模块 |
| U3 | （弱） | axiom-runtime + Cell/Signal |
| U4 | Composer/Pipeline | planner + composer 库 |
| U5 | L4 Guardian + L5 Observation | oversight + alert + entropy governor |
| U6 | Workbench / Handoff | axiom-agent / mcp / identity |
| U7 | cmd + arch-manager | cli + viz |

Runtime Tier（Oversight → Agent → Validate → Exec）**保留为 Cell 间发送约束**，与 U 层正交：U 层管“代码放哪”，Runtime Tier 管“谁能给谁发 Signal”。

---

## 4. 执行语义（大一统运行模型）

### 4.1 标准路径（唯一合法）

```
Signal 进入 Cell
    → Guard / Axiom 校验
    → Cell 内同步执行 Composer（组合 Atom / Port / Adapter）
    → 每一步状态变迁 append Witness（哈希链）
    → 指标进入 Entropy
    → Governor 可熔断 / 限流 / 拒绝后续 Signal
    → 可选：产出对外 Signal / Lens 投影
```

### 4.2 硬性规定

1. **Composer 不跨 Cell 隐式共享可变状态**；跨 Cell 只许 Signal。  
2. **Port 调用**必须可被 Witness 记录（至少：port 名、入参摘要哈希、结果状态、耗时）。  
3. **Atom** 失败以类型化错误返回；不得吞错。  
4. **重试/熔断等韧性**是 U2 装饰器，包在 Port 或 Composer 边界，不得复制三套。  
5. **Handoff** = 结构化 Signal payload（任务意图、上下文引用、权限、截止时间），进入目标 Cell 后仍走 §4.1。

### 4.3 Witness 为唯一历史

Witness 最低字段（宪法级，实现可扩展）：

| 字段 | 含义 |
|------|------|
| `witness_id` | 唯一 ID |
| `cell_id` | 发生单元 |
| `correlation_id` / `trace_id` | 关联与追踪 |
| `timestamp_ns` | 时间 |
| `prev_hash` / `hash` | 链 |
| `kind` / `event` | 变迁类型与载荷 |
| `state_before_hash` / `state_after_hash` | 状态摘要 |

**ExecutionStep 兼容策略**：若需保留 LE 习惯字段（step 名、输入输出摘要、父子 span），一律作为 `WitnessEvent` 的变体或扩展属性，**禁止**第二套 append-only 主存储。

---

## 5. 熵与熔断（唯一 Governor）

### 5.1 度量

熵为可计算标量（可多维，但**决策出口只有一个 Governor**）：

- 输入信号示例：Axiom 违规、Witness 异常、消息丢弃、重复、超时、熔断触发、Cell 重启、意图漂移。  
- 级别：Green / Yellow / Red / Critical（阈值可配置，默认对齐 Axiom 现网常量思路）。

### 5.2 决策权

| 角色 | 可做 | 不可做 |
|------|------|--------|
| 指标采集 | 各模块上报 | 自行停机 |
| **Governor** | 告警、限流、拒绝 Signal、熔断 Port、请求监督重启 | 被业务旁路 |
| 业务 Composer | 本地重试策略（U2） | 覆盖全局 Red/Critical 决策 |

LE 的 Guardian 规则与 Axiom Oversight **迁入 Governor 策略表**，不保留双决策器长期并行。

---

## 6. 语言与仓库边界

### 6.1 语言

| 层级 | 语言 | 说明 |
|------|------|------|
| 内核 + 标准库 + 默认业务 API | **Rust** | 唯一实现真相 |
| 外部集成 SDK | Go / TS 等（二期） | 纯客户端，无第二 runtime |
| Workbench 用户代码 | 受控沙箱（优先 WASM；subprocess 为例外） | 结果必须回 Witness |

### 6.2 仓库目标态

- **一个 monorepo**（可暂以 `axiom-core` 为宿主仓演进，或新建 `ule`）。  
- `low-entropy-core`：迁移窗口内为 **只读资产源**，迁完归档。  
- 禁止长期 “双仓 + 同步脚本” 当产品形态。

### 6.3 门禁

统一至少包含：

1. `architecture.toml`（或后继）层依赖  
2. clippy / rustfmt / 测试  
3. **禁止 API**：第二历史写入、第二熵决策入口、Cell 外 Composer runtime  
4. CI：`archcheck` 类违规必须为空

---

## 7. 能力归属（谁拥有什么）

| 能力 | 最终归属 | 来源 |
|------|----------|------|
| Cell / Signal / Witness / Axiom / Guard / Lens | U0–U3 内核 | Axiom |
| Atom / Port / Adapter / Composer | U1 | LE 语义 → Rust 实现 |
| Retry / Circuit / Bulkhead / RateLimit / … | U2 | LE 模式 → Rust 实现 |
| Governor / Alert | U5 | 双方合并 |
| Workbench / Handoff | U6 | LE 叙事 → 挂 Axiom Agent |
| MCP / Identity / Prompt | U6 | Axiom |
| Store / Replay | U0/U3 旁路存储 | axiom-store 统一 |
| Dashboard | U7 | 单一 API；UI 可重做壳 |

---

## 8. 变更程序（如何改宪法）

1. 任何新增“一等公民”名词 → 必须改本文并升版本。  
2. 任何双写/双权威提案 → 默认拒绝，除非给出灭双日期。  
3. MVP 未完成前，不扩展商用运营件（多租户/计费/跨区 HA）进内核宪法。  
4. 本文 0.x 期间允许修订；**1.0** 起破坏性变更需迁移指南。

---

## 9. 明确不在宪法内（防止范围膨胀）

以下 **不是** U0–MVP 的成功条件：

- 完整 Temporal 级 durable timer / 多集群自动转移  
- 全量 LE HTML 审计页迁入  
- 多语言 SDK 齐全  
- 生产级多租户与计费  
- 把历史全部 Go 测试 1:1 机器翻译而不重建语义测试  

这些可以进路线图，但 **不得阻塞** 宪法冻结与 MVP。

---

## 10. 签署栏（决策冻结）

| 项 | 选择 | 状态 |
|----|------|------|
| 大一统 vs 联邦 | **大一统** | 已定 |
| 宿主 | **Axiom / Rust** | 已定 |
| 历史权威 | **Witness** | 已定 |
| 业务 ISA | **四原语** | 已定 |
| 编排位置 | **Composer-in-Cell** | 已定 |
| 熵权威 | **单一 Governor** | 已定 |
| 业务默认语言 | **Rust 优先；Go SDK 二期** | 已定 |
| MVP 垂直场景 | 见 MVP_ACCEPTANCE | 待你确认场景名 |

---

**本文生效后，工程上的下一动作只有：**  
按 [DEPRECATION.md](./DEPRECATION.md) 执行灭双，并按 [MVP_ACCEPTANCE.md](./MVP_ACCEPTANCE.md) 做出可演示统一核。
