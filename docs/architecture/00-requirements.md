# Axiom Core - 需求文档

> **架构就是一切。** 一切失败都是架构失败；一切成功都是架构成功。
> 架构不仅约束使用者，也约束架构自身——约束者必先受约束。

---

## 九大设计哲学

| 哲学 | 含义 |
|------|------|
| **可视化** | 系统内部状态必须像看仪表盘一样一目了然，不是靠日志猜 |
| **工程化** | 不是玩具框架，是生产级运行时——可监控、可调试、可回滚 |
| **结构化** | 一切都有类型、有边界、有Schema，拒绝字符串拼接的消息传递 |
| **极简化** | 最少的原语、最少的概念、最少的配置——5个原语构建世界 |
| **低熵化** | 系统无序度必须被监控、被约束、被主动消减，而非放任增长 |
| **一秒速查** | 任何问题"为什么发生？从哪来？怎么修？"一秒内能定位 |
| **自愈化** | 局部崩溃不扩散，自动重启、自动回滚、自动熔断，像Erlang一样 |
| **架构就是一切** | 不依赖模型能力解决架构问题——模型会犯错，架构不能；约束者必先受约束 |
| **智能体专用** | 从零为Agent场景设计，配套完整工具链，不是把Web框架硬套到智能体上 |

---

## 一、问题陈述：为什么需要 Axiom Core

UC Berkeley 对 1642+ 条多智能体执行轨迹的研究发现：**41%–86.7% 的失败源于架构缺陷，而非AI能力不足**。Google Research 证实多智能体网络相比单智能体将错误放大了 **17.2 倍**。

现有智能体框架（LangChain、CrewAI、AutoGPT 等）的根本问题：

| 痛点 | 表现 |
|------|------|
| **黑盒运行** | 你不知道Agent在想什么，不知道它为什么做这个决定，出错了只能看一堆混乱的日志 |
| **消息是字符串** | Agent之间用自然语言传数据，没有Schema，没有类型，理解偏差即失败 |
| **静默退化** | 系统不崩溃，但输出质量悄悄下降——没有错误，没有告警，直到用户发现 |
| **上下文爆炸** | 把所有历史塞进窗口，Token爆炸、Lost-in-the-Middle、Context Rot |
| **交接悬崖** | 8-10次交接后意图漂移概率急剧上升，没有任何机制阻止 |
| **错误传染** | 一个Agent产生的错误数据像病毒一样感染整个系统，3轮内100%感染率 |
| **调试地狱** | 出了问题要翻几万行日志找因果链，复现靠运气 |
| **无法自愈** | Agent挂了就挂了，没有监督树，没有自动重启，没有回滚 |
| **工具碎片化** | LLM客户端自己写、提示词模板自己拼、记忆系统自己搭、评估自己搞——从零造轮子 |
| **框架无约束** | 框架本身不约束你怎么写代码，怎么分层、怎么通信都随意——这正是架构腐化的根源 |
| **无全局监督** | 谁来监督监督者？系统出了架构级问题（如跨层调用、状态分歧）谁能发现？ |
| **无开发工具** | 没有CLI、没有脚手架、没有REPL、没有内置测试工具——开发体验像在裸奔 |

**根本原因**：这些框架是"把LLM调用串起来"的工具库，不是运行时架构。它们没有解决分布式系统的经典问题——状态一致性、故障隔离、因果追踪、架构约束、工具配套——这些问题在非确定性的LLM场景下被指数级放大。

---

## 二、设计原则

### 1. 确定性优先（Determinism First）

```
┌─────────────────────────────────────────────────────────┐
│  Layer 0: 监督层（Oversight）  ← 元层，监督一切          │
│  熵治理、架构合规、意图审计，不直接执行业务逻辑           │
├─────────────────────────────────────────────────────────┤
│  Layer 3: 推理层（LLM/非确定性）                         │
│  输出必须经过Axiom验证，不能直接产生副作用                │
├─────────────────────────────────────────────────────────┤
│  Layer 2: 验证层（确定性）                               │
│  Schema校验、规则引擎、Axiom不变量检查                    │
├─────────────────────────────────────────────────────────┤
│  Layer 1: 执行层（确定性）                               │
│  数据库、API调用、计算，幂等+自动重试                     │
└─────────────────────────────────────────────────────────┘
```

- Layer 0（监督层）独立于业务层，监督一切但不执行业务逻辑
- LLM只做判断，不直接执行有副作用的操作
- 能用代码/规则/SQL解决的，绝不用LLM
- 不可逆操作必须有审批关卡
- **调用方向铁律**：Layer 3 → Layer 2 → Layer 1（编译期强制），Layer 0 可观察所有层

### 2. 五个原语构建一切（5 Primitives）

不发明100个概念，用5个核心原语组合出整个系统：

| 原语 | 一句话说明 | 类比 |
|------|-----------|------|
| **Cell** | 隔离的状态单元，私有状态+消息信箱，单线程执行 | Erlang Actor / 细胞 |
| **Signal** | 类型化不可变消息，带因果时钟+链路追踪ID | 类型安全的信件 |
| **Lens** | 按需从事件日志投影状态，不是塞全部历史进上下文 | 数据库视图/显微镜 |
| **Axiom** | 全局不变量约束，违反就熔断，是"熵的减压阀" | 物理定律/断言 |
| **Witness** | 每次状态转换自动产生的不可篡改审计记录 | 黑匣子/区块链 |

