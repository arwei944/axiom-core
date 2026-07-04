# Axiom Core 代码质量与架构能力审查标准（v1.0）

> 本标准用于对 Axiom Core 进行系统性、可量化的代码质量与架构能力审查。
> 所有条目均为**强制要求**，除非标注为“建议”。

---

## 1. 编译与静态检查

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| C-01 | 编译零错误 | `cargo check --workspace` 必须通过 | 基础门槛 |
| C-02 | 测试零失败 | `cargo test --workspace` 必须全绿 | 含单元/集成/文档测试 |
| C-03 | Clippy 零警告 | `cargo clippy --workspace -D warnings` 必须通过 | 不允许任何 warning |
| C-04 | 格式化统一 | `cargo fmt --all -- --check` 必须通过 | 禁止混用风格 |
| C-05 | 文档测试通过 | `cargo test --doc` 必须通过 | 所有 `///` 示例可运行 |
| C-06 | 架构门禁通过 | `cargo run -p archcheck --` 和 `cargo run -p xtask -- gatecheck --strict` 必须通过 | 依赖分层、架构规则 |
| C-07 | 无未使用代码 | 不允许存在 `#[allow(dead_code)]`、未使用导入、未使用字段 | 除宏生成的代码外 |
| C-08 | 无安全漏洞 | `cargo audit` 不得存在 High/Critical 级别漏洞 | Low/Info 可记录待处理 |

---

## 2. 架构设计

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| A-01 | 单一职责 | 每个模块/文件职责单一，不超过 300 行 | 超限必须拆分 |
| A-02 | 依赖方向正确 | `axiom-core` ← `axiom-runtime` ← `axiom-store` ← `axiom-viz`，严禁反向依赖 | 由 archcheck 强制 |
| A-03 | 无循环依赖 | `cargo tree -e normal` 不得存在循环 | 任意 crate 间 |
| A-04 | 抽象层次清晰 | 核心 trait 与实现分离，不允许 trait 泄露实现细节 | 如 `SignalCodec` trait + 实现 |
| A-05 | 接口最小化 | `pub` 项占 crate 总项比例 ≤ 40% | 过度暴露是技术债 |
| A-06 | 配置内聚 | 配置结构体集中定义，不散落在多个文件 | 如 `RuntimeConfig` |
| A-07 | 错误类型内聚 | 每个 crate 有统一的 `Error` 类型，禁止混用 `anyhow`/`String` | 核心层必须用 `thiserror` |
| A-08 | 并发模型明确 | 共享状态必须明确使用 `Arc<Mutex<>>`/`Arc<RwLock<>>`/`parking_lot`，禁止 `unsafe` 并发 | 除非经过安全审查 |
| A-09 | 状态机清晰 | 有状态组件必须有明确的状态转换图或文档 | 如 Runtime 生命周期 |
| A-10 | 无隐藏副作用 | 函数签名必须清晰表明副作用，禁止在 `fn` 中悄悄修改全局状态 | 除非明确标注 `#[must_use]` |

---

## 3. 代码质量

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| Q-01 | 函数长度 | 单个函数 ≤ 80 行，超过必须拆分 | 提高可读性 |
| Q-02 | 参数数量 | 函数参数 ≤ 6 个，超过必须使用结构体 | 如 `DispatchContext` |
| Q-03 | 嵌套深度 | `if`/`match` 嵌套 ≤ 3 层 | 超过需提取函数 |
| Q-04 | 复杂度 | Cyclomatic complexity ≤ 10 | 使用 `cargo complexity` 或人工审查 |
| Q-05 | 魔法数字 | 禁止硬编码数字，必须提取为常量 | 如超时时间、重试次数 |
| Q-06 | 命名规范 | 遵循 Rust API  guidelines，变量/函数 snake_case，类型 PascalCase | `clippy::style` 系列 |
| Q-07 | 文档覆盖 | 所有 `pub` 函数/结构体必须有 `///` 文档 | 至少说明用途和参数 |
| Q-08 | 错误信息质量 | 错误信息必须包含上下文，禁止 `unwrap()`/`expect()` 在生产代码 | 测试代码可放宽 |
| Q-09 | 克隆最小化 | 禁止不必要的 `.clone()`，优先使用引用 | clippy::clone_on_copy |
| Q-10 | 所有权清晰 | 禁止 `Rc<RefCell<>>` 除非经过架构评审 | 优先 `Arc<Mutex<>>` 或 `Arc<RwLock<>>` |

