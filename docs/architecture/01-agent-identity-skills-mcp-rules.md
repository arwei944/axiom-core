# Agent 身份·技能·规则·MCP 配套设计

> **身份定义"是谁"，技能定义"会什么"，规则定义"守什么底线"，MCP定义"连什么外部世界"。**
> 这四者叠加在 Cell/Signal/Lens/Axiom/Witness 五原语之上，构成 Agent 开发的完整配套体系。

---

## 一、概念层次关系

在 axiom-core 的五层原语之上，Agent开发层有四个配套概念。它们不是新的原语，而是**原语的组合模式**——一切最终仍归结为 Cell 和 Signal。

```
┌─────────────────────────────────────────────────────────────┐
│                   Agent 开发配套层                          │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │ Identity │  │  Skill   │  │  Rules   │  │    MCP    │  │
│  │ (是谁)   │  │ (会什么) │  │(守什么)  │  │ (连什么)  │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └─────┬─────┘  │
│       │              │              │              │        │
│       └──────────────┴──────┬───────┴──────────────┘        │
│                             ▼                               │
│                    ┌────────────────┐                       │
│                    │  Agent Cell    │  ← Layer::Agent Cell  │
│                    │  (有身份的Cell) │                       │
│                    └───────┬────────┘                       │
└────────────────────────────┼────────────────────────────────┘
                             ▼
┌─────────────────────────────────────────────────────────────┐
│              核心原语层（5 Primitives）                      │
│  Cell · Signal · Lens · Axiom · Witness (+ Layer/Entropy)  │
└─────────────────────────────────────────────────────────────┘
```

---

## 二、Identity（身份）

### 2.1 定位

Identity 定义 Agent 是"谁"——它不是原语，而是 Agent Cell 的**结构化配置**。一个没有 Identity 的 Cell 只是一个通用计算单元；挂载了 Identity 的 Cell 才是一个"角色"。

### 2.2 核心原则

- **身份即配置**：Identity 是纯数据结构，不是代码，可以序列化、版本化、热替换
- **身份决定边界**：Identity 声明了 Agent 能访问什么（Lenses）、能调用什么（Tools/Skills）、遵守什么（Rules/Axioms）
- **身份可组合**：一个Agent可以挂载多个身份片段（如"代码审查者"+"安全专家"），组合产生复合身份
- **身份有生命周期**：身份可以在运行时动态挂载/卸载（热切换角色）

### 2.3 数据结构

```rust
pub struct Identity {
    pub id: String,
    pub name: String,
    pub version: String,

    pub persona: Persona,
    pub capabilities: CapabilitySet,
    pub rules: RuleSet,
    pub skills: Vec<SkillId>,
    pub permissions: PermissionSet,

    pub system_prompt_template: Option<PromptTemplateId>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct Persona {
    pub role: String,
    pub tone: ToneStyle,
    pub expertise: Vec<String>,
    pub values: Vec<String>,
    pub language: String,
}

pub enum ToneStyle {
    Professional, Casual, Academic, Playful, Custom(String),
}
```

### 2.4 Identity 在系统中的作用

| 维度 | Identity 的作用 |
|------|----------------|
| **提示词构建** | Persona → system prompt 的核心部分 |
| **能力过滤** | capabilities.skills/tools → 限制Agent只能看到/使用挂载的技能和工具 |
| **规则注入** | rules → 合并到全局Rules集，约束Agent行为 |
| **权限控制** | permissions → Oversight的ResourceManager/ComplianceGuard依据此检查 |
| **熵度量** | 角色漂移（输出偏离Identity定义）→ intent_drift 分量 |
| **Witness标记** | 每个Witness记录当时活跃的Identity，便于审计 |

### 2.5 身份的分层

| 层级 | 说明 | 示例 |
|------|------|------|
| **系统身份** | Runtime级默认身份，所有Agent继承 | "axiom-agent v0.1" |
| **角色身份** | 具体角色的Persona定义 | "代码审查专家"、"订单处理员" |
| **临时身份** | 单任务/单会话临时覆盖 | "本次对话扮演面试官" |
| **技能身份叠加** | 激活Skill时叠加Skill带来的子身份片段 | 激活"debug"技能时，附加"调试专家"思维模式 |

---

## 三、Skill（技能）

### 3.1 定位

