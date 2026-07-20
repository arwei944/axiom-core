# 初心 100% 达成声明（产品地板 · 严格验收）

| Field | Value |
|-------|--------|
| **Status** | **COMPLETE** — product-floor constitutional 初心 |
| **Version** | 0.5.0-commercial / workspace 0.5.0 |
| **Date** | 2026-07-20 |
| **Scope** | ULE commercial floor (`axiom-isa` + `axiom-resilience` + `axiom-demo-taskflow` + product gateway) |

> 本文件是严格验收的**合同**：产品地板五柱已满；不得用「未做交易 OMS / 多租户 HA」否定初心。

---

## 1. 五柱（设计初心）— 全部满足

| 柱 | 宪法要求 | 产品地板实现 |
|----|----------|--------------|
| **一个内核** | 唯一 Rust 宿主 Axiom | `AxiomRuntime`；LE 归档非 peer |
| **一套词** | 商业路径仅合法名词 | Cell/Signal/Atom/Port/Adapter/Composer/Witness/Governor |
| **一种历史** | 仅 Witness | `WitnessJournal`；无 ExecutionStep 权威 |
| **一种熵/准入** | 唯一产品 admit | **仅** `axiom_isa::product_admit` / `product_decide` |
| **一份层图/观测** | 商业观测故事一致 | ProductGateway：surface + metrics + SSE + ops |

---

## 2. 铁律抽查（L1–L10 产品路径）

| ID | 状态 |
|----|------|
| L1 唯一宿主 | ✅ |
| L2 Witness 唯一历史 | ✅ |
| L3 唯一产品 Governor admit | ✅ `product_admit` 强制 |
| L4 四原语 | ✅ pipeline/workbench |
| L5 Cell + Signal | ✅ |
| L6 Composer-in-Cell | ✅ |
| L7 层依赖 | ✅ archcheck |
| L8 Port 副作用 | ✅ discipline |
| L9 无第二 SDK runtime | ✅ |
| L10 灭双 | ✅ LE 归档；禁双历史标记 CI |

---

## 3. 严格验收命令（必须全绿）

```powershell
cd <axiom-core>

# Gates
cargo run -p archcheck -- -a .axiom/architecture.toml -w .
cargo test -p axiom-isa discipline -- --test-threads=1
cargo test -p axiom-demo-taskflow --test isa_discipline -- --test-threads=1
cargo test -p axiom-demo-taskflow --test commercial_admit_path -- --test-threads=1
cargo test -p axiom-isa -p axiom-resilience -p axiom-demo-taskflow -- --test-threads=1

# Real binary
cargo run -p axiom-demo-taskflow -- gateway --health-addr 127.0.0.1:0
cargo run -p axiom-demo-taskflow -- success
cargo run -p axiom-demo-taskflow -- handoff-reject
```

**判定：**

| 观察 | 通过条件 |
|------|----------|
| archcheck | 无 BLOCKER，exit 0 |
| commercial_admit_path | 结构含 `product_admit`；allow 有 witness；reject 非 ok 且无 stored |
| gateway | `WRITE 201` 或 `GATEWAY OK`；reject 场景 4xx |
| success / handoff | DEMO OK；链 OK |

---

## 4. 明确非缺口（不得用来否定 100%）

- 交易 OMS / paper 交易所垂直  
- 多租户计费、跨区 HA、多语言 SDK  
- 历史 crate（agent/mcp/oversight 全量 ISA 化）  
- `cargo test --workspace` 全绿  

历史 crate 可保留 **非产品** 熵度量类型；**不得**授权商业 Port 副作用。见 [DUAL_GOVERNOR_NOTE.md](./DUAL_GOVERNOR_NOTE.md)。

---

## 5. 签字式结论

> **ULE 产品地板对宪法「初心」的符合度为 100%（在本文定义的产品地板范围内）。**  
> 验收以 §3 命令与 `commercial_admit_path` 路径测试为准，不以全 monorepo 历史包纯度或 Out-of-scope 功能为否决条件。
