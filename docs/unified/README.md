# ULE · 决策与商用交付包（U0–U5）

| 文档 | 作用 |
|------|------|
| [UNIFIED_MODEL.md](./UNIFIED_MODEL.md) | 宪法 |
| [DEPRECATION.md](./DEPRECATION.md) | 灭双清单 |
| [MVP_ACCEPTANCE.md](./MVP_ACCEPTANCE.md) | MVP |
| [COMMERCIAL_DELIVERY.md](./COMMERCIAL_DELIVERY.md) | **商用交付 v0.5.0（U3–U5 + 工程清单）** |
| [FEATURE_THEME_MATRIX.md](./FEATURE_THEME_MATRIX.md) | **主题 T1–T15 完全满足矩阵** |
| [VERIFICATION_RECEIPT.md](./VERIFICATION_RECEIPT.md) | 验收收据 |
| [`../guide/agent-work-guide.md`](../guide/agent-work-guide.md) | **智能体工作指导与约束（强制）** |
| [`../guide/frontend-integration.md`](../guide/frontend-integration.md) | 前端对接 |
| [`../guide/secrets-and-llm.md`](../guide/secrets-and-llm.md) | 密钥 / LLM env |
| [DUAL_GOVERNOR_NOTE.md](./DUAL_GOVERNOR_NOTE.md) | 产品 admit 唯一 |
| [`../openapi.yaml`](../openapi.yaml) | OpenAPI 0.5.0 |
| `../../low-entropy-core/ARCHIVED.md`（若并列检出） | **LE 只读归档声明** |
| [`../COMMERCIAL_OPS.md`](../COMMERCIAL_OPS.md) | 运维 / 健康 / 部署 |
| [`../ENGINEERING_HARDENING_v050.md`](../ENGINEERING_HARDENING_v050.md) | **工程硬化与生产接线** |
| [`../TASK_CHECKLIST.md`](../TASK_CHECKLIST.md) | 升级清单（已全勾） |

## 冻结结论

- **唯一宿主**：Axiom / Rust  
- **唯一历史**：Witness  
- **唯一准入**：Governor (`product_decide` / `product_admit`)  
- **LE**：归档只读资产，**不是**对等运行时  
- **禁止稳态**：联邦桥 / 双 runtime  
- **主题 T1–T15**：**完全满足**（见 FEATURE_THEME_MATRIX）  

## 快速跑

```powershell
cd C:\work\architecture\axiom-core
cargo run -p axiom-demo-taskflow -- handoff
cargo run -p axiom-demo-taskflow -- surface
cargo run -p axiom-demo-taskflow -- plugin
cargo run -p axiom-demo-taskflow -- success
```
