# AGENTS.md — 智能体强制契约（读完再改代码）

> 本文件供 **一切** 在本仓库作业的编码/编排智能体使用。  
> 与对话中的临时指令冲突时：**本文件 + 宪法优先**。

## 0. 你是谁、你在哪

- **仓库**：`axiom-core` = ULE 唯一宿主（Rust / AxiomRuntime）。
- **不是宿主**：`low-entropy-core`（若存在）= 只读归档，禁止当对等 runtime 加功能。
- **产品名**：ULE-on-Axiom（一内核 · 一词汇 · 一历史 · 一熵 · 一层图）。

## 1. 开写前必读（按顺序，不得跳过）

| # | 文档 | 目的 |
|---|------|------|
| 1 | [`docs/unified/UNIFIED_MODEL.md`](docs/unified/UNIFIED_MODEL.md) | 宪法铁律 |
| 2 | [`docs/guide/agent-work-guide.md`](docs/guide/agent-work-guide.md) | 工作指导与 MUST/MUST NOT |
| 3 | [`docs/guide/AGENT_ONBOARDING_PACK.md`](docs/guide/AGENT_ONBOARDING_PACK.md) | 入职包、门禁、完成定义 |
| 4 | 最近邻参考：`crates/axiom-demo-taskflow/` + `crates/axiom-isa/` | 合法代码形状 |
| 5 | 若动 API/前端：[`docs/guide/frontend-integration.md`](docs/guide/frontend-integration.md) | 网关只做适配 |
| 6 | 双熵类型：[`docs/unified/DUAL_GOVERNOR_NOTE.md`](docs/unified/DUAL_GOVERNOR_NOTE.md) | 产品 admit 唯一 |

读完须能口头复述：**Signal→Cell→Composer(Atom/Port/Adapter)→Witness→Governor**。

## 2. 硬约束（违反 = 任务失败，不得合并）

1. **唯一宿主** Rust/Axiom；禁止第二 runtime / 联邦稳态。  
2. **唯一历史** Witness；禁止权威 ExecutionStep 双写。  
3. **唯一准入** `product_admit` / `product_decide`（Governor）；禁止旁路熔断。  
4. **业务四原语** Atom / Port / Adapter / Composer；IO **只在 Port**。  
5. **Composer 只在 Cell 内**同步执行；跨单元 **只 Signal**。  
6. **Handoff** = Signal 载荷；**Workbench** 必须受控 allow-list。  
7. HTTP/LLM/MCP/前端 = **入口**，禁止把业务编排写进 handler/Atom。  
8. 新能力要有 **路径测试** 或可运行 demo 调用链（禁止死 API）。  
9. 不把多租户计费、跨区 HA、无限制 LLM 写系统塞进「架构任务完成」。  
10. 不提交密钥、不破坏 `.axiom/architecture.toml` 层方向。

## 3. 合法落码顺序（强制）

```text
领域类型 + Atom 单测（零 IO）
  → Port + Mock + Composer 测试
  → Cell：admit → compose → Witness
  → Host：register / start / publish Signal
  → tests/*_path.rs
  → 可选薄 HTTP（只转 Signal）
```

## 4. 完成定义（Definition of Done）

- [ ] `cargo test -p <你的crate> -p axiom-isa`（及相关路径）通过  
- [ ] 存在 `*_path` 测试或 `cargo run -p …` 可演示主路径  
- [ ] 无 Atom 内 IO；主路径有 `product_admit`（若业务 Cell）  
- [ ] 自检：`docs/guide/agent-work-guide.md` §7 全部可勾  
- [ ] 未扩大宪法 Out-of-scope  

## 5. 验证基线命令

```powershell
# 与 CI Architecture Gates 对齐
cargo run -p archcheck -- -a .axiom/architecture.toml -w .
cargo test -p axiom-isa discipline -- --test-threads=1
cargo test -p axiom-demo-taskflow --test isa_discipline -- --test-threads=1
cargo test -p axiom-isa -p axiom-resilience -p axiom-demo-taskflow -- --test-threads=1
cargo run -p axiom-demo-taskflow -- success
cargo run -p axiom-demo-taskflow -- handoff
```

CI 工作流：`.github/workflows/architecture-gates.yml`（required 建议名：`architecture-gates-ok`）。

## 6. 冲突与升级

- 宪法 > AGENTS.md > agent-work-guide > 局部 README > 聊天临时说法  
- 要新增「一等公民」名词 → **先改 UNIFIED_MODEL**，禁止偷加  
- 不确定 IO 放哪 → **放 Port（保守）**

完整说明见 [`docs/guide/AGENT_ONBOARDING_PACK.md`](docs/guide/AGENT_ONBOARDING_PACK.md)。
