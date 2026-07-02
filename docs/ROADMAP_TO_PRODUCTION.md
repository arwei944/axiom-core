# Axiom Core 商用架构开发文档

> **当前阶段**: P2 架构债务修复 (进行中)
> **目标阶段**: P4 生产就绪 (商用发布)
> **预估工期**: 8-12 周

---

## 一、架构核心需求回顾

### 1.1 五大核心原语

| 原语 | 定位 | 当前状态 |
|------|------|---------|
| **Cell** | 隔离状态单元 | ✅ 完成 |
| **Signal** | 类型化消息 | ✅ 完成 |
| **Lens** | 状态投影 | ✅ 基础实现 |
| **Axiom** | 全局约束 | ✅ 完成 |
| **Witness** | 审计记录 | ✅ 完成，持久化未接线 |

### 1.2 四层架构约束

```
Oversight (0) → Agent (3) → Validate (2) → Exec (1)
     │              │              │            │
     ↓              ↓              ↓            ↓
  监督层          推理层          验证层        执行层
```

**约束规则**: 只能向下或同层调用，禁止向上或跨层跳跃。

### 1.3 绝对约束实现

| 约束层面 | 实现方式 | 状态 |
|---------|---------|------|
| **编译期约束** | `LayeredCellContext` + `CanSendTo` trait | ✅ 完成 |
| **运行时约束** | `CellContext::send/emit_event` 内部校验 | ✅ 完成 |
| **架构自约束** | 架构组件自身受约束，Witness记录架构操作 | ✅ 完成 |

---

## 二、外部智能体与架构的交互机制

### 2.1 外部智能体接入模型

外部智能体（如 LLM、第三方Agent服务）通过以下方式与 axiom-core 交互：

```
┌──────────────────────────────────────────────────────────────┐
│                    外部智能体（External Agent）               │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐    │
│  │   LLM    │  │ 第三方Agent│  │   MCP    │  │  人类用户  │    │
│  │  (OpenAI)│  │  (LangChain)│ │ Server  │  │           │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └─────┬─────┘    │
│       │              │              │              │          │
│       └──────────────┴──────────────┼──────────────┘          │
│                                     ▼                        │
│                    ┌──────────────────────────┐               │
│                    │      接入边界层           │               │
│                    │  (axiom-mcp / HTTP API)  │               │
│                    └─────────────┬────────────┘               │
│                                  ▼                            │
│              ┌─────────────────────────────────────────┐      │
│              │           axiom-core Runtime             │      │
│              │                                         │      │
│              │  ┌──────────┐  ┌──────────┐  ┌───────┐  │      │
│              │  │ Oversight│  │  Agent   │  │Validate│  │      │
│              │  │  Cell    │  │  Cell    │  │  Cell  │  │      │
│              │  └────┬─────┘  └────┬─────┘  └───┬───┘  │      │
│              │       │            │             │      │      │
│              │       └────────────┴─────┬───────┘      │      │
│              │                         ▼              │      │
│              │                    ┌────────┐          │      │
│              │                    │ Exec   │          │      │
│              │                    │  Cell  │          │      │
│              │                    └────────┘          │      │
│              └─────────────────────────────────────────┘      │
└──────────────────────────────────────────────────────────────┘
```

### 2.2 交互路径详解

#### 路径1: MCP协议接入（推荐）

外部智能体通过 MCP（Model Context Protocol）协议接入：

```
外部智能体
    │
    ├─→ MCP Tool调用 → axiom-mcp Client → Permission检查 → ToolRegistry → Exec Cell
    │
    ├─→ MCP Resource请求 → axiom-mcp Client → Lens投影 → 返回状态视图
    │
    └─→ MCP Prompt请求 → axiom-mcp Client → PromptTemplateEngine → 返回模板
```

**约束点**:
- 所有 MCP Tool 调用经过 `Permission → Rules → Axiom → Human-in-the-loop` 四层检查
- 每次调用产生完整的 Witness 记录
- 高危工具（如 `execute_command`）需 Oversight 审批

#### 路径2: HTTP API接入

外部智能体通过 REST API 接入：

```
POST /api/v1/signal
Content-Type: application/json

{
    "signal_type": "UserRequest",
    "payload": {"message": "审查这个PR"},
    "target_cell": "code-reviewer",
    "target_layer": "agent"
}
```

