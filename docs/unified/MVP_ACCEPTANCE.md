# MVP 验收标准（最小可演示统一核）

**状态**：U0 草案  
**版本**：0.1.0  
**日期**：2026-07-19  
**关联**：[UNIFIED_MODEL.md](./UNIFIED_MODEL.md) · [DEPRECATION.md](./DEPRECATION.md)

> MVP 证明的是：**真统一已经发生**，不是“文档写了统一”。  
> 未通过本文件清单，不得进入商用件与多垂直业务并行扩张。

---

## 0. MVP 一句话目标

> **一个 Cell 内跑 Composer（四原语），全链路只有 Witness 历史，Governor 能熔断，端到端 demo 可演示。**

---

## 1. 范围内 / 范围外

### 1.1 必须在 MVP 内（In）

| # | 能力 |
|---|------|
| 1 | Rust 宿主 monorepo（可基于 `axiom-core` 演进） |
| 2 | U1 四原语 traits：`Atom` / `Port` / `Adapter` / `Composer` |
| 3 | Composer-in-Cell：Signal 触发 → 同步编排 → 结束 |
| 4 | 每次关键变迁写 **Witness 链**（可查询、可打印、可校验 prev_hash） |
| 5 | 至少一个真实 **Port**（例如时钟/HTTP mock/文件），禁止伪 Port |
| 6 | U2 至少 **2** 种韧性装饰：建议 `retry` + `circuit_breaker` |
| 7 | **Governor** 最小实现：熵超阈值 → 拒绝新 Signal 或打开熔断 |
| 8 | **一个** 垂直 demo（见 §3）端到端可跑 |
| 9 | 自动化测试：原语 + Witness 链完整性 + 熔断路径 |
| 10 | 文档：统一层图 + 本三件套链接；无联邦长期推荐 |

### 1.2 明确不在 MVP（Out）

| # | 能力 | 放到 |
|---|------|------|
| 1 | 完整 Workbench 写代码闭环 | U3 |
| 2 | MCP 全量 / 多 Agent 协作 | U3+ |
| 3 | LE arch-manager 全量 UI 迁移 | U4 |
| 4 | Go SDK | 二期 |
| 5 | 多租户 / 鉴权产品化 / 计费 | 商用阶段 |
| 6 | 分布式多节点共识 | 后置 |
| 7 | 全量 LE 测试 1:1 搬迁 | 渐进 |
| 8 | WASM 插件生态完善 | 后置 |

---

## 2. 成功标准（可判定）

### 2.1 架构标准（宪法合规）

| ID | 标准 | 验证方法 |
|----|------|----------|
| A1 | 仅一套对外层图 U0–U7 | 主文档审查 |
| A2 | 无 ExecutionStep 权威写入 | 代码搜索 + CI |
| A3 | 无第二 runtime 决策路径 | 代码审查 |
| A4 | 业务 demo 只用四原语表达 | demo 源码审查 |
| A5 | 跨单元若存在，仅 Signal | demo / 测试 |

### 2.2 功能标准

| ID | 标准 | 验证方法 |
|----|------|----------|
| F1 | demo 一条命令可启动并跑完主路径 | `cargo run -p …` 或 xtask |
| F2 | 主路径产生 **≥ N 条** Witness（N 由 demo 定义，建议 ≥ 5） | 断言 / CLI 导出 |
| F3 | Witness 链 `prev_hash` 可校验无断裂 | 单测 |
| F4 | 注入 Port 连续失败 → circuit 打开 → 后续快速失败 | 单测 |
| F5 | 注入违规或推高熵 → Governor 拒绝/熔断 | 单测 |
| F6 | 熔断恢复条件可配置且可测（时间或手动 reset） | 单测 |

### 2.3 工程标准

| ID | 标准 | 验证方法 |
|----|------|----------|
| E1 | `cargo test -p <mvp-crates>` 绿 | CI |
| E2 | clippy（mvp 相关 pack）无新增 deny | CI |
| E3 | 无 `legacy/` 新增依赖（基线锁定） | CI 计数 |
| E4 | README 含 5 分钟上手：clone → run demo → 看 Witness | 人工走查 |

### 2.4 体验标准（演示用）

| ID | 标准 |
|----|------|
| X1 | 演示者 10 分钟内讲清：四原语 + Cell + Witness + 熔断 |
| X2 | 观众不需要对照 LE/Axiom 映射表 |
| X3 | 故意搞坏一次（失败注入）能展示治理，而不是只展示 happy path |

---

## 3. 垂直 Demo 候选（选 1 个做死）

> 只做 **一个**。做深，不做三个半吊子。

### 选项 V1 — 任务流水线（推荐默认）

