# Axiom Agent 完整使用指南

## 目录

- [快速开始](#快速开始)
- [核心概念](#核心概念)
- [AutoAgent - 全自动模式](#autoagent---全自动模式)
- [AgentBuilder - 手动配置模式](#agentbuilder---手动配置模式)
- [AgentManifest - 声明式配置](#agentmanifest---声明式配置)
- [意图路由系统](#意图路由系统)
- [自我监控系统](#自我监控系统)
- [自然语言信号](#自然语言信号)
- [记忆系统](#记忆系统)
- [工具调用](#工具调用)
- [插件系统](#插件系统)
- [架构约束](#架构约束)
- [常见问题](#常见问题)
- [API 参考](#api-参考)

---

## 快速开始

### 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
axiom-agent = "0.4"
```

### 最简使用（全自动模式）

```rust
use axiom_agent::AutoAgent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建智能体（一行代码）
    let agent = AutoAgent::new("my-assistant")?
        .with_llm(axiom_llm::LlmClient::mock())?;

    // 2. 使用（自动启动 + 自动路由 + 自动修复）
    let response = agent.process("Hello, who are you?").await?;
    println!("{}", response);

    // 3. 查看健康状态
    let report = agent.health_report();
    println!("Health: {:?}", report.health_status);

    Ok(())
}
```

---

## 核心概念

### 架构分层

Axiom 采用四层架构，智能体运行在 Agent 层：

```
┌─────────────────────────────────────┐
│  Layer 0  CLI / Applications        │  用户接口
├─────────────────────────────────────┤
│  Layer 3  Oversight                 │  监督治理
├─────────────────────────────────────┤
│  Layer 2  Agent                     │  ← 智能体在这里
├─────────────────────────────────────┤
│  Layer 1  Validate                  │  验证守卫
├─────────────────────────────────────┤
│  Layer 0  Exec                      │  执行层
└─────────────────────────────────────┘
```

**层间调用规则**：只能向下或同层调用，编译期强制检查。

### 核心组件

| 组件 | 说明 | 文件 |
|------|------|------|
| `AutoAgent` | 全自动智能体，零配置即用 | [auto_agent.rs](file:///d:/work/trae/axiom-core-project/crates/axiom-agent/src/auto_agent.rs) |
| `AgentCell` | 智能体核心单元 | [agent.rs](file:///d:/work/trae/axiom-core-project/crates/axiom-agent/src/agent.rs) |
| `AgentBuilder` | 构建器，灵活配置 | [builder.rs](file:///d:/work/trae/axiom-core-project/crates/axiom-agent/src/builder.rs) |
| `AgentManifest` | 声明式配置（YAML/JSON） | [agent_manifest.rs](file:///d:/work/trae/axiom-core-project/crates/axiom-agent/src/agent_manifest.rs) |
| `IntentRouter` | 意图路由器 | [intent_router.rs](file:///d:/work/trae/axiom-core-project/crates/axiom-agent/src/intent_router.rs) |
| `SelfMonitor` | 自我监控器 | [self_monitor.rs](file:///d:/work/trae/axiom-core-project/crates/axiom-agent/src/self_monitor.rs) |
| `NaturalSignal` | 自然语言信号 | [natural_signal.rs](file:///d:/work/trae/axiom-core-project/crates/axiom-agent/src/natural_signal.rs) |

---

## AutoAgent - 全自动模式

### 特点

- **零配置**：创建即用，无需手动设置
- **自动启动**：首次调用自动启动
- **自动发现**：自动注册常用意图路由
- **自动路由**：自动检测意图并分发
- **自动监控**：持续监控健康状态
- **自动修复**：出错自动恢复
- **自动调优**：定期优化参数

### 创建

```rust
use axiom_agent::AutoAgent;

// 最简创建
let agent = AutoAgent::new("assistant")?;

// 带 LLM
let agent = AutoAgent::new("assistant")?
    .with_llm(LlmClient::new("gpt-4"))?;

// 设置运行模式
agent.with_mode(AutoMode::Balanced);  // 平衡模式（默认）
agent.with_mode(AutoMode::Conservative);  // 保守模式
agent.with_mode(AutoMode::Aggressive);  // 激进模式
```

### 使用

```rust
// 处理自然语言输入
let response = agent.process("What is Rust?").await?;

// 处理 NaturalSignal
use axiom_agent::NaturalSignal;
let signal = NaturalSignal::new("Explain Rust ownership")
    .with_intent("explain", 0.9);
let response = agent.process_natural(signal).await?;
```

### 健康监控

```rust
let report = agent.health_report();

// 健康状态
match report.health_status {
    HealthStatus::Healthy => println!("✅ 健康"),
    HealthStatus::Degraded => println!("⚠️ 轻微降级"),
    HealthStatus::Unhealthy => println!("❌ 不健康"),
    HealthStatus::Critical => println!("🔥 危急"),
}

// 性能指标
println!("平均响应: {:.0}ms", report.performance_metrics.avg_response_time_ms);
println!("错误率: {:.2}%", report.performance_metrics.error_rate * 100.0);

// 置信度统计
println!("平均置信度: {:.2}", report.confidence_summary.avg_confidence);
println!("低置信度次数: {}", report.confidence_summary.low_confidence_count);
println!("置信度趋势: {:?}", report.confidence_summary.confidence_trend);

// 改进建议
for suggestion in &report.suggested_actions {
    println!("[优先级 {}] {}", suggestion.priority, suggestion.action);
    println!("  原因: {}", suggestion.reason);
    println!("  预期改进: {:.0}%", suggestion.expected_improvement * 100.0);
}
```

### 生命周期管理

```rust
// 启动（通常不需要手动调用，首次 process 自动启动）
agent.start().await?;

// 停止
agent.stop().await?;

// 检查状态
if agent.is_running() {
    println!("Agent is running");
}
```

### 自定义自动配置

```rust
use axiom_agent::{AutoConfig, AutoMode};

let config = AutoConfig {
    auto_start: true,              // 自动启动
    auto_discover: true,           // 自动发现能力
    auto_tune: true,               // 自动调优
    auto_heal: true,               // 自动修复
    auto_evolve: false,            // 自动进化（预留）
    mode: AutoMode::Balanced,      // 运行模式
    health_check_interval_secs: 30, // 健康检查间隔
    tune_interval_interactions: 100, // 调优间隔（交互次数）
    min_confidence_threshold: 0.5,  // 最低置信度阈值
};

agent.with_auto_config(config);
```

### 自愈机制

| 健康状态 | 自愈策略 |
|---------|---------|
| Healthy | 无操作 |
| Degraded | 重置监控统计 |
| Unhealthy | 停止 → 等待 500ms → 重启 |
| Critical | 停止 → 完全重置 → 等待 2s → 重启 |

---

## AgentBuilder - 手动配置模式

### 基础构建

```rust
use axiom_agent::AgentBuilder;

let agent = AgentBuilder::new("my-agent")
    .with_llm(LlmClient::new("gpt-4"))
    .with_memory_budget(8000)
    .with_auto_summarize(true)
    .with_planner_strategy(PlannerStrategy::PlanAndExecute)
    .with_max_iterations(15)
    .build()?;

agent.start()?;
```

### 完整配置示例

```rust
use axiom_agent::{
    AgentBuilder, IntentRouter, IntentRoute, SelfMonitor,
    PlannerStrategy,
};
use axiom_identity::{AgentIdentity, AgentPersona, Skill};
use axiom_memory::WorkingMemory;
use axiom_tool::ToolRegistry;

// 1. 创建意图路由器
let mut router = IntentRouter::new("agent:my-agent");
router.add_route(IntentRoute {
    intent_pattern: "contains:search".to_string(),
    target_cell_id: "exec:search-tool".to_string(),
    confidence_threshold: 0.6,
    priority: 1,
});
router.add_route(IntentRoute {
    intent_pattern: "*".to_string(),
    target_cell_id: "agent:my-agent".to_string(),
    confidence_threshold: 0.3,
    priority: 0,
});

// 2. 创建自我监控器
let monitor = SelfMonitor::new("my-agent");

// 3. 创建身份
let identity = AgentIdentity::new("agent-001", "My Agent");
let mut persona = AgentPersona::new(identity);
persona.add_skill(Skill::new("coding", "Writing code", 0.9));
persona.add_skill(Skill::new("analysis", "Analyzing data", 0.85));

// 4. 构建智能体
let agent = AgentBuilder::new("my-agent")
    .with_llm(LlmClient::new("gpt-4"))
    .with_memory_budget(16000)
    .with_auto_summarize(true)
    .with_planner_strategy(PlannerStrategy::ReAct)
    .with_max_iterations(20)
    .with_timeout_secs(120)
    .with_intent_router(router)
    .with_self_monitor(monitor)
    .with_persona(persona)
    .build()?;

agent.start()?;
```

### 构建器方法速查

| 方法 | 说明 | 默认值 |
|------|------|--------|
| `new(id)` | 创建构建器 | - |
| `with_llm(client)` | 设置 LLM 客户端 | None |
| `with_memory_budget(n)` | 设置记忆 token 预算 | 4000 |
| `with_auto_summarize(b)` | 启用自动摘要 | false |
| `with_planner_strategy(s)` | 设置规划策略 | ReAct |
| `with_max_iterations(n)` | 最大迭代次数 | 10 |
| `with_timeout_secs(n)` | 超时时间（秒） | 60 |
| `with_intent_router(r)` | 设置意图路由器 | 默认路由 |
| `with_self_monitor(m)` | 设置自我监控器 | 默认监控 |
| `with_persona(p)` | 设置角色身份 | None |
| `with_skill(s)` | 添加技能 | - |
| `build()` | 构建 AgentCell | - |

---

## AgentManifest - 声明式配置

### YAML 配置示例

创建 `agent.yaml`：

```yaml
id: "research-assistant"
name: "Research Assistant"
version: "1.0.0"
description: "A research assistant that helps find and summarize information"
role: "Research and information gathering expert"

instructions: |
  You are a research assistant. You help users find information,
  summarize documents, and answer questions with cited sources.
  Always be thorough and cite your sources.

capabilities:
  - name: "web_search"
    description: "Search the web for information"
    required: true
    activation:
      Intent: "web_search"
    parameters:
      - name: "query"
        description: "The search query"
        type: "string"
        required: true

  - name: "summarize"
    description: "Summarize long documents"
    required: false
    activation:
      Keyword: "summarize"
    parameters:
      - name: "text"
        description: "The text to summarize"
        type: "string"
        required: true

  - name: "explain"
    description: "Explain complex topics"
    required: true
    activation:
      Context: "explanation"
    parameters:
      - name: "topic"
        description: "The topic to explain"
        type: "string"
        required: true

dependencies:
  - name: "web-search"
    version: "1.0.0"
    source: "Plugin(wasm)"

memory_config:
  max_tokens: 8000
  auto_summarize: true
  retention_policy: "hybrid"
  recall_strategy: "hybrid"

planner_config:
  strategy: "PlanAndExecute"
  max_iterations: 15
  timeout_seconds: 120
  replan_on_failure: true

constraints:
  forbidden_actions:
    - "delete_file"
    - "send_email"
  required_actions:
    - "cite_sources"
  max_tool_calls: 20
  min_confidence: 0.6
  disclosure_level: "detailed"
```

### 从配置创建

```rust
use axiom_agent::{AgentBuilder, AgentManifest};

// 从 YAML 文件加载
let yaml = std::fs::read_to_string("agent.yaml")?;
let manifest = AgentManifest::from_yaml(&yaml)?;

// 从 manifest 创建智能体
let agent = AgentBuilder::from_manifest(&manifest)
    .with_llm(LlmClient::new("gpt-4"))
    .build()?;

agent.start()?;
```

### JSON 配置

```rust
// 从 JSON 加载
let json = r#"{
    "id": "my-agent",
    "name": "My Agent",
    "version": "1.0.0",
    "role": "Assistant"
}"#;

let manifest = AgentManifest::from_json(json)?;

// 导出为 JSON
let json = manifest.to_json()?;
```

### 激活条件

| 类型 | 说明 | 示例 |
|------|------|------|
| `Always` | 始终激活 | `Always` |
| `Intent` | 匹配意图时激活 | `Intent: "web_search"` |
| `Keyword` | 包含关键词时激活 | `Keyword: "summarize"` |
| `Context` | 上下文匹配时激活 | `Context: "explanation"` |
| `Never` | 永不激活 | `Never` |

### 保留策略

| 策略 | 说明 |
|------|------|
| `Recent` | 保留最近的记忆 |
| `Importance` | 按重要性保留 |
| `Semantic` | 按语义相关性保留 |
| `Hybrid` | 混合策略（默认） |

### 召回策略

| 策略 | 说明 |
|------|------|
| `Exact` | 精确匹配 |
| `Semantic` | 语义匹配 |
| `Fuzzy` | 模糊匹配 |
| `Hybrid` | 混合策略（默认） |

### 披露级别

| 级别 | 说明 |
|------|------|
| `None` | 不披露内部信息 |
| `Basic` | 基本信息（默认） |
| `Detailed` | 详细信息 |
| `Full` | 完全披露 |

---

## 意图路由系统

### 工作原理

```
用户输入 → 意图检测 → 置信度评估 → 路由决策 → 目标 Cell
```

### 创建路由器

```rust
use axiom_agent::{IntentRouter, IntentRoute, RoutingDecision};

let mut router = IntentRouter::new("agent:default");
```

### 添加路由规则

```rust
// 精确匹配
router.add_route(IntentRoute {
    intent_pattern: "web_search".to_string(),
    target_cell_id: "exec:search".to_string(),
    confidence_threshold: 0.6,
    priority: 1,
});

// 包含匹配
router.add_route(IntentRoute {
    intent_pattern: "contains:search".to_string(),
    target_cell_id: "exec:search".to_string(),
    confidence_threshold: 0.5,
    priority: 1,
});

// 前缀匹配
router.add_route(IntentRoute {
    intent_pattern: "prefix:calc_".to_string(),
    target_cell_id: "exec:calculator".to_string(),
    confidence_threshold: 0.5,
    priority: 1,
});

// 正则匹配
router.add_route(IntentRoute {
    intent_pattern: "regex:^[0-9]+[+\\-*/][0-9]+$".to_string(),
    target_cell_id: "exec:calculator".to_string(),
    confidence_threshold: 0.7,
    priority: 2,
});

// 通配符（兜底）
router.add_route(IntentRoute {
    intent_pattern: "*".to_string(),
    target_cell_id: "agent:default".to_string(),
    confidence_threshold: 0.3,
    priority: 0,
});
```

### 路由查询

```rust
let result = router.route("web_search", 0.85);

match result.routing_decision {
    RoutingDecision::Routed => {
        println!("路由到: {}", result.target_cell_id.unwrap());
        println!("匹配意图: {}", result.matched_intent);
        println!("置信度: {:.2}", result.confidence);
    }
    RoutingDecision::Direct => {
        println!("直接处理");
    }
    RoutingDecision::Ambiguous => {
        println!("意图模糊，需要澄清");
    }
    RoutingDecision::Rejected => {
        println!("请求被拒绝");
    }
}
```

### 路由管理

```rust
// 批量添加
router.add_routes(vec![
    IntentRoute { ... },
    IntentRoute { ... },
]);

// 移除路由
router.remove_route("contains:search");

// 清空所有路由
router.clear();

// 路由数量
let count = router.route_count();
```

---

## 自我监控系统

### 创建监控器

```rust
use axiom_agent::SelfMonitor;

let monitor = SelfMonitor::new("my-agent");
```

### 记录交互

```rust
monitor.record_interaction(
    true,      // 是否成功
    250,       // 响应时间（毫秒）
    0.85,      // 置信度
    "search",  // 意图
);
```

### 生成报告

```rust
let report = monitor.generate_report();

// 健康状态
println!("Health: {:?}", report.health_status);

// 性能指标
println!("Avg response: {:.0}ms", report.performance_metrics.avg_response_time_ms);
println!("Error rate: {:.2}%", report.performance_metrics.error_rate * 100.0);
println!("Throughput: {:.2}/s", report.performance_metrics.throughput);
println!("Memory usage: {:.1}%", report.performance_metrics.memory_usage_percent);

// 行为统计
println!("Total interactions: {}", report.behavior_summary.total_interactions);
println!("Successful: {}", report.behavior_summary.successful_interactions);
println!("Failed: {}", report.behavior_summary.failed_interactions);

// 置信度分析
println!("Avg confidence: {:.2}", report.confidence_summary.avg_confidence);
println!("Low confidence count: {}", report.confidence_summary.low_confidence_count);
println!("High confidence count: {}", report.confidence_summary.high_confidence_count);
println!("Confidence trend: {:?}", report.confidence_summary.confidence_trend);

// 改进建议
for action in &report.suggested_actions {
    println!("[P{}] {} - {:.0}% improvement", 
        action.priority, action.action, action.expected_improvement * 100.0);
}
```

### 健康状态分级

| 级别 | 触发条件 | 建议动作 |
|------|---------|---------|
| **Healthy** | 错误率 < 15% 且 置信度 > 50% | 正常运行 |
| **Degraded** | 错误率 15-30% 或 置信度 35-50% | 关注并调整 |
| **Unhealthy** | 错误率 30-50% 或 置信度 20-35% | 需要修复 |
| **Critical** | 错误率 > 50% 或 置信度 < 20% | 紧急处理 |

### 置信度趋势

| 趋势 | 说明 |
|------|------|
| `Increasing` | 置信度上升 |
| `Stable` | 置信度稳定（默认） |
| `Decreasing` | 置信度下降 |

### 重置

```rust
monitor.reset();
```

---

## 自然语言信号

### 创建信号

```rust
use axiom_agent::NaturalSignal;

// 基础创建
let signal = NaturalSignal::new("Hello, how are you?");

// 带意图
let signal = NaturalSignal::new("Search for Rust tutorials")
    .with_intent("web_search", 0.92);

// 带实体
let signal = NaturalSignal::new("Find papers by John Smith about AI")
    .with_entity("author", "person", "John Smith", 0.85)
    .with_entity("topic", "topic", "AI", 0.9);

// 带上下文
let signal = NaturalSignal::new("Summarize this document")
    .with_context("document_id", "doc-123")
    .with_context("language", "en");

// 带追踪
use axiom_kernel::id::TraceId;
let signal = NaturalSignal::new("...")
    .with_trace(TraceId::new("trace-001".to_string()));

// 带附件
let signal = NaturalSignal::new("Analyze this code")
    .with_attachment("source", "text/plain", "fn main() {}");
```

### 信号字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `msg_id` | `MsgId` | 消息唯一 ID |
| `correlation_id` | `CorrelationId` | 关联 ID，用于追踪调用链 |
| `trace_id` | `Option<TraceId>` | 追踪 ID，全链路追踪 |
| `vector_clock` | `VectorClock` | 向量时钟，因果关系 |
| `intent` | `String` | 检测到的意图 |
| `confidence` | `f64` | 意图置信度 |
| `entities` | `Vec<Entity>` | 提取的实体 |
| `context` | `HashMap<String, String>` | 上下文键值对 |
| `content` | `String` | 原始文本内容 |
| `attachments` | `Vec<Attachment>` | 附件 |

### 实体结构

```rust
pub struct Entity {
    pub name: String,         // 实体名称
    pub r#type: String,       // 实体类型
    pub value: String,        // 实体值
    pub confidence: f64,      // 识别置信度
}
```

### 附件结构

```rust
pub struct Attachment {
    pub name: String,         // 附件名称
    pub r#type: String,       // MIME 类型
    pub content: String,      // 内容（文本或 Base64）
}
```

---

## 记忆系统

### 添加记忆

```rust
use axiom_memory::MemoryItem;

// 观察记忆
agent.remember(MemoryItem::observation("User likes concise answers"));

// 结果记忆
agent.remember(MemoryItem::result("The answer is 42"));

// 计划记忆
agent.remember(MemoryItem::plan("Step 1: Search, Step 2: Summarize"));

// 反思记忆
agent.remember(MemoryItem::reflection("Should have asked for clarification first"));
```

### 查询记忆

```rust
// 获取所有记忆
let items = agent.memory_items();

// 获取记忆提示（用于 LLM）
let prompt = agent.memory_prompt();
```

### 记忆配置

```rust
AgentBuilder::new("agent")
    .with_memory_budget(8000)       // token 预算
    .with_auto_summarize(true)       // 自动摘要
```

---

## 工具调用

### 执行工具

```rust
let result = agent.execute_tool(
    "web_search",
    &serde_json::json!({"query": "Rust programming"}),
).await?;

println!("Tool result: {}", result);
```

### 查看可用工具

```rust
let tools = agent.available_tools();
for tool in tools {
    println!("{} - {}", tool.name, tool.description);
}
```

### 工具注册表

```rust
use axiom_tool::ToolRegistry;

let mut registry = ToolRegistry::new();
registry.register(my_tool);
```

---

## 插件系统

### WASM 插件

```rust
use axiom_kernel::plugin::loader::NativePluginLoader;
use axiom_kernel::plugin::registry::PluginRegistry;

let registry = PluginRegistry::new();

// 加载 WASM 插件
let plugin = NativePluginLoader::load_wasm("plugins/web-search.wasm")?;
registry.register(plugin)?;

// 注册到智能体
agent.register_plugin("web-search")?;
```

### 插件配置

插件目录下的 `plugin.yaml`：

```yaml
id: "web-search"
name: "Web Search Plugin"
version: "1.0.0"
description: "Adds web search capability"
capabilities:
  - name: "web_search"
    description: "Search the web"
    parameters:
      - name: "query"
        type: "string"
        required: true
```

---

## 架构约束

### 层间调用规则

Axiom 架构强制层间调用方向，编译期检查：

```
Layer N 只能调用 Layer <= N 的层

合法：
  Agent → Validate → Exec
  Agent → Agent
  Oversight → 所有层

非法（编译失败）：
  Exec → Agent
  Validate → Oversight
```

### 编译期保证

- `CanSendTo<T>` trait bound - 编译期检查调用方向
- Sealed trait pattern - 禁止自定义层
- Gate 系统 - 验证依赖方向

### 运行时保证

- ArchitectureGuardian - 运行时拦截违规信号
- LoopDetector - 检测消息循环
- EntropyGovernor - 监控系统熵值

---

## 常见问题

### Q1: AutoAgent 和 AgentCell 有什么区别？

**A**: `AutoAgent` 是全自动包装，提供零配置体验，自动处理启动、路由、监控、修复等。`AgentCell` 是核心单元，需要手动配置和管理。

- 快速原型 / 简单场景 → `AutoAgent`
- 需要精细控制 → `AgentCell` + `AgentBuilder`

### Q2: 如何添加自定义意图？

**A**:

```rust
let mut router = IntentRouter::new("agent:default");
router.add_route(IntentRoute {
    intent_pattern: "contains:my_keyword".to_string(),
    target_cell_id: "exec:my_handler".to_string(),
    confidence_threshold: 0.5,
    priority: 1,
});

let agent = AgentBuilder::new("agent")
    .with_intent_router(router)
    .build()?;
```

### Q3: 智能体出错了怎么办？

**A**: AutoAgent 会自动修复：
- Degraded：重置统计
- Unhealthy：自动重启
- Critical：完全重置后重启

如果使用 AgentCell，需要手动处理：

```rust
match agent.query(input, intent).await {
    Ok(response) => println!("{}", response),
    Err(e) => {
        eprintln!("Error: {}", e);
        agent.stop().ok();
        agent.start()?;  // 重启
    }
}
```

### Q4: 如何调试智能体？

**A**:

1. **查看健康报告**：`agent.health_report()`
2. **查看记忆**：`agent.memory_items()`
3. **启用 tracing**：设置 `RUST_LOG=debug`
4. **Witness 链**：通过 Witness 审计链追踪所有状态转换
5. **热图系统**：查看信号流量和延迟分布

### Q5: 可以同时运行多个智能体吗？

**A**: 可以。每个智能体是独立的 `AgentCell`，通过总线通信。建议使用 Runtime 来管理：

```rust
use axiom_runtime::Runtime;

let rt = Runtime::new();
rt.spawn(agent1);
rt.spawn(agent2);
rt.run().await?;
```

### Q6: 如何持久化智能体状态？

**A**: 使用状态快照和事件溯源：

- 每 100 个事件自动创建快照
- 重启时从最新快照恢复，重放后续事件
- Witness 链验证数据完整性

### Q7: 支持哪些规划策略？

**A**:

| 策略 | 说明 | 适用场景 |
|------|------|---------|
| `ReAct` | 推理 + 行动交替 | 通用任务 |
| `PlanAndExecute` | 先规划后执行 | 复杂任务 |
| `ChainOfThought` | 思维链推理 | 推理任务 |
| `Auto` | 自动选择 | 自适应 |

---

## API 参考

### AutoAgent

```rust
impl AutoAgent {
    pub fn new(id: impl Into<String>) -> AgentResult<Self>;
    pub fn with_llm(self, llm: LlmClient) -> AgentResult<Self>;
    pub fn with_mode(&self, mode: AutoMode);
    pub fn with_auto_config(&self, config: AutoConfig);
    pub fn id(&self) -> &str;
    pub fn is_running(&self) -> bool;
    pub async fn start(&self) -> AgentResult<()>;
    pub async fn stop(&self) -> AgentResult<()>;
    pub async fn process(&self, input: &str) -> AgentResult<String>;
    pub async fn process_natural(&self, signal: NaturalSignal) -> AgentResult<String>;
    pub fn health_report(&self) -> SelfReport;
    pub fn agent(&self) -> &AgentCell;
}
```

### AgentBuilder

```rust
impl AgentBuilder {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn from_manifest(manifest: &AgentManifest) -> Self;
    pub fn with_llm(self, llm: LlmClient) -> Self;
    pub fn with_memory_budget(self, tokens: usize) -> Self;
    pub fn with_auto_summarize(self, enable: bool) -> Self;
    pub fn with_planner_strategy(self, strategy: PlannerStrategy) -> Self;
    pub fn with_max_iterations(self, n: u32) -> Self;
    pub fn with_timeout_secs(self, n: u64) -> Self;
    pub fn with_intent_router(self, router: IntentRouter) -> Self;
    pub fn with_intent_router_arc(self, router: Arc<IntentRouter>) -> Self;
    pub fn with_self_monitor(self, monitor: SelfMonitor) -> Self;
    pub fn with_self_monitor_arc(self, monitor: Arc<SelfMonitor>) -> Self;
    pub fn with_persona(self, persona: AgentPersona) -> Self;
    pub fn with_skill(self, skill: Skill) -> Self;
    pub fn build(self) -> AgentResult<AgentCell>;
}
```

### AgentCell

```rust
impl AgentCell {
    pub fn new(id: impl Into<String>, config: AgentConfig) -> Self;
    pub fn id(&self) -> &str;
    pub fn stats(&self) -> AgentStats;
    pub fn is_started(&self) -> bool;
    pub async fn start(&self) -> AgentResult<()>;
    pub async fn stop(&self) -> AgentResult<()>;
    pub async fn query(&self, user_input: &str, intent: Option<&str>) -> AgentResult<String>;
    pub async fn execute_tool(&self, name: &str, params: &Value) -> AgentResult<String>;
    pub fn remember(&self, item: MemoryItem);
    pub fn memory_items(&self) -> Vec<MemoryItem>;
    pub fn memory_prompt(&self) -> String;
    pub fn available_tools(&self) -> Vec<ToolInfo>;
    pub fn intent_router(&self) -> Option<&Arc<IntentRouter>>;
    pub fn self_monitor(&self) -> Option<&Arc<SelfMonitor>>;
    pub fn self_report(&self) -> SelfReport;
    pub fn route_signal(&self, intent: &str, confidence: f64) -> RoutingResult;
}
```

### IntentRouter

```rust
impl IntentRouter {
    pub fn new(fallback_target: &str) -> Self;
    pub fn add_route(&self, route: IntentRoute);
    pub fn add_routes(&self, routes: Vec<IntentRoute>);
    pub fn remove_route(&self, intent_pattern: &str);
    pub fn route(&self, intent: &str, confidence: f64) -> RoutingResult;
    pub fn clear(&self);
    pub fn route_count(&self) -> usize;
}
```

### SelfMonitor

```rust
impl SelfMonitor {
    pub fn new(agent_id: &str) -> Self;
    pub fn record_interaction(&self, success: bool, response_time_ms: u64, confidence: f64, intent: &str);
    pub fn generate_report(&self) -> SelfReport;
    pub fn reset(&self);
}
```

### AgentManifest

```rust
impl AgentManifest {
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error>;
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error>;
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error>;
    pub fn to_json(&self) -> Result<String, serde_json::Error>;
}
```

### NaturalSignal

```rust
impl NaturalSignal {
    pub fn new(content: &str) -> Self;
    pub fn with_intent(self, intent: &str, confidence: f64) -> Self;
    pub fn with_entity(self, name: &str, r#type: &str, value: &str, confidence: f64) -> Self;
    pub fn with_context(self, key: &str, value: &str) -> Self;
    pub fn with_trace(self, trace: TraceId) -> Self;
    pub fn with_attachment(self, name: &str, r#type: &str, content: &str) -> Self;
}
```

---

## 更多资源

- [架构文档](../ARCHITECTURE.md) - 详细的架构设计
- [插件系统](../PLUGIN_SYSTEM.md) - 插件开发指南
- [热图系统](../HEATMAP_SYSTEM.md) - 性能监控
- [迁移指南](../MIGRATION.md) - 从 v0.3 迁移
- [路线图](../ROADMAP_0.4.md) - v0.4.0 版本规划