**约束点**:
- API Gateway 验证 `target_layer` 是否符合调用规则
- 非法跨层调用（如 Exec → Agent）在网关层直接拒绝
- 所有请求产生 Witness，包含请求来源信息

#### 路径3: 直接Cell调用

外部智能体作为 axiom-core 的 Cell 直接集成：

```rust
#[cell(layer = "agent")]
impl Cell for ExternalAgentCell {
    type Message = ExternalAgentRequest;
    type Layer = AgentLayer;
    
    fn handle<'a>(
        &'a mut self,
        signal: ExternalAgentRequest,
        ctx: LayeredCellContext<'a, Self::Layer>,
    ) -> impl Future<Output = (Result<()>, Vec<OutgoingEnvelope>, Vec<OutgoingWitness>)> + Send + 'a {
        async move {
            // 调用外部LLM API
            let llm_response = self.llm_client.complete(signal.prompt).await?;
            
            // 只能发送给 Agent 或 Validate 层（编译期约束）
            ctx.send_to::<ValidateLayer, _>(ValidationRequest { ... }, "validator")?;
            
            let (outgoing, witnesses) = ctx.end_processing();
            (Ok(()), outgoing, witnesses)
        }
    }
}
```

**约束点**:
- `LayeredCellContext` 只暴露合法的 `send_to` 方法
- 非法跨层调用无法通过编译
- 每次消息处理产生 Witness

#### 路径4: Runtime信号提交（推荐的外部入口）

外部智能体通过 `Runtime::submit_signal()` 直接提交强类型信号：

```rust
use axiom_core::{Layer, Signal};
use axiom_runtime::AxiomRuntime;

// 外部智能体创建信号
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[signal(kind = "command", layer = "agent")]
struct UserRequest {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    message: String,
}

// 外部智能体提交信号到 Runtime
async fn submit_to_runtime(rt: &AxiomRuntime, message: &str) -> Result<u64, AxiomError> {
    let signal = UserRequest {
        msg_id: MsgId::new("req-1"),
        correlation_id: CorrelationId::new("corr-1"),
        vector_clock: VectorClock::new(),
        message: message.to_string(),
    };
    
    // 提交到 Agent 层的 code-reviewer Cell
    // submit_signal 会自动校验层约束
    rt.submit_signal(signal, Some("code-reviewer"), Layer::Agent).await
}
```

**约束点**:
- `submit_signal` 自动执行信号验证（`validate()`）
- 自动校验层约束（`source_layer.can_send_to(target_layer)`）
- 非法跨层调用返回 `AxiomError::LayerViolation`
- 信号经过 MessageBus 的所有拦截器（ArchitectureGuardian、HopLimit、Idempotency等）
- 完整的因果链追踪（VectorClock、CorrelationId）

**交互序列**:
```
外部智能体
    │
    ├─→ Runtime::submit_signal(signal, target_cell, target_layer)
    │       │
    │       ├─→ signal.validate()     # 信号校验
    │       ├─→ source_layer.can_send_to(target_layer)  # 层约束校验
    │       ├─→ SignalEnvelope::to_cell()  # 包装为类型擦除信封
    │       └─→ bus.publish(env)      # 发布到消息总线
    │               │
    │               ├─→ ArchitectureGuardian  # 架构约束校验
    │               ├─→ HopLimitInterceptor   # 跳数限制
    │               ├─→ IdempotencyInterceptor # 幂等性检查
    │               ├─→ SchemaVersionInterceptor # 版本兼容
    │               ├─→ ThrottleInterceptor   # 熵治理限流
    │               ├─→ EmergencyInterceptor  # 紧急模式检查
    │               └─→ Mailbox.push()        # 投递到目标Cell信箱
    │                       │
    │                       └─→ Runtime.dispatch()  # 调度执行
    │                               │
    │                               └─→ Cell::handle()  # Cell处理
    │                                       │
    │                                       └─→ 产生Witness + 发送OutgoingEnvelope
```

### 2.3 外部智能体的约束层次

#### 第一层：编译期约束（最强）

通过 Rust 类型系统实现，**无法绕过**：

```rust
// ✅ 合法：Agent → Validate
ctx.send_to::<ValidateLayer, _>(signal, "validator");

// ❌ 编译失败：Agent → Oversight（违反规则）
ctx.send_to::<OversightLayer, _>(signal, "governor");

// ❌ 编译失败：Exec → Agent（违反规则）
ctx.send_to::<AgentLayer, _>(signal, "agent");
```

