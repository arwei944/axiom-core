# ULE Commercial Delivery Note

| Field | Value |
|-------|--------|
| **Product** | ULE-on-Axiom (Unified Low-Entropy Kernel) |
| **Version** | **0.5.0-commercial** (U3–U5 + 主题完全满足 T1–T15) |
| **Date** | 2026-07-19 |
| **Theme matrix** | [FEATURE_THEME_MATRIX.md](./FEATURE_THEME_MATRIX.md) — **完全满足** |
| **Host monorepo** | `C:\work\architecture\axiom-core` |
| **Decision pack** | `C:\work\architecture\unified\` |
| **LE status** | **Archived / read-only assets** (`low-entropy-core/ARCHIVED.md`) |

## 1. What you are adopting

A **single-kernel** architecture product:

| Pillar | Choice (non-negotiable for new work) |
|--------|--------------------------------------|
| Host runtime | **Axiom / Rust** (`AxiomRuntime`) |
| Execution history | **Witness** hash chain only |
| Admit / entropy authority | **Governor** only (`product_decide` / `product_admit`) |
| Business shape | **Atom / Port / Adapter / Composer** |
| Orchestration | **Composer-in-Cell** |
| Agent chain (U3) | **HandoffRequest** Signal + controlled **Workbench** |
| Observation (U4) | Surface + Prometheus metrics + Lens + plugins |
| ISA discipline (T8) | `axiom_isa::discipline` commercial source gates |
| Workbench (T11) | Allow-list + limits + mock LLM Port + step Witness |
| LE Go core (U5) | **Not a product host** — archive/read-only |

**Not steady state:** dual-runtime federation, dual history, dual entropy decision engines, LE as peer runtime.

## 2. In-scope (shipped)

| Capability | Path |
|------------|------|
| ISA + Governor + Handoff + discipline | `crates/axiom-isa` |
| Resilience (retry/circuit/rate/bulkhead) | `crates/axiom-resilience` |
| Task + Agent + Surface + Lens + Plugin CLI | `crates/axiom-demo-taskflow` (`taskflow`) |
| Runtime / Kernel / Plugin sandbox | `axiom-runtime`, `axiom-kernel` |
| Theme satisfaction matrix | `unified/FEATURE_THEME_MATRIX.md` |
| Ops / auth docs | `docs/COMMERCIAL_OPS.md` |
| 工程硬化说明 | `docs/ENGINEERING_HARDENING_v050.md` |
| 升级任务清单 | `docs/TASK_CHECKLIST.md`（open = 0） |

### Commands

```powershell
cd C:\work\architecture\axiom-core
cargo test -p axiom-isa -p axiom-resilience -p axiom-demo-taskflow
cargo run -p axiom-demo-taskflow -- success
cargo run -p axiom-demo-taskflow -- handoff
cargo run -p axiom-demo-taskflow -- handoff-reject
cargo run -p axiom-demo-taskflow -- surface
cargo run -p axiom-demo-taskflow -- plugin
cargo run -p axiom-demo-taskflow -- health
cargo run -p axiom-demo-taskflow -- fail
cargo run -p axiom-demo-taskflow -- melt
```

### Paths exercised

- **Task**: Signal → TaskCell → Governor → Composer → Witness  
- **Agent (U3)**: AgentHandoff Signal → AgentCell → product_admit → Workbench (LLM propose Port + sandbox) → Witness  
- **Surface (U4/T12)**: `/api/v1/surface` · `/metrics` · `/api/v1/lens/{id}` · `/api/v1/plugins`  
- **Plugin**: `ProductPluginHost` register → sandbox invoke → hot-reload  
- **T8 discipline**: commercial sources scanned for Port-only side effects  

## 3. Deploy

See `docs/COMMERCIAL_OPS.md`. Docker: `Dockerfile.taskflow`, `docker-compose.yml` (`ule-taskflow` + `axiom-api`).

## 4. Explicit remaining non-goals（非主题缺口）

- Full mechanical LE Go → Rust rewrite of every file  
- Multi-tenant billing / SaaS metering / multi-region HA  
- Unrestricted public LLM code-exec Workbench / full MCP marketplace  
- Pixel LE arch-manager UI port  
- Green `cargo test --workspace` for every historical crate  

**主题层（T1–T15）已完全满足** — 见 [FEATURE_THEME_MATRIX.md](./FEATURE_THEME_MATRIX.md)。

## 5. Phase status

| Phase | Status |
|-------|--------|
| U0 constitution | done |
| U1 / U1.1 runtime task path | done |
| U2 resilience | done |
| **U3 Handoff/Workbench** | **done**（含 T11 limits + mock LLM Port） |
| **T8/T12/Lens/Plugin 关闭** | **done** |
| **U4 unified surface** | **done** |
| **U5 LE archive narrative** | **done** |
| **工程 TASK_CHECKLIST（P0–P3）** | **done（open = 0）** |

### 工程硬化要点（摘要）

- Witness 篡改可检；层校验统一 `can_send_to`  
- dispatch **真心跳** + health poller **真 degraded**  
- `metrics_enabled` 在 `start()` 消费  
- Guard 经 `GuardRegistry` / `intercept` 真拦截  
- DLQ 持久化 peek/ack/retry；熔断可写 EventStore 并恢复  

详见：`axiom-core/docs/ENGINEERING_HARDENING_v050.md`。
