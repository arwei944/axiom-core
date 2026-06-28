# Phase 9: LLM 抽象 + 工具注册 Implementation Plan

> **Goal:** 新建 axiom-llm 和 axiom-tool crate：LLM抽象层（支持多provider、重试、缓存、结构化输出）和工具注册框架（Tool trait、ToolRegistry、参数校验）。验收标准：可以通过统一接口调用LLM，支持结构化JSON输出，工具可以注册和调用，自动重试和缓存工作。

> **New crates:** axiom-llm, axiom-tool

---

## Global Constraints
- axiom-llm 不直接依赖具体LLM SDK（通过feature flag）
- 所有LLM调用必须有超时、重试、token计数
- Tool调用必须有参数验证和权限检查
- 敏感数据（API key）不写入Witness和日志
- cargo build/clippy/test 零警告

---

### Task 9.1: 创建 axiom-tool crate（Tool trait + 注册表）

**Files:** New crates/axiom-tool/

- [ ] 定义 ToolId, ToolMetadata（name/description/version/parameters_schema/permissions）
- [ ] 定义 Tool trait：
  ```rust
  pub trait Tool: Send + Sync {
      fn metadata(&self) -> ToolMetadata;
      async fn call(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError>;
  }
  ```
- [ ] ToolParametersSchema 使用JSON Schema描述参数
- [ ] ToolResult：{ result: Value, is_error: bool, tokens_used: u64, duration_ms: u64 }
- [ ] ToolContext：包含identity_id/permissions/correlation_id/cell_id
- [ ] ToolRegistry：注册/查询/调用工具，权限检查
- [ ] 参数验证：调用前根据JSON Schema校验params
- [ ] 单元测试：工具注册、参数校验、权限拒绝
- [ ] Commit: `feat(axiom-tool): Tool trait, ToolRegistry, JSON Schema parameter validation, permission checks`

### Task 9.2: 创建 axiom-llm crate（LLM抽象层）

**Files:** New crates/axiom-llm/

- [ ] 定义 LlmProvider trait：
  ```rust
  #[async_trait]
  pub trait LlmProvider: Send + Sync {
      async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;
      fn provider_name(&self) -> &'static str;
      fn model_name(&self) -> &str;
  }
  ```
- [ ] LlmRequest：{ messages, temperature, max_tokens, response_format, tools }
- [ ] Message：system/user/assistant/tool role
- [ ] LlmResponse：{ content, tool_calls, usage: TokenUsage, finish_reason }
- [ ] TokenUsage：{ prompt_tokens, completion_tokens, total_tokens }
- [ ] ResponseFormat：Text/JsonSchema(schema)
- [ ] LlmError：ProviderError/RateLimit/Timeout/ContextLengthExceeded/InvalidResponse
- [ ] 单元测试：消息构造、token计数估算
- [ ] Commit: `feat(axiom-llm): LlmProvider trait, request/response types, token usage tracking, error types`

### Task 9.3: 实现 LLM 客户端（OpenAI兼容 + 重试 + 缓存）

- [ ] 实现 OpenAI-compatible provider（支持OpenAI/Azure OpenAI/本地Ollama/vLLM等兼容接口）
- [ ] 使用 reqwest（feature-gated）作为HTTP客户端
- [ ] 指数退避重试：对RateLimit/5xx/网络错误重试3次
- [ ] 超时：默认60秒
- [ ] 请求缓存：基于请求hash的内存/磁盘缓存（用于开发测试，避免重复调用）
- [ ] API key从环境变量读取（AXIOM_LLM_API_KEY），不硬编码
- [ ] 单元测试：重试逻辑、超时、缓存命中
- [ ] Commit: `feat(axiom-llm): OpenAI-compatible provider with exponential backoff retry, timeout, request cache`

### Task 9.4: 实现结构化输出（Structured Output）

- [ ] 支持 response_format: json_schema，要求LLM输出符合schema的JSON
- [ ] 自动JSON Schema生成（从serde的derive结构）
- [ ] 响应解析：自动反序列化为指定类型T
- [ ] 解析失败：重试1次（提示LLM修复JSON格式）
- [ ] 提供 `async fn call_structured<T: DeserializeOwned>(&self, request: LlmRequest) -> Result<T, LlmError>`
- [ ] 单元测试：JSON解析成功/失败重试
- [ ] Commit: `feat(axiom-llm): structured JSON output with schema validation, auto-retry on parse failure, call_structured helper`

### Task 9.5: 实现 Tool Calling 集成

- [ ] LlmRequest 支持 tools 参数（传递可用工具列表给LLM）
- [ ] LlmResponse 解析 tool_calls
- [ ] ToolCall 循环：LLM返回tool_calls → 执行工具 → 将结果返回LLM → 直到最终响应
- [ ] 设置最大迭代次数（默认10轮）防止无限循环
- [ ] Token预算控制：总token不超过context window
- [ ] 单元测试：单轮tool call、多轮tool call、超过最大迭代次数
- [ ] Commit: `feat(axiom-llm): tool calling loop with max iterations, token budget control, multi-round tool use`

---

## P9 验收标准

| # | 验收项 |
|---|--------|
| 1 | Tool trait + ToolRegistry + 参数校验 + 权限检查 |
| 2 | LlmProvider trait + OpenAI兼容实现 |
| 3 | 重试/超时/缓存工作 |
| 4 | 结构化JSON输出+解析失败重试 |
| 5 | Tool Calling循环+迭代限制 |
| 6 | cargo test -p axiom-llm -p axiom-tool 通过（≥20个测试） |

---

# Phase 10: MCP 桥接 Implementation Plan

> **Goal:** 新建 axiom-mcp crate：MCP Client/Server实现，安全约束。验收标准：可以连接MCP Server使用外部工具，可以作为MCP Server暴露Axiom Cell能力。