**极简化**：学会5个概念就能理解整个系统。

### 3. 低熵是第一公民（Low-Entropy as First-Class Concern）

熵不是隐喻，是可度量、可监控、需要主动管理的系统属性：

- **熵度量**：基于Axiom违反率、Witness异常率、消息循环数、意图漂移分数计算实时熵值
- **熵阈值**：每个Cell有黄线/红线，超过黄线告警，超过红线自动减熵
- **自动减熵**：状态快照重置 → 垃圾回收 → Cell冷启动 → 熔断，分级响应
- **架构减熵**：Axiom就是"减熵器"，Witness就是"熵监控器"，监督树就是"熵隔离墙"，Oversight就是"熵免疫系统"

### 4. 架构自约束（Architecture Self-Constraint）

**约束者必先受约束。** 架构不仅约束用户代码，也约束框架自身：

- 分层规则在类型系统中强制执行，编译期阻止非法跨层调用
- 模块依赖方向在编译期检查（core ← runtime ← agent ← viz/cli，不能反向依赖）
- 框架内部代码同样受Axiom约束——不是只有用户代码需要守规矩
- 架构规则可被机器检查，不是靠文档约定

### 5. 可视化内生（Built-in Visualization）

不是加个Dashboard就叫可视化——可视化能力是架构内生的：

- **Witness链**天然提供完整的状态转换历史→时间轴可视化
- **Vector Clock**天然提供因果关系图→依赖图可视化
- **Axiom违反**天然提供违规点标注→热力图可视化
- **Cell监督树+Oversight**天然提供系统拓扑→架构图可视化
- **熵度量**天然提供健康度仪表盘→红绿黄状态灯

一秒速查的基础：每个错误都带 Witness 链，点击就能看到完整因果路径。

### 6. 自愈不是选项，是默认（Self-Healing by Default）

借鉴Erlang/OTP 40年验证的"让它崩溃"哲学：

- **监督树**：Cell崩溃1ms内被检测到，按策略重启/停止/升级
- **隔离边界**：一个Cell崩溃不影响其他Cell，不共享内存，只通过消息通信
- **事件溯源**：崩溃后可以从事件日志重放恢复状态
- **幂等Signal**：重复消息不产生重复副作用
- **熔断机制**：连续失败自动断开，避免雪崩

### 7. 工具体系完备（Complete Toolchain）

不是只给你一个运行时库就完事——从项目创建到部署调试，全流程有工具支撑：
- **CLI**：`axm` 命令行工具，项目脚手架+运行+调试+监控一体化
- **Agent工具库**：LLM客户端、工具调用、记忆系统、规划器、RAG、评估，开箱即用
- **测试工具**：LLM Mock、确定性重放、故障注入、Golden Set测试

---

## 三、四层架构总览

在原有的三层（执行/验证/推理）之上，增加第零层——**监督层（Oversight Layer）**：

```
┌─────────────────────────────────────────────────────────┐
│  Layer 0: 监督层（Oversight）  ← 元层，监督一切          │
│  熵治理、意图审计、架构合规、资源管控、全局健康监督        │
│  监督层自身也被Axiom约束，独立于业务Cell监督树            │
├─────────────────────────────────────────────────────────┤
│  Layer 3: 推理层（LLM/非确定性）                         │
│  输出必须经过Axiom验证，不能直接产生副作用                │
├─────────────────────────────────────────────────────────┤
│  Layer 2: 验证层（确定性）                               │
│  Schema校验、规则引擎、Axiom不变量检查                    │
├─────────────────────────────────────────────────────────┤
│  Layer 1: 执行层（确定性）                               │
│  数据库、API调用、计算，幂等+自动重试                     │
└─────────────────────────────────────────────────────────┘
```

**关键规则**：
- 监督层可以观察所有层，但不执行业务逻辑
- Layer 3 → Layer 2 → Layer 1 是唯一的调用方向（编译期强制）
- Layer 1 永远不能回调 Layer 2 或 Layer 3
- 监督层的Cell有最高优先级运行保证

---

## 四、核心原语详细需求

### Cell（单元）

| 需求 | 说明 |
|------|------|
| 私有状态 | Rust所有权系统强制，外部只能通过消息访问，无共享内存 |
| 单线程执行 | 一个Cell同一时间只处理一条消息，无锁、无竞态条件 |
| 类型状态 | Cell生命周期（Created→Running→Suspended→Crashed→Stopped）在编译期保证 |
| 层标签 | 每个Cell必须声明所属层级（Exec/Validate/Agent/Oversight），编译期检查调用方向 |
| 监督策略 | Restart（N次重试）/ Stop / Escalate（上报父监督者） |
| 心跳检测 | 内置心跳，静默失败（死锁/无限循环）能被检测 |
| 有界信箱 | 信箱容量上限+背压机制，防止OOM |
| 热升级 | 不停止系统替换Cell实现 |
| 元信息 | 每个Cell有name、version、layer标签，用于监督层审计 |

### Signal（信号）