---

## 4. 安全性

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| S-01 | 依赖审计 | 无 High/Critical 级别漏洞 | 已通过 cargo audit 验证 |
| S-02 | 审计依赖列表 | 所有新依赖必须加入 `[audited-deps]` | 在 `architecture.toml` 中 |
| S-03 | 输入验证 | 所有外部输入必须验证 | 如 Signal payload、用户配置 |
| S-04 | 拒绝服务防护 | 有速率限制、熔断、超时机制 | Circuit breaker、backoff |
| S-05 | 敏感信息 | 日志中不得打印密钥、token、密码 | 使用 `tracing` 时注意 |
| S-06 | 序列化安全 | 反序列化必须验证数据完整性 | 如 bincode 长度检查 |
| S-07 | 权限最小化 | Cell/Tool 权限必须显式声明，默认拒绝 | `#[tool(permission = "read")]` |
| S-08 | 架构规则不可绕过 | 层间调用规则必须编译期或运行期强制 | `Layer::can_send_to` |
| S-09 | 供应链安全 | 使用 `cargo vendor` 或私有 registry 锁定依赖 | 生产环境建议 |
| S-10 | 故障隔离 | Cell panic 不得拖垮整个 Runtime | Supervisor + circuit break |

---

## 5. 性能

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| P-01 | 零成本抽象 | 核心路径使用零成本抽象，避免动态分发 | 除非 trait object 必要 |
| P-02 | 内存安全 | 无内存泄漏，无悬垂指针 | 使用 `cargo valgrind` 或 `asan` |
| P-03 | 锁粒度 | 锁范围最小化，禁止持有锁进行 I/O | 如 SQLite 查询 |
| P-04 | 批量操作 | 支持批量写入/读取，避免逐条 I/O | Witness 批量写入 |
| P-05 | 缓存策略 | 热点数据有缓存，缓存失效策略明确 | Lens cache、snapshot |
| P-06 | 序列化效率 | 内部总线优先使用 bincode，JSON 仅用于调试 | 体积减少 ≥ 50% |
| P-07 | 并发度可配置 | 关键路径并发度可配置，禁止硬编码 | dispatch workers、batch size |
| P-08 | 性能回归防护 | 关键路径有基准测试，CI 中运行 | `cargo bench` |
| P-09 | 延迟 P99 | 消息投递 P99 延迟 < 1ms（单 Cell） | 需压测验证 |
| P-10 | 吞吐量 | 单实例支持 ≥ 10K msg/s | 已通过 stress test |

---

## 6. 测试策略

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| T-01 | 测试分层 | 必须包含单元测试、集成测试、端到端测试 | 缺一不可 |
| T-02 | 测试命名 | 测试函数名清晰表达意图，禁止 `test1`/`foo` | `test_xxx_when_yyy_should_zzz` |
| T-03 | 断言明确 | 每个测试至少 1 个 `assert!`，禁止空测试 | 禁止 `#[test] fn dummy() {}` |
| T-04 | 异常路径 | 必须测试失败/错误/超时场景 | 不能只测 happy path |
| T-05 | 并发安全 | 必须包含并发/race condition 测试 | `loom` 或 tokio::test |
| T-06 | 属性测试 | 核心数据结构必须有 property-based tests | `proptest` 或 `quickcheck` |
| T-07 | 测试数据工厂 | 禁止在测试中硬编码复杂数据，使用工厂函数 | `make_witness()` 等 |
| T-08 | 测试隔离 | 测试之间无共享状态，可独立运行 | 禁止全局 static mut |
| T-09 | 快照测试 | 关键输出必须有快照测试 | 防止意外 API 变更 |
| T-10 | 覆盖率 | 核心 crate 行覆盖率 ≥ 80% | 使用 `cargo tarpaulin` |

