# 事前约束机制 — 在开发开始前强制执行

> **原则**: 约束必须在开发的最早阶段生效，而非事后检查
> **目标**: 让违规代码无法编写，而非编写后再修复

---

## 一、事前约束总览

| 约束维度 | 事前机制 | 强制执行点 |
|----------|---------|-----------|
| Crate层依赖 | `axm new-crate` 命令自动验证 | 创建crate时 |
| 层间调用 | `#[cell]` / `#[signal]` 宏自动标注 | 编写代码时 |
| 依赖审核 | 脚手架自动导入已审核依赖 | 创建项目时 |
| 禁止依赖 | `axm gate pre-check` 命令 | 开发开始前 |
| 错误处理 | IDE配置自动提示 | 编写代码时 |
| 文档规范 | 模板自动生成文档结构 | 创建文件时 |

---

## 二、事前约束机制实现

### 机制1: Crate创建脚手架 — 自动验证层级

**命令**: `axm new-crate <name> --layer <level>`

**流程**:
1. 检查crate名称是否已存在
2. 验证层级是否合理（0-7）
3. 验证依赖方向（只能依赖同层或下层）
4. 自动生成符合约束的 `Cargo.toml`
5. 自动生成 `lib.rs` 模板（含层约束）

**示例**:
```bash
# 创建 axiom-rag crate，层级4（依赖runtime和core）
axm new-crate axiom-rag --layer 4

# 自动验证：层级4可以依赖层级>=4的crate
# 自动生成：Cargo.toml 只包含合法依赖
```

**强制执行**: 不通过 `axm new-crate` 创建的crate，CI会拒绝

---

### 机制2: Cell创建脚手架 — 自动标注层归属

**命令**: `axm new-cell <name> --layer <layer>`

**流程**:
1. 验证层名称（oversight/agent/validate/exec）
2. 自动生成 `#[cell(layer="...")]` 宏标注
3. 自动生成 `LayerMarker` 类型绑定
4. 自动生成 `handle()` 方法模板（含合法的 `send_to` 调用）

**示例**:
```bash
# 创建Agent层Cell
axm new-cell MyAgentCell --layer agent

# 自动生成：
#[cell(layer = "agent")]
impl Cell for MyAgentCell {
    type Message = MyMessage;
    type Layer = AgentLayer;
    
    fn handle<'a>(&'a mut self, signal: Self::Message, ctx: LayeredCellContext<'a, Self::Layer>) -> ... {
        async move {
            // 自动提示：只能调用 AgentLayer 和 ValidateLayer
            ctx.send_to::<ValidateLayer, _>(...); // ✅ 合法
            // ctx.send_to::<OversightLayer, _>(...); // ❌ 被禁止的调用不会生成
            Ok(())
        }
    }
}
```

**强制执行**: 不通过脚手架创建的Cell，编译期会报错

---

### 机制3: Signal创建脚手架 — 自动标注层归属

**命令**: `axm new-signal <name> --kind <kind> --layer <layer>`

**流程**:
1. 验证kind（command/event/query/reply）
2. 验证层名称
3. 自动生成 `#[signal(kind="...", layer="...")]` 宏标注
4. 自动生成 `Signal` trait实现
5. 自动生成 `Schema` trait实现（验证框架）

**示例**:
```bash
axm new-signal UserRequest --kind command --layer agent

# 自动生成：
#[signal(kind = "command", layer = "agent")]
struct UserRequest {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    // 用户自定义字段
}

impl Signal for UserRequest {
    fn signal_type(&self) -> &str { "UserRequest" }
    fn kind(&self) -> SignalKind { SignalKind::Command }
}

impl Schema for UserRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        // 自动生成验证框架
        Ok(())
    }
}
```

---

### 机制4: Tool创建脚手架 — 自动添加权限控制

**命令**: `axm new-tool <name> --permission <permission>`

**流程**:
1. 验证工具名称
2. 验证权限名称（或默认none）
3. 自动生成 `Tool` trait实现模板
4. 自动生成参数验证逻辑
5. 自动生成调用历史记录

**示例**:
```bash
axm new-tool FileReader --permission read

# 自动生成：
struct FileReader;

#[async_trait]
impl Tool for FileReader {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "file_reader".to_string(),
            description: "Read file content".to_string(),
            parameters: vec![ToolParameter {
                name: "path".to_string(),
                description: "File path".to_string(),
                required: true,
                schema: serde_json::json!({"type": "string"}),
            }],
            required_permission: Some("read".to_string()),
            version: "1.0.0".to_string(),
        }
    }
    
    async fn execute(&self, parameters: &Value) -> Result<Value, ToolError> {
        // 参数验证自动生成
        let path = parameters.get("path")
            .ok_or(ToolError::InvalidParameters("path is required".to_string()))?;
        // 执行逻辑
        Ok(serde_json::json!({"content": "..."}))
    }
}
```

---

### 机制5: 预提交钩子 — 开发前自动检查