Skill 是 Agent 的**专业能力包**。遵循 Anthropic Agent Skills 开放标准（agentskills.io），但在 axiom-core 中做了架构级增强。

> **核心区分**：
> - **Tool（工具）** = 手（可执行函数，有副作用，MCP/axiom-tool提供）
> - **Skill（技能）** = 脑+手的组合（知道什么时候用什么工具、怎么组合、按什么流程）

### 3.2 渐进式披露（Progressive Disclosure）

Skill 采用三层加载策略，解决"system prompt膨胀"问题：

```
Level 1: 元数据（始终加载，~几十token）
┌────────────────────────────────────────┐
│ name: code-review                      │
│ description: 审查代码质量、安全性、风格 │
│ triggers: [PR事件, "review"关键词]     │
│ ── Agent决定是否激活此Skill ──         │
└────────────────────────────────────────┘
         ↓ 激活时加载
Level 2: 指令集（按需加载，~几百token）
┌────────────────────────────────────────┐
│ SKILL.md 指令正文                      │
│ - 工作流程步骤                         │
│ - 输出格式规范                         │
│ - 决策树/检查清单                      │
│ - 使用哪些Tool/Lens/Axiom              │
│ ── Agent按指令执行 ──                  │
└────────────────────────────────────────┘
         ↓ 执行中需要时加载
Level 3: 资源（懒加载）
┌────────────────────────────────────────┐
│ scripts/   → 确定性脚本辅助            │
│ references/→ 参考文档/知识库           │
│ assets/    → 模板/示例/检查清单        │
│ examples/  → 输入输出示例              │
└────────────────────────────────────────┘
```

### 3.3 Skill 在 axiom-core 中的增强

相比于标准SKILL.md，axiom-core的Skill可以声明**架构级绑定**：

```
skill-code-review/
├── SKILL.md                    # 标准：元数据+指令（YAML+Markdown）
├── skill.toml                  # 🔶 axiom增强：架构绑定配置
├── scripts/                    # 可选：确定性辅助脚本
│   └── check_complexity.rs
├── references/                 # 可选：参考资料
│   └── rust_style_guide.md
├── axioms/                     # 🔶 axiom增强：技能级Axiom约束
│   └── no_unsafe_without_safety_comment.toml
└── lenses/                     # 🔶 axiom增强：技能激活时挂载的Lens
    └── code_diff_lens.toml
```

**skill.toml** 声明架构绑定：
```toml
[skill]
name = "code-review"
version = "1.0.0"
description = "审查代码质量、安全性和风格"

[trigger]
# 自动激活条件
on_message_contains = ["review", "审查", "代码检查"]
on_event_type = ["PullRequest", "CodeChanged"]
on_signal_type = ["ReviewRequest"]

[tools]
# 此Skill激活时可使用的工具
allowed = ["read_file", "search_code", "list_directory"]
# 此Skill禁止使用的工具
denied = ["delete_file", "execute_command"]

[axioms]
# 此Skill附加的Axiom（技能级硬约束）
enforce = ["no-unsafe-without-safety-comment", "max-cyclomatic-complexity-10"]

[lenses]
# 此Skill激活时自动挂载的Lens（状态视图）
mount = ["code-diff-lens", "project-structure-lens"]

[rules]
# 此Skill附加的行为规则（软约束）
include = ["code-review-rules"]

[identity]
# 此Skill激活时叠加的人格片段
persona_addon = "你是一位严谨的代码审查专家，注重安全性和可读性。"
```

### 3.4 Skill 分类体系

| 类型 | 说明 | 示例 |
|------|------|------|
| **Atomic Skill** | 单工具+单流程的原子技能 | "查天气"、"读文件" |
| **Composite Skill** | 多工具+多步骤的复合技能 | "代码审查"（读文件→分析→搜索模式→生成报告） |
| **Orchestration Skill** | 编排其他Skill/Agent的技能 | "项目管理"（分解任务→分配子Agent→汇总结果） |
| **Governance Skill** | 治理类技能，供Oversight使用 | "异常检测"、"合规审计" |
| **MCP Bridge Skill** | 包装外部MCP Server的技能 | "GitHub操作"（通过GitHub MCP Server） |

### 3.5 Skill 生命周期