**机制**: `CanSendTo` trait 只在合法方向上实现：

```rust
// sealed.rs - 合法方向
impl CanSendTo<AgentLayer> for AgentLayer {}
impl CanSendTo<ValidateLayer> for AgentLayer {}

// ❌ 非法方向没有实现
// impl CanSendTo<OversightLayer> for AgentLayer {} // 不存在
```

#### 第二层：运行时约束（兜底）

即使尝试绕过编译期检查，运行时仍会校验。`LayeredCellContext` 的 `inner()`、`inner_mut()` 和 `into_inner()` 方法已标记为 `pub(crate)`，外部代码无法直接访问原始 `CellContext`：

```rust
// context.rs - LayeredCellContext 的安全边界
pub(crate) fn inner(&self) -> &CellContext<'a> { ... }      // 仅 crate 内部可见
pub(crate) fn inner_mut(&mut self) -> &mut CellContext<'a> { ... }  // 仅 crate 内部可见
pub(crate) fn into_inner(self) -> &'a mut CellContext<'a> { ... }   // 仅 crate 内部可见
```

`CellContext` 的 `send`、`emit_event` 方法也已标记为 `pub(crate)`，确保外部代码无法绕过约束：

```rust
// context.rs - CellContext 内部方法
pub(crate) fn send<S: Signal>(
    &mut self,
    signal: S,
    target_cell: &str,
    target_layer: Layer,
) -> crate::Result<()> {
    if !self.layer.can_send_to(target_layer) {
        return Err(crate::AxiomError::LayerViolation {
            from: self.layer,
            to: target_layer,
            signal_type: signal.signal_type().to_string(),
        });
    }
    self.emit_internal(signal, Some(target_cell), target_layer)
}
```

**机制**: `Layer::can_send_to()` 实现约束矩阵：

```rust
// layer.rs
pub fn can_send_to(&self, target: Layer) -> bool {
    match self {
        Layer::Oversight => true,              // Oversight → 任意层
        Layer::Agent => matches!(target, Layer::Agent | Layer::Validate),
        Layer::Validate => matches!(target, Layer::Validate | Layer::Exec),
        Layer::Exec => matches!(target, Layer::Exec),
    }
}
```

#### 第三层：权限约束（业务级）

通过 `Identity` 和 `PermissionSet` 控制外部智能体的能力范围：

```rust
pub struct Identity {
    pub id: String,
    pub name: String,
    pub capabilities: CapabilitySet,    // 能做什么
    pub permissions: PermissionSet,     // 被允许做什么
    pub skills: Vec<SkillId>,           // 已激活技能
    pub rules: RuleSet,                 // 遵守的规则
}
```

**权限检查流程**:
```
外部智能体请求 → Identity匹配 → Permission检查 → Tool/Rules/Axiom检查 → 执行
```

#### 第四层：熵治理约束（系统级）

`EntropyGovernorCell` 监控系统无序度，自动执行治理动作：

| 熵等级 | 治理动作 | 对外部智能体的影响 |
|--------|---------|-------------------|
| **Green** | 正常运行 | 无限制 |
| **Yellow** | Warn | 记录告警，发送提醒 |
| **Orange** | Throttle | 限制消息速率，延迟响应 |
| **Red** | Emergency | 熔断，拒绝新请求 |
| **Critical** | ShutDown | 紧急停机，保存状态 |

**熵度量因子**:
```rust
pub struct CellEntropy {
    pub message_queue_depth: u32,      // 消息积压
    pub error_rate: f64,               // 错误率
    pub response_time_ms: u64,         // 响应延迟
    pub state_drift: f64,              // 状态漂移
    pub axiom_violations: u32,         // Axiom违反次数
    pub witness_chain_breaks: u32,     // 见证链断裂
    pub intent_drift: f64,             // 意图漂移（输出偏离Identity）
    pub resource_exhaustion: f64,      // 资源耗尽
}
```

#### 第五层：Witness审计约束（追溯级）

**一切可审计**——外部智能体的所有操作都被记录：

