# ULE Commercial Ops Floor

**Product**: ULE-on-Axiom (Unified Low-Entropy Kernel)  
**Version**: 0.5.0（产品面 U3–U5 + 工程硬化清单关闭）  
**Host**: Axiom (Rust) single kernel  

工程硬化与生产接线详见：[ENGINEERING_HARDENING_v050.md](./ENGINEERING_HARDENING_v050.md)。

## Auth & configuration boundary

| Variable / flag | Purpose | Default |
|-----------------|---------|---------|
| `AXIOM_ENVIRONMENT` | `development` / `test` / `production` | `development` |
| `AXIOM_API_KEY` | API key when auth mode is `api_key` | unset |
| `AXIOM_AUTH_MODE` | `disabled` / `api_key` / `jwt` / `oauth2` (via `AppConfig`) | `disabled` (dev) |
| `AXIOM_API_PORT` | API listen port (Docker / process) | `9092` |
| `RUST_LOG` | tracing filter | `info` |

**Production expectation**: enable `api_key` or `jwt` on the API gateway (`axiom-api` / `ApiServerBuilder::with_api_key`).  
**Liveness exception**: the taskflow health probe (`GET /health`, `GET /api/v1/health` on the demo health surface) is unauthenticated on purpose (k8s-style liveness). Protect management APIs separately with `AXIOM_API_KEY`.

Code anchors:

- `crates/axiom-api/src/auth.rs` — `AuthMode`, `AuthConfig`, `AuthService`
- `crates/axiom-api/src/config.rs` — `AppConfig`, `AXIOM_*` env loading
- `crates/axiom-demo-taskflow/src/health.rs` — commercial taskflow health surface

## Health check & unified surface (U4)

### Taskflow commercial binary

```powershell
cargo run -p axiom-demo-taskflow -- health --health-addr 127.0.0.1:19092
cargo run -p axiom-demo-taskflow -- surface --health-addr 127.0.0.1:19092
```

- `/health` — liveness (`"status":"ok"`, `"history":"witness-only"`, `"admit":"governor"`)
- `/api/v1/surface` — **sole observation surface**: health + governor decision + cells + recent_runs  
  Admit authority is always **Governor** via `axiom_isa::product_decide` / `product_admit` (no dual Guardian product API).

### Runtime 内建健康（AxiomRuntime）

`AxiomRuntime::start()` 后：

| 字段 | 含义 |
|------|------|
| `last_heartbeat_ms` | **dispatch 循环每 tick 写入**（生产路径，非测试 helper） |
| `degraded` | health poller 发现心跳超过 `heartbeat_stale_ms` 后为 `true` |
| `metrics_active` | 启动时若 `metrics_enabled=true` 则为 `true` |
| `metrics_endpoint` | 启用 metrics 且未配置时默认 `internal://metrics` |

相关 `RuntimeConfig` 字段：`dispatch_poll_interval_ms`、`heartbeat_stale_ms`、`metrics_enabled`、`backoff_*`。  
代码：`crates/axiom-runtime/src/dispatch/loop.rs`、`runtime/start.rs`。

### Agent handoff (U3)

```powershell
cargo run -p axiom-demo-taskflow -- handoff
cargo run -p axiom-demo-taskflow -- handoff-reject
```

### Full API gateway (existing)

```text
GET /api/v1/health
```

See `crates/axiom-api` and Docker `HEALTHCHECK` in root `Dockerfile`.

## Deploy recipe (shipping path)

### Binary

```powershell
cd C:\work\architecture\axiom-core
cargo build -p axiom-demo-taskflow --release
.\target\release\taskflow.exe success
.\target\release\taskflow.exe health
```

### Docker (API path)

Root `Dockerfile` builds `axiom-api` with:

```dockerfile
HEALTHCHECK CMD curl -f http://localhost:9092/api/v1/health || exit 1
ENV AXIOM_API_PORT=9092
```

### Docker (ULE taskflow path)

```powershell
docker build -f Dockerfile.taskflow -t ule-taskflow .
docker run --rm ule-taskflow success
docker run --rm -p 19092:19092 ule-taskflow health --health-addr 0.0.0.0:19092
```

### Compose / k8s

`docker-compose.yml` runs **axiom-api** (port 9092) and **ule-taskflow** health (port 19092).  
Point liveness at `/api/v1/health` (API) or `/health` (taskflow).  
Do **not** introduce a second long-lived LE Go runtime.

## Single-kernel ops rules

1. **One history**: Witness only (no ExecutionStep authority).  
2. **One admit**: Governor (no dual Guardian/Oversight decision long-term).  
3. **One host**: AxiomRuntime + ISA (four primitives).  
4. **SDK ≠ second kernel**: any future Go/TS SDK must call this kernel’s API only.
