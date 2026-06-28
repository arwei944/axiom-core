# Phase 12: 记忆系统 Implementation Plan

> **Goal:** 新建 axiom-memory crate：四层记忆（工作记忆/情景记忆/语义记忆/程序记忆）、自动摘要、Token预算管理。验收标准：Agent可以记住历史对话和操作，按需检索相关记忆，记忆在Token预算内自动压缩/摘要/遗忘。

> **New crate:** axiom-memory

---

### Task 12.1: 四层记忆类型定义

- [ ] MemoryId, MemoryEntry 基础类型
- [ ] WorkingMemory：当前对话上下文（滑动窗口，受token预算严格限制）
- [ ] EpisodicMemory：具体事件/经历（Witness链+Trace，带时间戳和correlation_id）
- [ ] SemanticMemory：提炼的知识/事实（从Episodic摘要提取，去重）
- [ ] ProceduralMemory：技能/工具使用经验（How-to知识，触发条件→步骤）
- [ ] MemoryEntry包含：content、embedding（可选）、tags、created_at、last_accessed_at、access_count、importance_score
- [ ] 单元测试：记忆创建、类型区分
- [ ] Commit: `feat(axiom-memory): four-layer memory types (Working/Episodic/Semantic/Procedural) with importance scoring`

### Task 12.2: Token预算管理器

- [ ] TokenBudget：跟踪总token使用，各层预算分配
- [ ] 自动压缩策略：当超预算时，按优先级遗忘
  - WorkingMemory: 保留最近N轮，旧的摘要后移入Episodic
  - Episodic: 按importance_score排序，低importance的先遗忘
  - Semantic: 去重合并相似事实
  - Procedural: 保留高使用频率的，低频率的降级为Episodic
- [ ] 可配置预算（默认working=4096, episodic=8192, semantic=4096, procedural=2048）
- [ ] 单元测试：预算超限触发压缩、压缩后不超预算
- [ ] Commit: `feat(axiom-memory): TokenBudget manager with automatic compression/forgetting strategies per layer`

### Task 12.3: 自动摘要

- [ ] MemorySummarizer trait：使用LLM将一组Episodic记忆摘要为Semantic记忆
- [ ] 摘要触发条件：Episodic条目数>阈值 or Token预算压力高
- [ ] 摘要保留关键事实（who/what/when/result）
- [ ] 摘要过程产生Witness
- [ ] 单元测试：摘要触发、摘要后token减少
- [ ] Commit: `feat(axiom-memory): automatic episodic-to-semantic summarization with LLM, trigger conditions, Witness logging`

### Task 12.4: 记忆检索

- [ ] MemoryRetrieval trait：按相关性查询记忆
- [ ] 关键词检索（初版，无vector DB依赖）
- [ ] 时间范围检索
- [ ] Tag检索
- [ ] 相关性评分：recency * 0.3 + importance * 0.4 + access_count * 0.3
- [ ] 混合检索：多条件组合，返回top-K最相关记忆
- [ ] 单元测试：检索准确性、top-K排序
- [ ] Commit: `feat(axiom-memory): memory retrieval with keyword/time/tag search, relevance scoring, top-K results`

### Task 12.5: 记忆持久化

- [ ] 使用axiom-store的EventStore持久化记忆条目
- [ ] 记忆条目作为特殊event_type存储（"_memory.entry"）
- [ ] 启动时从EventStore重建记忆状态
- [ ] 支持Snapshot加速加载
- [ ] 单元测试：持久化和重建
- [ ] Commit: `feat(axiom-memory): memory persistence via axiom-store EventStore, startup reconstruction, Snapshot support`

---

## P12 验收标准

| # | 验收项 |
|---|--------|
| 1 | 四层记忆类型完整 |
| 2 | Token预算自动压缩 |
| 3 | LLM自动摘要 |
| 4 | 相关性检索 |
| 5 | 持久化和重建 |
| 6 | cargo test -p axiom-memory 通过（≥20个测试） |

---

# Phase 13: 规划器 Implementation Plan

