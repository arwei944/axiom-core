# Axiom Core 生产部署指南

> **版本:** v0.5.0（对齐 ULE 商用交付 + 工程清单关闭）  
> **最后更新:** 2026-07-19  
> **补充文档:** [COMMERCIAL_OPS.md](./COMMERCIAL_OPS.md) · [ENGINEERING_HARDENING_v050.md](./ENGINEERING_HARDENING_v050.md)

---

## 0. 当前交付边界（摘要）

| 已具备 | 说明 |
|--------|------|
| 单核宿主 | AxiomRuntime（Rust） |
| 健康 / 降级 | dispatch 心跳 + poller `degraded` |
| 商用 CLI | `taskflow` success/handoff/surface/health |
| 工程清单 | `TASK_CHECKLIST.md` open = 0 |

| 后置 | 说明 |
|------|------|
| 多租户 / 计费 / 跨区 HA | 非当前范围 |
| 全 workspace 强制全绿 | 以 kernel/runtime/ULE 路径为准 |

---

## 1. 系统要求

### 1.1 硬件要求

| 组件 | 最低配置 | 推荐配置 |
|------|---------|---------|
| CPU | 2 核 | 4 核+ |
| 内存 | 2GB | 8GB+ |
| 磁盘 | 10GB SSD | 50GB NVMe |
| 网络 | 1Gbps | 10Gbps |

### 1.2 软件要求

| 软件 | 版本 | 用途 |
|------|------|------|
| Rust | 1.85+ | 编译运行 |
| SQLite | 3.40+ | 默认持久化后端 |
| Docker | 24.0+ | 容器化部署（可选） |
| Kubernetes | 1.28+ | 容器编排（可选） |

### 1.3 验证环境

```bash
# 验证 Rust 工具链
rustc --version  # 应 >= 1.85.0
cargo --version

# 验证项目编译
cargo check --workspace --all-features

# 验证测试通过
cargo test --workspace --all-features
```

---

## 2. 部署方式

### 2.1 二进制部署

```bash
# 编译 release 版本
cargo build --workspace --release

# 二进制位于 target/release/
ls target/release/ | grep axiom
```

### 2.2 Docker Compose 示例

```yaml
# docker-compose.yml
version: "3.8"

services:
  axiom-runtime:
    image: axiom-core:latest
    ports:
      - "8080:8080"   # 应用端口
      - "9090:9090"   # metrics 端口
    volumes:
      - axiom-data:/data
    environment:
      - AXIOM_DB_PATH=/data/events.db
      - AXIOM_METRICS_ADDR=0.0.0.0:9090
      - AXIOM_ENTROPY_THRESHOLD=100.0
    command: ["axiom-runtime", "--config", "/etc/axiom/config.toml"]
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  axiom-data:
```

### 2.3 Kubernetes Helm Chart 说明

```yaml
# values.yaml
replicaCount: 3

image:
  repository: axiom-core
  tag: v0.3.0

config:
  mailboxCapacity: 2048
  entropyThreshold: 100.0
  metricsEndpoint: "0.0.0.0:9090"

resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
  limits:
    memory: "2Gi"
    cpu: "2000m"

persistence:
  enabled: true
  size: 20Gi
  storageClass: "ssd"
```

```bash
# 安装
helm install axiom-core ./chart/axiom-core -f values.yaml

# 升级
helm upgrade axiom-core ./chart/axiom-core -f values.yaml
```

---

## 3. 配置说明

### 3.1 RuntimeConfig 参数

```rust
pub struct RuntimeConfig {
    pub mailbox_capacity: usize,       // 邮箱容量，默认 1024
    pub entropy_threshold: f64,        // 熵阈值，默认 100.0
    pub entropy_cooldown_ms: u64,      // 熵冷却时间，默认 60000ms
    pub dispatch_poll_interval_ms: u64,// 轮询间隔，默认 10ms
    pub metrics_endpoint: Option<String>, // metrics 地址
    pub telemetry_enabled: bool,       // 是否启用 OTel
}
```

### 3.2 熵阈值配置

| 环境 | 建议阈值 | 理由 |
|------|---------|------|
| 开发 | 200.0 | 宽松，便于调试 |
| 测试 | 150.0 | 中等， catching 问题 |
| 生产 | 100.0 | 严格，保障稳定 |