```rust
pub struct Witness {
    pub witness_id: WitnessId,
    pub cell_id: CellId,
    pub correlation_id: CorrelationId,
    pub timestamp_ns: u64,
    pub signal_type: String,
    pub outcome: TransitionOutcome,
    pub before_state_hash: Option<[u8; 32]>,
    pub after_state_hash: Option<[u8; 32]>,
    pub parent_hash: Option<WitnessHash>,
    pub signature: Option<[u8; 64]>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

**约束效果**:
- 外部智能体无法"偷偷"执行操作
- 完整的因果链可追溯
- 篡改证据可检测（哈希链）

### 2.4 外部智能体的身份与技能系统

#### Identity（身份）

外部智能体挂载 Identity 后成为"角色化"Agent：

```rust
pub struct Identity {
    pub id: String,
    pub name: String,
    pub persona: Persona,              // 角色定义
    pub capabilities: CapabilitySet,   // 能力范围
    pub permissions: PermissionSet,    // 权限集
    pub skills: Vec<SkillId>,          // 已激活技能
    pub rules: RuleSet,                // 行为规则
}

pub struct Persona {
    pub role: String,                  // 角色："代码审查专家"
    pub tone: ToneStyle,               // 语气：Professional/Casual
    pub expertise: Vec<String>,        // 专长：["Rust", "安全"]
    pub values: Vec<String>,           // 价值观：["安全性优先"]
}
```

**Identity 的架构级作用**:
- 提示词构建：Persona → system prompt
- 能力过滤：限制能看到/使用的技能和工具
- 规则注入：合并到全局 Rules 集
- 权限控制：Oversight 的 ResourceManager 依据此检查
- 熵度量：角色漂移 → intent_drift 分量
- Witness标记：每个Witness记录当时活跃的Identity

#### Skill（技能）

外部智能体通过技能系统获得专业能力：

```
Level 1: 元数据（始终加载，~几十token）
┌────────────────────────────────────────┐
│ name: code-review                      │
│ description: 审查代码质量、安全性、风格 │
│ triggers: [PR事件, "review"关键词]     │
└────────────────────────────────────────┘
         ↓ 激活时加载
Level 2: 指令集（按需加载，~几百token）
┌────────────────────────────────────────┐
│ SKILL.md 指令正文                      │
│ - 工作流程步骤                         │
│ - 输出格式规范                         │
│ - 使用哪些Tool/Lens/Axiom              │
└────────────────────────────────────────┘
         ↓ 执行中需要时加载
Level 3: 资源（懒加载）
┌────────────────────────────────────────┐
│ scripts/   → 确定性脚本辅助            │
│ references/→ 参考文档/知识库           │
│ assets/    → 模板/示例/检查清单        │
└────────────────────────────────────────┘
```

**Skill 的架构绑定**（`skill.toml`）：

```toml
[skill]
name = "code-review"
version = "1.0.0"

[tools]
allowed = ["read_file", "search_code"]
denied = ["delete_file", "execute_command"]

[axioms]
enforce = ["no-unsafe-without-comment", "max-cyclomatic-complexity-10"]

[lenses]
mount = ["code-diff-lens", "project-structure-lens"]

[rules]
include = ["code-review-rules"]
```

#### Rules（规则）

外部智能体遵守的行为规范：

| 规则类型 | 示例 | 违反后果 |
|---------|------|---------|
| **Safety Rules** | "禁止输出用户密码" | 拒绝/熔断 |
| **Format Rules** | "回复使用Markdown" | 自动重试 |
| **Behavior Rules** | "不知道时诚实说不知道" | 降低置信度 |
| **Tool Rules** | "优先使用read_file" | 告警 |
| **Quality Rules** | "代码必须有错误处理" | 要求修正 |

**三层执行机制**:

```
Layer 1: Prompt注入（推理前）
    → Rules作为system prompt的一部分

Layer 2: 输出验证（推理后，执行前）
    → 确定性Validator检查LLM输出

Layer 3: 升级为Axiom（执行时）
    → Critical级别Rule违反 → 升级为Axiom违反 → 熔断
```

---

## 三、商用架构开发任务清单

### 3.1 Phase 0: 基础完备（2周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P0-01** | 统一 `EntropyLevel` 定义 | `axiom-oversight` 通过 re-export 使用 |
| **P0-02** | 统一 `now_ns()` 函数 | 全局只有一处定义 |
| **P0-03** | 错误路径测试补齐（5场景） | LayerViolation/Witness断裂/序列化失败/崩溃恢复/信箱溢出 |
| **P0-04** | 并发测试补齐（3场景） | 多Cell并发/串行处理/背压测试 |
| **P0-05** | 消除所有 clippy 警告 | `cargo clippy --workspace` 零警告 |
| **P0-06** | 零 unwrap/expect（非测试代码） | 安全的错误处理 |

### 3.2 Phase 1: API稳定性（1周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P1-01** | 定义 v1 API 边界 | 标记不稳定API为 `#[cfg(feature = "unstable")]` |
| **P1-02** | 版本策略文档 | 语义化版本规则、弃用流程、breaking change通知 |
| **P1-03** | 错误类型完善 | `AxiomError` 覆盖所有错误场景 |
| **P1-04** | 公共API文档完备 | 每个公开函数/类型有文档注释 |