---

## 7. 可维护性

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| M-01 | 模块大小 | 单个 `.rs` 文件 ≤ 400 行，超过必须拆分子模块 | 已执行 |
| M-02 | 导入顺序 | 标准库 → 第三方 → 本地，分组用空行 | `cargo fmt` 自动处理 |
| M-03 | 公共 API 稳定性 | `pub` 项变更必须记录 CHANGELOG | semver 规则 |
| M-04 | 弃用策略 | 废弃 API 必须标注 `#[deprecated]` 并给出迁移路径 | 禁止直接删除 |
| M-05 | 配置外部化 | 所有配置项支持外部注入，禁止硬编码 | 环境变量/配置文件 |
| M-06 | 日志规范 | 使用 `tracing`，关键路径有日志 | 禁止 `println!` |
| M-07 | 指标规范 | 所有关键操作有对应 metrics | counter/gauge/histogram |
| M-08 | 链路追踪 | 跨 Cell 调用有 trace_id 传播 | OpenTelemetry |
| M-09 | 版本策略 | 遵循 semver，Cargo.toml 版本准确 | 当前 0.3.0 |
| M-10 | 贡献指南 | CONTRIBUTING.md 存在且内容完整 | 代码风格、提交流程 |

---

## 8. 文档完整性

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| D-01 | README | 必须包含：项目介绍、快速开始、架构图、示例 | 首页文档 |
| D-02 | 变更日志 | CHANGELOG.md 存在且格式正确 | Keep a Changelog |
| D-03 | API 文档 | `cargo doc --no-deps` 可生成，无警告 | 在线文档 |
| D-04 | 架构文档 | 包含架构决策记录（ADR）或设计文档 | 解释“为什么” |
| D-05 | 部署文档 | 包含生产部署、配置、备份恢复 | PRODUCTION.md |
| D-06 | 运维手册 | 包含监控、告警、排查流程 | OPERATIONS.md |
| D-07 | 性能指南 | 包含基准结果、调优建议 | PERFORMANCE.md |
| D-08 | 迁移指南 | 包含从其他框架迁移的步骤 | MIGRATION.md |
| D-09 | 示例代码 | 每个核心功能有最小可运行示例 | examples/ 目录 |
| D-10 | 安全策略 | 包含漏洞报告流程、安全策略 | SECURITY.md |

---

## 9. 依赖管理

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| G-01 | 依赖最小化 | 每个 crate 的依赖项 ≤ 30 个 | 超过需评审 |
| G-02 | 版本锁定 | 关键依赖版本锁定在 Cargo.lock | 防止意外升级 |
| G-03 | 特性最小化 | 每个 crate 的 features ≤ 5 个 | 过多特性增加复杂度 |
| G-04 | 传递依赖可见 | 关键 crate 的传递依赖经过审计 | `cargo tree` 审查 |
| G-05 | 废弃依赖清理 | 定期清理未使用的依赖 | `cargo udeps` |
| G-06 | 许可兼容 | 所有依赖许可证兼容（MIT/Apache-2.0/BSD） | 禁止 GPL 传染 |
| G-07 | 平台兼容 | 支持目标平台的最小 Rust 版本声明 | MSRV |
| G-08 | 构建脚本最小化 | 禁止不必要的 build.rs | 必须要有充分理由 |
| G-09 | 依赖更新策略 | 使用 `cargo deny` 或 `dependabot` | 自动化 |
| G-10 | 私有 registry | 生产环境使用私有 registry 或 vendor | 防止供应链攻击 |

---

