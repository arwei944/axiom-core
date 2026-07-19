# 验收收据 — ULE 0.5.0 + 工程硬化

**版本**: 0.5.0-commercial  
**日期**: 2026-07-19  
**结果**: **通过**

## 产品面（U3–U5）

| 闸门 | 观察 |
|------|------|
| U3 测试 / CLI | handoff + handoff-reject；Witness 链完整 |
| U4 surface | `/api/v1/surface`：admit_authority=governor、recent_runs |
| U5 叙事 | LE `ARCHIVED.md`；单核 Axiom；禁止双 runtime 稳态 |
| 回归 | isa / resilience / demo-taskflow 绿；success×2 |

## 工程面（TASK_CHECKLIST）

| 闸门 | 观察 |
|------|------|
| 清单 | `docs/TASK_CHECKLIST.md` **open = 0**（91 项已勾） |
| 生产接线 | dispatch 写 `last_heartbeat_ms`；poller 写 `degraded`；`metrics_enabled` 在 start 消费；Guard `check_all` |
| 路径测试 | heartbeat 无 helper、degraded 真路径、metrics 分叉、registered guard Reject |
| 说明文档 | `docs/ENGINEERING_HARDENING_v050.md` |

## 命令

```powershell
cd C:\work\architecture\axiom-core
cargo test -p axiom-kernel -p axiom-runtime -p axiom-store --lib
cargo test -p axiom-isa -p axiom-resilience -p axiom-demo-taskflow
cargo run -p axiom-demo-taskflow -- success
cargo run -p axiom-demo-taskflow -- handoff
cargo run -p axiom-demo-taskflow -- surface
```
