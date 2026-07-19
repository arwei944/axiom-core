# 新智能体入职包：文档 · 约束 · 门禁（逼近架构合规）

| Field | Value |
|-------|--------|
| **目的** | 给**新智能体**一套可执行的输入，使其写代码时**默认且持续**遵守 ULE/Axiom 架构 |
| **诚实上限** | 纯文档 **不能**数学意义上「百分百」；须 **契约 + 机械化门禁 + 拒绝权** 叠防 |
| **配套根契约** | 仓库根目录 [`AGENTS.md`](../../AGENTS.md)（工具常自动加载） |
| **工作细则** | [`agent-work-guide.md`](./agent-work-guide.md) |

---

## 0. 先打破幻觉：什么叫「百分百按照架构」

| 手段 | 能保证什么 | 不能保证什么 |
|------|------------|--------------|
| 宪法 + 指导文档 | 智能体**知道**规则 | 每次都遵守 |
| 系统提示词 / AGENTS.md | 提高默认遵守率 | 长对话漂移、故意绕过 |
| 代码评审清单 | 人/审阅 Agent 抓漏 | 漏审、赶工跳过 |
| **archcheck / ISA discipline / 路径测试 / CI** | **违规往往编不过或合不进** | 语义级钻空（形式上 Port、实质上乱） |
| 类型系统 + 宏 + 小 API 面 | 错误用法难写 | 智能体新建平行抽象 |

**结论（工程上的「百分百」定义）：**

> 不是「模型永远不犯错」，而是：  
> **凡违反宪法的实现，要么无法通过强制门禁，要么被 DoD 判定为未完成。**  
> 智能体被剥夺「静默违宪还能标完成」的能力。

下文按这个定义设计入职包。

---

## 1. 控制栈（Defense in Depth）

给新智能体时，必须**整包**提供，而不是只丢一篇 README：

```text
L0  身份与目标      — 你在 ULE monorepo；唯一宿主 Axiom
L1  宪法（不可谈判）— UNIFIED_MODEL
L2  行为契约        — AGENTS.md + agent-work-guide
L3  形状范例        — demo-taskflow / isa 黄金路径
L4  任务模板        — 用例填空表（先设计后代码）
L5  机械化门禁      — archcheck · discipline · path tests · CI
L6  完成定义 DoD    — 勾不满 = 禁止声称完成
L7  拒绝权          — Reviewer/CI 对 Blocker 一票否决
```

缺 L5–L7，只有 L1–L4 → **最多 70–85% 自觉遵守**。  
L0–L7 齐全 → **可验收的「架构合规交付」**（违宪难上岸）。

---

## 2. 必须提供的文档清单（按注入顺序）

### 2.1 强制核心（每次会话 / 每个 Agent 启动注入）

| 优先级 | 文档 | 为何必须 | 不给的后果 |
|--------|------|----------|------------|
| **P0** | [`AGENTS.md`](../../AGENTS.md) | 一页契约，工具可自动读 | 漂移到通用 CRUD 写法 |
| **P0** | [`docs/unified/UNIFIED_MODEL.md`](../unified/UNIFIED_MODEL.md) | 唯一词汇与铁律 | 双历史/双核/Service 堆 |
| **P0** | [`docs/guide/agent-work-guide.md`](./agent-work-guide.md) | MUST/MUST NOT、提示词、§7 清单 | 知宪法不知怎么落码 |
| **P0** | 本文件 `AGENT_ONBOARDING_PACK.md` | 门禁与 DoD、包结构 | 有文档无执法 |

**注入方式建议：**

- 系统提示：粘贴 `agent-work-guide.md` §5 + 「必读 AGENTS.md」  
- 或 Cursor/Codex/Claude：**自动附带 AGENTS.md**  
- 长任务：每个子 Agent 重启时重注 P0（防上下文压缩丢约束）

### 2.2 强制参考（改代码前打开，不必全文背）

| 文档 / 路径 | 用途 |
|-------------|------|
| `crates/axiom-isa/` | 四原语、Governor、Handoff、discipline **合法 API** |
| `crates/axiom-demo-taskflow/` | **黄金路径**：Cell + Composer + Surface |
| `crates/axiom-resilience/` | 韧性只许用这里的装饰器形状 |
| `docs/unified/FEATURE_THEME_MATRIX.md` | 已有能力；避免重复造轮子或误判缺口 |
| `docs/unified/DEPRECATION.md` | 灭双清单；禁止回潮 |

### 2.3 按任务类型附加（条件注入）

| 任务类型 | 追加文档 |
|----------|----------|
| 动 runtime/dispatch/监督 | `docs/ENGINEERING_HARDENING_v050.md`、`crates/axiom-runtime/` |
| 动插件/WASM | `docs/PLUGIN_SYSTEM.md`、kernel `plugin/` |
| 动 HTTP/鉴权 | `docs/COMMERCIAL_OPS.md`、`crates/axiom-api/` |
| 声称「主题/商用完成」 | `FEATURE_THEME_MATRIX` + `COMMERCIAL_DELIVERY` + Out-of-scope 边界 |
| 前端对接 | 工作指导中的「入口≠内核」+ Surface 路由（见 demo surface） |
| 交易/OMS 类 | 人类立项设计（若有 TRADING 设计文）；仍服从四原语 |

