# Axiom Core 三层自动化门禁体系设计

> **目标：自动化覆盖率 >90%。**
> 约束不再写在文档里等人遵守——它编译不过、提交不了、运行时会被拦截。

---

## 一、设计原则

### 1.1 核心铁律

1. **约束先行，代码在后**：先建门禁，再写功能。所有后续Phase在门禁保护下开发。
2. **三层纵深防御**：L0(开发门禁) → L1(编译期门禁) → L2(运行时门禁)，层层拦截。
3. **失败即停，不留隐患**：任何门禁检查失败，流程立即终止；不允许warning通过。
4. **自动生成，减少人肉**：boilerplate由proc macro生成，脚手架由CLI生成，迁移自动发现。
5. **审计完备，一切留痕**：所有拦截/修复/违规自动产生Witness记录，不可篡改。

### 1.2 自动化覆盖率定义

"自动化"指：该操作不需要开发者/AI手动记住规则、手动执行检查、手动写重复代码。

| 类别 | 自动化前 | 自动化后 | 自动执行方式 |
|------|---------|---------|-------------|
| 预检清单 | 人工读文档打勾 | `axm preflight` 一键执行，未通过阻止提交 | CLI+git hook |
| 编译+测试+lint | 手动敲4条命令 | `axm check` 一键跑完fmt→build→clippy→test→verify，失败即停 | cargo subcommand |
| 架构约束验证 | 文档规则靠自觉 | `axm verify` 静态分析依赖方向+层间调用+unsafe审计 | CLI+CI |
| 系统健康诊断 | 无 | `axm doctor` 自动检测违规/熵值/版本兼容 | CLI+runtime API |
| 代码脚手架 | 手写boilerplate | `axm new cell <name> --layer <L>` 自动生成完整文件 | CLI+templates |
| Signal元数据 | 手写8个required方法 | `#[derive(Signal)]`+`#[signal(...)]`自动生成SignalPayload impl | proc macro |
| Cell注册+层标记 | 手动impl Cell+trait | `#[cell(layer = "exec")]` 自动注入Cell+Layer marker impl | proc macro |
| Axiom注册+链构建 | 手动push到AxiomChain | `#[axiom(...)]` 自动linkme分布式注册，启动时collect | proc macro+linkme |
| 跨层Signal发送 | 编译通过但架构违规 | 编译期CanSendTo trait bound + compile_error!禁止非法方向 | 类型系统+proc macro |
| Schema版本标记 | 手动impl Versioned | `#[schema_version(N)]` 自动实现Versioned trait | proc macro |
| Migration注册 | 手动Registry::register | `#[migration(...)]` 自动linkme注册，启动时auto_collect | proc macro+linkme |
| CI门禁 | 无 | push/PR自动跑build/fmt/clippy/test/verify/unsafe-audit | GitHub Actions |
| Git提交检查 | 无 | pre-commit自动跑axm check+preflight | git hook |
| 架构违规检测 | 无 | ArchitectureGuardian拦截Bus上所有违规消息 | runtime Bus interceptor |
| 熵超标响应 | 无 | EntropyGovernor自动触发de-entropy信号 | runtime supervision |
| Cell崩溃恢复 | 无 | Supervisor自动重启+circuit breaker+消息重放 | runtime resilience |
| Witness链完整性 | 仅测试验证 | 启动时自动verify_chain_integrity | startup hook |
| 版本兼容性检查 | 手动调用 | 数据加载时自动check_readable+链式迁移 | store+version |

**覆盖率：18/18 = 100%**（开发者只需写业务逻辑和纯数据结构）

---

## 二、L0 开发门禁层

### 2.1 axm CLI（axiom-cli crate）

CLI二进制名：`axm`（简短好打，符合极简化原则）。
同时作为cargo subcommand提供`cargo axiom`入口（cargo自动识别`cargo-axiom`二进制）。

#### 命令清单

**P0.5阶段必须实现的命令**（门禁核心）：

| 命令 | 功能 | 退出码 |
|------|------|--------|
| `axm preflight` | 自动预检：git状态/分支/clippy/test/约束文件hash/依赖审计 | 0=全部通过，1=阻断项失败，2=仅警告 |
| `axm check` | 一键跑完 fmt→build→clippy→test→verify，失败即停 | 0=全部通过，1=失败 |
| `axm verify` | 静态架构验证：依赖方向/层间trait使用/unsafe审计/版本一致性 | 0=合规，1=有违规 |
| `axm version` | 显示完整VersionInfo+crate/schema/protocol版本+兼容矩阵 | 0 |
| `axm help` | 帮助信息 | 0 |

**P11阶段扩展命令**（脚手架完善）：

