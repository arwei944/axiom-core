# Axiom Core 商用架构开发文档

> **当前阶段**: v0.1.0 生产就绪
> **目标阶段**: v0.2.0 生产深度
> **预估工期**: 6周

---

## 一、架构核心需求回顾

### 1.1 五大核心原语

| 原语 | 定位 | 当前状态 | v0.2.0 目标 |
|------|------|---------|------------|
| **Cell** | 隔离状态单元 | ✅ 完成 | 保持 |
| **Signal** | 类型化消息 | ✅ 完成 | 保持 |
| **Lens** | 状态投影 | ⚠️ 仅 LensId | ✅ 完整实现 |
| **Axiom** | 全局约束 | ✅ 完成 | 保持 |
| **Witness** | 审计记录 | ✅ 完成 | 持久化 |

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
| **能力版本约束** | `#[capability]` 宏 + `CapabilityVersionRegistry` | ✅ 完成 |

---

## 二、v0.2.0 核心升级方向

### 2.1 三大架构缺口修复

| 缺口 | 严重程度 | v0.2.0 修复方向 |
|------|---------|---------------|
| **Lens 原语缺失** | 高 | 实现完整的 Lens trait、Registry、Cache 和宏 |
| **Witness 链无持久化** | 高 | 添加 SQLite 和文件系统后端，支持崩溃恢复 |
| **编译期→运行期约束断层** | 中 | 总线层统一集成约束验证 |

### 2.2 开发策略：生产深度 > 功能广度

- **不新增 crate** — 当前16个 crate 表面面积已足够大
- **深化现有能力** — 让每个 crate 具备完整的生产级功能
- **消除架构缺口** — 补齐 Lens 和持久化等核心能力
- **统一约束体系** — 让编译期和运行期约束形成闭环

---

## 三、v0.2.0 任务清单

### Phase 1: Lens 原语实现 (1周)

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P1-01** | 定义 `Lens` trait（投影逻辑接口） | 编译通过 |
| **P1-02** | 定义 `LensRegistry`（linkme 自动注册） | 类似 `CAPABILITY_REGISTRY` |
| **P1-03** | 实现 `ProjectionCache`（缓存策略 + 增量更新） | 命中率 > 90% |
| **P1-04** | 实现 `#[lens]` 宏（自动生成 Lens 实现） | 集成测试通过 |
| **P1-05** | 导出 Lens 到 lib.rs | `pub use lens::*` |
| **P1-06** | Lens 测试（单元 + 集成 + 宏测试） | 测试覆盖率 > 80% |

### Phase 2: Store 持久化 (2周)

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P2-01** | 添加 SQLite 后端（sqlx） | 集成测试通过，支持事务 |
| **P2-02** | 添加文件系统后端（append-only） | 查询响应 < 50ms |
| **P2-03** | Store 抽象层重构（`StoreFactory`） | 运行时选择后端 |
| **P2-04** | SnapshotStore 持久化（磁盘 + 压缩） | 压缩比 > 50% |
| **P2-05** | Witness 自动持久化接线 | 重启后 Witness 链完整 |
| **P2-06** | 崩溃恢复测试 | 重启后状态一致 |
| **P2-07** | 性能测试（写入吞吐量 > 1000 events/s） | 性能达标 |

### Phase 3: 约束运行时统一 (1周)

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P3-01** | 总线拦截器集成 `CapabilityVersionRegistry` | 版本不兼容时拒绝消息 |
| **P3-02** | 创建 `ConstraintValidator` 统一验证上下文 | 验证结果自动记录到 Witness |
| **P3-03** | 权限运行时检查（Guard 在总线层强制执行） | 权限不足时拒绝并记录 |
| **P3-04** | 约束测试（单元 + 集成 + 联合测试） | 非法消息在运行时被正确拦截 |

### Phase 4: 现有 crate 深化 (1周)

| crate | 深化内容 | 验收标准 |
|-------|---------|---------|
| **axiom-identity** | 密钥管理、签名验证、证书链 | 集成测试通过 |
| **axiom-prompt** | 模板编译、变量验证、版本管理、token预算感知 | 集成测试通过 |
| **axiom-planner** | 计划验证、步骤依赖、回滚策略、Witness记录 | 集成测试通过 |
| **axiom-memory** | 向量搜索、语义检索、过期策略、内存大小限制 | 集成测试通过 |

### Phase 5: API 稳定与发布 (1周)

| 任务 | 描述 | 验收标准 |
|------|------|---------|
| **P5-01** | 定义 v1 API 边界（标记不稳定API） | 编译通过 |
| **P5-02** | 更新版本策略文档（语义化版本规则） | 文档完整 |
| **P5-03** | 错误类型完善（覆盖所有错误场景） | 错误分类清晰 |
| **P5-04** | 公共API文档完备 | `cargo doc --workspace` 无警告 |
| **P5-05** | 更新版本号和 CHANGELOG | `cargo publish --dry-run` 通过 |

---

## 四、8大能力维度版本管理

### 4.1 能力维度定义

