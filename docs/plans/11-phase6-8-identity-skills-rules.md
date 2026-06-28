# Phase 6: 身份系统 Implementation Plan

> **Goal:** 实现 axiom-agent 中的身份系统：Identity/Persona/PermissionSet，支持身份版本化，Agent Cell可以挂载身份并受权限约束。验收标准：Cell在处理消息时绑定身份，身份变化有版本迁移，权限不足时返回PermissionDenied。

> **Baseline:** axiom-agent 当前只是空门面 crate（re-export axiom-core/axiom-runtime），需要新建身份模块。

---

## Global Constraints
- axiom-agent 依赖 axiom-core/axiom-runtime
- Identity本身是版本化的（SchemaVersion）
- 权限检查在消息发送/接收两个点都做
- 身份切换必须产生Witness
- cargo build/clippy/test 零警告

---

### Task 6.1: 定义 Identity 基础类型

**Files:**
- Create: `crates/axiom-agent/src/identity.rs`
- Modify: `crates/axiom-agent/src/lib.rs`

- [ ] 定义 IdentityId 强类型ID（UUID v4）
- [ ] 定义 Persona（人设：name/description/system_prompt/traits）
- [ ] 定义 PermissionSet：集合类型，包含权限字符串（如 "tool:fs.read", "cell:exec:send", "llm:call"）
- [ ] 定义 Identity 结构体：id/persona/permissions/created_at/version/parent_id（支持身份派生）
- [ ] 单元测试：Identity序列化/反序列化、PermissionSet包含/合并/交集
- [ ] Commit: `feat(axiom-agent): Identity, Persona, PermissionSet types with versioning`

### Task 6.2: 实现 IdentityRegistry 和权限检查

- [ ] 定义 IdentityRegistry：存储所有已知身份，支持按id查询
- [ ] 实现 `fn check_permission(identity: &Identity, permission: &str) -> bool`
- [ ] 实现 CellContext 的 identity 绑定：在 CellContext 中增加 identity_id 字段（P1已有），AgentCell处理消息时自动设置
- [ ] 在消息发送时检查权限：发送到某些层/目标Cell需要特定权限
- [ ] 权限不足返回 AxiomError::PermissionDenied + Witness
- [ ] 单元测试：权限检查通过/拒绝/继承
- [ ] Commit: `feat(axiom-agent): IdentityRegistry with permission checking, PermissionDenied enforcement in CellContext`

### Task 6.3: 身份版本化和迁移

- [ ] IdentityVersion 使用 SchemaVersion（复用axiom-core的版本系统）
- [ ] 身份变更产生新版本，旧版本可追溯
- [ ] IdentityMigration trait（类似Signal Migration）
- [ ] 启动时验证身份迁移链完整性
- [ ] 单元测试：身份版本升级、旧身份数据迁移
- [ ] Commit: `feat(axiom-agent): Identity versioning with migration chain, startup validation`

### Task 6.4: 身份挂载到 Cell 和 Witness

- [ ] AgentCell 可以挂载 Identity（在spawn时指定）
- Witness 中的 identity_id 字段自动填充（P1已有字段，此阶段实际使用）
- [ ] 身份切换（impersonation）必须有显式授权和Witness记录
- [ ] 单元测试：绑定身份处理消息产生带identity的Witness
- [ ] Commit: `feat(axiom-agent): Identity binding to Cells, Witness identity_id populated, impersonation audit`

---

## P6 验收标准

| # | 验收项 |
|---|--------|
| 1 | Identity/Persona/PermissionSet 完整定义和测试 |
| 2 | IdentityRegistry 权限检查正确拒绝未授权操作 |
| 3 | 身份版本化+迁移链验证 |
| 4 | Witness 正确记录 identity_id |
| 5 | cargo test -p axiom-agent 通过（≥12个测试） |

---

# Phase 7: 技能系统 Implementation Plan

> **Goal:** 实现 Skill/SKILL.md 解析/渐进式披露/激活/触发。Skill可以自动触发激活，挂载Tools/Lenses/Axioms到Agent Cell。验收标准：定义SKILL.md后，Agent自动发现技能、按需激活、激活后获得对应工具和能力。

---

### Task 7.1: 定义 Skill 类型和 SKILL.md 格式

**Files:**
- Create: `crates/axiom-agent/src/skill.rs`

- [ ] 定义 SkillId、SkillMetadata（name/version/description/author/tags）
- [ ] 定义 SkillManifest：解析自 SKILL.md 文件的front-matter（YAML或TOML）
- [ ] SKILL.md 格式：front-matter（元数据+权限+工具依赖）+ Markdown正文（技能描述/触发条件/指令）
- [ ] 定义 TriggerCondition：关键词/意图模式/事件类型/定时
- [ ] 定义 SkillActivation：active/inactive/cooldown状态
- [ ] 单元测试：SKILL.md解析（front-matter+正文分离）
- [ ] Commit: `feat(axiom-agent): Skill types, SKILL.md front-matter parsing, trigger conditions`

### Task 7.2: 实现 SkillRegistry 和发现