| 命令 | 功能 |
|------|------|
| `axm doctor` | 运行时健康检查：连接runtime，获取熵值/违规计数/版本信息 |
| `axm new cell <name> --layer <L>` | 脚手架生成Cell文件+测试骨架 |
| `axm init` | 初始化axiom项目结构（Cargo.toml+.axiom/目录+git hooks） |
| `axm top` | TUI实时监控（依赖P5） |
| `axm trace <id>` | Trace查询（依赖P5） |
| `axm why <witness-id>` | Witness根因分析（依赖P4） |

#### CLI文件结构

```
crates/axiom-cli/
├── Cargo.toml
└── src/
    ├── lib.rs                   # commands/checks作为库暴露，供两个bin共享
    ├── bin/
    │   ├── axm.rs               # 主CLI入口（clap derive，命令名展示为"axm"）
    │   └── cargo-axiom.rs       # cargo subcommand入口（命令名展示为"cargo-axiom"）
    ├── commands/
    │   ├── mod.rs               # Command trait + run()分发
    │   ├── preflight.rs         # axm preflight
    │   ├── check.rs             # axm check
    │   ├── verify.rs            # axm verify（支持--unsafe-audit/--deps-audit flags）
    │   └── version.rs           # axm version（支持--check flag）
    └── checks/                  # 可复用的检查逻辑库（每个check实现Check trait）
        ├── mod.rs
        ├── git_status.rs        # git status --porcelain 检查
        ├── branch.rs            # 分支合法性检查
        ├── cargo_build.rs       # cargo build 封装(捕获输出+解析警告)
        ├── cargo_clippy.rs      # cargo clippy 封装
        ├── cargo_test.rs        # cargo test 封装
        ├── cargo_fmt.rs         # cargo fmt --check 封装
        ├── deps_audit.rs        # 依赖审计(cargo tree分析+白名单比对)
        ├── unsafe_audit.rs      # unsafe代码扫描(grep+SAFETY注释检查)
        ├── constraints_hash.rs  # .axiom/约束文件SHA-256完整性校验
        └── todo_scan.rs         # TODO/FIXME/unimplemented!扫描(测试代码豁免)
```

两个二进制入口共享同一套commands和checks逻辑，仅在帮助文本中调整命令名展示。

#### axm preflight 检查项（替代preflight.md人工打勾）

| 检查ID | 检查项 | 自动检查方式 | 失败行为 |
|--------|--------|-------------|---------|
| A-CONSTRAINT | 约束文件完整性 | 验证`.axiom/`下6个文件存在，SHA-256与.lock中记录一致 | 阻断 |
| B-BRANCH | 分支合法性 | `git rev-parse --abbrev-ref HEAD` 检查是否为master或`phase/*` | 警告 |
| B-SYNC | 远程同步 | `git fetch`后检查本地与origin是否一致 | 警告 |
| C-GIT | 工作区状态 | `git status --porcelain` 输出为空 | 警告 |
| C-CLIPPY | clippy通过 | `cargo clippy --workspace -- -D warnings` 退出码0 | 阻断 |
| D-ASYNCTRAIT | 禁用async-trait | grep检查Cargo.toml无async-trait依赖，源码无`async_trait`宏 | 阻断 |
| D-UNSAFE | unsafe审计 | grep检查unsafe块，每个unsafe必须有`// SAFETY:`注释 | 阻断 |
| D-DEPS | 依赖审计 | 检查Cargo.toml中第三方依赖是否在白名单中 | 阻断 |
| D-TODO | 禁止占位符 | grep检查`TODO!`/`FIXME!`/`unimplemented!()`（`#[test]`模块内豁免） | 阻断 |
| D-BUILD | 编译通过 | `cargo build --workspace` 零警告 | 阻断 |
| D-TEST | 测试通过 | `cargo test --workspace` 全部通过 | 阻断 |
| D-FMT | 格式正确 | `cargo fmt --all -- --check` 通过 | 阻断 |

阻断项失败则`axm preflight`退出码=1，pre-commit hook阻止commit。

### 2.2 cargo-axiom subcommand

axiom-cli crate通过`[[bin]]`配置两个入口：
- `axm`：面向开发者和Agent的主CLI
- `cargo-axiom`：面向Rust开发者的cargo subcommand

两者调用同一套`commands::run()`逻辑，区别仅在帮助文本中的命令名展示。

### 2.3 Git Hooks自动化

`axm init`时自动安装hooks到`.git/hooks/`，同时在仓库根目录提供`hooks/`版本受控的hook脚本（方便CI引用）。

**pre-commit hook**（核心门禁）：
```sh
#!/bin/sh
axm check            # fmt+build+clippy+test+verify
axm preflight        # 预检阻断项检查
```