| 需求 | 说明 |
|------|------|
| 类型安全 | 必须实现Schema trait，编译期检查消息类型 |
| 不可变 | 发送后不可修改，像事件溯源中的Event |
| Vector Clock | 每个Signal携带版本向量，追踪因果关系 |
| 关联ID | correlation_id贯穿全链路，支持分布式追踪 |
| 新鲜度 | 携带时间戳，接收方可检查数据是否过期 |
| 幂等ID | msg_id全局唯一，自动去重 |
| 层标签 | Signal标记来源层和目标层，编译期+运行时双层检查跨层调用 |
| 三类消息 | Command（请求操作）、Event（已发生事实）、Query（只读查询） |
| 发送者标记 | 记录发送者Cell ID和层级，用于拓扑追踪和架构合规检查 |

### Lens（透镜）

| 需求 | 说明 |
|------|------|
| 按需投影 | 不是塞全部历史进上下文，而是精确查询需要的状态 |
| 可组合 | Lens可以组合其他Lens，像函数式编程的透镜组合子 |
| 缓存失效 | 基于Vector Clock自动失效缓存 |
| 时间旅行 | 支持查询任意历史时间点的状态（事件重放） |
| 权限边界 | 编译期保证一个Lens只能看到授权的状态子集 |
| Token预算感知 | Lens投影结果自动估算Token数，超预算时自动摘要（但保留关键信息） |

**解决的问题**：上下文工程不是"怎么塞更多token"，而是"怎么精确地给Agent它需要的信息"。

### Axiom（公理）

| 需求 | 说明 |
|------|------|
| 纯函数 | Axiom检查必须是确定性纯函数，无副作用，易测试 |
| 可组合 | 多个Axiom可以组成Axiom链，全部通过才能修改状态 |
| 违规策略 | Reject（拒绝）/ Warn（告警）/ CircuitBreak（熔断）/ Rollback（回滚） |
| 可命名 | 每个Axiom有名称，违规时精确到哪条公理被违反 |
| 层感知 | Axiom可以声明适用于哪些层（如"执行层Axiom"、"推理层Axiom"） |
| 熵触发器 | Axiom违反率是熵计算的核心输入 |
| 框架自约束 | 框架内部操作同样经过Axiom检查 |

**Axiom示例**：
- "订单金额不能为负数"（业务规则）
- "Agent连续失败3次必须熔断"（可靠性规则）
- "交接次数不能超过8次"（低熵规则）
- "推理层Signal不能直接发送到执行层"（架构规则）
- "单请求Token消耗不超过预算"（资源规则）

### Witness（见证）

| 需求 | 说明 |
|------|------|
| 自动生成 | 每次状态转换自动产生Witness，无需手动埋点 |
| 链式哈希 | 前一个Witness的hash包含在后一个中，防篡改 |
| 记录内容 | 触发消息、前状态hash、后状态hash、转换结果、时间戳、所属层级 |
| 层标签 | 记录该转换发生在哪一层 |
| 结果分类 | Success / Failed / AxiomViolated（带违规的公理名） |
| 可采样 | 高吞吐场景支持配置采样率，不是必须全量记录 |
| 一秒速查 | Witness链就是问题的"时间线录像机" |
| 监督层专用 | 监督层可以查询所有Cell的Witness，业务Cell只能查自己的 |

---

## 五、监督层（Oversight Layer）需求

监督层是区别于普通"监督树"的**元治理层**。监督树处理的是"Cell崩溃了重启"，监督层处理的是"系统整体是否在正确地运行"。

### 5.1 监督层的定位

| 对比维度 | 监督树（Supervision Tree） | 监督层（Oversight Layer） |
|---------|--------------------------|--------------------------|
| 处理问题 | Cell崩溃、panic、超时 | 架构违规、熵超标、意图漂移、资源滥用 |
| 作用范围 | 父子Cell之间 | 全局系统级 |
| 决策速度 | 毫秒级 | 秒级（聚合分析后决策） |
| 类比 | 免疫系统（自动应答） | 大脑前额叶（全局思考决策） |
| 可否重启 | 可以被监督树重启 | 监督层Cell由独立的最小内核保证运行 |

### 5.2 监督层职责

| 职责 | 说明 |
|------|------|
| **全局熵监控** | 聚合所有Cell的熵值，计算系统级熵值，触发全局减熵动作 |
| **架构合规检查** | 检测非法跨层调用、依赖方向违规、Axiom绕过尝试 |
| **意图对齐审计** | 对比Agent输出与初始目标的偏离程度，检测角色漂移 |
| **资源治理** | Token预算分配、API速率限制、公平调度、防止单Cell垄断资源 |
| **死锁/循环检测** | 全局检测消息循环（不仅仅是两Cell间），检测资源死锁 |
| **敏感操作拦截** | PII泄露检测、危险操作二次确认、合规审计 |
| **降级决策** | 监督层自身异常时，系统进入"安全模式"——只允许确定性操作 |
| **全局审计日志** | 聚合所有Witness，生成系统级审计报告 |

### 5.3 监督层的Cell组成

| Oversight Cell | 职责 |
|---------------|------|
| **EntropyGovernor** | 熵监控、熵阈值告警、触发分级减熵动作 |
| **ArchitectureGuardian** | 架构合规检查、跨层调用检测、依赖方向验证 |
| **IntentAuditor** | 意图对齐审计、角色漂移检测、目标偏离告警 |
| **ResourceManager** | Token预算、API限流、资源公平分配 |
| **LoopDetector** | 全局消息循环检测、死锁检测、交接次数限制执行 |
| **ComplianceGuard** | 敏感数据检测、危险操作审批、合规记录 |
| **OversightOversight** | 元监督——监督监督层自身，防止监督层失控 |

### 5.4 监督层安全保障

