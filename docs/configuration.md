# Axiom Core 配置参考

## 概述

Axiom Core 支持通过环境变量、`.env` 文件和 TOML 配置文件进行配置。所有配置项均使用 `AXIOM_` 前缀。

## 配置加载顺序

1. **`.env` 文件** — 自动加载（如果存在），通过 `dotenvy` 库
2. **TOML 配置文件** — 通过 `AXIOM_CONFIG_FILE` 环境变量指定路径
3. **环境变量** — 覆盖文件配置中的值

```
AppConfig::load()
  ├── dotenvy::dotenv()                    // 加载 .env 文件
  ├── 检查 AXIOM_CONFIG_FILE 环境变量
  │     ├── 存在 → 从 TOML 文件加载
  │     └── 不存在 → 从环境变量加载
  └── validate()                           // 验证配置
```

## 环境变量

### 基础配置

| 环境变量 | 默认值 | 说明 |
|---------|--------|------|
| `AXIOM_ENVIRONMENT` | `development` | 运行环境：`development`/`test`/`production` |
| `AXIOM_API_ADDR` | `0.0.0.0:9092` | API 服务器监听地址 |
| `AXIOM_BODY_LIMIT` | `10485760` (10MB) | 请求体大小限制（字节） |

### 速率限制

| 环境变量 | 默认值 | 说明 |
|---------|--------|------|
| `AXIOM_RATE_LIMIT_MAX_REQUESTS` | `100` | 每时间窗口最大请求数 |
| `AXIOM_RATE_LIMIT_WINDOW_SECS` | `60` | 时间窗口大小（秒） |

### CORS

| 环境变量 | 默认值 | 说明 |
|---------|--------|------|
| `AXIOM_CORS_ALLOWED_ORIGINS` | `*` | 允许的源，逗号分隔 |

### 日志与追踪

| 环境变量 | 默认值 | 说明 |
|---------|--------|------|
| `AXIOM_LOG_LEVEL` | `info` | 日志级别 |
| `AXIOM_LOG_FORMAT` | `json` | 日志格式：`json`/`text` |
| `AXIOM_TRACING_ENABLED` | `false` | 是否启用 OpenTelemetry 分布式追踪 |
| `AXIOM_OTLP_ENDPOINT` | `http://localhost:4318/v1/traces` | OTLP 追踪数据端点 |
| `AXIOM_LOG_FILE` | (无) | 日志文件路径 |
| `AXIOM_LOG_ROTATION_SIZE_MB` | `10` | 日志轮转大小（MB） |
| `AXIOM_LOG_MAX_FILES` | `5` | 保留的日志文件数量 |

### 认证

| 环境变量 | 默认值 | 说明 |
|---------|--------|------|
| `AXIOM_AUTH_MODE` | `disabled` | 认证模式：`disabled`/`api_key`/`jwt` |
| `AXIOM_JWT_SECRET` | (无) | JWT 签名密钥（jwt 模式必需） |
| `AXIOM_API_KEY` | (无) | API Key（api_key 模式必需） |

### 数据库与备份

| 环境变量 | 默认值 | 说明 |
|---------|--------|------|
| `AXIOM_DB_PATH` | `./data/axiom.db` | SQLite 数据库路径 |
| `AXIOM_BACKUP_DIR` | `./backups` | 备份目录 |
| `AXIOM_BACKUP_INTERVAL_MINUTES` | `60` | 备份间隔（分钟） |
| `AXIOM_MAX_BACKUPS` | `10` | 最大保留备份数 |

## 环境预设

### Development

```bash
AXIOM_ENVIRONMENT=development
```

- 速率限制：1000 请求/分钟
- CORS：允许所有源
- 日志级别：debug
- 认证：禁用

### Test

```bash
AXIOM_ENVIRONMENT=test
```

- 速率限制：10000 请求/分钟
- CORS：允许所有源
- 日志级别：warn
- 认证：禁用

### Production

```bash
AXIOM_ENVIRONMENT=production
AXIOM_JWT_SECRET=your-secret-key
AXIOM_CORS_ALLOWED_ORIGINS=https://your-app.com
```

- 速率限制：100 请求/分钟
- CORS：需指定允许的源
- 日志级别：info
- 认证：必需（JWT 或 API Key）

## .env 文件示例

在项目根目录创建 `.env` 文件：

```env
# 基础
AXIOM_ENVIRONMENT=development
AXIOM_API_ADDR=0.0.0.0:9092

# 日志
AXIOM_LOG_LEVEL=debug
AXIOM_LOG_FORMAT=json
AXIOM_TRACING_ENABLED=true
AXIOM_OTLP_ENDPOINT=http://localhost:4318/v1/traces

# 认证
AXIOM_AUTH_MODE=api_key
AXIOM_API_KEY=your-api-key-here

# 数据库
AXIOM_DB_PATH=./data/axiom.db
AXIOM_BACKUP_DIR=./backups
AXIOM_BACKUP_INTERVAL_MINUTES=30
AXIOM_MAX_BACKUPS=20
```

## TOML 配置文件示例

通过 `AXIOM_CONFIG_FILE=config.toml` 指定配置文件：

```toml
environment = "production"
api_addr = "0.0.0.0:9092"
body_limit = 10485760

[rate_limit]
max_requests = 100
window_secs = 60

[cors]
allowed_origins = ["https://app.example.com"]

[logging]
level = "info"
format = "json"
tracing_enabled = true
otlp_endpoint = "http://jaeger:4318/v1/traces"

[auth]
mode = "jwt"
jwt_secret = "your-secret-key"

[database]
path = "/var/lib/axiom/axiom.db"

[backup]
dir = "/var/lib/axiom/backups"
interval_minutes = 60
max_backups = 10
```

## 配置验证

启动时自动调用 `AppConfig::validate()` 检查：

- `auth_mode = "jwt"` 时必须有 `jwt_secret`
- `auth_mode = "api_key"` 时必须有 `api_key`
- 数据库和备份目录的父目录存在或可创建
- `rate_limit_max_requests > 0`
- `body_limit > 0`

验证失败将返回 `ConfigError`，阻止服务启动。

## 代码中使用

```rust
use axiom_api::config::AppConfig;

// 加载配置（自动加载 .env 文件）
let config = AppConfig::load().expect("failed to load config");

// 验证配置
config.validate().expect("invalid config");

// 使用预设
let dev_config = AppConfig::development();
let prod_config = AppConfig::production();
```