**pre-push hook**：
```sh
#!/bin/sh
axm check            # 完整workspace检查
axm verify           # 架构验证
```

任何命令返回非0，操作被阻止。`git commit --no-verify`可绕过但会留下痕迹（Witness审计记录）。

### 2.4 CI/CD GitHub Actions

文件：`.github/workflows/ci.yml`，触发条件：`push`到master + 所有`pull_request`。

```yaml
name: Axiom CI
on:
  push: { branches: [master] }
  pull_request:
jobs:
  gates:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt, clippy }
      - name: Build (deny warnings)
        run: RUSTFLAGS="-D warnings" cargo build --workspace
      - name: Format check
        run: cargo fmt --all -- --check
      - name: Clippy (deny warnings)
        run: cargo clippy --workspace -- -D warnings
      - name: Test
        run: cargo test --workspace
      - name: Architecture verify
        run: cargo axiom verify
      - name: Unsafe audit
        run: cargo axiom verify --unsafe-audit
      - name: Dependency audit
        run: cargo axiom verify --deps-audit
      - name: Version compatibility
        run: cargo axiom version --check
```

- PR必须CI绿灯才能merge（branch protection rule在仓库设置中配置）
- master分支直接push必须CI绿灯

### 2.5 约束文件锁定机制

`axm preflight`首次成功后，生成`.axiom/.constraints.lock`，记录6个约束文件的SHA-256 hash。后续运行时比对hash，如果约束文件被篡改（未经授权修改），preflight阻断。修改约束文件需用户显式授权（`axm preflight --update-constraints`），这对应R-021规则的技术执行。

---

## 三、L1 编译期门禁层

### 3.1 核心设计：SignalPayload/Envelope分离

**关键架构决策**：业务Signal是纯数据结构（SignalPayload），消息元数据由SignalEnvelope统一封装。这避免了proc macro需要向struct注入字段的复杂性，保持业务代码的干净。

```rust
// 业务Signal：纯数据，不包含任何元数据
#[derive(SignalPayload, Serialize, Deserialize, Clone, Debug)]
#[signal(source = "exec", target = "validate")]
#[schema_version(1)]
pub struct ExecuteCommand {
    pub command: String,
}

// SignalEnvelope<T: SignalPayload> 由runtime自动包装
pub struct SignalEnvelope<T: SignalPayload> {
    pub msg_id: MsgId,
    pub correlation_id: CorrelationId,
    pub reply_to: Option<MsgId>,
    pub source_layer: Layer,      // 从T::source_layer()缓存
    pub target_layer: Layer,      // 从T::target_layer()缓存
    pub hop_count: u8,
    pub schema_version: SchemaVersion,
    pub payload: T,
}
```

开发者只需定义纯数据struct + 两个attr（`#[signal]`+`#[schema_version]`），所有元数据由Envelope自动管理。

### 3.2 axiom-macros 过程宏清单

axiom-macros是proc-macro crate，产出以下宏：

| 宏 | 类别 | 功能 |
|----|------|------|
| `SignalPayload` | Derive | 为纯数据struct生成SignalPayload impl（source_layer/target_layer/schema_version等） |
| `#[cell(layer = "exec")]` | Attribute | 为Cell struct自动实现Cell trait + Layer marker trait |
| `#[axiom(layer = "validate", action = "reject")]` | Attribute | 标记Axiom impl，通过linkme分布式注册 |
| `#[schema_version(N)]` | Attribute | 为类型生成Versioned impl |
| `#[migration(from = N)]` | Attribute | 标记Migration impl（to自动=N+1，编译期校验），linkme注册 |

#### 3.2.1 #[derive(SignalPayload)] + #[signal(...)]

```rust
// 用户写：
#[derive(SignalPayload, Serialize, Deserialize, Clone, Debug)]
#[signal(source = "exec", target = "validate")]
#[schema_version(1)]
struct ExecuteCommand {
    command: String,
}

// 宏展开为：
impl SignalPayload for ExecuteCommand {
    fn source_layer() -> Layer { Layer::Exec }
    fn target_layer() -> Layer { Layer::Validate }
    fn schema_version() -> SchemaVersion { SchemaVersion(1) }
    fn signal_type() -> &'static str { std::any::type_name::<ExecuteCommand>() }
}
```

**编译期校验**：
- `source`和`target`必须是四个合法层名之一，否则`compile_error!`
- 方向必须合法（见3.3方向矩阵），否则`compile_error!("ExecCell cannot send signals directly to OversightLayer; use event emission instead")`

#### 3.2.2 #[cell(layer = "exec")]