- **Fail-Safe降级**：监督层异常时，系统进入安全模式（推理层暂停，只允许确定性操作）
- **最小内核**：监督层运行在独立的最小执行内核上，不依赖业务Cell的正确性
- **Axiom约束**：监督层Cell同样受Axiom约束，有自己的Axiom集合
- **Witness全记录**：监督层的所有决策都产生Witness，可审计
- **不直接执行业务**：监督层只做决策（发Signal），不直接修改业务状态

---

## 六、架构自约束需求（Architecture Self-Constraint）

**"谁来监督监督者？"——答案是：类型系统+Axiom+Oversight三者联合。**

### 6.1 编译期约束

| 约束 | 实现方式 |
|------|---------|
| **分层调用方向** | `ExecCell` trait没有发消息给AgentCell的方法；`AgentCell`只能通过`ValidatorRef`访问验证层；编译期阻止直接引用 |
| **模块依赖方向** | Rust模块系统+独立crate保证：core不依赖任何上层；runtime只依赖core；agent只依赖core+runtime；cli/viz依赖所有但不被依赖 |
| **私有状态隔离** | Cell的state字段是私有（private），只有Cell自身的impl块能访问 |
| **Signal不可变** | Signal的所有字段都是`pub(self)`或通过getter访问，无setter |
| **unsafe隔离** | 所有unsafe代码必须放在`unsafe_impl`模块中，模块外无法调用unsafe函数 |

### 6.2 运行期约束

| 约束 | 实现方式 |
|------|---------|
| **跨层Signal检查** | 每条Signal携带source_layer和target_layer，Oversight的ArchitectureGuardian实时检查 |
| **Axiom全局执行** | 状态修改前必须通过当前层注册的所有Axiom |
| **依赖环检测** | 启动时检测Cell依赖图中的环，运行时动态检测新注册的Cell是否引入环 |
| **资源配额检查** | 每个Cell有Token/CPU/内存配额，超配额Oversight介入 |

### 6.3 框架自约束

- 框架内部的Cell（如runtime内置的Cell、store的Cell）同样标注层级、同样受Axiom约束
- 框架不能"绕过"Axiom直接修改状态——所有状态修改必须走`apply_signal`路径
- CLI和Viz只能通过公开API与系统交互，没有"后门"
- `axiom-macros`生成的代码自动注入Axiom检查，不会遗漏

---

## 七、CLI工具需求（`axm` CLI）

命令行工具 `axm`（发音"axiom"）是开发、调试、运维Axiom系统的一体化工具。类似 `cargo` + `kubectl` + `erl`（Erlang shell）的结合体。

### 7.1 项目脚手架

```bash
axm new my-agent-project          # 创建新项目（带完整目录结构+示例Cell+Cargo.toml）
axm new cell OrderService         # 在已有项目中创建新Cell模板
axm new axiom "NoNegativeAmount"  # 创建新Axiom模板
axm new lens OrderHistory         # 创建新Lens模板
```

### 7.2 运行与开发

```bash
axm run                           # 启动系统（开发模式，带热重载）
axm run --release                 # 生产模式启动
axm run --profile perf            # 带性能分析启动
axm dev                           # 开发模式（自动重载+详细日志）
```

### 7.3 实时监控（类htop/top）

```bash
axm top                           # 实时TUI仪表盘：Cell状态、熵值、消息吞吐、延迟
axm top --cell order-service      # 聚焦某个Cell的详细状态
axm top --entropy                 # 熵值监控视图
axm top --messages                # 消息流向实时视图
```

### 7.4 调试与诊断（一秒速查的CLI入口）

```bash
axm trace <correlation_id>        # 追踪完整调用链（Witness链+跨Cell路径）
axm why <entity_id>               # 一秒速查："为什么X变成这个状态？"——输出完整因果链
axm witness <cell_id>             # 查看Cell的Witness历史
axm witness <cell_id> --last 10   # 最近10条Witness
axm entropy                       # 查看当前系统/各Cell熵值
axm doctor                        # 系统健康诊断（检测异常、建议修复）
```

### 7.5 事件重放与测试

```bash
axm replay <correlation_id>       # 重放特定请求（用于调试复现）
axm replay --from timestamp       # 从某个时间点重放
axm test                          # 运行所有确定性测试
axm test --chaos                  # 运行Chaos测试（故障注入）
axm test --record                 # 录制运行轨迹作为Golden Set
axm test --replay <recording>     # 用录制的轨迹重放测试
```

### 7.6 运维与治理

```bash
axm cell list                     # 列出所有Cell及其状态
axm cell restart <cell_id>        # 手动重启Cell
axm cell stop <cell_id>           # 停止Cell
axm cell suspend <cell_id>        # 暂停Cell（不销毁状态）
axm axiom list                    # 列出所有已注册Axiom
axm axiom check <axiom_name>      # 手动触发Axiom检查
axm entropy threshold --set yellow=0.6 red=0.8  # 设置熵阈值
axm logs <cell_id>                # 查看Cell日志（自动关联correlation_id）
axm snapshot <cell_id>            # 创建Cell状态快照
axm rollback <cell_id> <snapshot_id>  # 回滚到快照
```

### 7.7 交互式Shell

```bash
axm shell                         # 进入Axiom REPL（类似Erlang shell）
> cell.list()                     # 在REPL中执行操作
> cell.send("order-svc", GetOrder { id: 123 })
> witness.query({ cell: "order-svc", outcome: "AxiomViolated" })
> entropy.system()
> help
```