**命令**: `axm install-hooks`

**流程**:
1. 安装 `pre-commit` 钩子（已实现）
2. 安装 `pre-push` 钩子（新增）
3. 安装 `prepare-commit-msg` 钩子（新增）

**pre-commit 钩子**:
```bash
#!/bin/bash
echo "[AXIOM] Running pre-commit checks..."

# L0: 格式检查
cargo fmt --all --check || exit 1

# L1: 编译期检查
cargo clippy --workspace --all-targets --all-features -D warnings || exit 1

# L2: 依赖检查
cargo run --bin axm -- gate check || exit 1

echo "[AXIOM] All pre-commit checks passed ✓"
```

**pre-push 钩子**:
```bash
#!/bin/bash
echo "[AXIOM] Running pre-push checks..."

# 运行所有测试
cargo test --workspace || exit 1

# 基准测试编译检查
cargo bench -p axiom-bench --no-run || exit 1

echo "[AXIOM] All pre-push checks passed ✓"
```

---

### 机制6: IDE配置 — 实时约束提示

**文件**: `.vscode/settings.json`

**配置**:
```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.checkOnSave.extraArgs": [
        "--all-targets",
        "--all-features",
        "-D", "warnings"
    ],
    "editor.formatOnSave": true,
    "editor.codeActionsOnSave": {
        "source.organizeImports": true
    },
    "rust-analyzer.cargo.features": "all"
}
```

**效果**:
- 保存时自动运行 Clippy
- 零容忍警告（所有警告视为错误）
- 自动格式化代码
- 自动组织导入

---

### 机制7: 开发环境检查 — 启动时验证

**命令**: `axm env-check`

**流程**:
1. 检查Rust版本（>=1.75）
2. 检查工具链组件（rustfmt/clippy）
3. 检查git hooks是否安装
4. 检查环境变量配置
5. 检查依赖状态

**示例**:
```bash
axm env-check

# 输出:
[AXIOM] Environment Check
[✓] Rust version: 1.76.0 (required: >=1.75)
[✓] rustfmt: installed
[✓] clippy: installed
[✓] Git hooks: installed
[✓] Environment variables: OK
[✓] Dependencies: up to date
[✓] All checks passed!
```

---

### 机制8: 模板仓库 — 确保新代码符合约束

**模板结构**:
```
templates/
├── crate/                    # crate模板
│   ├── Cargo.toml            # 含层约束的Cargo.toml
│   └── src/
│       └── lib.rs            # 含层标注的lib.rs
├── cell/                     # Cell模板
│   └── cell.rs               # 含层标注的Cell实现
├── signal/                   # Signal模板
│   └── signal.rs             # 含层标注的Signal实现
├── tool/                     # Tool模板
│   └── tool.rs               # 含权限控制的Tool实现
├── test/                     # 测试模板
│   └── integration.rs        # 含Witness验证的集成测试
└── agent/                    # Agent模板
    └── agent.rs              # 含Builder配置的Agent实现
```

**使用**:
```bash
# 创建新crate时使用模板
axm new-crate axiom-rag --layer 4 --template crate

# 创建新Cell时使用模板
axm new-cell MyAgentCell --layer agent --template cell
```

---

## 三、事前约束流程

### 开发前检查清单

```
1. 运行 axm env-check → 确保开发环境符合要求
2. 运行 axm install-hooks → 确保预提交钩子已安装
3. 确认Rust版本 >= 1.75 → 确保支持原生async trait
4. 确认IDE配置正确 → 确保实时约束提示
```

### 创建新组件流程

```
1. 使用 axm new-xxx 命令 → 自动生成符合约束的代码
2. 检查自动生成的层标注 → 确认层归属正确
3. 检查自动生成的依赖 → 确认依赖方向正确
4. 检查自动生成的权限控制 → 确认安全策略正确
5. 编写业务逻辑 → IDE实时提示约束违规
```

### 提交代码流程

```
1. 保存文件 → IDE自动运行Clippy（实时检查）
2. 暂存代码 → pre-commit钩子运行检查
3. 提交代码 → pre-commit钩子运行完整检查
4. 推送代码 → pre-push钩子运行测试和基准检查
5. CI运行 → 最终验证所有约束
```

---

## 四、事前约束强制执行矩阵

| 约束 | 开发前 | 创建时 | 编写时 | 保存时 | 提交时 | 推送时 | CI时 |
|------|--------|--------|--------|--------|--------|--------|------|
| Crate层依赖 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 层间调用 | - | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 依赖审核 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 禁止依赖 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 错误处理 | - | - | ✅ | ✅ | ✅ | ✅ | ✅ |
| 文档规范 | - | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 代码格式 | - | - | ✅ | ✅ | ✅ | ✅ | ✅ |

**✅**: 该阶段强制执行约束

---

## 五、事前约束工具实现

### Task: 实现 `axm new-crate` 命令

