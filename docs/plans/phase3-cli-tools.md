# Phase 3: CLI工具

> **预估工期**: 3周
> **前置条件**: Phase 2 完成（Witness持久化）
> **后续阶段**: Phase 4 - MCP协议桥接

---

## 阶段目标

实现完整的 CLI 工具链，包括项目脚手架、运行命令、实时监控、调试诊断和运维命令。

---

## 任务清单

### Task 3.1: 项目脚手架

**描述**: `axm new <name>` 创建完整的项目结构。

**涉及文件**:
- `crates/axiom-cli/src/commands/new.rs`

**生成结构**:
```
my-agent/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cells/
│   │   └── mod.rs
│   ├── signals/
│   │   └── mod.rs
│   └── axioms/
│       └── mod.rs
└── .axiom/
    └── config.toml
```

**步骤**:
1. 实现 `new` 命令
2. 创建项目模板
3. 添加必要依赖到 `Cargo.toml`
4. 生成 `main.rs` Runtime启动模板

**验收标准**:
- `axm new my-agent && cargo run` 成功

---

### Task 3.2: 运行命令

**描述**: 实现 `axm run` 和 `axm dev` 命令。

**涉及文件**:
- `crates/axiom-cli/src/commands/run.rs`

**功能**:
- `axm run`: 生产模式启动，加载配置，连接到生产环境
- `axm dev`: 开发模式，热重载，详细日志，开发环境配置

**步骤**:
1. 实现 `run` 命令
2. 实现 `dev` 命令
3. 支持配置文件加载
4. 支持环境变量覆盖

**验收标准**:
- 命令可启动Runtime
- 开发模式有详细日志

---

### Task 3.3: 实时监控TUI

**描述**: 使用 `ratatui` 实现类似 htop 的终端界面。

**涉及文件**:
- `crates/axiom-cli/src/commands/top.rs`

**功能**:
- Cell状态列表（Running/Crashed/Suspended）
- 熵值仪表盘（红/黄/绿）
- 消息吞吐量
- 延迟统计
- 层分布视图

**步骤**:
1. 添加 `ratatui` 依赖
2. 实现 TUI 界面
3. 定期从 Runtime 获取状态
4. 实时更新界面

**验收标准**:
- `axm top` 显示实时状态
- 界面响应流畅

---

### Task 3.4: 调试诊断

**描述**: 实现 `axm trace`、`axm why`、`axm witness` 命令。

**涉及文件**:
- `crates/axiom-cli/src/commands/trace.rs`
- `crates/axiom-cli/src/commands/why.rs`
- `crates/axiom-cli/src/commands/witness.rs`

**功能**:
- `axm trace <correlation_id>`: 追踪完整调用链
- `axm why <entity_id>`: 显示因果链（为什么会发生）
- `axm witness <cell_id>`: 查看Witness历史
- `axm witness verify`: 验证Witness链完整性

**步骤**:
1. 实现 `trace` 命令
2. 实现 `why` 命令
3. 实现 `witness` 命令及其子命令

**验收标准**:
- 命令返回正确结果
- 输出格式清晰易读

---

### Task 3.5: 运维命令

**描述**: 实现 Cell 管理和熵值查看命令。

**涉及文件**:
- `crates/axiom-cli/src/commands/cell.rs`
- `crates/axiom-cli/src/commands/entropy.rs`

**功能**:
- `axm cell list`: 列出所有Cell
- `axm cell restart <cell_id>`: 重启指定Cell
- `axm cell stop <cell_id>`: 停止指定Cell
- `axm cell status <cell_id>`: 查看Cell状态
- `axm entropy`: 查看系统熵值
- `axm entropy reset`: 重置熵值
- `axm entropy threshold`: 查看/设置阈值

**步骤**:
1. 实现 `cell` 命令及其子命令
2. 实现 `entropy` 命令及其子命令

**验收标准**:
- 命令执行成功
- 权限检查正确

---

## 质量门禁

```bash
# 每次任务完成后必须通过
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -D warnings
cargo build --workspace --all-targets
cargo test --workspace
```

---

## 阶段验收标准

- [ ] 项目脚手架可用（`axm new`）
- [ ] 运行命令可用（`axm run` / `axm dev`）
- [ ] 实时监控TUI可用（`axm top`）
- [ ] 调试诊断命令可用（`axm trace` / `axm why` / `axm witness`）
- [ ] 运维命令可用（`axm cell` / `axm entropy`）
- [ ] CLI测试覆盖主要命令
- [ ] `cargo test --workspace` 全部通过

---

## 关键文件索引

| 文件 | 说明 |
|------|------|
| [crates/axiom-cli/src/commands/new.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-cli/src/commands/new.rs) | 项目脚手架 |
| [crates/axiom-cli/src/commands/run.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-cli/src/commands/run.rs) | 运行命令 |
| [crates/axiom-cli/src/commands/top.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-cli/src/commands/top.rs) | 实时监控 |
| [crates/axiom-cli/src/commands/trace.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-cli/src/commands/trace.rs) | 追踪命令 |
| [crates/axiom-cli/src/commands/cell.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-cli/src/commands/cell.rs) | Cell管理 |