### 7.8 CLI技术要求

- 使用 `clap`  derive宏定义CLI
- TUI界面使用 `ratatui`（类htop的终端界面）
- 通过Unix socket/TCP与运行中的Axiom系统通信（类似Erlang分布式shell）
- 输出支持 `--json` 格式，便于脚本集成
- 支持自动补全（bash/zsh/powershell）
- 彩色输出、进度条、表格格式化

---

## 八、智能体工具链需求（axiom-agent）

Axiom Core不只是运行时——它提供一整套Agent开发工具，让用户不用从零造轮子。

### 8.1 axiom-llm：LLM客户端抽象

| 需求 | 说明 |
|------|------|
| 多模型支持 | OpenAI/Anthropic/本地Ollama/vLLM等统一接口 |
| 自动重试 | 指数退避+抖动，处理速率限制和临时错误 |
| 内置缓存 | 基于prompt hash的响应缓存（开发模式+可选生产模式） |
| 限流 | Token桶限流，防止超限 |
| Fallback | 主模型失败自动切换备用模型 |
| 结构化输出 | 强制JSON Schema输出（不是"请输出JSON"，是底层guided decoding） |
| 流式支持 | 原生支持streaming响应 |
| 成本追踪 | 自动统计Token消耗和费用 |
| 遥测 | 每个LLM调用自动产生Witness（含token数、延迟、成本） |
| Mock模式 | 内置Mock LLM客户端，用于测试（返回预设响应） |

### 8.2 axiom-tool：工具调用框架

| 需求 | 说明 |
|------|------|
| 类型安全工具定义 | 用Rust trait定义工具，参数自动Schema化 |
| 自动注册 | 工具自动注册到ToolRegistry，Agent可发现可用工具 |
| 参数验证 | 工具调用前自动验证参数类型和约束 |
| 权限控制 | 工具可以标注权限级别，高危工具需Axiom审批 |
| 超时控制 | 每个工具调用有超时，防止挂起 |
| 重试策略 | 可配置重试策略（与执行层幂等保证配合） |
| 工具Witness | 每次工具调用产生Witness（输入、输出、耗时、成功/失败） |

### 8.3 axiom-memory：记忆系统

| 需求 | 说明 |
|------|------|
| 工作记忆（Working Memory） | 当前对话/任务的短期记忆，通过Lens投影给Agent |
| 情景记忆（Episodic Memory） | 历史经历存储，支持语义检索 |
| 语义记忆（Semantic Memory） | 知识存储，向量检索+关键词混合 |
| 程序记忆（Procedural Memory） | 已学会的技能/策略存储（成功的执行路径） |
| 自动摘要 | 工作记忆超长时自动生成摘要（不丢失关键信息） |
| 记忆衰减 | 时间久远、相关度低的记忆自动降权 |
| 记忆溯源 | 每条记忆有来源（哪个Witness/哪个Signal产生的） |
| Token预算感知 | Lens投影记忆时考虑Token预算 |

### 8.4 axiom-planner：规划器抽象

| 需求 | 说明 |
|------|------|
| 多策略 | ReAct / Plan-and-Execute / Tree of Thoughts / 自定义策略 |
| 规划Axiom | 计划必须满足约束（如"步骤不超过N步"、"禁止循环"） |
| 计划重规划 | 执行失败时自动触发重规划（基于当前状态） |
| 计划Witness | 每个规划决策产生Witness（为什么选这个方案） |
| 可观测 | 规划过程完全可追溯（思考链+决策点+替代方案） |

### 8.5 axiom-prompt：类型安全提示词模板

| 需求 | 说明 |
|------|------|
| 类型安全模板 | 编译期检查模板变量是否存在、类型是否正确 |
| 模板组合 | 提示词可以像函数一样组合（system + few-shot + user template） |
| 版本管理 | 提示词模板版本化，A/B测试支持 |
| 自动转义 | 自动处理特殊字符、注入防护 |
| 渲染Witness | 每次渲染产生Witness（用了哪个模板、参数是什么） |

### 8.6 axiom-rag：RAG基础组件

| 需求 | 说明 |
|------|------|
| 文档摄入 | 支持PDF/Markdown/HTML/纯文本摄入 |
| 分块策略 | 可插拔分块器（固定大小/语义分块/结构化分块） |
| 向量存储 | trait抽象，内置内存实现，可接Qdrant/Milvus/pgvector |
| 混合检索 | 向量检索+关键词检索+重排（Rerank） |
| 引用溯源 | 检索结果带来源文档片段+位置，Agent可引用 |
| 检索Witness | 每次检索产生Witness（查询、结果数、延迟、来源） |

### 8.7 axiom-eval：评估框架

| 需求 | 说明 |
|------|------|
| Golden Set测试 | 用录制的输入→期望输出对做回归测试 |
| 轨迹重放评估 | 用axm test --record录制的轨迹做评估 |
| LLM-as-Judge | 用LLM评估输出质量（但评估结果本身经过Axiom验证） |
| 指标统计 | 成功率、平均Token消耗、平均延迟、意图漂移分数 |
| 回归对比 | 对比不同版本prompt/model/architecture的指标变化 |
| CI/CD集成 | 评估命令可在CI中运行，指标不达标阻断发布 |

### 8.8 axiom-test：Agent测试工具