## 10. 错误处理与韧性

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| E-01 | 错误类型统一 | 每个 crate 有统一的 `Error` enum | 禁止 `Box<dyn Error>` |
| E-02 | 错误信息完整 | 错误必须包含上下文（file/line/operation） | 便于排查 |
| E-03 | 超时机制 | 所有 I/O 操作有超时 | SQLite、网络、文件 |
| E-04 | 重试策略 |  transient 错误有指数退避重试 | 禁止无限重试 |
| E-05 | 熔断机制 | 连续失败触发熔断，暂停处理 | entropy/circuit break |
| E-06 | 优雅降级 | 非关键功能失败不影响核心路径 | metrics 失败不影响 runtime |
| E-07 | 资源清理 | 所有资源有 `Drop` 或 `finally` 清理 | 文件句柄、连接池 |
| E-08 | 避免 unwrap | 生产代码禁止 `unwrap()`/`expect()` | 测试代码可放宽 |
| E-09 | 错误传播 | 使用 `?` 运算符，禁止吞掉错误 | `let _ = x?` 需评审 |
| E-10 | 恢复文档 | 每个错误码有恢复建议 | 如 `EventStore::append` |

---

## 11. API 设计

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| I-01 | 一致性 | 相似功能命名/签名一致 | 如所有 store 的 `append`/`read` |
| I-02 | 最小惊讶 | API 行为符合直觉，无隐藏副作用 | `get` 不修改状态 |
| I-03 | 不可变优先 | 优先返回 `&T`，需要所有权时返回 `T` | 避免不必要的克隆 |
| I-04 | Builder 模式 | 复杂配置使用 Builder，禁止 10+ 参数函数 | `RuntimeBuilder` |
| I-05 | 默认值合理 | 所有配置项有合理默认值 | `Default` trait |
| I-06 | 类型安全 | 禁止 `Stringly typed`，使用强类型 | `CellId` 而非 `String` |
| I-07 | 零成本抽象 | 核心路径无运行时开销 | trait + generics |
| I-08 | 向后兼容 | semver 规则严格执行 | major 版本才允许 breaking |
| I-09 | 异步一致 | 异步函数统一使用 `async fn` | 禁止手动实现 Future |
| I-10 | 特征选择 | `pub trait` 仅当需要多实现时使用 | 单实现用 `struct` |

---

## 12. 并发与并行

| 编号 | 检查项 | 标准 | 说明 |
|------|--------|------|------|
| R-01 | 无数据竞争 | `miri` 或 `loom` 验证无 data race | 核心并发路径 |
| R-02 | 锁顺序固定 | 多锁场景有固定获取顺序 | 防止死锁 |
| R-03 | 无忙等待 | 禁止 `loop {}`/`while {}` 空转 | 使用 `parking_lot` |
| R-04 | 线程安全 | 所有 `pub` 类型自动实现 Send/Sync | 除非明确不需要 |
| R-05 | 原子操作 | 计数器使用 `AtomicU64` 而非 `Mutex` | 性能优化 |
| R-06 | 通道选择 | 消息传递优先 `mpsc`/`broadcast`，共享状态次之 | Go 原则 |
| R-07 | 并发度可控 | 并发任务数量可配置，禁止无界 spawn | semaphore 限制 |
| R-08 | 取消安全 | 异步任务支持取消，不泄漏资源 | `CancellationToken` |
| R-09 | 运行时检测 | CI 中运行 `cargo miri test` | UB 检测 |
| R-10 | 性能可扩展 | 并发场景下线性扩展，无锁争用热点 | 使用 `DashMap` 等 |

---

## 审查流程

1. **自检**：开发者按本标准自查
2. **同行评审**：至少 1 人 review，使用本清单逐项核对
3. **自动化**：CI 中运行 `cargo check/test/clippy/audit`
4. **架构评审**：复杂变更需架构师评审，更新 `architecture.toml`
5. **回归测试**：所有测试通过，无性能回归

---

## 评分规则

- 每个条目：PASS / FAIL / WARN
- FAIL：必须修复，阻断合并
- WARN：建议修复，需在 PR 中说明原因
- PASS：符合标准

**准入门槛**：所有 C 类、A 类条目必须 PASS，允许最多 3 个 WARN。