> **Goal:** 新建 axiom-planner crate：ReAct和Plan-Execute两种规划模式。验收标准：Agent可以分析目标、制定多步计划、执行、反思、调整。

> **New crate:** axiom-planner

---

### Task 13.1: Plan 类型定义

- [ ] PlanId, PlanStep, PlanStatus（Planning/Executing/Waiting/Completed/Failed/Aborted）
- [ ] Plan：{ goal, steps: Vec<PlanStep>, current_step, created_at, updated_at, status }
- [ ] PlanStep：{ id, description, tool_calls: Vec<ToolCall>, dependencies: Vec<StepId>, status, result: Option<Value>, error: Option<String> }
- [ ] 单元测试：Plan创建、步骤状态转换
- [ ] Commit: `feat(axiom-planner): Plan/PlanStep types with dependency tracking and status transitions`

### Task 13.2: ReAct 模式实现

- [ ] ReAct循环：Thought → Action → Observation → Thought → ...
- [ ] 每轮：LLM根据当前状态决定下一步（思考或直接回答）
- [ ] 支持工具调用（通过axiom-tool）
- [ ] 最大步数限制（默认20步）
- [ ] 每步产生Witness（可审计）
- [ ] 单元测试：ReAct单步/多步、步数限制
- [ ] Commit: `feat(axiom-planner): ReAct loop with Thought/Action/Observation cycle, tool calling, step limit, Witness logging`

### Task 13.3: Plan-Execute 模式实现

- [ ] Plan阶段：LLM制定完整计划（分解为多步骤）
- [ ] Execute阶段：按依赖顺序执行步骤
- [ ] Replan：步骤失败时重新规划
- [ ] 支持并行执行无依赖的步骤
- [ ] 进度追踪和状态报告
- [ ] 单元测试：计划制定、顺序执行、失败重规划
- [ ] Commit: `feat(axiom-planner): Plan-Execute mode with dependency-based execution, parallel step execution, replanning on failure`

### Task 13.4: 反思和自我修正

- [ ] 执行结果反思：步骤完成后LLM评估是否达到预期
- [ ] 错误分析：失败时分析原因（工具错误/逻辑错误/信息不足）
- [ ] 计划调整：根据反思结果修改后续步骤
- [ ] 学习反馈：将成功/失败模式写入ProceduralMemory
- [ ] 单元测试：反思触发、错误分析
- [ ] Commit: `feat(axiom-planner): reflection and self-correction, failure analysis, learning feedback to ProceduralMemory`

---

## P13 验收标准

| # | 验收项 |
|---|--------|
| 1 | Plan/PlanStep类型和状态机 |
| 2 | ReAct循环完整 |
| 3 | Plan-Execute+依赖+重规划 |
| 4 | 反思和自我修正 |
| 5 | cargo test -p axiom-planner 通过（≥15个测试） |

---

# Phase 14: 提示词模板 + RAG Implementation Plan

> **Goal:** 新建 axiom-prompt 和 axiom-rag crate：类型安全的提示词模板和RAG检索。验收标准：提示词可组合、类型安全参数替换，RAG从文档库检索相关片段注入上下文。

---

### Task 14.1: 类型安全提示词模板（axiom-prompt）

- [ ] PromptTemplate：带占位符的模板字符串
- [ ] 编译期模板解析（宏）或运行时解析+参数校验
- [ ] 参数类型约束（String/i64/bool/Value等）
- [ ] 模板组合：`template_a + template_b` 分段拼接
- [ ] SystemPrompt/UserPrompt/AssistantPrompt 强类型区分
- [ ] Token估算：渲染后估计token数
- [ ] 单元测试：参数替换、类型校验、模板组合、token估算
- [ ] Commit: `feat(axiom-prompt): type-safe prompt templates with parameter validation, composition, token estimation`

### Task 14.2: PromptRegistry 和版本化

- [ ] PromptRegistry：按名称存储和查询模板
- [ ] 提示词版本化（SchemaVersion复用）
- [ ] A/B测试支持：同一prompt名可有多个active版本
- [ ] 从文件/目录加载模板
- [ ] 单元测试：注册、查询、版本管理
- [ ] Commit: `feat(axiom-prompt): PromptRegistry with versioning, A/B testing support, file loading`