### 2.4 明确不要整包塞给模型的（防噪声）

| 材料 | 原因 |
|------|------|
| 全部 50+ 历史 docs 一次丢入 | 冲掉宪法焦点 |
| LE 全量 Go 源码 | 诱导双核实现 |
| 无关 crate 的全部测试快照 | 上下文爆炸 |
| 「可以先实现再重构」类旧笔记 | 与灭双冲突 |

**原则：少而硬 > 多而软。**

---

## 3. 约束分类（必须写进 Agent 输入）

### 3.1 宪法约束（Normative）

来自 UNIFIED_MODEL L1–L10，摘要须出现在系统提示：

- 唯一宿主 / 唯一 Witness / 唯一 Governor  
- 四原语 + Composer-in-Cell + Signal-only 跨单元  
- Port 唯一副作用；SDK 非第二内核；灭双优先  

### 3.2 过程约束（Procedural）

- 先填「用例原语表」再写代码（见 agent-work-guide §6）  
- 落码顺序：Atom→Port/Mock→Composer→Cell→Host→path test  
- 禁止「先打通 HTTP 再补架构」  

### 3.3 接口约束（API Surface）

- 准入：`product_admit` / `product_decide`  
- 记史：`WitnessJournal` + `run_*`  
- Handoff：`HandoffRequest`  
- 禁止新建平行 `*Service::execute_business_pipeline` 当内核  

### 3.4 范围约束（Scope）

Out-of-scope 不得当作失败理由，也不得偷偷做完冒充主题：

- 多租户计费、跨区 HA、无限制 LLM Workbench、LE 全量重写、workspace 历史全绿  

### 3.5 安全约束（Safety）

- 无密钥入库  
- Workbench allow-list  
- 生产 Port 凭证只走环境/SecretsPort  

### 3.6 社交约束（Multi-agent）

- 交接用 Handoff 语义字段  
- Reviewer 对 Blocker 可一票否决  
- 冲突：宪法 > AGENTS.md > 聊天  

---

## 4. 机械化门禁（没有这些就没有「保证」）

新智能体入职时，**人类/编排器必须打开或声明**下列门禁；智能体 DoD 绑定它们。

### 4.1 已有（仓库内）

| 门禁 | 作用 | 智能体如何面对 |
|------|------|----------------|
| `.axiom/architecture.toml` + `archcheck` | 层依赖方向 | 改 Cargo.toml 后须能过 archcheck |
| `axiom_isa::discipline` + `tests/isa_discipline.rs` | 商业路径 Atom/Port 纪律 | 新 Composer 加入扫描列表 |
| `*_path.rs` 测试（demo） | 生产路径非死代码 | 新 Cell 必须有 path 测试 |
| `cargo test -p axiom-isa -p …` | 回归 | DoD 必跑 |
| Guard / layer / Witness 链测试 | 内核不变量 | 不削弱断言 |

### 4.2 建议补齐（要「强保证」请立项做）

| 门禁 | 作用 |
|------|------|
| CI job：`archcheck` 失败阻断 PR | 人不在也拦 |
| CI job：`isa_discipline` 扩到所有 `crates/*/src/composers*.rs` | 防新 crate 漏网 |
| CI：禁止新增 `ExecutionStep` 权威写（ripgrep 规则） | 灭双自动化 |
| CI：禁止 `product` 路径外的 `Governor` 旁路关键字（启发式） | 降双决策 |
| `#[deny]` / API 可见性：Port trait 放 isa，业务 crate 不 pub 乱 IO 工具 | 缩小作恶面 |
| Review Agent 强制跑 §7 checklist 输出 | 流程保证 |
| 分支保护：required checks | 真「合不进」 |

**没有 4.2，只能保证「示范路径」合规，不能保证「智能体新开的 crate」合规。**

---

## 5. 推荐的「入职包」文件结构（给编排器/人类）

```text
axiom-core/
  AGENTS.md                          ← 根契约（P0，自动加载）
  docs/
    guide/
      AGENT_ONBOARDING_PACK.md       ← 本文件
      agent-work-guide.md            ← 细则 + 提示词 + checklist
    unified/
      UNIFIED_MODEL.md               ← 宪法
      FEATURE_THEME_MATRIX.md
      DEPRECATION.md
      COMMERCIAL_DELIVERY.md
  crates/axiom-isa/                  ← 合法 API
  crates/axiom-demo-taskflow/        ← 黄金路径
  tools/archcheck/
  .axiom/architecture.toml
```

### 5.1 启动一个新编码 Agent 时的标准消息模板

