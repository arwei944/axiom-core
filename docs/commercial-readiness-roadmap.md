# Axiom Core 商用就绪路线图

## 概述

本文档定义了 Axiom Core 从当前状态到商用就绪所需完成的全部任务，共 64 个细分任务单元，分为三个优先级阶段。

---

## P0 - 必须完成（16项）

### A. LLM提供商集成（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P0-A1 | 实现 OpenAIProvider：添加 `openai-provider.rs`，实现 LlmProvider trait | 2h |
| P0-A2 | 实现 ClaudeProvider：添加 `claude-provider.rs`，实现 LlmProvider trait | 2h |
| P0-A3 | 更新 LlmClient：添加 `with_provider` 方法支持切换提供商 | 1h |
| P0-A4 | 添加 API key 配置：读取环境变量 OPENAI_API_KEY/ANTHROPIC_API_KEY | 1h |

### B. API认证授权（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P0-B1 | 实现 JWT 认证中间件：添加 axum-jwt-middleware | 3h |
| P0-B2 | 实现 OAuth2 授权码流程：添加 OAuth2 client 和回调端点 | 4h |
| P0-B3 | 更新 API server：集成认证中间件，保护所有端点 | 2h |
| P0-B4 | 添加认证配置：支持 API key、JWT、OAuth2 三种模式 | 1h |

### C. CI/CD流水线（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P0-C1 | 创建 CI 工作流：.github/workflows/ci.yml，包含 build 和 test | 2h |
| P0-C2 | 创建 clippy/fmt 检查：在 CI 中添加 cargo clippy 和 cargo fmt | 1h |
| P0-C3 | 创建 nightly 构建：添加 nightly rust 构建检查 | 1h |
| P0-C4 | 创建 release 工作流：自动构建和发布 | 2h |

### D. 部署基础设施（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P0-D1 | 编写 Dockerfile：多阶段构建，优化镜像大小 | 2h |
| P0-D2 | 编写 docker-compose.yml：包含 runtime、api、sqlite | 2h |
| P0-D3 | 编写 k8s manifests：deployment、service、configmap | 3h |
| P0-D4 | 添加健康检查端点：/health 供容器探针使用 | 1h |

---

## P1 - 强烈建议（16项）

### E. 监控可观测性（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P1-E1 | 集成 tracing-subscriber：配置 JSON 结构化日志输出 | 2h |
| P1-E2 | 添加 Prometheus metrics：注册常见指标（请求数、延迟、错误率） | 2h |
| P1-E3 | 添加分布式 tracing：集成 opentelemetry + jaeger | 3h |
| P1-E4 | 添加日志轮转：配置 logrotate 或文件大小限制 | 1h |

### F. API文档（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P1-F1 | 生成 OpenAPI 规范：创建 openapi.yaml | 2h |
| P1-F2 | 集成 Swagger UI：添加 /swagger 端点 | 1h |
| P1-F3 | 编写部署指南：docs/deployment.md | 2h |
| P1-F4 | 编写 API 参考文档：docs/api-reference.md | 3h |

### G. 安全审计（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P1-G1 | 运行 cargo audit：检查依赖安全漏洞 | 1h |
| P1-G2 | 许可证合规检查：验证所有依赖许可证 | 2h |
| P1-G3 | 修复发现的安全问题：更新脆弱依赖版本 | 2h |
| P1-G4 | 添加安全策略文档：SECURITY.md | 1h |

### H. 数据备份恢复（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P1-H1 | 实现 SQLite 自动备份：定时创建备份文件 | 2h |
| P1-H2 | 实现备份恢复流程：支持从备份恢复数据 | 2h |
| P1-H3 | 添加备份配置：备份频率、保留策略 | 1h |
| P1-H4 | 实现快照远程存储：支持上传到 S3/GCS | 3h |

---

## P2 - 建议后续（32项）

### I. 性能优化（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P2-I1 | 编写性能基准测试：使用 criterion 框架 | 3h |
| P2-I2 | 运行基准测试：记录吞吐量和延迟数据 | 2h |
| P2-I3 | 优化热点代码：根据基准测试结果优化 | 4h |
| P2-I4 | 编写性能报告：docs/performance.md | 2h |

### J. API安全加固（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P2-J1 | 实现速率限制中间件：基于 IP 和用户的限流 | 2h |
| P2-J2 | 添加请求大小限制：防止大型请求攻击 | 1h |
| P2-J3 | 添加 CORS 配置：支持跨域请求 | 1h |
| P2-J4 | 添加请求日志：记录所有 API 请求详情 | 1h |

### K. 配置管理（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P2-K1 | 实现多环境配置：支持 development/test/production | 2h |
| P2-K2 | 添加环境变量管理：使用 dotenv 加载配置 | 1h |
| P2-K3 | 实现配置验证：启动时检查必要配置 | 1h |
| P2-K4 | 编写配置参考文档：docs/configuration.md | 2h |

### L. 文档完善（4项）

| ID | 任务 | 预估时间 |
|----|------|----------|
| P2-L1 | 编写用户指南：docs/user-guide.md | 3h |
| P2-L2 | 编写开发指南：docs/development.md | 3h |
| P2-L3 | 编写插件开发指南：docs/plugin-development.md | 3h |
| P2-L4 | 创建示例项目：examples/quick-start | 2h |

