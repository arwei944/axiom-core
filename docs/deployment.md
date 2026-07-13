# Axiom Core 部署指南

## 概述

本指南详细说明如何部署 Axiom Core 运行时环境，包括开发、测试和生产环境的配置。

## 1. 环境准备

### 1.1 系统要求

| 指标 | 最低配置 | 推荐配置 |
|------|---------|---------|
| CPU | 2 核 | 4 核+ |
| 内存 | 2GB | 8GB+ |
| 磁盘 | 10GB SSD | 50GB NVMe |
| Rust 版本 | 1.85+ | 1.85+ |

### 1.2 依赖安装

```bash
# Ubuntu/Debian
sudo apt-get update && sudo apt-get install -y build-essential libssl-dev pkg-config

# macOS (Homebrew)
brew install openssl pkg-config
```

## 2. 编译与构建

### 2.1 开发构建

```bash
# 克隆仓库
git clone https://github.com/arwei944/axiom-kernel.git
cd axiom-kernel

# 编译所有 crate
cargo build --workspace

# 运行测试
cargo test --workspace
```

### 2.2 生产构建

```bash
# 优化编译
cargo build --workspace --release

# 查看编译产物
ls target/release/
```

## 3. 配置文件

### 3.1 主配置文件 (config.toml)

```toml
[runtime]
mailbox_capacity = 2048
entropy_threshold = 100.0
entropy_cooldown_ms = 60000
dispatch_poll_interval_ms = 10

[api]
addr = "0.0.0.0:9092"
swagger_enabled = true

[logging]
level = "info"
format = "json"
log_file = "/var/log/axiom/runtime.log"
rotation_size_mb = 10
max_log_files = 5

[metrics]
enabled = true

[telemetry]
enabled = false
otlp_endpoint = "http://localhost:4318/v1/traces"

[store]
backend = "sqlite"
database_url = "/data/axiom_events.db"

[backup]
enabled = true
backup_dir = "/data/backups"
backup_interval_minutes = 60
max_backups = 24
compress = true
```

### 3.2 环境变量

| 变量 | 描述 | 默认值 |
|------|------|--------|
| `AXIOM_LOG_LEVEL` | 日志级别 | info |
| `AXIOM_LOG_FORMAT` | 日志格式 (json/text) | json |
| `AXIOM_API_ADDR` | API 监听地址 | 0.0.0.0:9092 |
| `AXIOM_DB_PATH` | 数据库路径 | ./axiom_events.db |
| `AXIOM_TELEMETRY_ENABLED` | 是否启用遥测 | false |
| `AXIOM_OTLP_ENDPOINT` | OTLP 端点 | http://localhost:4318/v1/traces |

## 4. 运行方式

### 4.1 直接运行

```bash
# 运行 Runtime
cargo run -p axiom-runtime -- --config config.toml

# 运行 API
cargo run -p axiom-api

# 运行 CLI
cargo run -p axiom-cli -- help
```

### 4.2 二进制运行

```bash
# 使用 release 二进制
./target/release/axiom-runtime --config config.toml
./target/release/axiom-api
```

## 5. Docker 部署

### 5.1 Dockerfile

```dockerfile
FROM rust:1.85-slim AS builder

WORKDIR /app
COPY . .

RUN cargo build --release --package axiom-runtime --package axiom-api

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/axiom-runtime .
COPY --from=builder /app/target/release/axiom-api .

EXPOSE 9092

CMD ["./axiom-runtime"]
```

### 5.2 Docker Compose

```yaml
version: "3.8"

services:
  axiom-runtime:
    image: axiom-core:latest
    ports:
      - "9092:9092"
    volumes:
      - ./data:/data
      - ./logs:/var/log/axiom
      - ./config.toml:/etc/axiom/config.toml
    environment:
      - AXIOM_CONFIG_PATH=/etc/axiom/config.toml
    restart: unless-stopped

  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "16686:16686"
      - "4318:4318"
    restart: unless-stopped

  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    ports:
      - "9090:9090"
    restart: unless-stopped
```

## 6. 监控与可观测性

### 6.1 Prometheus 配置

