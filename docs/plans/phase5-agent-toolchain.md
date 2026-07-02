# Phase 5: Agent工具链

> **预估工期**: 6周
> **前置条件**: Phase 4 完成（MCP协议桥接）
> **后续阶段**: Phase 6 - 生产就绪

---

## 阶段目标

实现完整的 Agent 工具链，包括 LLM 客户端、工具调用框架、工作记忆、规划器、提示词模板和 Identity/Skill 系统。

---

## 任务清单

### Task 5.1: LLM客户端抽象

**描述**: 实现多模型支持的 LLM 客户端抽象层。

**涉及文件**:
- `crates/axiom-llm/src/client.rs`（新建）

**功能**:
- 多模型支持（OpenAI、Anthropic、Google等）
- Mock实现（测试用）
- 自动重试 + 指数退避
- 结构化输出（JSON Schema约束）
- Token预算管理

**API设计**:
```rust
trait LlmProvider {
    async fn complete(&self, prompt: &str) -> Result<CompletionResponse, LlmError>;
    async fn chat(&self, messages: &[ChatMessage]) -> Result<ChatResponse, LlmError>;
    async fn structured_output<T>(&self, prompt: &str, schema: &Schema) -> Result<T, LlmError>
        where T: DeserializeOwned;
}
```

**验收标准**:
- Mock模式测试通过
- 真实LLM调用产生Witness

---

### Task 5.2: 工具调用框架

**描述**: 实现类型安全的工具调用框架。

**涉及文件**:
- `crates/axiom-tool/src/tool.rs`（新建）
- `crates/axiom-tool/src/registry.rs`（新建）

**功能**:
- 类型安全工具定义
- 自动参数验证
- 权限控制
- Witness记录
- 工具组合

**API设计**:
```rust
#[tool]
fn read_file(path: String) -> Result<String, ToolError> {
    std::fs::read_to_string(&path)
}

struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    fn register<T: Tool>(&mut self, tool: T);
    fn call(&self, name: &str, args: Value) -> Result<Value, ToolError>;
}
```

**验收标准**:
- 工具调用完整链路测试通过

---

### Task 5.3: 工作记忆

**描述**: 实现工作记忆系统。

**涉及文件**:
- `crates/axiom-memory/src/memory.rs`（新建）

**功能**:
- Working Memory（当前任务短期记忆）
- 自动摘要（超长时自动生成摘要）
- Token预算感知（投影时考虑Token数）
- 记忆检索和更新

**API设计**:
```rust
struct WorkingMemory {
    items: Vec<MemoryItem>,
    token_budget: usize,
}

impl WorkingMemory {
    fn add(&mut self, item: MemoryItem);
    fn retrieve(&self, query: &str) -> Vec<MemoryItem>;
    fn summarize(&mut self) -> String;
}
```

**验收标准**:
- 记忆系统集成测试通过

---

### Task 5.4: 规划器

**描述**: 实现 ReAct 和 Plan-and-Execute 规划策略。

**涉及文件**:
- `crates/axiom-planner/src/planner.rs`（新建）

**功能**:
- ReAct策略（思考-行动循环）
- Plan-and-Execute策略（先规划后执行）
- 规划决策产生Witness
- 失败重试和修正

**API设计**:
```rust
trait Planner {
    async fn plan(&self, goal: &str) -> Result<Plan, PlannerError>;
    async fn execute(&self, plan: &Plan) -> Result<ExecutionResult, PlannerError>;
}

struct ReActPlanner {
    llm: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
}

struct PlanAndExecutePlanner {
    llm: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
}
```

**验收标准**:
- 规划器集成测试通过

---

### Task 5.5: 提示词模板

**描述**: 实现类型安全的提示词模板系统。

**涉及文件**:
- `crates/axiom-prompt/src/template.rs`（新建）

**功能**:
- 类型安全模板定义
- 模板组合
- 版本管理
- Persona注入

**API设计**:
```rust
#[prompt_template]
struct CodeReviewPrompt {
    #[persona]
    role: String,
    #[context]
    code: String,
    #[rules]
    rules: Vec<String>,
}

impl CodeReviewPrompt {
    fn render(&self) -> String;
}
```

**验收标准**:
- 模板渲染测试通过

---

### Task 5.6: Identity/Skill系统

**描述**: 实现身份挂载和技能激活系统。

**涉及文件**:
- `crates/axiom-identity/src/identity.rs`（新建）
- `crates/axiom-skill/src/skill.rs`（新建）

**功能**:
- Identity定义和挂载
- Skill注册和激活
- 渐进式披露（按需加载）
- 权限和规则绑定

**API设计**:
```rust
struct Identity {
    id: String,
    name: String,
    persona: Persona,
    capabilities: CapabilitySet,
    permissions: PermissionSet,
    skills: Vec<SkillId>,
    rules: RuleSet,
}

struct Skill {
    id: SkillId,
    name: String,
    metadata: SkillMetadata,
    instructions: String,
    tools: Vec<ToolId>,
    axioms: Vec<AxiomId>,
}
```

**验收标准**:
- Identity/Skill集成测试通过

---

## 质量门禁

```bash
# 每次任务完成后必须通过
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -D warnings
cargo build --workspace --all-targets
cargo test --workspace
```

---

## 阶段验收标准

- [ ] LLM客户端抽象实现
- [ ] 工具调用框架实现
- [ ] 工作记忆实现
- [ ] 规划器实现
- [ ] 提示词模板实现
- [ ] Identity/Skill系统实现
- [ ] 集成测试全部通过
- [ ] `cargo test --workspace` 全部通过

---

## 关键文件索引

| 文件 | 说明 |
|------|------|
| [crates/axiom-llm/src/client.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-llm/src/client.rs) | LLM客户端 |
| [crates/axiom-tool/src/tool.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-tool/src/tool.rs) | 工具定义 |
| [crates/axiom-memory/src/memory.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-memory/src/memory.rs) | 工作记忆 |
| [crates/axiom-planner/src/planner.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-planner/src/planner.rs) | 规划器 |
| [crates/axiom-prompt/src/template.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-prompt/src/template.rs) | 提示词模板 |
| [crates/axiom-identity/src/identity.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-identity/src/identity.rs) | Identity系统 |
| [crates/axiom-skill/src/skill.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-skill/src/skill.rs) | Skill系统 |
