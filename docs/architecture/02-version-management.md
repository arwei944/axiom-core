# Axiom Core 版本管理层设计

> **版本是架构的DNA。** 没有严格的版本管理，就没有确定性重放、没有安全升级、没有兼容保证——也就没有低熵。

---

## 一、设计哲学

版本管理在Axiom Core中不是"发布时才想起来的事"，而是**核心架构层的一部分**，与五个原语同等地位：

1. **确定性重放**：旧版本产生的Witness/Event必须能在新版本中正确读取和验证
2. **热升级安全**：Cell运行时替换Impl版本，in-flight消息不丢失、不损坏
3. **架构自约束**：版本不兼容的通信在编译期或启动时被阻止
4. **可审计**：每个Witness记录当时运行的crate版本、schema版本、identity版本
5. **自动迁移**：从v1到vN的数据升级路径可被编译器检查完整性

---

## 二、五层版本体系

Axiom Core管理**五个正交维度**的版本，每层有独立的兼容性规则：

| 版本维度 | 类型 | 兼容性规则 | 存储位置 | 变更频率 |
|---------|------|-----------|---------|---------|
| **CrateVersion** | SemVer (MAJOR.MINOR.PATCH) | 同MAJOR兼容 | Cargo.toml | 发布时 |
| **SchemaVersion** | 单调递增u16 | 新版本可读旧版本（向前兼容） | 每个序列化结构体 | 结构体变更时 |
| **ProtocolVersion** | u16 | 完全匹配（网络通信） | 消息握手 | 协议变更时 |
| **ApiVersion** | SemVer | 同MAJOR兼容 | 公共API表面 | API变更时 |
| **IdentityVersion** | u16单调递增 | Witness记录当时版本 | Identity挂载 | Identity热切换时 |

---

## 三、Schema版本演化规则（R-023细化）

### 3.1 Schema版本递增规则

| 变更类型 | Schema版本动作 | 是否需要MigrateFrom | 兼容性 |
|---------|---------------|-------------------|--------|
| 新增可选字段 | 保持不变 | 否 | 完全兼容 |
| 新增必填字段 | +1 MINOR | 是（旧→新） | 向前兼容 |
| 删除字段 | +1 MAJOR | 否（丢弃数据） | Breaking |
| 字段重命名 | +1 MAJOR | 是 | 需要迁移 |
| 字段类型变更 | +1 MAJOR | 是 | 需要迁移 |
| 语义变更（同字段不同含义） | +1 MAJOR | 否（不自动迁移） | Breaking |

### 3.2 版本兼容性矩阵

```
                写入版本 (Writer)
                v1      v2      v3
读取 v1         ✓       ✗       ✗
     v2         ✓       ✓       ✗
     v3         ✓       ✓       ✓
```

**铁律**：新版本Reader永远能读旧版本Writer的数据（can_read = reader_version >= writer_version）。

### 3.3 迁移框架

```rust
/// 迁移trait：每个迁移从一个schema版本升级到下一个版本（+1）
/// 注册时panic如果版本跳跃不为1，保证迁移链无gap
pub trait Migration: Send + Sync {
    fn source_version(&self) -> SchemaVersion;
    fn target_version(&self) -> SchemaVersion;
    fn migrate(&self, input: serde_json::Value) -> crate::Result<serde_json::Value>;
}

/// 迁移注册表：启动时验证迁移链完整性，支持链式自动迁移
pub struct MigrationRegistry {
    migrations: HashMap<(u16, u16), Box<dyn Migration>>,
}

impl MigrationRegistry {
    pub fn new() -> Self;
    /// 注册迁移（运行时检查FROM→TO必须+1，否则panic）
    pub fn register<M: Migration + 'static>(&mut self, m: M);
    /// 自动链式迁移：from → to，找不到路径则报错
    pub fn migrate(&self, data: serde_json::Value, from: SchemaVersion, to: SchemaVersion)
        -> crate::Result<serde_json::Value>;
    /// 验证读取兼容性：数据版本是否可读
    pub fn check_readable(&self, found: SchemaVersion, current: SchemaVersion) -> crate::Result<()>;
    /// 启动时校验：目标类型的迁移链无gap
    pub fn verify_complete<T: Versioned + 'static>(&self) -> crate::Result<()>;
}
```

### 3.4 迁移实现示例

```rust
/// SignalV1 → SignalV2: 新增 priority 字段
struct MigrateV1toV2;
impl Migration for MigrateV1toV2 {
    fn source_version(&self) -> SchemaVersion { SchemaVersion(1) }
    fn target_version(&self) -> SchemaVersion { SchemaVersion(2) }
    
    fn migrate(&self, mut data: serde_json::Value) -> crate::Result<serde_json::Value> {
        // v1没有priority字段，设为默认值
        data["priority"] = serde_json::json!("normal");
        Ok(data)
    }
}
```