```yaml
scrape_configs:
  - job_name: 'axiom-core'
    scrape_interval: 15s
    static_configs:
      - targets: ['axiom-runtime:9092']
```

### 6.2 日志轮转配置

日志轮转功能已内置，默认配置：
- 文件大小达到 10MB 时轮转
- 保留最近 5 个日志文件
- 自动清理旧日志

### 6.3 OpenTelemetry 配置

启用分布式追踪：

```toml
[telemetry]
enabled = true
otlp_endpoint = "http://jaeger:4318/v1/traces"
service_name = "axiom-core"
```

## 7. API 文档

### 7.1 Swagger UI

启动后访问 Swagger UI:

```
http://localhost:9092/swagger-ui
```

### 7.2 OpenAPI 规范

访问 OpenAPI 规范:

```
http://localhost:9092/openapi.yaml
```

### 7.3 API 端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v1/health` | GET | 健康检查 |
| `/api/v1/cells` | GET | 获取所有 Cell |
| `/api/v1/heatmap` | GET | 获取活动热力图 |
| `/api/v1/entropy` | GET | 获取熵值状态 |
| `/api/v1/metrics` | GET | 获取 Prometheus 指标 |

## 8. 备份与恢复

### 8.1 自动备份

系统会自动创建定时备份，默认配置：
- 每 60 分钟创建一次备份
- 保留最近 24 个备份
- 备份目录: `/data/backups`

### 8.2 手动备份

```bash
# 创建手动备份
cargo run -p axiom-cli -- backup create

# 列出所有备份
cargo run -p axiom-cli -- backup list

# 从备份恢复
cargo run -p axiom-cli -- backup restore <backup-path>
```

## 9. 安全配置

### 9.1 TLS 配置

```toml
[tls]
enabled = true
cert_file = "/etc/ssl/certs/axiom.crt"
key_file = "/etc/ssl/private/axiom.key"
```

### 9.2 JWT 认证

```toml
[jwt]
secret = "your-secret-key"
algorithm = "HS256"
expiration_minutes = 1440
```

## 10. 故障排查

### 10.1 查看日志

```bash
# 实时查看日志
tail -f /var/log/axiom/runtime.log

# 查看错误
grep ERROR /var/log/axiom/runtime.log

# 查看警告
grep WARN /var/log/axiom/runtime.log
```

### 10.2 健康检查

```bash
# API 健康检查
curl http://localhost:9092/api/v1/health

# 指标检查
curl http://localhost:9092/api/v1/metrics
```

### 10.3 常见问题

| 问题 | 原因 | 解决方案 |
|------|------|---------|
| 无法连接数据库 | 文件权限问题 | 检查数据库目录权限 |
| 熵值过高 | Cell 处理频繁失败 | 检查 Cell 逻辑 |
| API 无响应 | 端口占用 | 检查端口是否被占用 |
| 备份失败 | 磁盘空间不足 | 清理磁盘空间 |

## 11. 性能调优

### 11.1 配置优化

```toml
[runtime]
mailbox_capacity = 4096
dispatch_poll_interval_ms = 5

[store]
max_connections = 20
```

### 11.2 内存管理

- 调整 `mailbox_capacity` 控制内存使用
- 定期清理过期数据
- 监控内存使用趋势

## 附录

### A. 配置文件位置

```
/etc/axiom/config.toml          # 主配置
/data/axiom_events.db           # SQLite 数据库
/data/backups/                  # 备份目录
/var/log/axiom/                 # 日志目录
```

### B. 服务管理

```bash
# systemd 服务文件示例
# /etc/systemd/system/axiom-runtime.service

[Unit]
Description=Axiom Core Runtime
After=network.target

[Service]
User=axiom
Group=axiom
WorkingDirectory=/opt/axiom
ExecStart=/opt/axiom/axiom-runtime --config /etc/axiom/config.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
# 启用并启动服务
sudo systemctl daemon-reload
sudo systemctl enable axiom-runtime
sudo systemctl start axiom-runtime

# 查看状态
sudo systemctl status axiom-runtime

# 查看日志
sudo journalctl -u axiom-runtime -f
```