```
发现 → 注册 → 触发 → 激活 → 执行 → 停用 → （可能）学习
 │       │       │       │       │       │
 │       │       │       │       │       └─ 释放Level2/3资源
 │       │       │       │       └─ 按SKILL.md指令执行，Tools/Lenses可用
 │       │       │       └─ 加载Level2指令，挂载绑定的Tools/Lenses/Axioms
 │       │       └─ 匹配trigger条件（关键词/事件/Signal）
 │       └─ SkillRegistry注册到Runtime，元数据加载到Level1
 └─ 从文件系统/远程仓库/插件市场发现
```

### 3.6 Skill 与原语的映射

Skill 不是新原语，它的每一层最终都映射到现有原语：

| Skill组件 | 映射到原语 |
|-----------|-----------|
| Skill激活/停用 | Cell消息（ActivateSkill/DeactivateSkill Signal） |
| skill.toml的tools限制 | axiom-tool的ToolRegistry + PermissionCheck |
| skill.toml的axioms | 注册到Cell的AxiomChain |
| skill.toml的lenses | 挂载到Cell的LensSet |
| scripts/ | 确定性函数（Layer::Exec Cell） |
| Skill指令（SKILL.md） | 注入到LLM上下文的Prompt片段 |
| Skill激活状态 | Cell私有状态的一部分 |

---

## 四、Rules（规则）

### 4.1 定位

Rules 是 Agent 的**行为规范**。它们与 Axiom 有本质区别：

| 维度 | Axiom（公理） | Rules（规则） |
|------|--------------|--------------|
| **性质** | 硬约束（硬定律） | 软约束（行为指南） |
| **违反后果** | 拒绝/熔断/回滚（确定性动作） | 降低置信度/告警/重试（可柔性处理） |
| **适用范围** | 系统级/Cell级（架构层面） | Identity级/Skill级/会话级 |
| **检查时机** | 状态修改前（同步阻塞） | LLM推理前后（上下文注入/输出检查） |
| **类比** | 物理定律（不能违反） | 交通规则（应当遵守，违反扣分但不一定撞车） |
| **确定性** | 必须是纯确定性函数 | 可以包含启发式检查 |

### 4.2 Rules 分类

| 类型 | 说明 | 示例 |
|------|------|------|
| **Safety Rules（安全规则）** | 安全底线，接近Axiom但柔性 | "禁止输出用户的密码"、"删除操作前必须确认" |
| **Format Rules（格式规则）** | 输出格式要求 | "回复使用Markdown"、"JSON输出必须包含code字段" |
| **Behavior Rules（行为规则）** | 交互行为规范 | "不知道时诚实说不知道，不要编造"、"先思考再行动" |
| **Tool Rules（工具规则）** | 工具使用优先级/策略 | "优先使用read_file而非execute_command(cat)" |
| **Quality Rules（质量规则）** | 输出质量标准 | "代码必须有错误处理"、"回答要简洁" |
| **Domain Rules（领域规则）** | 业务领域约束 | "金融数据保留两位小数" |

### 4.3 Rules 数据结构

```rust
pub struct Rule {
    pub id: RuleId,
    pub name: String,
    pub description: String,
    pub severity: RuleSeverity,
    pub scope: RuleScope,
    pub check: RuleCheck,
    pub on_violation: ViolationAction,
}

pub enum RuleSeverity {
    Info,       // 提示级别
    Warning,    // 警告，记录但不阻止
    Strict,     // 严格，要求重试/修正
    Critical,   // 严重，升级为Axiom违反
}

pub enum RuleScope {
    Global,
    Identity(IdentityId),
    Skill(SkillId),
    Session,
}

pub enum RuleCheck {
    /// 注入到System Prompt中的指令（让LLM自觉遵守）
    PromptInstruction(String),
    /// 输出验证函数（确定性后检查）
    OutputValidator(/* 函数指针/trait object */),
    /// 混合：prompt注入 + 输出验证
    Hybrid { instruction: String, validator: /* */ },
}

pub enum ViolationAction {
    Log,
    Retry { max_attempts: u32 },
    RequestClarification,
    EscalateToOversight,
}
```

### 4.4 Rules 的三层执行

