# 智能体工作指导与约束

| Field | Value |
|-------|--------|
| **Status** | Normative for AI/coding agents working on this monorepo |
| **Audience** | 编码智能体、编排 Agent、人工 reviewer |
| **Host** | `axiom-core`（ULE-on-Axiom / Rust） |
| **Constitution** | [`../unified/UNIFIED_MODEL.md`](../unified/UNIFIED_MODEL.md) |
| **Theme matrix** | [`../unified/FEATURE_THEME_MATRIX.md`](../unified/FEATURE_THEME_MATRIX.md) |
| **Reference path** | `crates/axiom-demo-taskflow/` · `crates/axiom-isa/` |

> **本文是智能体在本仓库写代码、改架构、交 PR 时的强制行为规范。**  
> 与 [`UNIFIED_MODEL.md`](../unified/UNIFIED_MODEL.md) 冲突时，以宪法为准；与本文冲突的「图省事实现」一律视为违规。

---

## 0. 一句话

> 用 **Signal** 触发 **Cell**，在 Cell 内用 **Composer** 串 **Atom / Adapter / Port**，用 **Witness** 记史，用 **Governor** 熔断。  
> HTTP、CLI、LLM、MCP、Workbench **都只是入口**，不是第二套业务内核。

---

## 1. 硬约束（MUST — 违反即拒收）

### 1.1 宪法级（L1–L10 摘要）

| ID | 约束 | 智能体禁止行为 |
|----|------|----------------|
| **C1** | 唯一宿主 = Rust / Axiom（`axiom-kernel` + `axiom-runtime`） | 复活 Go LE 为对等 runtime；新建第二调度核 |
| **C2** | 唯一历史 = **Witness** 哈希链 | 新建权威 `ExecutionStep` 库；双写两套真相 |
| **C3** | 唯一熵/准入决策 = **Governor** | 自造 Guardian / 旁路 `if entropy` 停机；业务覆盖 Critical |
| **C4** | 业务形态只能是 **Atom / Port / Adapter / Composer** | 大号 `*Service` 堆 IO + 规则；随意 util 业务核 |
| **C5** | 运行单元 = **Cell**；跨单元 = **Signal** | 跨 Cell 共享 `&mut` 状态；裸 channel 当总线 |
| **C6** | **Composer 只在 Cell 内同步执行** | Cell 外 `PipelineRuntime`；后台隐式编排第二引擎 |
| **C7** | 依赖单向（层门禁 / `architecture.toml`） | 下层 import 上层；循环依赖 |
| **C8** | 副作用必须经 **Port**；Atom 必须纯 | Atom 内 `reqwest` / `sqlx` / `std::fs` / `Command` |
| **C9** | SDK ≠ 第二内核 | 在 Go/TS 客户端实现 admit / Witness 权威 |
| **C10** | 灭双优先于堆功能 | 为新功能同时维护 LE+Axiom 双路径 |

### 1.2 产品 API 级

| ID | 约束 |
|----|------|
| **P1** | 商业路径准入只调用 `axiom_isa::product_admit` 或 `Governor::admit`（及 `product_decide`） |
| **P2** | ISA 步骤写历史只用 `WitnessJournal` + `run_atom` / `run_port` / `run_adapter` |
| **P3** | Handoff 只能是 **Signal 载荷**（`HandoffRequest`），不是第二消息系统 |
| **P4** | Workbench **必须受控**（allow-list + limits）；禁止无沙箱任意 shell/写盘 |
| **P5** | 工具 / LLM / 交易所 / DB 调用在语义上必须是 **Port**，并进入 Witness |
| **P6** | 韧性（retry/circuit/rate/bulkhead）用 `axiom-resilience`，包在 Port 或 Composer 边界，禁止复制三套 |

### 1.3 工程级

| ID | 约束 |
|----|------|
| **E1** | 新能力要有 **非 test 生产调用者** 或明确 demo/host 路径（禁止只写死 API） |
| **E2** | 主路径必须有 **路径测试**（仿 `*_path.rs`），失败能暴露断线 |
| **E3** | 商业 Composer 源码须能过 ISA discipline（见 `axiom_isa::discipline`） |
| **E4** | 不提交密钥、生产连接串、真实交易所密钥到仓库 |
| **E5** | 不扩大 scope 到宪法 Out 项并声称「主题未完成」（多租户计费、跨区 HA、无限制 LLM 等） |

---

## 2. 应该怎么做（SHOULD）

### 2.1 标准执行路径（实现任何用例时对齐）

```text
Signal 进入 Cell
  → Guard / 层校验（若适用）
  → product_admit(Governor)
  → 解析 payload（Adapter 或 serde）
  → Composer:
       run_atom  …
       run_port  …   // 唯一 IO
       run_adapter …
  → Witness 链完整
  → 可选：回包 Signal / 更新 SharedOutcome / 推 Surface metrics
```

### 2.2 原语选型速查