### 3.3 Phase 2: Witness持久化（2周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P2-01** | Witness → Event 序列化完善 | 所有字段正确映射 |
| **P2-02** | 运行时持久化接线 | `handle_dyn` 返回的 witnesses 写入 event store |
| **P2-03** | 事件重放 API | `replay(correlation_id)` / `replay_from(cell_id, timestamp)` |
| **P2-04** | 状态快照/恢复 | `snapshot(cell_id)` / `restore(cell_id, snapshot_id)` |
| **P2-05** | 集成测试 | 崩溃恢复后状态一致 |

### 3.4 Phase 3: CLI工具（3周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P3-01** | 项目脚手架 | `axm new my-agent` 创建完整项目结构 |
| **P3-02** | 运行命令 | `axm run` / `axm dev` |
| **P3-03** | 实时监控TUI | `axm top` 显示Cell状态/熵值/消息吞吐 |
| **P3-04** | 调试诊断 | `axm trace` / `axm why` / `axm witness` |
| **P3-05** | 运维命令 | `axm cell list/restart/stop` / `axm entropy` |

### 3.5 Phase 4: MCP协议桥接（2周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P4-01** | MCP客户端实现 | 连接外部MCP Server |
| **P4-02** | MCP服务端实现 | 暴露axiom能力为MCP Tools |
| **P4-03** | Tool桥接 | MCP Tool ↔ axiom Tool 映射 |
| **P4-04** | 安全层 | Permission → Rules → Axiom → Human-in-the-loop |
| **P4-05** | 集成测试 | 完整MCP调用链路测试通过 |

### 3.6 Phase 5: Agent工具链（6周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P5-01** | LLM客户端抽象 | 多模型支持 + Mock + 自动重试 + 结构化输出 |
| **P5-02** | 工具调用框架 | 类型安全工具定义 + 权限控制 + Witness记录 |
| **P5-03** | 工作记忆 | Working Memory + 自动摘要 + Token预算感知 |
| **P5-04** | 规划器 | ReAct + Plan-and-Execute 策略 |
| **P5-05** | 提示词模板 | 类型安全模板 + 组合 + 版本管理 |
| **P5-06** | Identity/Skill系统 | 身份挂载/技能激活/渐进式披露 |

### 3.7 Phase 6: 生产就绪（2周）

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P6-01** | 性能基准测试 | 消息延迟/吞吐/内存使用基准数据 |
| **P6-02** | 压力测试 | 长时间运行稳定性验证 |
| **P6-03** | 用户文档完备 | 用户指南 + API文档 + 教程 + 示例 |
| **P6-04** | CI/CD配置 | GitHub Actions自动构建/测试/clippy |
| **P6-05** | 发布准备 | Cargo publish配置 + 版本号 + CHANGELOG |

---

## 四、质量门禁

### 4.1 代码质量

```bash
# 每次提交前必须通过
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features
cargo build --workspace --all-targets
cargo test --workspace
```

### 4.2 测试覆盖率

| 类别 | 目标数量 |
|------|---------|
| 单元测试 | ≥ 150 个 |
| 集成测试 | ≥ 30 个 |
| 编译失败测试 | ≥ 5 个（验证约束） |
| 架构一致性测试 | ≥ 10 个 |

### 4.3 性能基准

| 指标 | 目标 |
|------|------|
| 单消息投递延迟 | < 10µs |
| 消息总线吞吐 | > 100k msg/s |
| Witness写入开销 | < 1µs |
| CLI命令响应 | < 100ms |

### 4.4 安全性

| 检查项 | 要求 |
|--------|------|
| 零 unwrap/expect | 非测试代码 |
| 依赖审计 | 所有第三方依赖在 AUDITED_DEPS 中 |
| 反向依赖检测 | 无 crate 层循环依赖 |
| 禁止依赖 | 无 async-trait |

---

## 五、发布检查清单

### P0: 必须完成