```
┌─────────────────────────────────────────────────────────┐
│  Layer 1: Prompt 注入（推理前）                         │
│  Rules作为system prompt的一部分，引导LLM遵守             │
│  → 这是最常用、成本最低的方式                           │
├─────────────────────────────────────────────────────────┤
│  Layer 2: 输出验证（推理后，执行前）                    │
│  确定性Validator检查LLM输出是否合规                      │
│  不合规 → 自动重试/要求修正/告警                         │
│  → 这是"反幻觉"和"格式保证"的关键                       │
├─────────────────────────────────────────────────────────┤
│  Layer 3: 升级为Axiom（执行时）                         │
│  Critical级别的Rule违反 → 升级为Axiom违反 → 熔断        │
│  → 安全底线的最后一道防线                               │
└─────────────────────────────────────────────────────────┘
```

### 4.5 Rules 合并规则

Rules来自多个源头，按优先级合并：

```
优先级从高到低：
1. 会话级临时规则（用户本次对话的特殊要求）
2. Skill激活时附加的规则（skill.toml中的rules）
3. Identity级规则（身份定义的行为规范）
4. 全局规则（系统默认规则）

冲突时：高优先级覆盖低优先级；同优先级时Critical > Strict > Warning > Info
```

---

## 五、MCP 集成

### 5.1 定位

MCP（Model Context Protocol）是 axiom-core 接入**外部工具、资源、提示词**的标准协议。MCP在axiom-core中不是核心原语，而是一个**桥接层**——把外部世界的能力接入为axiom内部的Tool/Resource/Prompt。

> **axiom-core 既是MCP Client，也可以是MCP Server**：
> - 作为Client：连接外部MCP Server，使用其Tools/Resources/Prompts
> - 作为Server：把axiom内部的Cell能力暴露为MCP Tools，供其他AI应用使用

### 5.2 MCP 架构映射

```
┌──────────────────────────────────────────────────────────────┐
│                    axiom-core Runtime                        │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │axiom-tool    │  │axiom-memory  │  │axiom-prompt  │       │
│  │ ToolRegistry │  │  ResourceView│  │TemplateEngine│       │
│  └──────▲───────┘  └──────▲───────┘  └──────▲───────┘       │
│         │                 │                 │               │
│         └─────────────────┼─────────────────┘               │
│                           │                                 │
│                  ┌────────┴────────┐                        │
│                  │  MCP Bridge     │ ← 新增：axiom-mcp crate│
│                  │  (Client+Server)│                        │
│                  └────────┬────────┘                        │
└───────────────────────────┼──────────────────────────────────┘
                            │
          ┌─────────────────┼─────────────────┐
          │ stdio           │ HTTP/Streamable │
          ▼                 ▼                 ▼
   ┌────────────┐   ┌────────────┐   ┌────────────┐
   │ Local MCP  │   │ Remote MCP │   │  MCP Servers│
   │ Servers    │   │ Servers    │   │ (exposed)   │
   │ (filesystem│   │ (GitHub,   │   │ axiom's own │
   │  , git...) │   │  Slack...) │   │ Cells as   │
   └────────────┘   └────────────┘   │ MCP tools  │
                                     └────────────┘
```

### 5.3 MCP Primitives → axiom 映射

| MCP 原语 | 映射到 axiom | 说明 |
|----------|-------------|------|
| **MCP Tool** | axiom-tool Tool trait | MCP工具自动注册为axiom Tool；调用MCP Tool经过Axiom/Rules/Permission检查 |
| **MCP Resource** | axiom-memory / Lens | MCP Resource通过Lens按需投影给Agent，不是全部加载 |
| **MCP Prompt** | axiom-prompt 模板 | MCP Prompt模板注册为axiom PromptTemplate |
| **MCP Sampling** | axiom-llm LLMProvider | MCP Server请求LLM推理→桥接到axiom的LLM抽象 |
| **MCP Elicitation** | Oversight ComplianceGuard | MCP Server请求用户确认→通过Oversight的审批流程 |
| **MCP Logging** | axiom tracing/logging | MCP日志→axiom的tracing系统，产生Witness |

### 5.4 安全约束（关键！）

MCP带来巨大便利的同时也带来巨大安全风险（研究显示26.1%社区MCP技能包含漏洞）。axiom-mcp必须有严格的安全层：

```
MCP Tool调用链路：
LLM决定调用MCP Tool
    ↓
Permission检查（Identity权限是否允许调用此工具）
    ↓
Rules检查（是否符合行为规则）
    ↓
Axiom检查（是否违反系统不变量）
    ↓
Human-in-the-loop（危险工具需用户确认）
    ↓
参数验证（JSON Schema验证 + 消毒）
    ↓
MCP Client发送调用
    ↓
结果经过Oversight ComplianceGuard检查（PII检测/数据泄露）
    ↓
产生Witness（记录完整调用链+输入+输出）
    ↓
返回给Agent
```