---

## 四、Crate版本管理（SemVer）

### 4.1 版本号含义

```
0.1.0
│ │ └── PATCH: bug修复、性能优化、文档更新——无API变更
│ └──── MINOR: 新增功能、新增trait方法（有默认实现）、新增枚举变体——向后兼容
└────── MAJOR: 破坏性变更、删除API、trait方法签名变更——需要用户修改代码
```

### 4.2 0.x.y 特殊规则（开发阶段）

在1.0.0之前：
- MINOR版本变更可能包含破坏性变更（但会在CHANGELOG中明确标注）
- PATCH版本只包含bug修复
- 每个MINOR版本前会有migration guide

### 4.3 MSRV策略

- Minimum Supported Rust Version: 1.85（稳定版async fn in traits）
- MSRV变更视为MAJOR版本变更（即使是0.x阶段）
- CI中测试MSRV和stable两个版本

---

## 五、Protocol版本（网络通信）

### 5.1 握手协议

```
Client → Server: HELLO { protocol_version: v1, crate_version: "0.1.0" }
Server → Client: HELLO_ACK { protocol_version: v1, capabilities: [...] }
                 或 VERSION_MISMATCH { supported: [v1], requested: v2 }
```

### 5.2 兼容性

- Protocol版本必须**完全匹配**才能通信
- 不支持Protocol版本协商降级（防止降级攻击）
- Witness中记录通信双方的protocol_version

---

## 六、API版本（公共接口）

### 6.1 API稳定性保证

| API类型 | 稳定性保证 |
|---------|-----------|
| `pub trait` 中已有的方法 | 同MAJOR版本内不删除、不改签名 |
| `pub struct` 的public字段 | 同MAJOR版本内不删除、不改类型 |
| `pub enum` 的变体 | 同MAJOR版本内不删除 |
| 新增trait方法 | MINOR版本可加，必须有默认实现 |
| 新增struct字段 | MINOR版本可加，必须是Option<T>或有Default |
| 模块路径 | 同MAJOR版本内不重组 |
| `#[doc(hidden)]` API | 无保证，不建议使用 |

### 6.2 feature flags管理

```toml[features]
default = ["sha2-id", "uuid"]
sha2-id = ["dep:sha2"]      # SHA-256哈希Witness
uuid = ["dep:uuid"]         # UUID生成ID
macros = ["dep:axiom-macros"] # 派生宏
std = []                    # std支持（默认启用）
# 未来: no_std = ["core2", "alloc"]
```

**feature稳定性**：
- 新增feature flag是MINOR变更
- 删除feature flag是MAJOR变更
- feature之间的依赖关系不能形成循环

---

## 七、Identity版本（热切换）

### 7.1 Identity版本语义

- Identity挂载/卸载时，IdentityVersion单调递增
- 每个Witness记录产生时的`identity_version`
- 消息携带`identity_version`用于权限检查

### 7.2 热切换规则

```
旧Identity(v3) → 新Identity(v4)
  ├── in-flight消息：使用发送时的v3权限检查
  ├── 新消息：使用v4权限检查
  ├── 已有Witness：保留v3标记，不回溯修改
  └── 如果v4权限 < v3：警告但允许（降级）
```

---

## 八、Witness中的版本信息（审计核心）

每个Witness必须携带以下版本字段，确保证据链的完整性：

```rust
pub struct WitnessVersionInfo {
    /// 产生此Witness的axiom-core crate版本
    pub crate_version: CrateVersion,
    /// Witness自身的schema版本
    pub witness_schema: SchemaVersion,
    /// 触发Signal的schema版本
    pub signal_schema: SchemaVersion,
    /// 产生时的protocol版本
    pub protocol_version: ProtocolVersion,
    /// 当时挂载的Identity版本
    pub identity_version: Option<u16>,
}
```

**为什么Witness必须带版本？**
- 重放历史Witness时，知道当时用的是哪个版本的验证逻辑
- 如果发现某个版本有Witness哈希计算bug，可以定位受影响的Witness范围
- 合规审计要求——"你用什么版本的软件做出了这个决策？"

---

## 九、版本检查自动化

### 9.1 编译期检查

- `Versioned` trait确保每个序列化类型声明schema_version
- 过程宏`#[derive(Versioned)]`自动实现并增加schema版本常量
- `static_assertions`确保Migration链没有gap

### 9.2 启动时检查

Runtime启动时执行：
1. MigrationRegistry::verify_complete() — 迁移链完整性
2. EventStore中所有持久化数据的schema版本 ≤ 当前版本
3. 发现太新的数据（future version）→ 拒绝启动
4. 发现需要迁移的数据 → 自动执行迁移（或提示用户手动迁移）

### 9.3 运行时检查

- SignalEnvelope反序列化后立即检查schema_version
- 接收方版本 < 发送方版本 → 返回错误/拒绝
- 跨节点通信前握手验证protocol_version