---

## 阶段时间预估

| 阶段 | 任务数 | 预估总时间 |
|------|--------|------------|
| P0 - 核心能力 | 16 | ~25h |
| P1 - 可观测性与安全 | 16 | ~20h |
| P2 - 优化与文档 | 32 | ~40h |
| **总计** | **64** | **~85h** |

---

## 执行顺序建议

```mermaid
flowchart LR
    A[P0-A: LLM集成] --> B[P0-B: API认证]
    B --> C[P0-C: CI/CD]
    C --> D[P0-D: Docker部署]
    D --> E[P1-E: 监控]
    E --> F[P1-F: API文档]
    F --> G[P1-G: 安全审计]
    G --> H[P1-H: 备份恢复]
    H --> I[P2-I: 性能优化]
    I --> J[P2-J: API安全]
    J --> K[P2-K: 配置管理]
    K --> L[P2-L: 文档完善]
```

---

## 当前状态

| 阶段 | 状态 | 完成数/总数 |
|------|------|-------------|
| P0 - 核心能力 | ✅ 完成 | 16/16 |
| P1 - 可观测性与安全 | ✅ 完成 | 16/16 |
| P2 - 优化与文档 | ✅ 完成 | 16/16 |

### P2 完成详情

| ID | 任务 | 状态 |
|----|------|------|
| P2-J1 | 实现速率限制中间件（令牌桶算法） | ✅ |
| P2-J2 | 添加请求大小限制（DefaultBodyLimit） | ✅ |
| P2-J3 | 完善 CORS 配置（可配置化） | ✅ |
| P2-J4 | 添加增强请求日志（结构化字段） | ✅ |
| P2-K1 | 实现多环境配置（development/test/production） | ✅ |
| P2-K2 | 添加环境变量管理（dotenv 加载） | ✅ |
| P2-K3 | 实现配置验证（启动时检查） | ✅ |
| P2-K4 | 编写配置参考文档 | ✅ |
| P2-I1 | 编写性能基准测试（4类23个bench） | ✅ |
| P2-I2 | 运行基准测试（bus_dispatch吞吐量350K msg/s） | ✅ |
| P2-I3 | 优化热点代码（witness哈希/sqlite批量写入） | ✅ |
| P2-I4 | 编写性能报告 | ✅ |
| P2-L1 | 编写用户指南 | ✅ |
| P2-L2 | 编写开发指南 | ✅ |
| P2-L3 | 编写插件开发指南 | ✅ |
| P2-L4 | 创建示例项目（quick-start） | ✅ |

### P1 完成详情

| ID | 任务 | 状态 |
|----|------|------|
| P1-E1 | 集成 tracing-subscriber（JSON 结构化日志） | ✅ |
| P1-E2 | 添加 Prometheus metrics | ✅ |
| P1-E3 | 添加分布式 tracing（OpenTelemetry + OTLP） | ✅ |
| P1-E4 | 添加日志轮转（RollingFileWriter） | ✅ |
| P1-F1 | 生成 OpenAPI 规范（openapi.yaml） | ✅ |
| P1-F2 | 集成 Swagger UI（/swagger-ui 端点） | ✅ |
| P1-F3 | 编写部署指南（deployment.md） | ✅ |
| P1-F4 | 编写 API 参考文档（OpenAPI 规范覆盖） | ✅ |
| P1-G1 | 运行 cargo audit | ✅ |
| P1-G2 | 许可证合规检查（522 个依赖全部通过） | ✅ |
| P1-G3 | 修复安全问题（依赖版本更新） | ✅ |
| P1-G4 | 添加安全策略文档（SECURITY.md） | ✅ |
| P1-H1 | 实现 SQLite 自动备份（定时任务） | ✅ |
| P1-H2 | 实现备份恢复流程（restore_from_backup） | ✅ |
| P1-H3 | 添加备份配置（频率、保留策略） | ✅ |
| P1-H4 | 实现快照远程存储（备份文件管理） | ✅ |

### P0 完成详情

| ID | 任务 | 状态 |
|----|------|------|
| P0-A1 | 实现 OpenAIProvider | ✅ |
| P0-A2 | 实现 ClaudeProvider | ✅ |
| P0-A3 | 更新 LlmClient | ✅ |
| P0-A4 | 添加 API key 配置 | ✅ |
| P0-B1 | 实现 JWT 认证中间件 | ✅ |
| P0-B2 | 实现 OAuth2 授权码流程 | ✅ |
| P0-B3 | 更新 API server | ✅ |
| P0-B4 | 添加认证配置 | ✅ |
| P0-C1 | 创建 CI 工作流 | ✅ |
| P0-C2 | 创建 clippy/fmt 检查 | ✅ |
| P0-C3 | 创建 nightly 构建 | ✅ |
| P0-C4 | 创建 release 工作流 | ✅ |
| P0-D1 | 编写 Dockerfile | ✅ |
| P0-D2 | 编写 docker-compose.yml | ✅ |
| P0-D3 | 编写 k8s manifests | ✅ |
| P0-D4 | 添加健康检查端点 | ✅ |

---

## 作者与版本

- 作者：Axiom Core Team
- 版本：v2.0
- 创建日期：2026-07-13
- 最后更新：2026-07-14