### 5.5 MCP Server 能力（反向暴露）

axiom-core也可以作为MCP Server，把内部能力暴露给外部AI应用：

| 暴露的MCP Tool | 对应axiom内部能力 |
|---------------|-----------------|
| `axm_cell_list` | 列出所有Cell状态 |
| `axm_trace` | 查询correlation_id的Witness链 |
| `axm_entropy` | 查询系统熵值 |
| `axm_witness_query` | 查询Witness历史 |
| `axm_send_signal` | 向Cell发送Signal（受权限控制） |
| `axm_cell_snapshot` | 创建Cell快照 |

---

## 六、四者的协作：一个完整场景

以"代码审查Agent"为例，看 Identity + Skill + Rules + MCP 如何协作：

### 场景：用户请求"审查这个PR"

**1. 身份挂载**
```
Identity: "code-reviewer"
  Persona: 资深代码审查专家，注重安全性和性能
  Capabilities: [code-review skill, read-file tool, search-code tool]
  Permissions: { read: true, write: false, execute: false }
```

**2. 技能激活（Progressive Disclosure）**
- Level1：元数据匹配（"审查"关键词触发 code-review skill）
- Level2：加载SKILL.md指令："先理解变更→检查安全性→检查性能→检查风格→生成报告"
- Level3：按需加载references（安全漏洞清单）

**3. 规则注入**
- 来自Identity："审查要全面，不放过潜在问题"
- 来自Skill："每个问题必须标注严重程度和修复建议"
- 来自全局："不能编造代码中不存在的问题"

**4. MCP工具连接**
- 连接GitHub MCP Server → 获得`get_pull_request`、`create_review_comment`等工具
- 这些工具经Permission检查后注册到ToolRegistry
- 工具描述经精简后注入LLM上下文（避免90+工具消耗50k token的问题）

**5. 执行流程**
```
用户Signal: "审查PR #123"
    ↓
Identity + Skill + Rules 组装 system prompt
    ↓
Lens投影：代码diff + 项目结构 + 历史PR数据（按需）
    ↓
LLM推理 → 决定调用get_pull_request (MCP Tool)
    ↓
Rules验证输出格式 → Axiom检查权限 → 执行
    ↓
MCP返回PR数据 → 产生Witness
    ↓
Lens按需投影更多上下文（发现unsafe块→加载references/安全清单）
    ↓
LLM继续推理 → 发现问题 → 按SKILL.md格式生成报告
    ↓
Rules验证报告格式 → 输出给用户
    ↓
产生完整Witness链，Oversight监控熵值
```

**6. 如果出问题**
- 如果Agent尝试调用`delete_file` → Permission拒绝 → Witness记录 → Oversight告警
- 如果Agent编造了不存在的bug → Quality Rule检测到输出与代码不一致 → 要求重试
- 如果MCP Server返回异常 → 自动重试 → 失败则降级到内置工具

---

## 七、配套 Crate 规划

在现有 workspace 基础上新增/扩展：

