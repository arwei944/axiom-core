# 灭双清单（DEPRECATION）

**状态**：U5 叙事冻结（U3–U5 已交付）  
**版本**：0.5.0  
**日期**：2026-07-19  
**关联**：[UNIFIED_MODEL.md](./UNIFIED_MODEL.md) · [COMMERCIAL_DELIVERY.md](./COMMERCIAL_DELIVERY.md) · LE [ARCHIVED.md](../low-entropy-core/ARCHIVED.md)

> **灭双** = 消灭双词、双历史、双熵、双运行时、双层图、双真相源。  
> 没有灭双日期的“兼容层”一律视为假统一。

---

## 0. 使用方法

| 字段 | 含义 |
|------|------|
| **对象** | 被废弃的概念/模块/路径 |
| **现状** | 今天还存在的位置 |
| **替代** | 统一后的唯一合法形态 |
| **窗口** | 允许暂时共存的阶段 |
| **废弃日** | 目标关闭点（阶段门，不是随意拖延） |
| **验收** | 如何证明已灭 |

阶段代号：

| 阶段 | 含义 |
|------|------|
| **U0** | 宪法冻结（本文档期） |
| **U1** | 内核合并（四原语 + Witness 唯一） |
| **U2** | 韧性标准库迁入 |
| **U3** | Agent / Workbench / Handoff 统一 — **done** (`AgentHandoff` + Workbench) |
| **U4** | Governor + Dashboard 统一 — **done** (`/api/v1/surface`, `product_decide`) |
| **U5** | 旧仓归档、文档一本化 — **done** (LE `ARCHIVED.md`, delivery 0.5.0) |

---

## 1. P0 — 必须在 MVP 前灭掉（否则不算统一）

### D-01 双历史：ExecutionStep 权威

| 项 | 内容 |
|----|------|
| 对象 | LE `ExecutionStep` 作为独立执行史 |
| 现状 | `low-entropy-core` observation / types；与 Axiom `Witness` 并存 |
| 替代 | **Witness 唯一**；Step 字段 → `WitnessEvent` 扩展或 Lens 视图 |
| 窗口 | U0–U1 |
| 废弃日 | **U1 完成门**：新代码禁止写入 Step 主存储 |
| 验收 | CI 无 Step 权威 API；demo 全链路只有 Witness 链可回放 |

### D-02 双熵决策：Guardian vs Oversight/Entropy

| 项 | 内容 |
|----|------|
| 对象 | 两套熔断/告警决策器并行 |
| 现状 | LE `guardian*`；Axiom `entropy` + `oversight` |
| 替代 | **单一 Governor**（策略表可吸收双方规则） |
| 窗口 | U0–U4（采集可双写指标，**决策不可双出口** 从 U1 起） |
| 废弃日 | **U4 完成门** |
| 验收 | 全局仅一个 `decide()` / 等价入口；文档只出现 Governor |

### D-03 双运行时：LE 业务 runtime 叙事

| 项 | 内容 |
|----|------|
| 对象 | “Go core 自己就是运行时内核” 的产品叙事与隐含第二核 |
| 现状 | LE go-core + cmd 可独立跑完整故事 |
| 替代 | 仅 Axiom runtime；LE 逻辑迁入后以库/测例形式存在直至归档 |
| 窗口 | U0–U5 |
| 废弃日 | **U5 归档 LE 仓** |
| 验收 | README/对外文档不再把 LE 描述为对等运行时 |

### D-04 联邦桥长期态

| 项 | 内容 |
|----|------|
| 对象 | Control plane + Worker 双核、长期 gRPC/HTTP 桥 |
| 现状 | 曾作为整合备选；**明确否决为长期态** |
| 替代 | 单核；迁移期允许 **临时** 适配器，必须带废弃日 |
| 窗口 | 仅迁移临时，≤ U3 |
| 废弃日 | **U3 结束** 前删除临时桥 |
| 验收 | 仓库无 `bridge`/`federation` 长期模块；无双写路径 |

### D-05 双层图对外叙事

| 项 | 内容 |
|----|------|
| 对象 | LE L0–L7 与 Axiom Crate Layer / Runtime Tier 同时对外教学 |
| 现状 | 两仓 ARCHITECTURE 各讲各的 |
| 替代 | 对外 **仅 U0–U7**（见宪法 §3）；旧映射表仅 `docs/migration/` |
| 窗口 | U0 起文档切换 |
| 废弃日 | **U0 结束**：主文档只留统一层 |
| 验收 | 主 README / ARCHITECTURE 无第二套层图 |

---

## 2. P1 — MVP 后、商用前灭掉

### D-10 LE 独立 EventStore 主路径

| 项 | 内容 |
|----|------|
| 对象 | `eventstore*.go` 作为业务事件权威 |
| 替代 | `axiom-store` + Witness 回放 |
| 窗口 | U1–U4 |
| 废弃日 | U4 |
| 验收 | 统一 store API；旧 EventStore 仅只读迁移工具 |