### Task 14.3: RAG 文档索引（axiom-rag）

- [ ] Document类型：{ id, content, metadata, embedding, chunk_strategy }
- [ ] DocumentChunk：文档分块（按段落/固定长度/token数）
- [ ] ChunkStrategy 实现：Paragraph/TokenCount/Recursive
- [ ] 简单关键词索引（BM25，无vector DB依赖的初版）
- [ ] DocumentStore trait + MemoryDocumentStore实现
- [ ] 单元测试：文档分块、BM25检索
- [ ] Commit: `feat(axiom-rag): Document types, chunking strategies, BM25 keyword index, in-memory store`

### Task 14.4: RAG 检索和上下文注入

- [ ] Retriever trait：query → top-K relevant chunks
- [ ] ContextAssembler：将检索结果组装为prompt上下文
- [ ] Token预算感知：选择chunk直到塞满预算
- [ ] 去重和排序：MMR（Maximal Marginal Relevance）避免冗余
- [ ] 引用标注：每个chunk标注来源文档
- [ ] 单元测试：检索top-K、token预算控制、去重
- [ ] Commit: `feat(axiom-rag): Retriever with token-budget-aware assembly, MMR deduplication, source citations`

---

## P14 验收标准

| # | 验收项 |
|---|--------|
| 1 | 类型安全PromptTemplate |
| 2 | PromptRegistry版本化 |
| 3 | BM25文档检索 |
| 4 | Token预算感知的上下文组装 |
| 5 | cargo test -p axiom-prompt -p axiom-rag 通过（≥20个测试） |

---

# Phase 15: 测试 + 评估 Implementation Plan

> **Goal:** 新建 axiom-test 和 axiom-eval crate：Mock LLM/故障注入/录制重放/Golden Set测试。验收标准：可以写确定性的Agent测试，评估框架可量化Agent性能。

---

### Task 15.1: Mock LLM（axiom-test）

- [ ] MockLlmProvider：实现LlmProvider trait，返回预设响应
- [ ] 按匹配规则返回不同响应（关键词/regex匹配prompt）
- [ ] 记录所有调用历史（prompt/response/timestamp/latency）
- [ ] 支持顺序响应（第1次调用返回A，第2次返回B...）
- [ ] 单元测试：MockLLM返回预设、调用记录
- [ ] Commit: `feat(axiom-test): MockLlmProvider with pattern matching, sequential responses, call history recording`

### Task 15.2: 故障注入

- [ ] FaultInjector：模拟各种故障场景
  - LLM超时/5xx错误/RateLimit
  - Tool调用失败/超时
  - 网络延迟
  - MemoryStore不可用
- [ ] 故障概率配置（如10%概率超时）
- [ ] ChaosTest场景：随机组合故障
- [ ] 单元测试：各故障类型正确触发
- [ ] Commit: `feat(axiom-test): fault injection for LLM/tool/network/memory failures, chaos test scenarios`

### Task 15.3: 录制重放

- [ ] RecordingLlmProvider：包装真实LLM，录制所有请求/响应对
- [ ] ReplayLlmProvider：从录制文件重放，不调用真实LLM
- [ ] 录制格式：JSONL（每行一个request/response对）
- [ ] 确定性重放：相同输入必须返回相同输出
- [ ] 单元测试：录制/重放往返
- [ ] Commit: `feat(axiom-test): record/replay LLM interactions as JSONL, deterministic replay`

### Task 15.4: Golden Set 测试（axiom-eval）

- [ ] GoldenSet：{ input, expected_output, evaluation_criteria }
- [ ] TestCase：使用Mock/Replay LLM运行Agent，断言输出符合标准
- [ ] 评估指标：
  - 正确率：输出是否包含预期信息
  - 工具使用正确性：是否调用了预期工具
  - 步数效率：是否在预期步数内完成
  - 无违规：无AxiomViolation/PermissionDenied