**子任务**:
- [ ] 实现命令解析（name, layer参数）
- [ ] 验证crate名称唯一性
- [ ] 验证层级合法性（0-7）
- [ ] 验证依赖方向（只能依赖>=当前层级）
- [ ] 自动生成 `Cargo.toml`（含合法依赖）
- [ ] 自动生成 `lib.rs`（含层标注）
- [ ] 在 `gate.rs` 的 `CRATE_LAYERS` 中注册

**验收标准**:
- 命令能正确创建crate
- 非法层级被拒绝
- 非法依赖被拒绝
- 生成的代码编译通过

---

### Task: 实现 `axm new-cell` 命令

**子任务**:
- [ ] 实现命令解析（name, layer参数）
- [ ] 验证层名称（oversight/agent/validate/exec）
- [ ] 自动生成 `#[cell(layer="...")]` 宏标注
- [ ] 自动生成 `LayerMarker` 类型绑定
- [ ] 自动生成 `handle()` 方法模板
- [ ] 自动生成测试模板

**验收标准**:
- 命令能正确创建Cell
- 生成的代码编译通过
- 非法层间调用在编译期被拒绝

---

### Task: 实现 `axm new-signal` 命令

**子任务**:
- [ ] 实现命令解析（name, kind, layer参数）
- [ ] 验证kind合法性（command/event/query/reply）
- [ ] 验证层名称
- [ ] 自动生成 `#[signal(kind="...", layer="...")]` 宏标注
- [ ] 自动生成 `Signal` trait实现
- [ ] 自动生成 `Schema` trait实现

**验收标准**:
- 命令能正确创建Signal
- 生成的代码编译通过
- Schema验证框架完整

---

### Task: 实现 `axm new-tool` 命令

**子任务**:
- [ ] 实现命令解析（name, permission参数）
- [ ] 验证工具名称
- [ ] 自动生成 `Tool` trait实现
- [ ] 自动生成参数验证逻辑
- [ ] 自动生成权限控制
- [ ] 自动生成测试模板

**验收标准**:
- 命令能正确创建Tool
- 生成的代码编译通过
- 权限控制正确

---

### Task: 实现 `axm env-check` 命令

**子任务**:
- [ ] 检查Rust版本
- [ ] 检查工具链组件
- [ ] 检查git hooks安装状态
- [ ] 检查环境变量
- [ ] 检查依赖状态

**验收标准**:
- 命令能正确检查所有项目
- 检查失败返回非零退出码
- 输出清晰易懂

---

### Task: 实现预提交钩子管理

**子任务**:
- [ ] 完善 `axm install-hooks` 命令
- [ ] 添加 pre-push 钩子
- [ ] 添加 prepare-commit-msg 钩子
- [ ] 添加钩子状态检查

**验收标准**:
- 钩子安装正确
- 钩子能自动运行检查
- 检查失败阻止提交/推送

---

## 六、事前约束效果

### 违规代码无法编写

```rust
// ❌ 无法编写：层间调用违规
#[cell(layer = "exec")]
impl Cell for MyExecCell {
    fn handle(...) {
        ctx.send_to::<AgentLayer, _>(...); // ❌ 编译失败：无法通过脚手架生成
    }
}
```

### 正确代码自动生成

```rust
// ✅ 通过脚手架自动生成：层间调用合法
#[cell(layer = "agent")]
impl Cell for MyAgentCell {
    fn handle(...) {
        ctx.send_to::<ValidateLayer, _>(...); // ✅ 自动生成的合法调用
        ctx.send_to::<AgentLayer, _>(...);     // ✅ 自动生成的合法调用
    }
}
```

### 依赖方向自动验证

```rust
// gate.rs 自动注册新crate
pub const CRATE_LAYERS: &[(&str, usize)] = &[
    ("axiom-rag", 4), // 自动添加
    // ...
];

// Cargo.toml 自动生成合法依赖
[dependencies]
axiom-core = { workspace = true }      // ✅ 层级7 >= 4
axiom-runtime = { workspace = true }   // ✅ 层级4 >= 4
# axiom-oversight = { workspace = true } // ❌ 不会生成：层级3 < 4
```

---

## 七、关键文件索引

| 文件 | 说明 |
|------|------|
| `crates/axiom-cli/src/commands/new_crate.rs`（新建） | `axm new-crate` 命令 |
| `crates/axiom-cli/src/commands/new_cell.rs`（新建） | `axm new-cell` 命令 |
| `crates/axiom-cli/src/commands/new_signal.rs`（新建） | `axm new-signal` 命令 |
| `crates/axiom-cli/src/commands/new_tool.rs`（新建） | `axm new-tool` 命令 |
| `crates/axiom-cli/src/commands/env_check.rs`（新建） | `axm env-check` 命令 |
| `crates/axiom-core/src/gate.rs` | Crate层依赖约束 |
| `templates/`（新建） | 代码模板目录 |
| `.vscode/settings.json`（新建） | IDE配置 |