| 维度 | 用途 | 典型场景 |
|------|------|---------|
| **Witness** | 审计链版本 | 状态转换记录格式 |
| **Schema** | 信号协议版本 | 消息序列化格式 |
| **Layer** | 架构层版本 | 层间调用规则 |
| **Tool** | 工具接口版本 | 工具执行协议 |
| **Guard** | 约束规则版本 | 权限检查规则 |
| **Identity** | 身份协议版本 | Agent身份/权限集 |
| **Entropy** | 熵治理版本 | 阈值策略/治理动作 |
| **Runtime** | 运行时协议版本 | 监督策略/邮箱配置 |

### 4.2 使用方式

```rust
#[axiom_kernel::capability(dim = "witness", version = "1.0.0")]
struct WitnessCapability;

#[axiom_kernel::capability(dim = "identity", version = "1.0.0")]
struct IdentityCapability;

#[axiom_kernel::capability(dim = "entropy", version = "1.0.0")]
struct EntropyCapability;

#[axiom_kernel::capability(dim = "runtime", version = "1.0.0")]
struct RuntimeCapability;
```

### 4.3 兼容性检查

```rust
CapabilityVersionRegistry::auto_check_compatibility()?;
```

---

## 五、质量门禁检查清单

### 每次提交前必须通过

```bash
# L0: 开发期检查
cargo fmt --all --check

# L1: 编译期检查
cargo clippy --workspace --all-targets --all-features -D warnings
cargo build --workspace --all-targets
cargo run --bin axm -- gate check

# L2: 运行期检查
cargo test --workspace
cargo bench -p axiom-bench --no-run
```

### 任务完成验收标准

| 检查项 | 要求 |
|--------|------|
| 编译 | 零错误 |
| Clippy | 零警告 |
| 测试 | 全部通过 |
| 格式 | `cargo fmt` 通过 |
| 依赖 | 无反向依赖、无不审核依赖 |
| 文档 | 公共API有完整文档 |
| 错误处理 | 无 `unwrap()` / `expect()`（非测试） |
| 能力版本 | 所有核心能力已注册版本 |

---

## 六、约束对开发的硬性要求

### 约束1: Crate层依赖规则（编译期强制）

**规则**: crate at level N may only depend on crates at level >= N

| Level | Crate | 允许依赖 |
|-------|-------|---------|
| 0 | axiom-cli | >=0 (所有crate) |
| 1 | axiom-viz | >=1 |
| 2 | axiom-agent | >=2 |
| 3 | axiom-oversight | >=3 |
| 4 | axiom-runtime | >=4 |
| 5 | axiom-store | >=5 |
| 6 | axiom-macros | >=6 (仅core) |
| 7 | axiom-kernel | 仅第三方依赖 |

### 约束2: 四层架构调用规则（编译期+运行期强制）

| 来源 | Oversight | Agent | Validate | Exec |
|------|-----------|-------|----------|------|
| Oversight | ✅ | ✅ | ✅ | ✅ |
| Agent | ❌ | ✅ | ✅ | ❌ |
| Validate | ❌ | ✅ | ✅ | ✅ |
| Exec | ❌ | ❌ | ❌ | ✅ |

### 约束3: 禁止依赖规则（编译期强制）

- `async-trait` — Rust 1.75+ 支持原生 async fn in traits

### 约束4: 审核依赖规则（编译期强制）

所有第三方依赖必须在 `AUDITED_DEPS` 列表中。

### 约束5: 错误处理规则（编译期+运行期强制）

非测试代码中禁止使用 `unwrap()` / `expect()`。

### 约束6: 架构自约束规则（设计期强制）

约束者必先受约束。

### 约束7: 能力维度版本管理规则（编译期强制）

所有核心能力必须通过 `#[capability]` 宏注册版本。

---

## 七、预期成果

| 指标 | v0.1.0 | v0.2.0 目标 |
|------|--------|------------|
| 核心原语完整性 | 4/5 | 5/5 |
| 持久化支持 | 仅内存 | SQLite + 文件系统 |
| 约束覆盖 | 编译期为主 | 编译期 + 运行期闭环 |
| 能力维度版本 | 8/8 | 8/8 |
| 测试数量 | 391+ | 500+ |
| 生产就绪度 | 原型 | 可用于生产环境 |
| 消息吞吐 | ~1949 msg/s | > 1000 msg/s |
| Witness 链验证 | < 1ms/100条 | < 1ms/100条 |

---

## 八、关键文件索引

| 文件 | 说明 |
|------|------|
| [docs/plans/v0.2.0-development-plan.md](plans/v0.2.0-development-plan.md) | v0.2.0 详细开发计划 |
| [crates/axiom-kernel/src/capability.rs](../crates/axiom-kernel/src/capability.rs) | 能力维度版本管理 |
| [crates/axiom-kernel/src/lens.rs](../crates/axiom-kernel/src/lens.rs) | Lens 原语（待创建） |
| [crates/axiom-store/src/store.rs](../crates/axiom-store/src/store.rs) | EventStore 抽象层 |
| [crates/axiom-runtime/src/bus.rs](../crates/axiom-kernel-project/crates/axiom-runtime/src/bus.rs) | 消息总线与拦截器链 |
| [crates/axiom-kernel/src/gate.rs](../crates/axiom-kernel/src/gate.rs) | Crate层依赖约束 |
| [crates/axiom-kernel/src/sealed.rs](../crates/axiom-kernel/src/sealed.rs) | 层间调用编译期约束 |