- [ ] EvaluationReport：通过率/平均步数/平均token消耗/违规数
- [ ] 单元测试：GoldenSet评估框架
- [ ] Commit: `feat(axiom-eval): Golden Set testing framework, evaluation metrics (correctness/efficiency/safety), EvaluationReport`

### Task 15.5: 回归测试和基准

- [ ] 基准性能测试：固定场景的token消耗/延迟基线
- [ ] 回归检测：新代码导致token消耗增加>20%或正确率下降时失败
- [ ] 基准结果持久化（可比较历史版本）
- [ ] Commit: `feat(axiom-eval): performance baselines, regression detection on token cost/latency/accuracy`

---

## P15 验收标准

| # | 验收项 |
|---|--------|
| 1 | MockLlmProvider完整可用 |
| 2 | 故障注入和Chaos测试 |
| 3 | LLM录制/重放 |
| 4 | Golden Set评估框架 |
| 5 | 性能基准和回归检测 |
| 6 | cargo test -p axiom-test -p axiom-eval 通过（≥20个测试） |

---

# Phase 16: CLI 完善 Implementation Plan

> **Goal:** 完善 axiom-cli 所有命令：shell/replay/test/cell管理。验收标准：完整CLI功能+REPL交互模式。

---

### Task 16.1: axm shell（REPL）

- [ ] 交互式REPL，可以直接发送命令给运行的Axiom系统
- [ ] 命令：send/signal/cells/stats/help/exit
- [ ] Tab补全（命令名和cell名）
- [ ] 实时显示返回结果和Witness
- [ ] Commit: `feat(axiom-cli): axm shell interactive REPL with tab completion, real-time Witness display`

### Task 16.2: axm replay（事件重放）

- [ ] axm replay <correlation_id or witness_range>
- [ ] 从EventStore读取事件，重放到独立沙盒Runtime
- [ ] 支持断点、单步执行
- [ ] 显示每步的Cell状态、消息、Witness
- [ ] Commit: `feat(axiom-cli): axm replay with sandbox playback, breakpoints, step-by-step execution`

### Task 16.3: axm test（测试运行器）

- [ ] axm test [test_name]
- [ ] 运行axiom-test测试用例
- [ ] 输出测试结果（pass/fail/metrics）
- [ ] 支持GoldenSet评估（调用axiom-eval）
- [ ] 覆盖率报告（哪些Cell/Signal被测试覆盖）
- [ ] Commit: `feat(axiom-cli): axm test runner with GoldenSet evaluation and coverage report`

### Task 16.4: Cell管理命令

- [ ] axm cell list：列出所有cell（id/layer/state/message_count）
- [ ] axm cell show <id>：显示cell详情（meta/state/stats/recent_witnesses）
- [ ] axm cell restart <id>：手动重启cell
- [ ] axm cell stop <id>：停止cell
- [ ] axm cell log <id>：显示cell的最近日志/Witness
- [ ] Commit: `feat(axiom-cli): axm cell list/show/restart/stop/log commands for runtime cell management`

### Task 16.5: axm config（配置管理）

- [ ] axm config get <key>
- [ ] axm config set <key> <value>
- [ ] axm config list
- [ ] 配置变更的验证（类型/范围检查）
- [ ] 配置变更产生Witness
- [ ] Commit: `feat(axiom-cli): axm config get/set/list with validation and Witness logging`

---

## P16 验收标准

| # | 验收项 |
|---|--------|
| 1 | axm shell REPL |
| 2 | axm replay 重放+断点 |
| 3 | axm test 运行+评估+覆盖率 |
| 4 | Cell管理命令完整 |
| 5 | axm config 配置管理 |
| 6 | 所有axm命令有--help文档 |
| 7 | cargo test -p axiom-cli 通过（≥35个测试） |

---

# Phase 17: 示例 + 文档 Implementation Plan

> **Goal:** 完整多Agent示例项目 + 完整文档。验收标准：examples/中有end-to-end示例可以直接运行，文档完整覆盖API和架构。

---

### Task 17.1: Hello Cell 示例完善