- [ ] SkillRegistry：扫描skills/目录，加载所有SKILL.md
- [ ] 支持运行时动态加载/卸载技能
- [ ] 技能依赖解析（Skill A 依赖 Skill B）
- [ ] 版本冲突检测
- [ ] 单元测试：目录扫描、依赖解析、冲突检测
- [ ] Commit: `feat(axiom-agent): SkillRegistry with directory scanning, dependency resolution, conflict detection`

### Task 7.3: 实现渐进式披露（Progressive Disclosure）

- [ ] Skill 有详细描述但不全量注入prompt
- [ ] 先注入摘要（name+one-line description）
- [ ] Agent判断需要时，主动加载完整SKILL.md
- [ ] 工具和Lenses在Skill激活后才注册（减少初始上下文）
- [ ] 单元测试：摘要/完整内容的token预算估算
- [ ] Commit: `feat(axiom-agent): progressive disclosure with summary-first injection, on-demand full loading`

### Task 7.4: 实现自动触发激活

- [ ] 基于TriggerCondition自动激活Skill：
  - 关键词匹配：消息中包含特定关键词
  - 意图匹配：基于信号类型/correlation上下文
  - 事件触发：特定类型Event发生时
- [ ] 激活冷却期（同一Skill在N分钟内不重复激活）
- [ ] 激活/停用产生Witness
- [ ] 单元测试：触发条件匹配、冷却期
- [ ] Commit: `feat(axiom-agent): automatic skill activation by triggers, cooldown period, activation Witness`

---

## P7 验收标准

| # | 验收项 |
|---|--------|
| 1 | SKILL.md格式解析正确 |
| 2 | SkillRegistry发现和依赖解析 |
| 3 | 渐进式披露token预算控制 |
| 4 | 自动触发激活工作 |
| 5 | cargo test -p axiom-agent 通过（累计≥22个测试） |

---

# Phase 8: 规则引擎 Implementation Plan

> **Goal:** 实现 Ruleset/Validator/Prompt注入/三层执行。规则违规可检测、可重试、可升级为Axiom。验收标准：规则在消息处理前后执行，违规可重试N次，反复违规的规则可通过进化引擎升级为Axiom。

---

### Task 8.1: 定义 Rule 类型

**Files:**
- Create: `crates/axiom-agent/src/rule.rs`

- [ ] 定义 RuleId、RuleSeverity（Info/Warn/Error/Critical）
- [ ] 定义 RuleStage：PreHandle（消息处理前检查输入）、PostHandle（处理后检查输出/副作用）、Periodic（定时检查）
- [ ] 定义 Rule trait：
  ```rust
  pub trait Rule: Send + Sync {
      fn id(&self) -> &'static str;
      fn stage(&self) -> RuleStage;
      fn severity(&self) -> RuleSeverity;
      fn check(&self, context: &RuleContext) -> RuleResult;
  }
  ```
- [ ] RuleContext 包含：incoming signal/current state/outgoing signals/witnesses
- [ ] RuleResult：Allow/Deny(reason)/Retry(after_ms)/Warn(message)
- [ ] 单元测试：RuleResult构造
- [ ] Commit: `feat(axiom-agent): Rule trait with PreHandle/PostHandle/Periodic stages, severity levels, RuleResult`

### Task 8.2: 实现 RulesEngine 和执行链

- [ ] RulesEngine：注册和执行规则
- [ ] PreHandle链：在Cell::handle前执行，Deny阻止处理，Retry延迟重试
- [ ] PostHandle链：在handle后执行，检查outgoing signals
- [ ] 重试机制：Retry(after_ms)延迟后重新投递消息，最多重试N次
- [ ] 重试耗尽后：Deny+Witness(AxiomViolated)
- [ ] 单元测试：PreHandle拒绝、PostHandle检测、重试逻辑
- [ ] Commit: `feat(axiom-agent): RulesEngine execution chains for PreHandle/PostHandle, retry with backoff`

### Task 8.3: 实现 Prompt 注入规则

- [ ] PromptRule：在Agent调用LLM前注入额外prompt指令
- [ ] 注入位置：system prompt尾部
- [ ] 基于Rule的priority排序注入顺序
- [ ] Prompt注入受token预算约束
- [ ] 单元测试：prompt注入顺序、token预算
- [ ] Commit: `feat(axiom-agent): PromptRule for LLM pre-call injection, priority ordering, token budget enforcement`

### Task 8.4: 规则违规升级为 Axiom

- [ ] 追踪每个Rule的触发频率
- [ ] 如果同一条Rule在1小时内被触发>10次且都是Deny/Error级别，自动建议升级为Axiom
- [ ] 升级建议发送给EvolutionGovernor作为ImprovementSignal(AxiomAddition)
- [ ] 这连接了规则引擎和自动进化引擎
- [ ] 单元测试：触发频率统计、升级建议生成
- [ ] Commit: `feat(axiom-agent): rule violation frequency tracking, automatic Axiom upgrade suggestions to evolution engine`

---

## P8 验收标准

| # | 验收项 |
|---|--------|
| 1 | Rule trait和三种stage执行正确 |
| 2 | Retry退避重试机制 |
| 3 | Prompt注入规则和token预算 |
| 4 | 高频违规升级为Axiom建议 |
| 5 | cargo test -p axiom-agent 通过（累计≥32个测试） |