> **New crate:** axiom-mcp

---

### Task 10.1: MCP 协议类型定义

- [ ] 定义 MCP 协议消息类型（JSON-RPC 2.0）
- [ ] Initialize/InitializeResult
- [ ] Tools/ListTools/CallTool
- [ ] Resources/ListResources/ReadResource
- [ ] Prompts/ListPrompts/GetPrompt
- [ ] Transport trait（stdio/SSE/WebSocket）
- [ ] 单元测试：消息序列化/反序列化
- [ ] Commit: `feat(axiom-mcp): MCP protocol types, JSON-RPC message handling, Transport trait`

### Task 10.2: MCP Client 实现

- [ ] McpClient：连接MCP Server，能力协商
- [ ] stdio传输：子进程启动MCP Server
- [ ] SSE传输：HTTP SSE连接远程MCP Server
- [ ] 工具自动注册到axiom-tool的ToolRegistry
- [ ] 资源和Prompt发现
- [ ] 连接健康检查和自动重连
- [ ] 单元测试：协议握手、工具列表获取、工具调用
- [ ] Commit: `feat(axiom-mcp): McpClient with stdio/SSE transports, auto tool registration, health checks and reconnection`

### Task 10.3: MCP Server 实现

- [ ] McpServer：将Axiom工具/Cell暴露为MCP端点
- [ ] 将ToolRegistry中的工具暴露为MCP tools
- [ ] 将Lens投影暴露为MCP resources
- [ ] 将Skill暴露为MCP prompts
- [ ] 鉴权：API key / token验证
- [ ] 单元测试：MCP初始化、工具调用透传到ToolRegistry
- [ ] Commit: `feat(axiom-mcp): McpServer exposing Axiom tools/lenses/skills as MCP endpoints, authentication`

### Task 10.4: MCP 安全约束

- [ ] MCP工具调用受PermissionSet约束
- [ ] 外部工具默认在沙盒中运行（限制文件系统访问/网络访问）
- [ ] MCP连接需要显式审批（首次连接提示用户）
- [ ] 工具调用审计：所有MCP工具调用产生Witness
- [ ] 单元测试：权限拒绝、审计Witness
- [ ] Commit: `feat(axiom-mcp): MCP security - permission enforcement, sandboxing, approval flow, audit Witnesses`

---

## P10 验收标准

| # | 验收项 |
|---|--------|
| 1 | MCP协议类型和JSON-RPC处理 |
| 2 | McpClient（stdio/SSE）能连接Server并调用工具 |
| 3 | McpServer能暴露Axiom工具 |
| 4 | 安全约束：权限/沙盒/审计 |
| 5 | cargo test -p axiom-mcp 通过（≥15个测试） |

---

# Phase 11: CLI 脚手架完善 Implementation Plan

> **Goal:** 完善 axiom-cli：axm init/new/doctor/top/trace/why 命令。验收标准：可以创建新项目、健康诊断、TUI监控、Trace查询、Witness根因分析。

---

### Task 11.1: axm init 完善

- [ ] 当前axm init只安装hooks，补充：
- [ ] 创建 .axiom/ 目录结构（identity/skills/rules/preflight.md）
- [ ] 生成 axiom.toml 项目配置文件
- [ ] 生成示例 Cell 代码
- [ ] 自动执行 axm check 验证初始化
- [ ] 单元测试：init命令生成正确的目录结构
- [ ] Commit: `feat(axiom-cli): complete axm init with project scaffolding, axiom.toml config, example cell`

### Task 11.2: axm new（新项目创建）

- [ ] axm new <name>：创建完整的Axiom项目目录
- [ ] 生成Cargo workspace、axiom.toml、.gitignore、README模板
- [ ] 生成示例多Cell项目（hello world agent）
- [ ] 自动 axm init
- [ ] Commit: `feat(axiom-cli): axm new for project creation with workspace, example cells, full scaffolding`

### Task 11.3: axm doctor（健康诊断）

- [ ] 检查Rust工具链版本（≥1.75）
- [ ] 检查Git配置
- [ ] 检查cargo fmt/clippy可用
- [ ] 检查hooks已安装
- [ ] 检查constraints.lock一致性
- [ ] 检查当前workspace编译状态
- [ ] 输出诊断报告和修复建议
- [ ] Commit: `feat(axiom-cli): axm doctor with toolchain/git/hooks/config diagnostics and repair suggestions`

### Task 11.4: axm why（根因分析）

- [ ] axm why <witness_id_or_correlation_id>
- [ ] 从EventStore加载Witness链
- [ ] 反向追溯：从错误Witness沿着causality链回溯根因
- [ ] 显示因果链：触发信号→中间事件→错误结果
- [ ] 标注每个步骤的Layer和Cell
- [ ] Commit: `feat(axiom-cli): axm why for root cause analysis via Witness chain reverse tracing`

### Task 11.5: axm top TUI 完善

- [ ] 实时仪表盘（P5已设计，此阶段实现TUI渲染）
- [ ] 使用 crossterm 实现终端UI（不引入tui-rs等重依赖，保持轻量）
- [ ] 视图切换：overview/cells/entropy/traces
- [ ] 颜色编码：绿色健康/黄色警告/红色错误
- [ ] Commit: `feat(axiom-cli): axm top real-time TUI dashboard with crossterm, multiple views`

---

## P11 验收标准

| # | 验收项 |
|---|--------|
| 1 | axm init 完整项目初始化 |
| 2 | axm new 创建新项目 |
| 3 | axm doctor 诊断+修复建议 |
| 4 | axm why 根因分析 |
| 5 | axm top TUI实时仪表盘 |
| 6 | cargo test -p axiom-cli 通过（≥25个测试） |