- [ ] 更新hello_cell示例使用所有宏（#[derive(SignalPayload)], #[cell], #[axiom], #[schema_version]）
- [ ] 示例展示消息发送、Witness产生、Schema验证
- [ ] README注释清晰
- [ ] cargo run --example hello_cell 可以运行并输出
- [ ] Commit: `feat(examples): complete hello_cell example with all macros, message passing, Witness generation`

### Task 17.2: 多Cell协作示例

- [ ] examples/multi_cell/：包含3个Cell的完整示例
  - Exec层：UserInputCell（接收用户输入命令）
  - Validate层：CommandValidatorCell（验证命令合法性）
  - Exec层：CommandExecutorCell（执行命令）
  - Oversight层：使用内置ArchitectureGuardian
- [ ] 展示：消息路由、层约束、Schema校验、Axiom拦截、错误处理
- [ ] 可运行：cargo run --example multi_cell
- [ ] Commit: `feat(examples): multi_cell collaboration example demonstrating routing, layer constraints, validation, axioms`

### Task 17.3: Agent 示例（P9-P14完成后）

- [ ] examples/simple_agent/：使用LLM+Tool+Memory+Planner的完整Agent
- [ ] 包含：一个工具调用示例（如计算器+网页抓取）
- [ ] 展示：ReAct模式、工具调用、记忆使用、Witness链
- [ ] 需要配置LLM API key（通过环境变量）
- [ ] Commit: `feat(examples): simple_agent example with LLM, tools, memory, planning, Witness chain`

### Task 17.4: 完整文档

- [ ] 更新README.md：完整的项目介绍、快速开始、架构图
- [ ] 更新DEVELOPMENT.md：开发指南、添加新Cell/Signal/Axiom的教程
- [ ] 架构文档完善：docs/architecture/下所有文档补充代码示例
- [ ] CLI文档：每个axm子命令的man-page风格文档
- [ ] 教程：docs/tutorials/ 下三篇教程（hello_cell、multi_cell、simple_agent）
- [ ] API文档：确保所有public API有rustdoc注释，cargo doc生成完整文档
- [ ] Commit: `docs: complete README, DEVELOPMENT, architecture docs, CLI reference, and tutorials`

### Task 17.5: CI/CD 完善

- [ ] GitHub Actions workflow完整：
  - cargo build/test/clippy/fmt on every PR
  - cargo doc 发布
  - 示例编译验证
  - 跨平台测试（Linux/Windows/macOS）
- [ ] Release workflow：版本tag自动发布crate
- [ ] Commit: `ci: complete CI/CD pipelines for build/test/clippy/fmt/doc/examples, cross-platform, release workflow`

---

## P17 验收标准

| # | 验收项 |
|---|--------|
| 1 | hello_cell示例完整可运行 |
| 2 | multi_cell示例展示四层架构 |
| 3 | simple_agent示例展示Agent能力 |
| 4 | README/DEVELOPMENT/架构文档/教程完整 |
| 5 | cargo doc --no-deps --workspace 无警告 |
| 6 | CI/CD完整：所有check在PR上自动运行 |
| 7 | cargo test --workspace 全部通过 |
| 8 | axm check 全绿（branch check除外） |

---

# 全部阶段最终验收（P17结束时）

系统达到"可上线"标准：

1. **三层门禁全开**：L0(axm check+CI)、L1(编译期CanSendTo+宏)、L2(ArchitectureGuardian+EntropyGovernor)全生效
2. **核心功能完整**：多Cell通信、消息持久化、自愈重启、熔断降级、监督层、可视化
3. **进化引擎工作**：自动检测问题→沙盒验证→金丝雀部署→自动回滚
4. **Agent能力**：LLM+工具+MCP+记忆+规划+技能+规则+身份，可构建实用Agent
5. **CLI完整**：init/new/check/verify/doctor/top/trace/why/shell/replay/test/cell/config/evolution
6. **测试覆盖**：单元+集成+trybuild+Golden Set，≥300个测试
7. **文档完整**：API文档+教程+架构文档+示例
8. **零警告**：cargo build/clippy/fmt/test/doc全绿