### D-11 双仪表盘真相源

| 项 | 内容 |
|----|------|
| 对象 | arch-manager（LE）与 axiom-viz/cli 各读各的 |
| 替代 | **一个** Dashboard API（读 Governor + Witness + 拓扑） |
| 窗口 | U4 |
| 废弃日 | U4 完成门 |
| 验收 | UI 只绑统一 API；旧页可静态归档 |

### D-12 双 Agent 故事

| 项 | 内容 |
|----|------|
| 对象 | LE Workbench 闭环 vs Axiom Agent/MCP 闭环各说各话 |
| 替代 | U6 一条链：Identity → Workbench/Skill → MCP/Tool(Port) → Witness |
| 窗口 | U3 |
| 废弃日 | U3 完成门 |
| 验收 | 一个 demo 走通“提议→执行→审计”，无第二 Agent runtime |

### D-13 Go 业务默认语言（内核侧）

| 项 | 内容 |
|----|------|
| 对象 | “业务继续默认写 Go 进内核” |
| 替代 | 内核与默认 API **Rust**；Go 仅 SDK（二期） |
| 窗口 | 永久（SDK 可存在，内核默认不可回退） |
| 废弃日 | U1 起生效 |
| 验收 | 新 ISA/runtime PR 为 Rust；Go 目录不进 kernel |

---

## 3. P2 — 清理噪音（不挡 MVP，但要排期）

| ID | 对象 | 处理 |
|----|------|------|
| D-20 | LE 大量 HTML 审计/maturity 页 | 迁 `docs/archive/` 或独立 pages，不进内核 |
| D-21 | Axiom 临时 `cargo_check*.txt`、replace 脚本等 | 清出主线或 `.gitignore` |
| D-22 | LE `package core` 单包巨石 | **不整包搬进 monorepo**；只抽纯逻辑与测例语义 |
| D-23 | 重复熵/指标实现 | U4 合并权重表，删除影子实现 |
| D-24 | 旧整合文档中的“联邦推荐” | 标注 **superseded by UNIFIED_MODEL** |

---

## 4. 明确保留（不是废弃）

| 资产 | 原因 | 迁入方式 |
|------|------|----------|
| LE 四原语语义与测试意图 | 业务 ISA 核心 | 重写为 Rust trait + 等价测试 |
| LE L2 韧性模式清单 | 模式完整 | U2 标准库 |
| LE Workbench / Handoff 叙事 | Agent 开发闭环 | U3 协议与模块 |
| LE 示例场景（计算器、调度等） | MVP/回归素材 | 改编为统一 demo |
| Axiom Cell/Signal/Witness/门禁 | 宿主内核 | 保留增强 |
| Axiom MCP / WASM plugin | 扩展面 | 保留 |
| Axiom architecture 门禁 | 硬约束 | 扩展覆盖 U1+ |

---

## 5. 迁移期临时例外规则

允许临时存在的 **唯一** 条件（必须同时满足）：

1. 写在本清单，带 **废弃日**  
2. 名称含 `legacy_` 或目录 `legacy/`  
3. CI 对 `legacy` 计数，**只减不增**  
4. 不得被新业务 feature 依赖  

不满足 → 直接拒 PR。

---

## 6. 灭双门禁（CI / 流程）

### 6.1 建议检查项

| 检查 | 阶段 |
|------|------|
| 禁止新增 `ExecutionStep` 权威写入 API | U1+ |
| 禁止第二 `Governor`/`decide` 出口 | U1+ |
| 禁止 `federation`/`bridge` 无废弃日模块 | U0+ |
| 文档链接扫描：主文档不得双层图 | U0+ |
| `legacy/` 文件数 ≤ 基线 | 每 PR |

### 6.2 人工评审红线

- “先双写，以后再删” 且无日期 → **拒**  
- “Go 核与 Rust 核对等演进” → **拒**  
- “保留 bridge 给外部用” 却无 SDK 边界 → **拒**（SDK 可以，桥核不行）

---

## 7. 时间盒（默认假设：1 人 AI 辅助全职）

| 阶段 | 灭双焦点 | 约当 |
|------|----------|------|
| U0 | D-05 文档；清单冻结 | 3–7 天 |
| U1 | D-01, D-03 启动, D-13 | 1.5–2.5 人月 |
| U2 | 韧性单路径 | 1–2 人月 |
| U3 | D-04, D-12 | 1.5–2.5 人月 |
| U4 | D-02, D-10, D-11 | 1–2 人月 |
| U5 | D-03 完成归档；D-20+ 清理 | 1–2 人月 |

具体日历随人力调整；**顺序不建议打乱**（先历史唯一，再 Agent，再 UI）。

---

## 8. 签署

| 决策 | 状态 |
|------|------|
| 接受灭双清单为统一验收的一部分 | 待确认 |
| 允许的唯一临时例外机制（§5） | 待确认 |
| LE 仓最终归档（非对等长期维护） | 待确认 |

灭双未完成，不得宣称“已大一统”。
