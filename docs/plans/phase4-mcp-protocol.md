# Phase 4: MCP协议桥接

> **预估工期**: 2周
> **前置条件**: Phase 3 完成（CLI工具）
> **后续阶段**: Phase 5 - Agent工具链

---

## 阶段目标

实现 MCP（Model Context Protocol）协议桥接，使 axiom-core 能够作为 MCP Server 提供工具能力，同时作为 MCP Client 调用外部工具。

---

## 任务清单

### Task 4.1: MCP客户端实现

**描述**: 实现 MCP Client，连接外部 MCP Server 并调用其工具。

**涉及文件**:
- `crates/axiom-mcp/src/client.rs`（新建）

**功能**:
- 连接外部 MCP Server
- 发现可用工具
- 调用工具并获取结果
- 自动重试和错误处理

**API设计**:
```rust
struct McpClient {
    connection: McpConnection,
    tools: Vec<ToolInfo>,
}

impl McpClient {
    fn connect(url: &str) -> Result<Self, McpError>;
    fn discover_tools(&mut self) -> Result<Vec<ToolInfo>, McpError>;
    fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, McpError>;
}
```

**验收标准**:
- 连接外部 MCP Server 成功
- 工具调用测试通过

---

### Task 4.2: MCP服务端实现

**描述**: 实现 MCP Server，暴露 axiom-core 的能力为 MCP Tools。

**涉及文件**:
- `crates/axiom-mcp/src/server.rs`（新建）

**功能**:
- 暴露 axiom-core 工具为 MCP Tools
- 处理工具调用请求
- 返回结果
- 认证和授权

**API设计**:
```rust
struct McpServer {
    tools: Vec<Arc<dyn McpTool>>,
    address: SocketAddr,
}

impl McpServer {
    fn new(tools: Vec<Arc<dyn McpTool>>) -> Self;
    async fn serve(&self) -> Result<(), McpError>;
}
```

**验收标准**:
- 外部 MCP Client 可连接并调用工具

---

### Task 4.3: Tool桥接

**描述**: 实现 MCP Tool ↔ axiom Tool 的映射。

**涉及文件**:
- `crates/axiom-mcp/src/bridge.rs`（新建）

**功能**:
- 将 axiom Tool 注册为 MCP Tool
- 将 MCP Tool 调用转换为 axiom Tool 调用
- 参数转换和验证
- 返回结果转换

**步骤**:
1. 定义 `McpTool` trait
2. 实现 `AxiomTool → McpTool` 适配器
3. 实现 `McpTool → AxiomTool` 适配器

**验收标准**:
- 双向转换测试通过

---

### Task 4.4: 安全层

**描述**: 实现 MCP 调用的四层安全检查。

**涉及文件**:
- `crates/axiom-mcp/src/security.rs`（新建）

**安全检查流程**:
```
MCP Tool调用 → Permission检查 → Rules检查 → Axiom检查 → Human-in-the-loop → 执行
```

**功能**:
- Permission检查：验证调用者是否有权调用该工具
- Rules检查：验证调用是否符合规则
- Axiom检查：验证调用不会违反架构约束
- Human-in-the-loop：高危工具需人工审批

**验收标准**:
- 未授权调用被拒绝
- 违规调用被拦截
- 高危工具调用需审批

---

### Task 4.5: 集成测试

**描述**: 编写完整的 MCP 调用链路测试。

**涉及文件**:
- `crates/axiom-mcp/tests/integration_tests.rs`（新建）

**测试场景**:
1. **客户端调用**: axiom-core 作为 Client 调用外部 MCP Server
2. **服务端调用**: 外部 MCP Client 调用 axiom-core 提供的工具
3. **安全检查**: 测试权限拒绝、规则拦截、Axiom违反
4. **完整链路**: 端到端测试

**验收标准**:
- 集成测试全部通过

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

- [ ] MCP客户端实现
- [ ] MCP服务端实现
- [ ] Tool桥接实现
- [ ] 安全层实现
- [ ] 集成测试全部通过
- [ ] `cargo test --workspace` 全部通过

---

## 关键文件索引

| 文件 | 说明 |
|------|------|
| [crates/axiom-mcp/src/client.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-mcp/src/client.rs) | MCP客户端 |
| [crates/axiom-mcp/src/server.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-mcp/src/server.rs) | MCP服务端 |
| [crates/axiom-mcp/src/bridge.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-mcp/src/bridge.rs) | Tool桥接 |
| [crates/axiom-mcp/src/security.rs](file:///D:/work/trae/axiom-core-project/crates/axiom-mcp/src/security.rs) | 安全层 |