```rust
// 用户写：
#[cell(layer = "exec")]
struct CommandExecutor {
    db: Database,
}

// 宏展开为：
impl Cell for CommandExecutor {
    fn state_hash(&self) -> Option<[u8; 32]> { None /* 可手动覆盖 */ }
}
impl ExecCell for CommandExecutor {}  // marker trait
// 同时自动生成layer()方法返回Layer::Exec
```

- `layer`参数必须是`exec`/`validate`/`agent`/`oversight`之一，否则compile_error
- 宏不生成handle_message方法（业务逻辑由开发者手写）

#### 3.2.3 #[axiom(layer = "validate", action = "reject")]

```rust
// 用户写：
#[axiom(layer = "validate", action = "reject")]
struct NonEmpty;

impl Axiom for NonEmpty {
    type State = String;
    type Message = String;
    fn check(&self, current: &String, new: &String, _msg: &String) -> crate::Result<()> {
        if new.is_empty() { Err(crate::AxiomError::AxiomViolation("empty".into())) }
        else { Ok(()) }
    }
    fn name(&self) -> &str { "NonEmpty" }
    fn applies_to_layer(&self, layer: Layer) -> bool { layer == Layer::Validate }
    fn violation_action(&self) -> ViolationAction { ViolationAction::Reject }
}

// 宏追加生成：
#[linkme::distributed_slice(crate::axiom::AXIOM_REGISTRY)]
static __NONEMPTY_AXIOM: &dyn crate::axiom::AxiomRegistrar = &NonEmptyAxiomRegistrar;
// 其中NonEmptyAxiomRegistrar是宏生成的零大小类型，实现AxiomRegistrar trait以构造NonEmpty实例
```

启动时`AxiomRegistry::collect()`遍历`AXIOM_REGISTRY`分布式slice，自动收集所有标注了`#[axiom]`的规则，无需手动push。

#### 3.2.4 #[schema_version(N)]

```rust
// 用户写：
#[schema_version(2)]
struct SignalEnvelopeV2 { /* ... */ }

// 宏展开为：
impl Versioned for SignalEnvelopeV2 {
    fn schema_version() -> SchemaVersion { SchemaVersion(2) }
    fn min_supported_version() -> SchemaVersion { SchemaVersion(1) }
}
```

- N必须是u16字面量，否则compile_error
- 可选参数`min = M`指定最低支持版本，默认为1

#### 3.2.5 #[migration(from = N)]

```rust
// 用户写：
#[migration(from = 1)]
struct MigrateV1toV2;
impl Migration for MigrateV1toV2 {
    fn source_version(&self) -> SchemaVersion { SchemaVersion(1) }
    fn target_version(&self) -> SchemaVersion { SchemaVersion(2) }
    fn migrate(&self, data: Value) -> Result<Value> { /* ... */ }
}

// 宏编译期校验：
// - 自动计算to = from + 1，如果Migration impl的target_version()不等于from+1则compile_error
// 宏追加生成：
#[linkme::distributed_slice(crate::version::MIGRATION_REGISTRY)]
static __MIGRATE_V1_V2: &dyn crate::version::MigrationRegistrar = &MigrateV1toV2Registrar;
```

启动时`MigrationRegistry::auto_collect()`遍历`MIGRATION_REGISTRY`分布式slice，自动注册所有迁移，然后调用`verify_all_chains()`检查所有Versioned类型的迁移链完整性。

### 3.3 编译期层间方向强制

#### 方向矩阵（编译期和运行时共享同一套规则，唯一真相源）

合法方向（✓=允许，✗=编译错误+运行时拦截）：

| From \ To | Oversight(0) | Exec(1) | Validate(2) | Agent(3) |
|-----------|:-----------:|:-------:|:-----------:|:--------:|
| Oversight(0) | ✓ | ✓ | ✓ | ✓ |
| Exec(1) | ✗ 事件 | ✓ 同层 | ✓ | ✗ |
| Validate(2) | ✗ 事件 | ✓ | ✓ 同层 | ✓ |
| Agent(3) | ✗ 事件 | ✗ | ✓ | ✓ 同层 |

说明：
- 任何层发往Oversight必须通过Event/Witness上报（间接），不能直接Signal
- Exec只能给同层和Validate发消息（执行结果上报验证层）
- Validate可以给Exec和Agent发消息（验证通过回执行，验证失败通知Agent）
- Agent只能给同层和Validate发消息（Agent发消息必须经Validate）
- Oversight可以给任何层发治理信号（纠错、降级、重启命令）
- 同层通信始终允许

#### 编译期实现：CanSendTo trait