| 你要做的事 | 用 | 不要用 |
|------------|-----|--------|
| 校验、计价、状态机转移、指标公式 | **Atom** | Port |
| HTTP/DB/WS/文件/交易所/LLM 网络 | **Port** | Atom |
| DTO ↔ 领域模型、交易所 JSON ↔ 内部类型 | **Adapter** | 散落 mapper 复制三份 |
| 多步骤用例编排 | **Composer**（Cell 内） | 全局 Service 单例隐式流程 |
| 跨模块通知 | **Signal** | 直接调另一个 Cell 的私有方法 |
| 审计 / 追责 | **Witness** | 仅 tracing 日志 |
| 全局停机 / 拒新单 | **Governor** | 各模块私自 `AtomicBool` 停写 |

### 2.3 推荐落码顺序（编码 Agent）

1. **读** 宪法 + 本文件 + 最近邻参考实现  
2. **写** 领域类型 + Atom + 单元测试（零 IO）  
3. **写** Port trait + **Mock Port** + Composer 测试  
4. **写** Cell：`admit → compose → witnesses`  
5. **写** Host：注册 Cell、`start`、发 Signal  
6. **写** `tests/*_path.rs` 路径测试  
7. **可选** 薄 HTTP：只转 Signal，业务不进 handler  
8. **跑** `cargo test -p <crate>` 与相关 demo  

### 2.4 文件与 crate 建议

```text
crates/<feature>/
  src/
    domain.rs       # 类型，无 IO
    atoms.rs
    ports.rs        # trait + real + mock
    adapters.rs
    composers.rs
    cells/*.rs
    host.rs
  tests/
    *_path.rs       # 端到端路径
```

依赖只指向下层：`axiom-isa` / `axiom-kernel` / `axiom-runtime` / `axiom-resilience` / `axiom-store`。

### 2.5 观测

- 健康与治理快照走统一 Surface 思路（参考 `axiom-demo-taskflow` surface）  
- 计数器用产品 metrics 或 runtime health，不自建第二套「真相仪表盘」  
- 只读投影用 **Lens**，禁止跨 Cell 读私有状态  

---

## 3. 明确禁止（MUST NOT）

| # | 禁止 | 原因 |
|---|------|------|
| 1 | 在 Atom 内做 IO | 破坏纯性与可测性（C8） |
| 2 | 在 axum/actix handler 写完整业务编排 | 第二内核；应转 Signal |
| 3 | 绕过 `product_admit` 直接 `run_port` 下单/改生产 | 风控与熔断失效 |
| 4 | Agent/Workbench 无 allow-list 执行 shell | 安全与宪法 Workbench 定义 |
| 5 | 新增 `ExecutionStepStore` 当权威历史 | 双历史 |
| 6 | 为图快复制粘贴三套 retry/circuit | 违 U2 单一标准库 |
| 7 | 下层 crate 依赖 demo/应用 crate | 违层门禁 |
| 8 | 提交 `target/`、密钥、巨型二进制 | 工程卫生 |
| 9 | 未路径测试就声称「生产就绪」 | E1/E2 |
| 10 | 把「未做 SaaS 计费」写成架构失败 | 范围膨胀 |

---

## 4. 多智能体协作约束

### 4.1 分工（推荐）

| 角色 | 可写 | 不可写 |
|------|------|--------|
| Architect | 设计表、Cell/Signal 图、本文合规审查 | 随意改宪法名词 |
| Domain | Atom、状态机、domain 单测 | Port 真 IO、乱加 crate 层 |
| Integration | Port/Adapter/mock | 改 Governor 语义 |
| Runtime | Cell、host、路径测试 | 业务规则藏进 dispatch |
| API | HTTP↔Signal、鉴权 | 业务 Composer 放进路由 |
| Reviewer | 只读检查 §1/§3/§7 | — |

### 4.2 交接协议

- 任务交接使用 **`HandoffRequest` 语义**（哪怕人工 PR 描述也按此结构）：  
  `token` · `source_agent` · `target_agent` · `intent` · `payload` · `permissions`  
- `intent` 必须在 allow-list 内（若走 Workbench）  
- 交付物必须包含：**如何测试** + **触及的原语表**

### 4.3 冲突解决

1. 宪法 > 本指导 > 局部 README > 临时对话  
2. 双权威争议 → **默认拒绝**，先灭双再加功能  
3. 不确定是否 Port → **当 Port**（保守，可审）  

---

## 5. 给编码智能体的系统提示词（可复制）

```text
你在 GitHub monorepo axiom-core（ULE-on-Axiom）上工作。

强制阅读：
- docs/unified/UNIFIED_MODEL.md
- docs/guide/agent-work-guide.md
- 参考实现 crates/axiom-demo-taskflow、crates/axiom-isa

硬约束：
1. 业务只用 Atom/Port/Adapter/Composer；IO 只在 Port；经 run_atom/run_port/run_adapter。
2. Composer 只在 Cell 内同步执行；跨单元只用 Signal。
3. 准入只用 product_admit/product_decide（Governor）。
4. 历史只用 WitnessJournal；禁止第二套权威历史。
5. Workbench/Agent 工具调用必须受控，禁止无沙箱任意命令。
6. 先 Atom 单测 → Mock Port Composer → Cell 路径测试 → 再接线。
7. HTTP 只做 Signal 适配；不把业务写进 handler。
8. 不把多租户计费/跨区 HA/无限制 LLM 塞进本次「架构完成」范围。

交付：
- 原语映射表（步骤 → Atom|Port|Adapter|Composer）
- 代码 + tests/*_path.rs
- cargo test / cargo run 验证命令
- 自检 §7 checklist 全部勾选说明
```