| 需求 | 说明 |
|------|------|
| Mock LLM | axm test自动Mock LLM客户端，返回录制的响应 |
| 确定性时间 | 测试中时间可控，不依赖系统时钟 |
| 故障注入 | 模拟Cell崩溃、网络超时、LLM返回错误、高延迟 |
| 轨迹录制/重放 | 录制真实运行轨迹，在测试中确定性重放 |
| 属性测试 | proptest风格的属性-based testing |
| 覆盖度 | 测量Cell覆盖、Axiom触发覆盖、分支覆盖 |

---

## 九、可视化接口需求（Visualization API）

架构内生数据直接驱动可视化，无需额外埋点：

| 可视化能力 | 数据来源 |
|-----------|---------|
| **实时架构拓扑图** | 监督树结构+Oversight注册信息+层标签 |
| **消息流向图** | Signal的sender→receiver+correlation_id+layer标签 |
| **状态时间轴** | Witness链（每个Cell的状态变化历史+层级标注） |
| **因果依赖图** | Vector Clock偏序关系 |
| **熵值仪表盘** | 系统级/Cell级实时熵值，红黄绿状态灯 |
| **Axiom违规热力图** | Axiom违反频率和分布 |
| **错误因果链** | correlation_id串联的完整调用链（跨层可见） |
| **性能火焰图** | 每个Signal的处理延迟分布（按层着色） |
| **监督层面板** | Oversight各Cell的决策记录、治理动作、审计日志 |
| **Token/成本面板** | 各Cell/请求的Token消耗和成本实时统计 |

**一秒速查场景**：
- 问："为什么这个订单被退款了？" → `axm why order-123` → 看到完整Witness链→跨层路径→Axiom检查结果→结论
- 问："为什么Agent没有回复用户？" → `axm top`发现Cell Crashed→`axm witness`看最后一个Witness→AxiomViolated→原因→修复
- 问："系统为什么变慢了？" → `axm top --entropy`发现熵值飙升→`axm trace`找到循环消息→定位到两个Agent在辩论
- 问："这个功能花了多少Token？" → `axm trace <corr_id>`显示每个LLM调用的Token数+成本

---

## 十、工程化需求

### 可靠性

| 指标 | 目标 |
|------|------|
| 消息投递 | 至少一次（At-least-once） |
| 崩溃检测 | Cell崩溃后1ms内检测到 |
| 重启时间 | 崩溃后<5ms重启（热路径） |
| 消息不丢 | 有界信箱+背压，不静默丢弃 |
| 监督层可用性 | 监督层故障不导致业务停摆（降级到安全模式） |

### 可测试性

| 需求 | 说明 |
|------|------|
| 确定性执行器 | Cell可在模拟时间中测试，无flaky test |
| 事件重放测试 | 用录制的真实事件流重放验证 |
| Axiom单元测试 | Axiom是纯函数，像测试普通函数一样 |
| LLM Mock | 内置Mock LLM，测试不依赖真实API |
| 故障注入 | 内置Chaos Monkey能力，模拟Cell崩溃/网络分区/LLM故障 |
| 属性测试 | 支持proptest风格的随机测试 |

### 可观测性（四层监控）

| 层级 | 内容 |
|------|------|
| **执行追踪层** | correlation_id贯穿全链路，Vector Clock追踪因果 |
| **状态监控层** | 每次交接前后状态diff，状态传播延迟SLA，过期状态查询频率 |
| **意图对齐层** | 实时意图保留分数，角色漂移检测，消息循环检测 |
| **熵监控层** | 系统/Cell实时熵值，熵增速率，减熵动作记录，Oversight决策记录 |

### 性能

| 指标 | 目标（非LLM路径） |
|------|-----------------|
| 单消息投递延迟 | <10µs |
| 消息总线吞吐 | >100k msg/s |
| Witness写入开销 | <1µs（追加写） |
| Axiom检查开销 | <100ns（纯函数） |
| CLI命令响应 | `axm top`等实时命令延迟<100ms |
| axm why查询 | 1秒内返回完整因果链 |

---

## 十一、自愈机制需求

| 机制 | 触发条件 | 决策层 | 动作 |
|------|---------|--------|------|
| **Cell重启** | Cell panic/无限循环/心跳超时 | 监督树 | 按监督策略重启，最多N次 |
| **消息重试** | 临时错误/网络超时 | 执行层 | 指数退避重试，幂等保证 |
| **熔断** | 连续N次Axiom违反/崩溃率超阈值 | 监督树+Axiom | 断开Cell入口，停止接收消息 |
| **状态回滚** | Axiom违反且策略为Rollback | Axiom | 回滚到上一个有效快照 |
| **升级处理** | 重试次数耗尽 | 监督树 | Escalate给父监督者 |
| **垃圾回收** | 上下文膨胀/Token超限 | Lens+Oversight | 状态压缩，丢弃过时信息 |
| **循环检测** | Cell间消息往返超M次 | Oversight LoopDetector | 强制断开，标记死循环 |
| **全局减熵** | 系统熵超红线 | Oversight EntropyGovernor | 分级减熵（GC→快照重启→熔断→安全模式） |
| **架构违规阻断** | 检测到非法跨层调用 | Oversight ArchitectureGuardian | 立即阻断，产生高优先级告警 |
| **资源配额治理** | Cell超Token/CPU/内存配额 | Oversight ResourceManager | 限流→暂停→重启 |
| **监督层自愈** | Oversight Cell异常 | 最小内核 | 重启Oversight Cell，同时降级到安全模式 |