```rust
// Layer marker types
pub struct OversightLayer; pub struct ExecLayer; pub struct ValidateLayer; pub struct AgentLayer;

// 方向合法性编码为trait impl
pub trait CanSendTo<Target> {}
impl CanSendTo<ExecLayer> for OversightLayer {}
impl CanSendTo<ValidateLayer> for OversightLayer {}
impl CanSendTo<AgentLayer> for OversightLayer {}
impl CanSendTo<OversightLayer> for OversightLayer {}
impl CanSendTo<ExecLayer> for ExecLayer {}       // 同层
impl CanSendTo<ValidateLayer> for ExecLayer {}   // Exec→Validate
impl CanSendTo<ExecLayer> for ValidateLayer {}   // Validate→Exec
impl CanSendTo<ValidateLayer> for ValidateLayer {} // 同层
impl CanSendTo<AgentLayer> for ValidateLayer {}  // Validate→Agent
impl CanSendTo<ValidateLayer> for AgentLayer {}  // Agent→Validate
impl CanSendTo<AgentLayer> for AgentLayer {}     // 同层
// 缺少impl的组合 = 编译错误
```

CellContext的send方法以`CanSendTo<TargetLayer>`为trait bound，非法方向的send调用在编译期就报错。

#### Sealed trait保护

Layer marker trait使用sealed trait模式，防止外部crate实现CanSendTo绕过方向检查：

```rust
mod sealed { pub trait Sealed {} }
impl Sealed for OversightLayer {}
impl Sealed for ExecLayer {}
impl Sealed for ValidateLayer {}
impl Sealed for AgentLayer {}
pub trait LayerMarker: Sealed {}
impl LayerMarker for OversightLayer {}
impl LayerMarker for ExecLayer {}
impl LayerMarker for ValidateLayer {}
impl LayerMarker for AgentLayer {}
```

外部crate无法为这些类型impl更多CanSendTo，因为Sealed trait不可外部实现。

### 3.4 build.rs 编译期检查

axiom-core添加`build.rs`，在编译时执行：
- 检查rustc版本 >= 1.75（async fn in traits要求），低于此版本给出明确错误提示
- 检查feature flag组合合法性（互斥feature不能同时启用）

---

## 四、L2 运行时门禁层

### 4.1 ArchitectureGuardian（架构守护者）

**位置**：axiom-oversight crate。
**机制**：作为Bus的消息拦截器/中间件，所有SignalEnvelope在投递到目标Mailbox前经过ArchitectureGuardian审查。

```
Sender → Bus.enqueue() → [ArchitectureGuardian审查] → 合法? → Mailbox投递
                                                 ↘ 违规? → 拦截 + Witness(AxiomViolated) + 治理信号
```

**审查项（与编译期CanSendTo矩阵一致，唯一真相源）**：
1. **层间方向违规**：source→target不在允许矩阵
2. **Hop count溢出**：hop_count >= MAX_HOP(8)，防止循环转发
3. **Schema版本过新**：消息schema_version > target_cell的schema_version
4. **Correlation ID缺失**：非初始消息（reply_to.is_some()）必须有correlation_id
5. **目标Cell未注册**：target_cell_id不存在于CellRegistry
6. **Oversight治理信号防伪造**：Oversight层信号需携带特殊governance_token，其他层伪造的直接拦截

**违规处理流程**：
1. 消息不投递（丢弃或进入dead-letter queue）
2. 产生Witness记录（outcome = AxiomViolated，axiom_name = "ArchitectureGuardian"）
3. 向源Cell发送治理警告信号（LayerViolationWarning）
4. 更新EntropyGovernor中的违规计数（影响系统熵值）
5. 连续N次来自同一Cell的违规 → 通知Supervisor可能需要重启该Cell

### 4.2 EntropyGovernor 自动去熵

**位置**：axiom-oversight crate。

**熵值区域与自动响应**：

| 区域 | 阈值 | 自动响应 |
|------|------|---------|
| Green | 0.0 - 0.4 | 正常运行，无干预 |
| Yellow | 0.4 - 0.8 | Warn日志 + 向Supervisor发送DeentropySuggestion信号（清理建议） |
| Red | 0.8 - 1.0 | Error日志 + 触发circuit breaker：暂停非关键Cell消息处理 + 强制执行snapshot压缩 |
| Critical | > 1.0 | 紧急熔断：暂停所有新消息处理，进入self-healing模式（GC+重启高熵Cell+snapshot） |

**自动去熵动作**：
1. Mailbox中超过TTL的积压消息自动清理
2. 错误率超过阈值的Cell自动通知Supervisor重启
3. 触发State Snapshot压缩Witness链（将长Witness链合并为snapshot+短链）
4. 临时数据GC（过期缓存、超时correlation context等）

**熵值计算输入**：
- 架构违规次数（ArchitectureGuardian报告）
- 消息处理失败率（Supervisor统计）
- Mailbox积压深度
- Witness链长度
- Cell重启频率
- 消息平均hop count

### 4.3 Supervisor 自愈机制