---

## 6. 后端用例模板（智能体填空）

复制并填完再写代码：

```markdown
## 场景
<一句话>

## Signal
- name:
- payload 类型:

## Cell
- id:
- tier:

## Composer 步骤
| # | 步骤 | 原语 | 名称 |
|---|------|------|------|
| 1 | | Atom/Port/Adapter | |
| 2 | | | |

## Ports（外部 IO）
- 

## Governor
- [ ] product_admit 在 compose 前
- 拒绝时行为:

## 测试
- 单测:
- 路径测试:
- 失败注入:
```

---

## 7. PR / 任务完成自检清单

智能体在声称「完成」前必须能够回答 **是**：

### 架构

- [ ] 无第二 runtime / 第二历史权威 / 第二 admit 出口  
- [ ] 无 Atom 内 IO  
- [ ] Composer 仅在 Cell 内  
- [ ] 跨单元仅 Signal  

### 实现

- [ ] 使用 `run_atom` / `run_port`（及需要的 `run_adapter`）  
- [ ] `product_admit`（或等价唯一 API）在业务主路径上  
- [ ] Witness 链可验证（或路径测试覆盖）  
- [ ] 韧性装饰未手写三套  

### 工程

- [ ] 存在路径测试或 demo 命令  
- [ ] `cargo test -p <相关 crate>` 可通过  
- [ ] 无密钥提交  
- [ ] 未无故扩大 Out-of-scope 范围  

### 文档

- [ ] 若新增一等公民名词 → 已提议改宪法（不得偷加）  
- [ ] README/指导中的运行命令仍准确（若你改了 CLI）  

---

## 8. 范围边界（智能体勿膨胀）

以下 **不是** 默认任务成功条件（除非人类明确立项）：

| 项 | 说明 |
|----|------|
| 多租户 / 计费 / 计量 | SaaS 运营 |
| 跨区 HA / 共识 | 部署拓扑 |
| 无限制 LLM 写系统 | 违受控 Workbench |
| LE Go 全量 1:1 重写 | 归档资产对照即可 |
| `cargo test --workspace` 历史全绿 | 工程债，非主题门禁 |
| TradingView 级完整前端 | 前端独立工程 |

详见 [`../unified/FEATURE_THEME_MATRIX.md`](../unified/FEATURE_THEME_MATRIX.md) §5。

---

## 9. 权威参考路径

| 目的 | 路径 |
|------|------|
| 宪法 | `docs/unified/UNIFIED_MODEL.md` |
| 主题是否满足 | `docs/unified/FEATURE_THEME_MATRIX.md` |
| 商用交付 | `docs/unified/COMMERCIAL_DELIVERY.md` |
| 四原语 + Governor + Handoff | `crates/axiom-isa/` |
| 韧性 | `crates/axiom-resilience/` |
| 可运行商业路径 | `crates/axiom-demo-taskflow/` |
| 运行时 | `crates/axiom-runtime/` |
| 内核原语 | `crates/axiom-kernel/` |
| ISA 纪律扫描 | `crates/axiom-isa/src/discipline.rs` |
| 工程硬化 | `docs/ENGINEERING_HARDENING_v050.md` |

### 验证命令（回归基线）

```powershell
cargo test -p axiom-isa -p axiom-resilience -p axiom-demo-taskflow
cargo run -p axiom-demo-taskflow -- success
cargo run -p axiom-demo-taskflow -- handoff
cargo run -p axiom-demo-taskflow -- surface
cargo run -p axiom-demo-taskflow -- plugin
```

---

## 10. 违规处理（给 Reviewer / 编排器）

| 级别 | 例 | 动作 |
|------|-----|------|
| **Blocker** | Atom 含 IO；双历史；绕过 Governor 下单 | 拒绝合并；责令重写 |
| **Major** | Composer 在 Cell 外；无路径测试 | 拒绝或要求补齐后合并 |
| **Minor** | 命名偏离词汇表；文档链接旧 | 可先合后补，记债 |
| **Scope creep** | PR 夹带计费/HA「顺便做」 | 拆 PR；移出本次 |

---

## 11. 修订

- 本文随 ULE 宪法演进；**新增一等公民名词不得只改本文不改宪法**。  
- 智能体可建议修订，但 **不得自行发明与宪法冲突的长期例外**。  

**生效**：写入本仓库后，所有在本 monorepo 上作业的编码/编排智能体视为已知悉。