### 3.3 持久化配置

```toml
[store]
backend = "sqlite"
database_url = "sqlite:axiom_events.db"
max_connections = 10
migration_timeout_ms = 5000

[snapshot]
policy = "EveryN"
n = 1000
retention = "KeepLastN(10)"
```

### 3.4 监控配置

```toml
[metrics]
endpoint = "0.0.0.0:9090"

[telemetry]
otlp_endpoint = "http://jaeger:4317"
service_name = "axiom-runtime"
sample_ratio = 0.1
```

---

## 4. 扩缩容指南

### 4.1 Cell 水平扩展

- 无状态 Cell 可无限水平扩展
- 有状态 Cell 需考虑状态分片
- 建议单实例 Cell 数量 < 500

### 4.2 多实例部署

- 使用独立 EventStore 实例
- 使用负载均衡分发到不同 Runtime 实例
- 建议每实例 Cell 数量 < 200

---

## 5. 备份与恢复

### 5.1 SQLite 备份

```bash
# 在线备份（WAL 模式）
sqlite3 axiom_events.db "VACUUM INTO 'backup.db'"

# 定时备份脚本
0 2 * * * sqlite3 /data/axiom_events.db "VACUUM INTO '/backup/events_$(date +\%Y\%m\%d).db'"
```

### 5.2 Witness 链导出

```bash
# 导出 witness 链为 JSON
cargo run -p axiom-cli -- witness export --output witness-chain.json

# 验证链完整性
cargo run -p axiom-cli -- witness verify --input witness-chain.json
```

### 5.3 快照恢复流程

```bash
# 1. 停止 Runtime
cargo run -p axiom-cli -- runtime stop

# 2. 恢复快照
cp /backup/snapshot.latest /data/snapshot.latest

# 3. 重启 Runtime
cargo run -p axiom-cli -- runtime start

# 4. 验证
curl http://localhost:8080/health
```

---

## 6. 运维命令速查

| 操作 | 命令 |
|------|------|
| 启动 | `cargo run -p axiom-runtime --` |
| 停止 | `curl -X POST http://localhost:8080/shutdown` |
| 健康检查 | `curl http://localhost:8080/health` |
| 查看指标 | `curl http://localhost:9090/metrics` |
| 查看 Cell 状态 | `cargo run -p axiom-cli -- cell list` |
| 查看 Witness | `cargo run -p axiom-cli -- witness list` |
| 查看熵值 | `cargo run -p axiom-cli -- entropy` |

---

## 7. 安全配置

### 7.1 架构规则保护

- `.axiom/architecture.toml` 是唯一真相源
- 任何修改必须经过代码审查
- 禁止直接修改 `gate.rs` 硬编码常量

### 7.2 依赖审计

- 所有新依赖必须经过审计
- 定期审查 `[audited-deps]` 列表

### 7.3 TLS 配置

```toml
[tls]
cert_file = "/etc/axiom/tls/server.crt"
key_file = "/etc/axiom/tls/server.key"
```

---

## 8. 故障排查

### 8.1 常见告警

| 告警 | 原因 | 解决方案 |
|------|------|---------|
| 熵值过高 | Cell 处理失败频繁 | 检查 Cell 逻辑，调整阈值 |
| Cell 频繁重启 | Panic 或超时 | 查看日志，修复 root cause |
| DLQ 堆积 | 消息处理失败 | 增加消费者或修复处理逻辑 |
| Witness 链断裂 | 持久化失败 | 恢复备份或重建链 |

### 8.2 日志解读

```bash
# 查看 Runtime 日志
journalctl -u axiom-runtime -f

# 查看错误级别日志
grep "ERROR" /var/log/axiom/runtime.log

# 关键日志模式
grep "circuit_break" /var/log/axiom/runtime.log
grep "layer_violation" /var/log/axiom/runtime.log
grep "witness_chain" /var/log/axiom/runtime.log
```

### 8.3 性能调优

- 调整 `mailbox_capacity` 优化吞吐量
- 调整 `entropy_cooldown_ms` 优化恢复速度
- 使用 BincodeCodec 减少序列化开销

---

## 9. 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| v0.3.0 | 2026-07-04 | 完整生产部署指南 |
| v0.2.0 | 2025-12-01 | 初始版本 |