**位置**：axiom-runtime crate的supervisor模块。

**自愈策略矩阵**：

| 故障类型 | 检测方式 | 自动响应 | 产生Witness |
|---------|---------|---------|------------|
| Cell panic | `catch_unwind`包裹handle_message | 记录panic信息 → 重启Cell → 重放最后一条消息（幂等） | outcome=Failed |
| Cell超时无响应 | 消息处理超过5s（per-message timeout） | 警告+错误计数；连续3次超时→重启 | outcome=Failed |
| Cell错误率高 | 滑动窗口最近100条消息失败率>50% | Circuit breaker: open(暂停) → 30s后half_open(试探) → close(恢复) | outcome=Failed |
| 架构违规 | ArchitectureGuardian通知 | 标记源Cell为"throttled"，限制其发消息速率 | outcome=AxiomViolated |
| 进程崩溃 | OS进程管理器(systemd等) | 进程重启 → 从EventStore+Snapshot恢复状态 → verify witness chain | startup验证 |
| 熵值Critical | EntropyGovernor通知 | 进入紧急模式：暂停非关键Cell→GC→snapshot→逐步恢复 | outcome=Failed(entropy) |

**重启保证**：
- Cell重启后从上次Snapshot + 重放Event恢复状态
- 最多重启3次，如果3次重启后仍持续panic → 进入permanent-failure状态，需要人工介入
- 重启事件产生Witness，记录重启原因和次数

### 4.4 启动时自动验证链

Runtime启动时按顺序执行：

1. **Witness链完整性验证**：加载持久化Witnesses，调用`Witness::verify_chain_integrity()`，失败则abort启动（数据损坏）
2. **版本兼容性检查**：检查持久化数据的schema版本，调用`check_readable()`，不可读则abort
3. **自动迁移**：如果数据版本低于当前版本且有迁移链，自动执行迁移，迁移结果产生Witness
4. **Migration链完整性验证**：`registry.auto_collect()`后调用`verify_all_chains()`，发现gap则abort
5. **Cell注册表验证**：验证所有注册Cell的layer配置合法
6. **熵值初始化**：从上次快照恢复熵值，或从0开始
7. **健康服务注册**：启动Unix socket/HTTP health endpoint供`axm doctor`连接查询
8. **启动完成Witness**：产生一条系统启动Witness，记录VersionInfo和启动验证结果

任何步骤失败都终止启动并输出明确错误信息（不允许在不一致状态下运行）。

---

## 五、调整后的Roadmap

### 5.1 新阶段定义

| Phase | 名称 | Crates | 关键交付物 |
|-------|------|--------|-----------|
| **P0** | 基础设施 | workspace | ✅ 已完成 |
| **P0.5** | **L0开发门禁** | axiom-cli + .github/ | axm CLI(preflight/check/verify/version) + cargo-axiom + CI/CD + constraints.lock |
| **P0.6** | **L1编译期门禁** | axiom-macros + axiom-core | 全部5个proc宏 + Sealed CanSendTo方向trait + build.rs + linkme依赖 |
| **P1** | 核心原语（门禁保护下） | axiom-core | SignalPayload/SignalEnvelope重构 + CellContext适配宏 + WitnessBuilder自动注入VersionInfo |
| **P2** | 事件存储+自动迁移 | axiom-store | EventStore+Snapshot+Replay+auto_collect迁移+启动时自动验证 |
| **P3** | 运行时+自愈 | axiom-runtime | Bus拦截器机制+Mailbox+Dispatcher+Supervisor自愈+per-message timeout+circuit breaker |
| **P4** | 监督层+L2运行时门禁 | axiom-oversight | ArchitectureGuardian+EntropyGovernor自动触发+启动验证链+health endpoint |
| **P5** | 可视化导出 | axiom-viz | topology/timeline/entropy/trace/metrics |
| **P6-P10** | Agent体系 | axiom-agent等 | 身份/技能/规则/LLM/MCP（三层门禁保护下开发） |
| **P11** | CLI脚手架完善 | axiom-cli | axm init/new/doctor/top/trace/why |
| **P12-P17** | 高级功能 | 后续crate | 记忆/规划/提示词/RAG/测试/示例（门禁保护下开发） |

### 5.2 阶段依赖关系

```
P0 基础设施 ✅
  ↓
P0.5 L0开发门禁 ────── L0生效：CI+hooks+preflight自动门禁
  ↓
P0.6 L1编译期门禁 ──── L1生效：架构违规编译不过，boilerplate宏生成
  ↓
P1 核心原语（在L0+L1保护下开发，必须使用宏定义Signal/Cell/Axiom）
  ↓
P2 事件存储（自动迁移注册+验证）
  ↓
P3 运行时（Supervisor自愈+Bus拦截器就绪）
  ↓
P4 L2运行时门禁 ────── L2生效：运行时拦截+自愈+自动去熵，三层门禁全部就位
  ↓
P5-P17 （三层门禁全链路保护下推进）
```