- [ ] 所有测试通过（≥ 200 个）
- [ ] Clippy 零警告
- [ ] 零 unwrap/expect（非测试代码）
- [ ] Witness持久化接线完成
- [ ] CLI核心命令可用（new/run/top/trace/why）
- [ ] API稳定性文档完成
- [ ] 错误处理完善
- [ ] 架构自约束测试通过

### P1: 应该完成

- [ ] Agent工具链基础（LLM/工具/记忆/规划器）
- [ ] MCP协议桥接
- [ ] Identity/Skill系统
- [ ] 性能基准数据
- [ ] 用户文档完备
- [ ] 示例项目（Hello Agent）
- [ ] CI/CD配置

### P2: 锦上添花

- [ ] RAG组件
- [ ] 评估框架
- [ ] 可视化Dashboard
- [ ] 测试工具（Chaos Monkey）

---

## 六、风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| async fn in trait 兼容性 | 低 | 中 | 使用Rust 1.75+，保留RPITIT兼容路径 |
| 性能问题 | 中 | 高 | 提前做基准测试，优化热点路径 |
| API变更 | 低 | 高 | 严格版本策略，提前弃用警告 |
| 文档落后 | 高 | 中 | 每次功能完成同步更新文档 |
| 测试不充分 | 中 | 高 | 测试先行，集成测试覆盖关键路径 |
| MCP安全漏洞 | 中 | 高 | 四层安全检查，高危工具需审批 |

---

## 七、外部智能体约束总结

### 7.1 约束层次总览

```
┌─────────────────────────────────────────────────────────────┐
│ 第5层：Witness审计约束（追溯级）                              │
│   一切可审计，篡改可检测                                       │
├─────────────────────────────────────────────────────────────┤
│ 第4层：熵治理约束（系统级）                                   │
│   根据系统无序度自动执行治理动作                                │
├─────────────────────────────────────────────────────────────┤
│ 第3层：权限约束（业务级）                                     │
│   Identity + PermissionSet 控制能力范围                        │
├─────────────────────────────────────────────────────────────┤
│ 第2层：运行时约束（兜底）                                     │
│   CellContext内部 can_send_to() 校验                          │
├─────────────────────────────────────────────────────────────┤
│ 第1层：编译期约束（最强）                                     │
│   LayeredCellContext + CanSendTo trait                       │
│   非法跨层调用无法通过编译                                      │
└─────────────────────────────────────────────────────────────┘
```

### 7.2 约束效果

| 外部智能体行为 | 约束层 | 效果 |
|--------------|-------|------|
| Exec层尝试调用Agent层 | 第1层 | 编译失败 |
| Agent层尝试调用Oversight层 | 第1层 | 编译失败 |
| 通过反射绕过编译期检查 | 第2层 | 运行时错误 |
| 尝试调用未授权工具 | 第3层 | Permission拒绝 |
| 系统过载时发送大量请求 | 第4层 | Throttle/Emergency |
| 执行非法操作 | 第5层 | 完整审计记录 |

### 7.3 架构设计原则

> **架构就是一切。**
> 
> 好的架构让错误无处藏身，让故障自动恢复，让系统始终处于低熵、可观测、可理解、可约束的状态。
> 
> **约束者必先受约束**——架构自身也在约束之内，没有例外。

---

## 附录：关键文件索引

| 文件 | 说明 |
|------|------|
| [crates/axiom-core/src/layer.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/layer.rs) | Layer定义和 `can_send_to()` 方法 |
| [crates/axiom-core/src/sealed.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/sealed.rs) | LayerMarker和CanSendTo trait |
| [crates/axiom-core/src/context.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/context.rs) | CellContext和LayeredCellContext |
| [crates/axiom-core/src/cell.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/cell.rs) | Cell trait定义 |
| [crates/axiom-core/src/signal.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/signal.rs) | Signal和SignalEnvelope |
| [crates/axiom-core/src/witness.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/witness.rs) | Witness审计记录 |
| [crates/axiom-core/src/entropy.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-core/src/entropy.rs) | 熵度量 |
| [crates/axiom-macros/src/lib.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-macros/src/lib.rs) | 过程宏 |
| [crates/axiom-oversight/src/entropy_governor.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-oversight/src/entropy_governor.rs) | 熵治理单元 |
| [docs/architecture/01-agent-identity-skills-mcp-rules.md](file:///D:/work/trae/axiom-core-project/docs/architecture/01-agent-identity-skills-mcp-rules.md) | Agent配套设计文档 |
