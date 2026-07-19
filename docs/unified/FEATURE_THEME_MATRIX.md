# ULE 功能主题矩阵（FEATURE THEME MATRIX）

| Field | Value |
|-------|--------|
| **Status** | **完全满足**（宪法核心主题 T1–T15） |
| **Date** | 2026-07-19 |
| **Host** | `C:\work\architecture\axiom-core` |
| **Constitution** | [UNIFIED_MODEL.md](./UNIFIED_MODEL.md) |
| **Delivery** | [COMMERCIAL_DELIVERY.md](./COMMERCIAL_DELIVERY.md) |

> 本表回答：**功能清单与代码是否满足统一架构主题**。  
> 范围 = 宪法核心（一内核 / 一词汇 / 一历史 / 一熵 / 一层图 + U1–U6 产品地板）。  
> **不在范围**（仍非失败条件）：多租户计费、跨区 HA、全量 LE Go 重写、无限制 LLM Workbench SaaS。

---

## 1. 总判

| 维度 | 结论 |
|------|------|
| 宪法铁律 L1–L10 | **满足** |
| 商业产品地板（task + handoff + surface） | **满足** |
| 主题 T1–T15 | **完全满足**（见 §2） |
| SaaS 运营件 | **明确 Out of scope**（非主题缺口） |

---

## 2. 主题 × 代码锚点

| ID | 主题 | 状态 | 代码 / 验证锚点 |
|----|------|------|-----------------|
| **T1** | 唯一 Rust 宿主内核 | ✅ | `axiom-kernel` + `axiom-runtime`；LE `ARCHIVED.md` |
| **T2** | 唯一词汇表（Cell/Signal/…） | ✅ | `UNIFIED_MODEL` §2；demo 仅用统一名词 |
| **T3** | 唯一历史 = Witness | ✅ | `WitnessJournal`；无 ExecutionStep 权威写 |
| **T4** | 唯一熵决策 = Governor | ✅ | `product_admit` / `product_decide`；Agent/Task 仅此入口 |
| **T5** | 四原语 ISA | ✅ | `axiom-isa::{Atom,Port,Adapter,Composer}` |
| **T6** | Composer-in-Cell | ✅ | `TaskCell` / `AgentCell` 内同步 `compose` |
| **T7** | 单向依赖 / 层门禁 | ✅ | `.axiom/architecture.toml` + `archcheck` |
| **T8** | Atom 纯 / Port 副作用纪律 | ✅ | `axiom_isa::discipline` 扫描 + `tests/isa_discipline.rs` |
| **T9** | U2 韧性标准库 | ✅ | `axiom-resilience`（retry/circuit/rate/bulkhead） |
| **T10** | Handoff = Signal 载荷 | ✅ | `HandoffRequest` + `SIGNAL_HANDOFF` |
| **T11** | 受控 Workbench | ✅ | `workbench.rs`：allow-list + `WorkbenchLimits` + mock LLM Port + 逐步 Witness + plugin_echo 沙箱 |
| **T12** | 统一可观测 | ✅ | Surface：`/api/v1/surface` + `/metrics` + `/api/v1/metrics` + health 全字段 |
| **T13** | 单一层图 U0–U7 | ✅ | 宪法 §3；delivery 叙事 |
| **T14** | 商业路径可演示 | ✅ | `taskflow` CLI：success/fail/melt/handoff/surface/plugin |
| **T15** | Lens 只读投影 | ✅ | `lenses.rs` + `/api/v1/lens/{id}`（runs/governor/health/metrics/plugins） |
| **+P** | 插件产品路径 | ✅ | `plugin_host.rs`：registry + NativePluginSandbox + hot-reload |

---

## 3. 弱项关闭记录（本轮）

| 原弱项 | 关闭方式 |
|--------|----------|
| T8 全库纪律门禁 | `discipline::scan_source` + `COMMERCIAL_ISA_SOURCES` 路径测试；禁止 Composer 外 raw I/O |
| T11 Workbench 偏浅 | mock LLM propose Port、`WorkbenchLimits`、逐步执行与 Witness、`plugin_echo` 沙箱语义 |
| T12 观测不完整 | Prometheus `/metrics`、JSON metrics、degraded/heartbeat、observability 路由表 |
| Lens 未上商业路径 | 五个商业 Lens + surface 路由 |
| 插件未产品化 | `ProductPluginHost` + CLI `plugin` + surface `/api/v1/plugins` |

---

## 4. 验证命令

```powershell
cd C:\work\architecture\axiom-core
cargo test -p axiom-isa -p axiom-demo-taskflow
cargo run -p axiom-demo-taskflow -- success
cargo run -p axiom-demo-taskflow -- handoff
cargo run -p axiom-demo-taskflow -- surface
cargo run -p axiom-demo-taskflow -- plugin
```

路径测试：

- `tests/isa_discipline.rs` — T8  
- `tests/observability_path.rs` — T12 + Lens + plugins  
- `tests/handoff_path.rs` — T10/T11  
- `tests/runtime_path.rs` — T6/T9/T14  

---

## 5. 明确非缺口（防止范围膨胀）

| 项 | 说明 |
|----|------|
| 多租户 / 计费 / 计量 | 商用运营，非宪法主题 |
| 跨区 HA / 共识 | 后置 |
| 真·公网 LLM 供应商密钥链路 | Port 形状已满足；密钥与 SLA 属部署 |
| WASM 用户任意代码执行 | 宪法要求受控；现为 allow-list + plugin 沙箱地板 |
| LE arch-manager 全量 UI | Out |
| `cargo test --workspace` 历史包全绿 | 工程债，不阻塞主题满足 |

---

## 6. 签字式结论

> **统一架构核心主题（T1–T15）在宿主 monorepo 上已完全满足。**  
> 产品地板可演示、可测、可观测；剩余工作属于运营扩展与生态深化，而非宪法未完成。