### 9.4 CLI工具支持

```bash
axm version                  # 显示当前版本
axm version check            # 检查是否有新版本
axm version list-schemas     # 列出所有schema版本
axm version migrate <from> <to>  # 手动执行数据迁移
axm version verify-store     # 验证存储中数据版本完整性
axm version compatibility <crate> <ver>  # 检查与指定版本兼容性
```

---

## 十、CHANGELOG和发布流程

### 10.1 CHANGELOG格式（Keep a Changelog）

每个版本对应CHANGELOG.md中的一个section：
```markdown
## [0.2.0] - 2026-07-15

### Added
- 新增 axiom-macros 过程宏 crate
- 新增 Schema 迁移框架 (MigrationRegistry)

### Changed
- Signal trait 移除 async-trait 依赖，使用原生 async fn (BREAKING)

### Fixed
- Witness prev_hash 链验证逻辑修复

### Migration Guide
- 从0.1.x升级：所有Signal实现需要移除`#[async_trait]`标注
```

### 10.2 发布检查清单

发布新版本前必须：
- [ ] `cargo build --workspace` 零警告
- [ ] `cargo clippy --workspace -- -D warnings` 零警告
- [ ] `cargo test --workspace` 全部通过
- [ ] `cargo fmt --check` 通过
- [ ] MigrationRegistry::verify_complete() 通过
- [ ] CHANGELOG.md 更新
- [ ] Cargo.toml 版本号正确
- [ ] 文档中版本引用更新
- [ ] Git tag 格式为 `v{version}`（如 v0.1.0）

### 10.3 Git标签规则

- 每个发布版本都有annotated tag：`git tag -a v0.1.0 -m "Release v0.1.0"`
- 不移动已发布的tag（tag不可变）
- 预发布版本使用后缀：v0.1.0-alpha.1, v0.1.0-beta.2, v0.1.0-rc.1

---

## 十一、版本相关错误类型

在AxiomError中增加以下版本相关错误：

```rust
pub enum AxiomError {
    // ... 现有错误 ...
    
    /// Schema版本太新——当前代码无法读取未来版本的数据
    #[error("Schema version too new: found v{found}, max supported v{max}")]
    SchemaVersionTooNew { found: u16, max: u16 },
    
    /// Schema版本太旧，需要迁移但没有注册迁移路径
    #[error("Schema version too old: found v{found}, no migration path to v{current}")]
    MigrationPathNotFound { found: u16, current: u16 },
    
    /// 迁移链不完整（启动时检查失败）
    #[error("Migration chain incomplete: missing migration v{from}→v{to}")]
    MigrationChainGap { from: u16, to: u16 },
    
    /// Protocol版本不匹配
    #[error("Protocol version mismatch: expected v{expected}, got v{got}")]
    ProtocolMismatch { expected: u16, got: u16 },
    
    /// 数据迁移失败
    #[error("Migration failed from v{from} to v{to}: {reason}")]
    MigrationFailed { from: u16, to: u16, reason: String },
}
```

---

## 十二、axiom-core中版本管理的代码结构

```
crates/axiom-core/src/version/
├── mod.rs              # 版本管理层入口，re-export所有公共类型
├── semver.rs           # CrateVersion (SemVer)
├── schema.rs           # SchemaVersion + Versioned trait
├── protocol.rs         # ProtocolVersion
├── migration.rs        # Migration trait + MigrationRegistry + 链式迁移
├── compatibility.rs    # Compatibility枚举 + 兼容性检查
└── identity_version.rs # IdentityVersion（热切换版本）
```

---

## 十三、与其他模块的集成点

| 模块 | 版本集成点 |
|------|-----------|
| **Signal** | 每个Signal必须实现Versioned，schema_version()在Signal trait中有默认实现 |
| **Witness** | WitnessVersionInfo字段记录所有版本信息，哈希计算包含版本字段 |
| **EventStore** | 读取Event时检查schema_version，必要时自动迁移 |
| **Runtime** | 启动时验证迁移链完整性，消息分发时检查版本兼容性 |
| **Oversight** | ArchitectureGuardian检测版本不兼容的跨Cell通信 |
| **CLI** | axm version 系列命令 |
| **axiom-macros** | #[derive(Versioned)] 自动生成schema_version常量 |

---

## 十四、向后兼容性保证（v1.0后）

v1.0.0发布后：
- 至少2个MAJOR版本的向后兼容（v1能读v0数据）
- MAJOR版本变更前至少3个月的deprecation警告期
- 提供`cargo axiom upgrade`工具自动修复breaking API变更
- 旧版本Witness的验证永远不会被移除（区块链式的不可篡改）

> **版本管理的终极目标**：无论系统演化到什么版本，一年前产生的Witness依然能被验证、被理解、被重放——这才是真正的审计，真正的可追溯，真正的低熵。