---

## 十二、技术约束（Rust）

- **零成本抽象**：所有原语在编译期消解，运行时无额外开销
- **类型状态模式**：非法状态在编译期不可表达
- **unsafe隔离**：所有unsafe代码必须有SAFETY注释，隔离在`unsafe_impl`模块
- **Trait优先**：核心抽象通过trait定义，实现可插拔（内存存储→SQLite→Kafka；OpenAI→本地模型）
- **错误分层**：`thiserror`定义领域错误，`anyhow`仅用于应用边界
- **不引入async-trait开销**：使用原生async fn in traits（Rust 1.75+）
- **no_std兼容**：core crate支持no_std（可选），runtime需要std
- **无全局状态**：不使用全局可变静态变量，所有状态通过Runtime传递
- **依赖最小化**：核心crate依赖尽量少，可选择启用feature

---

## 十三、明确不做（v1范围）

| 不做 | 原因 |
|------|------|
| 分布式集群 | 第一版单进程，网络层后续独立crate（axiom-cluster）实现 |
| 持久化队列 | 内存事件存储起步，Kafka/NATS后续可插拔 |
| 工作流DSL | 用Rust代码定义流程，不发明新的DSL |
| "群体智能"/"涌现" | 高熵系统，违背Axiom核心哲学 |
| 解决数学上不可能的问题 | 我们不预测未来，只检测+约束+恢复 |
| Web框架集成 | Web只是一个I/O适配器（Cell），不是框架核心 |
| 内置向量数据库 | 通过trait抽象对接现有向量数据库 |
| GUI Dashboard（v1） | v1提供TUI（axm top）和数据导出API，Web GUI后续 |
| 多语言SDK（v1） | v1只有Rust原生API，其他语言SDK后续通过FFI/HTTP API |

---

## 十四、Crates规划