### 5.3 门禁生效里程碑

| 里程碑 | 生效门禁 | 效果 |
|--------|---------|------|
| P0.5完成 | L0开发门禁 | 不通过axm check/preflight的代码无法commit/merge |
| P0.6完成 | L1编译期门禁 | 跨层违规、手写boilerplate导致缺少trait impl → 编译错误 |
| P4完成 | L2运行时门禁 | 所有运行时消息经Guardian审查，崩溃自动恢复，熵超标自动治理 |
| P4以后 | 三层全链路 | 18项自动化点全部生效，自动化覆盖率>90% |

---

## 六、验收标准

### 6.1 每个Phase的通用验收标准（由axm check+CI自动强制执行）

以下检查全部自动执行，任何一项不通过则Phase不能完成：

1. `cargo build --workspace` 零警告（`RUSTFLAGS="-D warnings"`）
2. `cargo fmt --all -- --check` 通过
3. `cargo clippy --workspace -- -D warnings` 零警告
4. `cargo test --workspace` 全部通过
5. `axm verify` 架构验证通过（依赖方向/unsafe审计/层间trait使用）
6. `cargo doc --no-deps -p <crate>` 文档编译无警告
7. 无新增第三方依赖未经审计（R-022，deps_audit检查）
8. 无unsafe代码无SAFETY注释（unsafe_audit检查）
9. public API有rustdoc注释
10. schema版本号符合递增规则
11. 每个新增宏/CLI命令有对应的单元测试覆盖成功路径和失败路径
12. CI workflow在push/PR时实际运行并通过

### 6.2 P0.5（L0开发门禁）专项验收标准

| # | 验收项 | 验证方式 |
|---|--------|---------|
| 1 | `axm preflight` 命令可运行 | 执行`axm preflight`，退出码0（在干净工作区） |
| 2 | preflight阻断clippy警告 | 在代码中引入一个clippy警告→axm preflight退出码1 |
| 3 | preflight阻断async-trait依赖 | 在Cargo.toml中加入async-trait→axm preflight退出码1 |
| 4 | preflight阻断unsafe无SAFETY | 添加无SAFETY注释的unsafe块→axm preflight退出码1 |
| 5 | preflight阻断TODO/FIXME | 在非测试代码中加入unimplemented!()→axm preflight退出码1 |
| 6 | preflight约束文件hash检测 | 修改.axiom/rules文件→axm preflight报约束文件被篡改 |
| 7 | `axm check`一键完成所有检查 | 执行`axm check`，依次跑fmt→build→clippy→test→verify |
| 8 | `axm verify`架构验证 | 在干净代码上axm verify退出码0 |
| 9 | `axm version`显示完整版本信息 | 执行axm version，输出crate/schema/protocol版本 |
| 10 | `cargo axiom` subcommand可用 | `cargo axiom check` 等价于 `axm check` |
| 11 | CI workflow文件存在且语法正确 | `.github/workflows/ci.yml` 存在且包含全部检查步骤 |
| 12 | pre-commit hook安装脚本可用 | `axm init`（或hook安装脚本）生成的hook会调用axm check |
| 13 | 单元测试覆盖所有check模块 | checks/下每个模块有测试，覆盖成功+失败场景 |

### 6.3 P0.6（L1编译期门禁）专项验收标准

| # | 验收项 | 验证方式 |
|---|--------|---------|
| 1 | `#[derive(SignalPayload)]` 可使用 | 定义带#[signal]+#[schema_version]的struct，编译通过，SignalPayload impl正确 |
| 2 | 非法层名compile_error | `#[signal(source="invalid")]` → 编译错误，信息明确 |
| 3 | 非法方向compile_error | Exec→Oversight方向的#[signal] → 编译错误，提示使用event上报 |
| 4 | `#[cell(layer="exec")]` 生成Cell+ExecCell impl | 标注后struct自动impl Cell和ExecCell，可在CellContext中使用 |
| 5 | `#[axiom(...)]` 自动注册 | 标注axiom后，AxiomRegistry::collect()能收集到，无需手动push |
| 6 | `#[migration(from=N)]` 编译期检查N+1 | to_version不等于N+1时compile_error |
| 7 | `#[migration(...)]` 自动注册 | 标注migration后，MigrationRegistry::auto_collect()能发现 |
| 8 | CanSendTo非法方向编译错误 | 尝试在ExecCellContext中调用send_to_oversight → 编译错误 |
| 9 | Sealed trait保护 | 尝试在integration test中为自定义类型impl CanSendTo → 编译错误 |
| 10 | build.rs版本检查 | 使用rustc <1.75编译 → build.rs输出明确错误信息 |
| 11 | 所有宏有编译失败测试（trybuild/cargotest） | tests/compile-fail/下包含非法用法的.rs文件，验证编译错误信息 |
| 12 | linkme分布式收集在Windows上工作 | cargo test在windows target通过（不依赖inventory/ctor） |