```
crates/
├── axiom-core/              ✅ 已存在（5原语+Layer+Entropy）
├── axiom-runtime/           ✅ 已存在（运行时+监督树）
├── axiom-store/             ✅ 已存在（事件存储）
├── axiom-oversight/         ✅ 已存在（监督层）
├── axiom-macros/            ✅ 已存在（过程宏）
├── axiom-viz/               ✅ 已存在（可视化导出）
│
├── === 新增：Agent开发配套 ===
│
├── axiom-agent/             ← 扩展：fascade + Identity + Skill + Rules引擎
│   └── src/
│       ├── identity/        ← 身份系统
│       │   ├── mod.rs
│       │   ├── identity.rs       # Identity结构 + 合并逻辑
│       │   ├── persona.rs        # Persona定义
│       │   └── permission.rs     # 权限集
│       ├── skill/           ← 技能系统
│       │   ├── mod.rs
│       │   ├── registry.rs       # Skill注册表
│       │   ├── loader.rs         # SKILL.md加载+解析
│       │   ├── activator.rs      # 技能激活/停用/渐进式披露
│       │   └── trigger.rs        # 触发条件匹配
│       ├── rule/            ← 规则系统
│       │   ├── mod.rs
│       │   ├── ruleset.rs        # 规则集+合并优先级
│       │   ├── validator.rs      # 输出验证器
│       │   └── injector.rs       # Prompt规则注入
│       ├── context/         ← Agent上下文组装
│       │   ├── mod.rs
│       │   ├── assembler.rs      # 组装system prompt（Identity+Skills+Rules+Tools）
│       │   └── token_budget.rs   # Token预算管理
│       └── agent_cell.rs    # AgentCell：挂载了Identity的Cell实现
│
├── axiom-mcp/               ← 新增：MCP协议桥接
│   └── src/
│       ├── lib.rs
│       ├── client.rs        # MCP客户端（连接外部Server）
│       ├── server.rs        # MCP服务端（暴露axiom能力）
│       ├── transport/       # 传输层（stdio + HTTP）
│       ├── tool_bridge.rs   # MCP Tool ↔ axiom Tool 映射
│       ├── resource_bridge.rs # MCP Resource ↔ Lens 映射
│       ├── prompt_bridge.rs # MCP Prompt ↔ axiom-prompt 映射
│       └── security.rs      # MCP安全层（权限+消毒+审批）
│
└── axiom-cli/               ⏳ 后续（CLI二进制）
```

### Crate 依赖方向

```
axiom-macros
    ↓
axiom-core
    ↓
axiom-store → axiom-runtime → axiom-oversight
    ↓                           ↓
    └──────────┬────────────────┘
               ▼
         axiom-agent（新增）
          ↗    ↖
axiom-mcp      axiom-llm/tool/memory/...
          ↖    ↗
               ▼
           axiom-viz → axiom-cli
```

- `axiom-agent` 依赖 `axiom-core` + `axiom-runtime`
- `axiom-mcp` 依赖 `axiom-core` + `axiom-agent`（需要ToolRegistry接口）
- MCP是可选依赖，不启用MCP功能时不需要MCP相关依赖

---

## 八、与axiom CLI的集成

`axm` CLI增加Agent开发相关命令：

```bash
# 身份管理
axm identity list                          # 列出所有已注册身份
axm identity new code-reviewer             # 创建新身份模板
axm identity show code-reviewer            # 查看身份详情
axm identity activate code-reviewer        # 运行时切换身份

# 技能管理
axm skill list                             # 列出所有可用技能
axm skill new my-skill                     # 创建新技能模板（生成SKILL.md+skill.toml）
axm skill show code-review                 # 查看技能详情
axm skill test code-review --input "..."   # 测试技能（Dry Run）
axm skill install <source>                 # 从本地/远程安装技能
axm skill validate my-skill/               # 验证SKILL.md格式是否正确

# 规则管理
axm rule list                              # 列出所有规则
axm rule new "no-hallucination"            # 创建新规则
axm rule test --output "..." --rule <id>   # 测试规则验证器

# MCP管理
axm mcp list                               # 列出已配置的MCP Server
axm mcp add github npx -y @modelcontextprotocol/server-github  # 添加MCP Server
axm mcp test github                        # 测试MCP Server连接
axm mcp tools github                       # 列出MCP Server提供的工具
axm mcp serve                              # 启动axiom作为MCP Server

# 调试
axm agent run --identity code-reviewer     # 以指定身份运行Agent（交互模式）
axm agent trace <correlation_id>           # 查看Agent完整推理+工具调用链
```

---

## 九、设计原则总结

| 原则 | 说明 |
|------|------|
| **不发明新原语** | Identity/Skill/Rules/MCP都是5原语的组合模式，不是新原语 |
| **渐进式披露** | Skill元数据始终加载，指令按需加载，资源懒加载——Token经济 |
| **安全分层** | MCP工具调用经过 Permission → Rules → Axiom → Human-in-the-loop 四层检查 |
| **软硬约束分离** | Axiom是硬约束（违反即熔断），Rules是软约束（警告/重试/修正） |
| **标准兼容** | Skill遵循agentskills.io开放标准；MCP遵循官方MCP规范 |
| **架构级增强** | 相比标准SKILL.md，axiom的Skill可绑定Axiom/Lens/Permission，不只是prompt |
| **一切可审计** | Identity切换/Skill激活/Rule违反/MCP调用全部产生Witness |
| **热插拔** | Identity/Skill可以运行时挂载/卸载，不需要重启系统 |