```text
【角色】你是 ULE monorepo 的实现智能体。
【根契约】严格遵守仓库 AGENTS.md 与 docs/guide/agent-work-guide.md。
【宪法】docs/unified/UNIFIED_MODEL.md；冲突时宪法优先。
【任务】<一句话垂直场景，禁止夹带 SaaS/HA>
【参考】仅克隆形状自 axiom-demo-taskflow + axiom-isa。
【顺序】原语表 → Atom 单测 → Mock Port → Cell → path test → 再 HTTP。
【DoD】AGENTS.md §4；跑通指定 cargo test；输出 §7 自检。
【禁止】Atom IO；第二历史；绕过 product_admit；无路径测试称完成。
```

### 5.2 子 Agent 最小上下文（防爆）

只给：

1. AGENTS.md 全文  
2. agent-work-guide 硬约束表 + §5 提示词  
3. 本任务原语表  
4. 2–3 个参考文件路径（非全仓）  

---

## 6. 任务协议（智能体必须遵守的「写作前仪式」）

在写第一行业务代码前，输出并冻结：

```markdown
## 原语表
| 步骤 | 原语 | 名称 | IO? |
## Cell / Signal
## Governor 点
## 测试计划
## 非目标（本次不做）
```

**编排器规则：** 无此表 → 拒绝进入 coding 阶段。

---

## 7. 完成定义（DoD）— 智能体「完成」的唯一合法含义

### 7.1 代码

- [ ] 主路径：Signal → Cell → admit → Composer → Witness  
- [ ] IO 仅 Port + `run_port`  
- [ ] 无第二 admit / 第二历史  
- [ ] 层依赖不违规  

### 7.2 证明

- [ ] 路径测试或 demo 命令  
- [ ] `cargo test` 相关包绿  
- [ ] （若商业 Composer）discipline 扫描覆盖新文件  

### 7.3 陈述

- [ ] 变更说明含原语表  
- [ ] 明确 Out-of-scope 未偷做  
- [ ] 已知风险（若有）不藏  

**任一项否 → 状态只能是 in_progress / blocked，不能是 done。**

---

## 8. 现有材料 vs 缺口

### 8.1 已具备（可直接给新智能体）

| 材料 | 状态 |
|------|------|
| 宪法 UNIFIED_MODEL | ✅ |
| agent-work-guide（约束+提示词+清单） | ✅ |
| AGENTS.md 根契约 | ✅（本包） |
| 黄金路径 demo-taskflow | ✅ |
| ISA + discipline 扫描 | ✅（覆盖面可扩） |
| archcheck 层门禁 | ✅ |
| 主题矩阵 / 交付说明 / 灭双 | ✅ |
| 工程硬化说明 | ✅ |

### 8.2 缺口（要更高保证请补）

| 缺口 | 影响 |
|------|------|
| CI 强制 archcheck + discipline 全仓 | 本地可偷懒推送 |
| 前端集成专文 + OpenAPI 冻结 | API 形状靠口述 |
| 「禁止模式」编译期/测试期总表（ripgrep CI） | 语义钻空 |
| TRADING 等垂直设计若立项未入库 | 领域自由发挥 |
| Review 子 Agent 自动化脚本 | 依赖人工 |

---

## 9. 给「人类管理员」的操作清单

新智能体进场时你要做的事：

1. **贴** 启动模板（§5.1）  
2. **确认** 其工具能读到 `AGENTS.md`  
3. **限定** 任务一句话 + 非目标列表  
4. **要求** 先交原语表再允许改代码  
5. **绑定** DoD：path test + cargo test  
6. **打开** CI/archcheck（若已配 required）  
7. **Reviewer** 按 agent-work-guide §7 / §10 分级否决  
8. **禁止** 「先合再改架构」的临时豁免（除非书面 time-box + 灭双日）  

---

## 10. 风险与预期合规率（经验模型）

| 配置 | 预期「违宪合入」率 | 说明 |
|------|-------------------|------|
| 仅口头说「按 Axiom 写」 | 很高 | 几乎无保证 |
| + 宪法 + work-guide | 中 | 自觉期 |
| + AGENTS.md + 黄金路径 + DoD | 低 | 可验收 |
| + CI archcheck/discipline/path | 极低 | **工程百分百**含义 |
| + 人工/审阅 Agent Blocker 否决 | 接近目标 | 推荐生产姿态 |

---

## 11. 一页纸：新智能体最小物资

打印或置顶这 6 项即可开张：

1. `AGENTS.md`  
2. `docs/unified/UNIFIED_MODEL.md`  
3. `docs/guide/agent-work-guide.md`  
4. `docs/guide/AGENT_ONBOARDING_PACK.md`（本文件）  
5. `crates/axiom-isa` + `crates/axiom-demo-taskflow`  
6. DoD：path test + `product_admit` + 无 Atom IO + cargo test  

再加一条执法：

7. **CI/评审：Blocker 不合并**  

---

## 12. 修订

- 宪法变更 → 同步 AGENTS.md 与本包 §3  
- 新门禁上线 → 写入 §4 与 DoD  
- 智能体不得以「提示词太长」为由省略 P0  

**生效：** 与 `AGENTS.md` 同时作为 monorepo 智能体入职标准。