```
axiom-core/
├── Cargo.toml                    # Workspace根
├── crates/
│   │
│   ├── === 核心层（Layer 0+ 核心原语） ===
│   │
│   ├── axiom-core/               # 5个核心原语（Cell/Signal/Lens/Axiom/Witness）
│   │   ├── src/
│   │   │   ├── cell.rs           # Cell trait + CellId + 层标签 + SupervisionStrategy
│   │   │   ├── signal.rs         # Signal trait + VectorClock + SignalKind + 层标签
│   │   │   ├── lens.rs           # Lens trait + 组合子 + Token预算感知
│   │   │   ├── axiom.rs          # Axiom trait + ViolationAction + AxiomChain
│   │   │   ├── witness.rs        # Witness + WitnessHash + TransitionOutcome
│   │   │   ├── layer.rs          # Layer枚举（Exec/Validate/Agent/Oversight）
│   │   │   ├── entropy.rs        # 熵值计算原语
│   │   │   └── error.rs          # AxiomError + Result
│   │   └── examples/
│   │       └── hello_cell.rs     # 最小可运行示例
│   │
│   ├── axiom-runtime/            # Tokio运行时 + 监督树 + 消息总线
│   │   └── src/
│   │       ├── runtime.rs        # AxiomRuntime入口
│   │       ├── supervisor.rs     # 监督树实现
│   │       ├── mailbox.rs        # MPSC无锁信箱
│   │       ├── bus.rs            # 消息总线（直送/发布订阅/请求响应）
│   │       ├── dispatcher.rs     # 消息分发 + 跨层检查
│   │       └── kernel.rs         # 最小内核（保证Oversight运行）
│   │
│   ├── axiom-oversight/          # 监督层（Layer 0）
│   │   └── src/
│   │       ├── mod.rs            # OversightRuntime
│   │       ├── entropy_governor.rs     # 熵治理Cell
│   │       ├── architecture_guardian.rs # 架构合规Cell
│   │       ├── intent_auditor.rs       # 意图审计Cell
│   │       ├── resource_manager.rs     # 资源管理Cell
│   │       ├── loop_detector.rs        # 循环检测Cell
│   │       ├── compliance_guard.rs     # 合规检查Cell
│   │       └── oversight_oversight.rs  # 元监督Cell
│   │
│   ├── axiom-store/              # 事件存储
│   │   └── src/
│   │       ├── event.rs          # Event定义
│   │       ├── store.rs          # EventStore trait
│   │       ├── memory.rs         # 内存实现
│   │       ├── snapshot.rs       # 快照机制
│   │       └── replay.rs         # 事件重放
│   │
│   ├── axiom-macros/             # 过程宏
│   │   └── src/
│   │       ├── cell.rs           # #[cell] 宏
│   │       ├── axiom.rs          # #[axiom] 宏
│   │       ├── signal.rs         # #[signal] 宏（派生Schema）
│   │       └── lib.rs
│   │
│   ├── axiom-viz/                # 可视化数据导出层
│   │   └── src/
│   │       ├── topology.rs       # 拓扑图数据
│   │       ├── timeline.rs       # 时间轴数据
│   │       ├── entropy.rs        # 熵仪表盘数据
│   │       ├── trace.rs          # 链路追踪数据
│   │       └── metrics.rs        # 性能指标数据
│   │
│   │
│   ├── === 智能体工具层 ===
│   │
│   ├── axiom-agent/              # Agent开发工具集（fascade crate，re-export子crate）
│   │   └── src/lib.rs
│   │
│   ├── axiom-llm/                # LLM客户端抽象
│   │   └── src/
│   │       ├── provider.rs       # LLMProvider trait
│   │       ├── openai.rs         # OpenAI实现
│   │       ├── anthropic.rs      # Anthropic实现
│   │       ├── mock.rs           # Mock实现（测试用）
│   │       ├── cache.rs          # 响应缓存
│   │       ├── retry.rs          # 重试+限流+fallback
│   │       └── structured.rs     # 结构化输出（guided decoding）
│   │
│   ├── axiom-tool/               # 工具调用框架
│   │   └── src/
│   │       ├── registry.rs       # 工具注册中心
│   │       ├── tool.rs           # Tool trait
│   │       └── permission.rs     # 工具权限控制
│   │
│   ├── axiom-memory/             # 记忆系统
│   │   └── src/
│   │       ├── working.rs        # 工作记忆
│   │       ├── episodic.rs       # 情景记忆
│   │       ├── semantic.rs       # 语义记忆
│   │       ├── procedural.rs     # 程序记忆
│   │       └── summarizer.rs     # 自动摘要
│   │
│   ├── axiom-planner/            # 规划器
│   │   └── src/
│   │       ├── planner.rs        # Planner trait
│   │       ├── react.rs          # ReAct策略
│   │       └── plan_execute.rs   # Plan-and-Execute策略
│   │
│   ├── axiom-prompt/             # 类型安全提示词
│   │   └── src/
│   │       ├── template.rs       # 模板引擎
│   │       └── compose.rs        # 模板组合
│   │
│   ├── axiom-rag/                # RAG基础组件
│   │   └── src/
│   │       ├── ingest.rs         # 文档摄入
│   │       ├── chunking.rs       # 分块策略
│   │       ├── retriever.rs      # 检索trait
│   │       └── rerank.rs         # 重排
│   │
│   ├── axiom-eval/               # 评估框架
│   │   └── src/
│   │       ├── golden.rs         # Golden Set测试
│   │       ├── judge.rs          # LLM-as-Judge
│   │       └── metrics.rs        # 评估指标
│   │
│   ├── axiom-test/               # Agent测试工具
│   │   └── src/
│   │       ├── deterministic.rs  # 确定性执行器
│   │       ├── chaos.rs          # 故障注入/Chaos Monkey
│   │       ├── record.rs         # 轨迹录制
│   │       └── replay.rs         # 轨迹重放
│   │
│   │
│   ├── === CLI ===
│   │
│   └── axiom-cli/                # `axm` 命令行工具（二进制crate）
│       └── src/
│           ├── main.rs           # CLI入口
│           ├── commands/         # 各子命令实现
│           │   ├── new.rs        # axm new
│           │   ├── run.rs        # axm run/dev
│           │   ├── top.rs        # axm top（TUI仪表盘）
│           │   ├── trace.rs      # axm trace
│           │   ├── why.rs        # axm why（一秒速查）
│           │   ├── witness.rs    # axm witness
│           │   ├── replay.rs     # axm replay
│           │   ├── cell.rs       # axm cell list/restart/stop
│           │   ├── test.rs       # axm test
│           │   ├── doctor.rs     # axm doctor
│           │   └── shell.rs      # axm shell（REPL）
│           ├── tui/              # TUI界面（ratatui）
│           └── client.rs         # 与运行时通信的客户端
│
└── docs/
    └── architecture/
        └── 00-requirements.md    # 本文档
```

### Crate依赖方向

```
axiom-macros (proc-macro, 零依赖)
    ↓
axiom-core (5原语+层+熵+错误)
    ↓
axiom-store (事件存储)
    ↓
axiom-runtime (运行时+监督树+总线+内核)
    ↓
axiom-oversight (监督层)
    ↓
axiom-agent 工具集:
  axiom-llm, axiom-tool, axiom-memory, axiom-planner,
  axiom-prompt, axiom-rag, axiom-eval, axiom-test
  (都依赖 axiom-core + axiom-runtime，不互相依赖)
    ↓
axiom-viz (可视化导出)
    ↓
axiom-cli (CLI二进制，依赖所有)
```

**依赖铁律**：只能从上往下依赖，绝对不能反向。core不知道runtime的存在；runtime不知道agent工具的存在。

---

## 十五、成功标准

Axiom Core 成功的标志不是"功能多"，而是：

1. **新手能在30分钟内理解5个原语并写出第一个Cell**
2. **`axm new` 创建项目后1分钟内能跑起 "Hello Agent"**
3. **生产环境中任何问题能在1秒内定位根因**（`axm why`）
4. **Agent崩溃不影响系统整体运行**（自愈默认开启）
5. **系统熵值可监控、可报警、可主动治理**
6. **架构违规被编译期阻止或运行时Oversight即时拦截**
7. **非确定性被严格约束在推理层，不污染执行层**
8. **Agent工具链齐全：LLM/工具/记忆/规划/RAG/评估/测试开箱即用**
9. **零成本抽象：使用框架带来的运行时开销可忽略不计**
10. **工程团队不需要是分布式系统专家也能写出可靠的多Agent系统**

> **架构就是一切。**
> 好的架构让错误无处藏身，让故障自动恢复，让系统始终处于低熵、可观测、可理解、可约束的状态。
> 约束者必先受约束——架构自身也在约束之内，没有例外。