### 6.4 P4（三层门禁全开）最终验收标准 —— "自动化>90%"验证

| # | 指标 | 验收方式 | 目标 |
|---|------|---------|------|
| 1 | 编译期拦截率 | 在ExecCell中尝试`ctx.send_to_oversight(...)` | 编译错误，信息包含"cannot send directly to OversightLayer" |
| 2 | CI拦截率 | 提交含clippy警告的PR | CI红灯，无法merge |
| 3 | Hook拦截率 | 尝试commit含unimplemented!()的非测试代码 | pre-commit阻止commit，输出违规位置 |
| 4 | 运行时拦截率 | 构造一个层间方向违规的SignalEnvelope直接注入Bus | ArchitectureGuardian拦截，不投递，产生AxiomViolated Witness |
| 5 | 宏必须性 | 不使用宏直接impl SignalPayload for一个struct（绕过命名约定） | 编译失败（Sealed trait保护）或无法被ctx.send识别 |
| 6 | 自动迁移发现 | 添加#[migration(from=3)]到一个新struct，不手动register | 启动时auto_collect()发现该迁移，verify_all_chains()包含它 |
| 7 | 自愈触发 | 在Cell handle_message中panic | Supervisor catch_unwind→重启Cell→Witness记录→消息不丢失（幂等重放） |
| 8 | Circuit breaker | Cell连续3次超时 | Supervisor标记half_open，30s后试探，恢复后close |
| 9 | 熵控Yellow触发 | 注入架构违规事件使熵值>0.4 | EntropyGovernor发Warn log+DeentropySuggestion信号 |
| 10 | 熵控Red触发 | 注入足够错误使熵值>0.8 | EntropyGovernor触发circuit breaker暂停非关键Cell |
| 11 | 预检零人工 | `axm preflight` 覆盖所有preflight.md中的A-D项 | 0项需要人工打勾 |
| 12 | 一键检查 | `axm check` 一条命令跑完全部质量门 | 无需手动敲多条命令 |
| 13 | Witness链启动验证 | 持久化一条断裂的Witness链 | 启动时verify_chain_integrity失败，进程abort |
| 14 | 版本自动迁移 | 持久化schema v1数据，启动时schema为v2 | 自动执行MigrateV1toV2，迁移过程产生Witness |
| 15 | 版本不可读拒绝 | 持久化schema v99数据 | 启动时check_readable失败，报SchemaVersionTooNew错误 |
| 16 | axm doctor连接 | 启动runtime后运行axm doctor | 成功连接，输出健康状态/熵值/版本信息 |
| 17 | 启动Witness | 每次runtime启动 | 产生一条"system-startup"Witness，包含VersionInfo+验证结果 |
| 18 | 不使用unwrap()（非测试） | grep检查src/下unwrap()使用 | 0个（测试代码除外） |

**通过标准：以上18项全部通过 = 自动化覆盖率>90%。**

---

## 七、依赖新增清单

以下是需要新增到workspace的第三方crate（均在P0.5/P0.6引入，需经R-022依赖审计）：

| Crate | 用途 | 引入Phase |
|-------|------|----------|
| `clap` (derive feature) | CLI参数解析 | P0.5 |
| `linkme` | 分布式slice（自动注册Axiom/Migration） | P0.6 |
| `proc-macro2` / `quote` / `syn` | proc macro基础设施 | P0.6 |
| `trybuild` | 编译失败测试（验证compile_error!） | P0.6（dev-dependency） |
| `serde_json` | Migration中的JSON变换 | P0.6（已有传递依赖） |

现有依赖（sha2/uuid/tokio/serde/tracing等）保持不变。

---

## 八、不做的事情（YAGNI）

- 不做TUI/仪表盘（P5 axiom-viz处理，CLI只输出结构化文本）
- 不做REPL/interactive shell（P16再考虑）
- 不做远程部署/容器编排（不属于核心架构）
- 不做自动版本号bump（版本号由开发者管理，版本管理层保证兼容性）
- 不做IDE插件（CLI+CI+proc macro三层覆盖已足够）
- 不做inventory/ctor（用linkme替代，更好的跨平台支持）
- 不做自定义build script脚本语言（build.rs只做rustc版本和feature检查）
- 不做运行时热重载（Cell重启由Supervisor管理，不做代码热替换）
