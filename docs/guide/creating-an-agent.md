# 创建一个 Agent

本教程展示如何使用 `axiom-agent` crate 的 `AgentBuilder` 链式构建一个完整的智能体。你将学会配置 LLM 客户端、工具注册表、工作记忆、规划器、身份与技能，并运行一个可交互的示例。

`axiom-agent` 是一个聚合 crate（fascade），它把 LLM、Tool、Memory、Planner、Prompt、Identity 等组件整合进一个 `AgentCell`，让你无需手动拼装各个部件。

---

## 目录

- [架构总览](#架构总览)
- [添加依赖](#添加依赖)
- [AgentBuilder 链式构建](#agentbuilder-链式构建)
- [配置 LLM 客户端](#配置-llm-客户端)
- [配置工作记忆](#配置工作记忆)
- [配置工具注册表](#配置工具注册表)
- [配置身份与技能](#配置身份与技能)
- [配置规划器](#配置规划器)
- [配置提示模板](#配置提示模板)
- [完整可运行示例](#完整可运行示例)
- [运行与测试](#运行与测试)
- [生命周期与统计](#生命周期与统计)
- [下一步](#下一步)

---

## 架构总览

`AgentCell` 是一个集成体，内部组合了六大组件：

```text
┌─────────────────────────────────────────────────────┐
│                    AgentCell                         │
│  ┌───────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ Identity  │→ │ Prompt   │→ │     Planner      │  │
│  │ /Skill    │  │ Template │  │ (ReAct/PlanExec) │  │
│  └───────────┘  └──────────┘  └────────┬─────────┘  │
│                                         │            │
│  ┌───────────┐  ┌──────────┐  ┌────────▼─────────┐  │
│  │  Memory   │← │   LLM    │← │   Tool Registry  │  │
│  │ (Working) │  │  Client  │  │   (Execute)      │  │
│  └───────────┘  └──────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────┘
```

- **Identity**：Agent 是谁——身份、特征、语气、披露级别。
- **Skill**：Agent 会什么——按上下文激活的能力，绑定工具与提示片段。
- **Prompt Template**：结构化提示模板，支持变量替换。
- **Planner**：规划器，决定是直接调用 LLM 还是 ReAct/PlanAndExecute 多步推理。
- **LLM Client**：大模型客户端，支持多 provider 与 mock。
- **Tool Registry**：工具注册表，类型安全的工具调用。
- **Memory**：工作记忆，按 token 预算管理上下文。

> **注意**：`AgentCell` 位于 Layer 3（Agent 层），属于非确定性推理单元。它的输出在实际生产中应经过 Layer 2 验证层与 Axiom 校验后才下发到 Layer 1 执行层。

---

## 添加依赖

在你的项目 `Cargo.toml` 中添加：

```toml
[dependencies]
axiom-agent = "0.1"
axiom-llm = "0.1"
axiom-tool = "0.1"
axiom-identity = "0.1"
axiom-memory = "0.1"
axiom-planner = "0.1"
axiom-prompt = "0.1"

tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
async-trait = "0.1"
```

> **提示**：`axiom-agent` 已经 re-export 了所有上游 crate（`axiom_kernel`、`axiom_llm`、`axiom_tool` 等），你也可以只用 `axiom-agent` 一个依赖，通过 `axiom_agent::axiom_llm::LlmClient` 这样的路径访问。

---

## AgentBuilder 链式构建

`AgentBuilder` 是入口，采用链式 API。所有配置方法都消费并返回 `Self`，最终用 `build()` 或 `build_and_start()` 收尾。

```rust
use axiom_agent::{AgentBuilder, PlannerStrategy};

let agent = AgentBuilder::new("my-agent")
    .with_llm(axiom_llm::LlmClient::mock())
    .with_memory_budget(8000)
    .with_max_iterations(10)
    .with_planner_strategy(PlannerStrategy::ReAct)
    .build()?;
```

### 构建器方法一览

| 方法 | 作用 | 默认值 |
|------|------|--------|
| `new(id)` | 设置 Agent ID（必填） | — |
| `with_llm(client)` | 设置 LLM 客户端 | None（start 时报错） |
| `with_llm_arc(arc)` | 同上，但接受 `Arc<LlmClient>` | None |
| `with_memory_budget(n)` | 工作记忆 token 预算 | 4000 |
| `with_max_iterations(n)` | 规划器最大迭代次数 | 10 |
| `with_auto_summarize(bool)` | 是否自动摘要记忆 | true |
| `with_disclosure_level(level)` | 身份披露级别 | Basic |
| `with_planner_strategy(s)` | 规划策略 ReAct / PlanAndExecute | ReAct |
| `with_tools(registry)` | 设置工具注册表 | None |
| `with_planner(arc)` | 设置自定义规划器 | None |
| `with_prompt_registry(reg)` | 设置提示模板注册表 | None |
| `with_persona(persona)` | 设置身份+技能组合 | None |
| `with_identity(identity)` | 仅设置身份（会创建 persona） | None |
| `with_skill(skill)` | 添加一个技能到 persona | None |
| `with_config(config)` | 一次性设置全部配置 | Default |
| `build()` | 构建 AgentCell（未启动） | — |
| `build_and_start()` | 构建并立即启动 | — |

### 关键约束

- **LLM 客户端是必需的**：`build()` 本身不会报错，但 `start()` 会因缺少 LLM 返回 `AgentError::NotConfigured`。
- **`build_and_start()` = `build()` + `start()`**：一步到位，启动失败会立即返回错误。
- **`with_identity` 与 `with_persona`**：若已设置 persona，`with_identity` 会更新其内部身份；否则新建 persona。

---

## 配置 LLM 客户端

LLM 客户端抽象在 `axiom-llm` 中，支持多 provider 与 mock。本教程使用 `LlmClient::mock()` 以便无网络运行。

```rust
use axiom_llm::LlmClient;

// 测试/开发用 mock 客户端
let llm = LlmClient::mock();

let agent = AgentBuilder::new("dev-agent")
    .with_llm(llm)
    .build_and_start()?;
```

生产环境中，你可以接入真实 provider（OpenAI、Anthropic 等）。`LlmClient` 内置指数退避重试与 JSON Schema 结构化输出校验。若多个 Agent 共享同一客户端，用 `with_llm_arc` 传入 `Arc<LlmClient>`。

---

## 配置工作记忆

工作记忆（`WorkingMemory`）按 token 预算管理上下文，避免上下文爆炸。`AgentCell` 会自动把用户输入与 LLM 回复记入记忆。

```rust
let agent = AgentBuilder::new("mem-agent")
    .with_llm(LlmClient::mock())
    .with_memory_budget(8000)   // 8000 token 预算
    .with_auto_summarize(true)  // 超预算时自动摘要
    .build_and_start()?;

// 手动注入一条目标记忆
agent.remember(axiom_memory::MemoryItem::new(
    axiom_memory::MemoryItemType::Goal,
    "准确回答用户的问题",
));

// 查询记忆
let items = agent.memory_items();
let prompt = agent.memory_prompt(); // 渲染为提示文本
```

### 记忆项类型

```rust
pub enum MemoryItemType {
    System,     // 系统消息
    Goal,       // 目标
    Observation,// 观察（用户输入）
    Result,     // 结果（LLM 输出）
    // ...
}
```

---

## 配置工具注册表

工具注册表（`ToolRegistry`）提供类型安全的工具调用。每个工具实现 `Tool` trait，可用 `SimpleTool` 快速包装闭包。

```rust
use axiom_tool::{ToolRegistry, ToolInfo, ToolParameter, ToolError};
use axiom_tool::tool::SimpleTool;
use serde_json::{json, Value};

fn make_echo_tool() -> SimpleTool<impl Fn(&Value) -> Result<Value, ToolError>> {
    let info = ToolInfo {
        name: "echo".to_string(),
        description: "原样返回输入".to_string(),
        parameters: vec![ToolParameter {
            name: "message".to_string(),
            description: "要回显的消息".to_string(),
            required: true,
            schema: json!({ "type": "string" }),
        }],
        required_permission: None,
        version: "1.0.0".to_string(),
    };
    SimpleTool::new(info, |params| {
        let msg = params["message"].as_str().unwrap_or("");
        Ok(json!({ "echo": msg }))
    })
}

fn make_calculator_tool() -> SimpleTool<impl Fn(&Value) -> Result<Value, ToolError>> {
    let info = ToolInfo {
        name: "calculate".to_string(),
        description: "计算加法表达式".to_string(),
        parameters: vec![ToolParameter {
            name: "expression".to_string(),
            description: "如 2 + 3".to_string(),
            required: true,
            schema: json!({ "type": "string" }),
        }],
        required_permission: None,
        version: "1.0.0".to_string(),
    };
    SimpleTool::new(info, |params| {
        let expr = params["expression"].as_str().unwrap_or("0");
        let result = if expr.contains('+') {
            let parts: Vec<&str> = expr.split('+').collect();
            let a: f64 = parts[0].trim().parse().unwrap_or(0.0);
            let b: f64 = parts[1].trim().parse().unwrap_or(0.0);
            a + b
        } else {
            0.0
        };
        Ok(json!({ "result": result }))
    })
}

let registry = ToolRegistry::new();
registry.register(make_echo_tool());
registry.register(make_calculator_tool());

let agent = AgentBuilder::new("tool-agent")
    .with_llm(LlmClient::mock())
    .with_tools(registry)
    .build_and_start()?;

// 直接调用工具
let result = agent.execute_tool("calculate", &json!({ "expression": "2 + 3" })).await?;
assert_eq!(result["result"], 5.0);
```

`Tool::validate` 会自动校验必需参数是否存在，缺失则返回 `ToolError::InvalidParameters`。`required_permission` 字段可用于权限控制。

---

## 配置身份与技能

身份（`AgentIdentity`）定义 Agent 是谁；技能（`Skill`）定义 Agent 会什么，并按上下文激活。

### 身份

```rust
use axiom_identity::{AgentIdentity, DisclosureLevel};

let identity = AgentIdentity::new("agent-001", "AxiomBot")
    .with_description("一个严谨的代码助手")
    .with_system_prompt("你是一个专业的 Rust 开发助手。")
    .with_traits(vec!["严谨".to_string(), "简洁".to_string(), "准确".to_string()])
    .with_capabilities(vec!["代码生成".to_string(), "代码审查".to_string()])
    .with_tone("professional")
    .with_disclosure_level(DisclosureLevel::Full);
```

### 技能

技能有激活条件，只在相关上下文下生效，避免提示膨胀：

```rust
use axiom_identity::{Skill, ActivationCondition};

let coding_skill = Skill::new("coding", "代码助手")
    .with_description("帮助编写和审查代码")
    .with_activation(ActivationCondition::KeywordTrigger(vec![
        "code".to_string(),
        "代码".to_string(),
        "rust".to_string(),
    ]))
    .with_tools(vec!["echo".to_string(), "calculate".to_string()])
    .with_prompt_fragments(vec![
        "你可以编写 Rust 代码。".to_string(),
        "回答时附带代码示例。".to_string(),
    ]);

let agent = AgentBuilder::new("skill-agent")
    .with_llm(LlmClient::mock())
    .with_identity(identity)
    .with_skill(coding_skill)
    .build_and_start()?;

// 当用户提到 "代码" 时，技能自动激活，对应工具变为可用
agent.query("帮我写一段代码").await?;
let tools = agent.available_tools(); // 包含 "echo"、"calculate"
```

### 激活条件类型

```rust
pub enum ActivationCondition {
    Always,
    Never,
    KeywordTrigger(Vec<String>),  // 关键词触发
    ContextMatch(String),         // 上下文匹配
    UserRequest,                  // 用户主动请求
    Schedule(String),             // 定时（计划中）
    And(Vec<ActivationCondition>),
    Or(Vec<ActivationCondition>),
    Not(Box<ActivationCondition>),
}
```

### 披露级别

`DisclosureLevel` 控制 persona 提示中暴露多少身份信息，用于渐进式披露：

| 级别 | 暴露内容 |
|------|---------|
| Minimal | 仅名字 |
| Basic | 名字 + 角色 + 语气 |
| Full | + 特征 + 能力 |
| Transparent | + 身份 ID + 披露级别本身 |

---

## 配置规划器

规划器决定 Agent 如何处理查询：直接单次 LLM 调用，还是多步推理。

```rust
use axiom_agent::PlannerStrategy;
use std::sync::Arc;

// 方式一：用策略字符串，由 AgentCell 自动构造
let agent = AgentBuilder::new("react-agent")
    .with_llm(LlmClient::mock())
    .with_planner_strategy(PlannerStrategy::ReAct)
    .build_and_start()?;

// 方式二：传入自定义规划器实例
let planner = Arc::new(axiom_planner::ReActPlanner::new().with_max_iterations(5));
let agent = AgentBuilder::new("custom-planner-agent")
    .with_llm(LlmClient::mock())
    .with_planner(planner)
    .build_and_start()?;
```

### 两种策略

| 策略 | 说明 | 适用场景 |
|------|------|---------|
| `ReAct` | 思考-行动-观察循环，逐步推理 | 需要多步工具调用的复杂任务 |
| `PlanAndExecute` | 先制定完整计划再执行 | 步骤较确定、可提前规划的任务 |

若不设置规划器，`AgentCell::query` 会退化为直接 LLM 调用。规划器失败时会自动 fallback 到直接 LLM 调用并记录错误。

---

## 配置提示模板

提示模板注册表让你用命名模板 + 变量渲染提示，避免字符串拼接：

```rust
use axiom_prompt::{PromptTemplate, TemplateVariable, VariableType};
use std::collections::HashMap;
use serde_json::json;

let mut registry = axiom_prompt::registry::TemplateRegistry::new();

let template = PromptTemplate::new("greeting", "你好，{{name}}！我是 {{agent_name}}。")
    .with_variable(TemplateVariable::new("name", VariableType::String))
    .with_variable(TemplateVariable::new("agent_name", VariableType::String));

registry.register(template)?;

let agent = AgentBuilder::new("prompt-agent")
    .with_llm(LlmClient::mock())
    .with_prompt_registry(registry)
    .build_and_start()?;

let mut values = HashMap::new();
values.insert("name".to_string(), json!("Alice"));
values.insert("agent_name".to_string(), json!("AxiomBot"));

let rendered = agent.render_template("greeting", &values)?;
assert_eq!(rendered, "你好，Alice！我是 AxiomBot。");
```

模板支持版本管理，`render_latest` 总是渲染最新版本，便于 A/B 测试提示词。

---

## 完整可运行示例

下面把所有组件整合成一个完整可运行的 Agent：

```rust
//! 完整 Agent 示例：LLM + Tool + Memory + Identity + Skill + Planner + Prompt

use axiom_agent::{AgentBuilder, PlannerStrategy};
use axiom_identity::{AgentIdentity, DisclosureLevel, Skill, ActivationCondition};
use axiom_llm::LlmClient;
use axiom_memory::{MemoryItem, MemoryItemType};
use axiom_prompt::{PromptTemplate, TemplateVariable, VariableType};
use axiom_tool::{ToolRegistry, ToolInfo, ToolParameter, ToolError};
use axiom_tool::tool::SimpleTool;
use serde_json::{json, Value};
use std::collections::HashMap;

fn make_echo_tool() -> SimpleTool<impl Fn(&Value) -> Result<Value, ToolError>> {
    let info = ToolInfo {
        name: "echo".to_string(),
        description: "原样返回输入".to_string(),
        parameters: vec![ToolParameter {
            name: "message".to_string(),
            description: "要回显的消息".to_string(),
            required: true,
            schema: json!({ "type": "string" }),
        }],
        required_permission: None,
        version: "1.0.0".to_string(),
    };
    SimpleTool::new(info, |params| {
        let msg = params["message"].as_str().unwrap_or("");
        Ok(json!({ "echo": msg }))
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // 1. 准备工具
    let tool_registry = ToolRegistry::new();
    tool_registry.register(make_echo_tool());

    // 2. 准备身份
    let identity = AgentIdentity::new("full-001", "AxiomBot")
        .with_description("一个全功能代码助手")
        .with_tone("professional")
        .with_traits(vec!["严谨".to_string(), "简洁".to_string()])
        .with_capabilities(vec!["echo".to_string()]);

    // 3. 准备技能
    let coding_skill = Skill::new("coding", "代码助手")
        .with_description("帮助编写代码")
        .with_activation(ActivationCondition::KeywordTrigger(vec![
            "code".to_string(),
            "代码".to_string(),
        ]))
        .with_tools(vec!["echo".to_string()])
        .with_prompt_fragments(vec!["你可以编写代码。".to_string()]);

    // 4. 准备提示模板
    let mut prompt_registry = axiom_prompt::registry::TemplateRegistry::new();
    let tpl = PromptTemplate::new("intro", "我是 {{name}}，{{role}}。")
        .with_variable(TemplateVariable::new("name", VariableType::String))
        .with_variable(TemplateVariable::new("role", VariableType::String));
    prompt_registry.register(tpl)?;

    // 5. 链式构建并启动
    let agent = AgentBuilder::new("full-agent")
        .with_llm(LlmClient::mock())
        .with_tools(tool_registry)
        .with_identity(identity)
        .with_skill(coding_skill)
        .with_prompt_registry(prompt_registry)
        .with_memory_budget(4000)
        .with_max_iterations(5)
        .with_auto_summarize(true)
        .with_disclosure_level(DisclosureLevel::Full)
        .with_planner_strategy(PlannerStrategy::ReAct)
        .build_and_start()?;

    println!("Agent started: {}", agent.id());

    // 6. 注入初始记忆
    agent.remember(MemoryItem::new(
        MemoryItemType::Goal,
        "准确回答用户的代码问题",
    ));

    // 7. 调用工具
    let tool_result = agent
        .execute_tool("echo", &json!({ "message": "hello" }))
        .await?;
    println!("Tool result: {}", tool_result);

    // 8. 查询 Agent
    let response = agent.query("帮我写一段代码").await?;
    println!("Agent response: {}", response);

    // 9. 渲染模板
    let mut values = HashMap::new();
    values.insert("name".to_string(), json!("AxiomBot"));
    values.insert("role".to_string(), json!("代码助手"));
    let rendered = agent.render_template("intro", &values)?;
    println!("Rendered: {}", rendered);

    // 10. 查看统计
    let stats = agent.stats();
    println!("Stats: queries={}, tools={}, llm_calls={}",
        stats.queries_processed, stats.tools_executed, stats.llm_calls);

    // 11. 优雅停止
    agent.stop()?;
    println!("Agent stopped.");
    Ok(())
}
```

---

## 运行与测试

### 运行示例

把上面的完整代码放入 `src/main.rs`，确保 `Cargo.toml` 依赖齐全，然后：

```bash
cargo run
```

预期输出（节选，mock 客户端的实际回复可能不同）：

```
Agent started: full-agent
Tool result: {"echo":"hello"}
Agent response: ...
Rendered: 我是 AxiomBot，代码助手。
Stats: queries=1, tools=1, llm_calls=1
Agent stopped.
```

### 编写测试

`axiom-agent` 的测试通常用 `LlmClient::mock()` 避免真实网络调用。下面是一个典型的集成测试范式（参考仓库内 `crates/axiom-agent/tests/integration.rs`）：

```rust
use axiom_agent::*;
use axiom_llm::LlmClient;

#[tokio::test]
async fn test_agent_basic_query() {
    let agent = AgentBuilder::new("test-agent")
        .with_llm(LlmClient::mock())
        .with_memory_budget(2000)
        .build_and_start()
        .unwrap();

    let response = agent.query("Hello, who are you?").await;
    assert!(response.is_ok());
    let resp = response.unwrap();
    assert!(!resp.is_empty());

    let stats = agent.stats();
    assert_eq!(stats.queries_processed, 1);
    assert!(stats.llm_calls > 0);
}

#[tokio::test]
async fn test_agent_not_started_error() {
    let agent = AgentBuilder::new("test-agent")
        .with_llm(LlmClient::mock())
        .build()
        .unwrap();

    let result = agent.query("Hello").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AgentError::NotStarted));
}

#[tokio::test]
async fn test_agent_query_without_llm() {
    let agent = AgentCell::new("no-llm", AgentConfig::default());
    let result = agent.start();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AgentError::NotConfigured(_)));
}
```

### 运行测试

```bash
# 运行 axiom-agent 的全部测试
cargo test -p axiom-agent

# 只运行集成测试
cargo test -p axiom-agent --test integration

# 显示 println! 输出
cargo test -p axiom-agent -- --nocapture
```

---

## 生命周期与统计

### 生命周期

`AgentCell` 有显式的 start/stop 生命周期：

```rust
let agent = AgentBuilder::new("lc-agent")
    .with_llm(LlmClient::mock())
    .build()?;

assert!(!agent.is_started());

agent.start()?;           // 启动（注入 persona 到记忆）
assert!(agent.is_started());

assert!(agent.start().is_err()); // 重复启动报错 AlreadyStarted

agent.stop()?;            // 优雅停止，打印统计日志
assert!(!agent.is_started());

assert!(agent.stop().is_err());  // 重复停止报错 NotStarted
```

| 状态 | 方法 | 错误 |
|------|------|------|
| 未启动 → 已启动 | `start()` | 缺 LLM 报 `NotConfigured`；已启动报 `AlreadyStarted` |
| 已启动 → 未启动 | `stop()` | 未启动报 `NotStarted` |
| 已启动 → 查询 | `query()` | 未启动报 `NotStarted` |

### 统计

`AgentStats` 跟踪运行时指标，适合接入监控：

```rust
pub struct AgentStats {
    pub queries_processed: u64,   // 处理的查询数
    pub tools_executed: u64,      // 执行的工具数
    pub llm_calls: u64,           // LLM 调用次数
    pub plans_executed: u64,      // 规划执行次数
    pub errors: u64,              // 错误数
    pub total_duration_ms: u64,   // 总耗时
}
```

```rust
let stats = agent.stats();
println!("平均耗时: {}ms", stats.total_duration_ms / stats.queries_processed.max(1));
println!("错误率: {:.1}%", 
    stats.errors as f64 / stats.queries_processed as f64 * 100.0);
```

### 与核心原语的衔接

`AgentCell` 是 Layer 3 的推理单元，在生产架构中应与核心原语协作：

```
用户输入
   │
   ▼
AgentCell (Layer 3) ── LLM 推理，产出意图/命令
   │
   ▼ Signal (Command)
Validate (Layer 2) ── Schema + Axiom 校验
   │
   ▼ 校验通过
Exec Cell (Layer 1) ── 确定性执行（DB/API/IO）
   │
   ▼ Witness
审计链 + 熵监控
```

`AgentCell` 本身的 `query`/`execute_tool` 是同步入口，但你可以把它包装成一个 `Cell`，让它的输出 Signal 进入四层架构的校验与执行流水线。相关模式参见 [最佳实践](./best-practices.md)。

---

## 下一步

- **[最佳实践](./best-practices.md)**：学习如何把 AgentCell 接入四层架构、性能优化、安全实践与测试策略。
- **[核心概念](./core-concepts.md)**：若尚未阅读，建议先理解 Cell/Signal/Axiom/Witness/Lens 与层间约束。
- **仓库测试**：`crates/axiom-agent/tests/integration.rs` 包含十余个端到端测试，是最好的学习素材。

如果需要接入真实 LLM provider 或自定义 Planner，请参考 `axiom-llm` 与 `axiom-planner` 的 crate 文档。