**故事**：提交任务 → 校验 → 执行步骤 → 写结果 → 可回放。

| 原语用法 | 例子 |
|----------|------|
| Atom | 校验任务字段、计算优先级 |
| Adapter | 外部 JSON → 内部 Task |
| Port | 持久化结果（内存 store 可）、通知 |
| Composer | `Validate → Plan → Execute → Persist` |
| Cell | `TaskCell` 收 `SubmitTask` Signal |
| Witness | 每步状态 |
| Governor | 错误率过高熔断 Execute Port |

**为何适合**：概念全、依赖少、好测。

### 选项 V2 — 简易下单风控

**故事**：下单请求 → 风控规则 → 限流 → 模拟成交。

| 原语用法 | 例子 |
|----------|------|
| Atom | 风控规则纯函数 |
| Port | 行情/账本 mock |
| Composer | 风控管线 |
| Governor | 异常单突增熔断 |

**为何适合**：更“业务”；略增域复杂度。

### 选项 V3 — 文档处理小 macro

**故事**：收文件 → 解析 → 摘要（可 mock LLM Port）→ 存档。

**为何适合**：贴近 Agent；易滑向 LLM 范围膨胀，MVP 需强约束 mock。

---

**默认推荐：V1 任务流水线。**  
你确认后把本节未选选项标为 Out。

---

## 4. 目录与 crate 草图（实施时对齐，可微调）

目标形态（名称可改，职责不可糊）：

```
axiom-core/   (或 ule/)
  crates/
    axiom-kernel/          # U0 + 可放 ISA 最小 traits
    axiom-isa/             # 可选：四原语与测试（若不想挤爆 kernel）
    axiom-resilience/      # U2 retry/circuit…
    axiom-runtime/         # U3 Cell 调度
    axiom-governor/        # 或 oversight 内最小 Governor
    axiom-demo-taskflow/   # MVP 垂直 demo
  docs/
    unified/               # 本三件套（或链到 architecture/unified）
```

**禁止**：新建 `low-entropy-runtime` 对等 crate。

---

## 5. 验收仪式（Definition of Done）

全部勾选才算 MVP 通过：

### 5.1 演示脚本（必须可重复）

```text
1. 启动 demo
2. 提交 1 个成功任务 → 展示 Witness 链
3. 提交失败注入（Port 错误）→ 展示 retry/circuit
4. 推高熵或连续违规 → 展示 Governor 拒绝
5. 导出/打印 Witness，现场校验哈希链
6. 打开文档，指出 U 层与灭双状态（D-01 已灭）
```

### 5.2 签字清单

| 检查项 | 通过 |
|--------|------|
| A1–A5 架构标准 | ☐ |
| F1–F6 功能标准 | ☐ |
| E1–E4 工程标准 | ☐ |
| X1–X3 体验标准 | ☐ |
| D-01 已灭（无 Step 权威） | ☐ |
| D-05 已灭（单层图） | ☐ |
| 范围外需求未偷塞进 MVP | ☐ |

### 5.3 失败条件（一票否决）

- 仍需“LE 词表 ↔ Axiom 词表”才能讲懂 demo  
- Witness 与 Step 双写  
- demo 主要逻辑在 Cell 外裸跑、或 Go 第二进程当核  
- 只有 happy path，没有熔断演示  

---

## 6. 通过后立刻做什么 / 不做什么

### 做

1. 冻结 MVP crate API 的最小稳定面  
2. 进入 U2/U3：补韧性全集、Handoff/Workbench  
3. 按灭双清单推进 D-02 / D-10 / D-11  

### 不做

1. 立刻开多租户与商业化包装  
2. 并行第二个垂直大业务  
3. 复活联邦桥  

---

## 7. 待你拍板的 3 个开关

| # | 问题 | 建议 |
|---|------|------|
| 1 | 垂直 demo 选 V1 / V2 / V3？ | **V1** |
| 2 | monorepo 宿主用现有 `axiom-core` 还是新仓 `ule`？ | **现有 axiom-core 演进**（少搬迁税） |
| 3 | MVP 是否包含最小 HTTP API 还是 CLI-only？ | **CLI-only**（更快闭环） |

拍板后，U0 关闭，进入 U1 骨架实现。

---

## 8. 估算（与评估报告对齐）

| 项 | 量级 |
|----|------|
| 达到本 MVP | 约 **3–6 人月** 量级中的前半（视人力 **数周到 2+ 月**） |
| 未含 | 商用硬化、全量迁移、完整 UI |

---

**MVP 通过的社会含义：**

> 你已经拥有「可演示的单一内核」；  
> 剩下是迁移与产品化，而不是继续争论两套架构谁说了算。
