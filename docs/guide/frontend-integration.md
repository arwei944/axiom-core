# Frontend integration guide (ULE commercial floor)

| Field | Value |
|-------|--------|
| **Audience** | Frontend apps, ops shells, gateway consumers |
| **Host** | `axiom-demo-taskflow` ProductGateway + optional `axiom-api` |
| **OpenAPI** | [`../openapi.yaml`](../openapi.yaml) |

## 1. Two HTTP surfaces (do not dual-truth)

| Surface | When to use | Role |
|---------|-------------|------|
| **ProductGateway** (`taskflow gateway` / demo) | Commercial floor, write path, SSE, ops shell | **Primary product adapter** for taskflow vertical |
| **axiom-api** | Broader runtime gateway skeleton (health/cells/entropy/heatmap) | Shared API crate; extend with Signal adapters — **not** a second business kernel |

**Rule:** Business orchestration stays in Cells. Both are **U7 adapters**. Prefer one origin per UI (proxy `/api` → ProductGateway for taskflow demos).

## 2. Read APIs

| Method | Path | Notes |
|--------|------|-------|
| GET | `/health`, `/api/v1/health` | Liveness; unauthenticated probe style |
| GET | `/api/v1/surface` | Unified truth: health, governor, runs, metrics, write_api, events_api |
| GET | `/metrics` | Prometheus text |
| GET | `/api/v1/metrics` | JSON counters |
| GET | `/api/v1/lens`, `/api/v1/lens/{id}` | Read-only projections |
| GET | `/api/v1/plugins` | Plugin list |
| GET | `/api/v1/alerts` | Governor-linked demo alerts |
| GET | `/ops` | Minimal ops HTML shell |

## 3. Write API (Signal path)

| Method | Path | Backend path |
|--------|------|----------------|
| **POST** | `/api/v1/tasks` | JSON body → `publish_command(SubmitTask)` → **TaskCell** → `product_admit` → Composer → Witness |

Request body (minimum):

```json
{ "title": "ship", "priority": 1, "payload": "work item" }
```

Response (success ~201):

```json
{
  "ok": true,
  "witness_count": 5,
  "governor_level": "Green",
  "admit_authority": "governor",
  "path": "POST /api/v1/tasks → Signal SubmitTask → TaskCell → Composer"
}
```

Governor reject → **403** with `ok: false` and error text (no dual admit).

## 4. Push (SSE)

| Method | Path | Content-Type |
|--------|------|----------------|
| GET | `/api/v1/events` | `text/event-stream` |

Event types:

| `event:` | Meaning |
|----------|---------|
| `stream.open` | Connection hello |
| `task.completed` | After POST /api/v1/tasks finishes |
| `governor.alert` | Governor reject / run governance failure |

Browser:

```js
const es = new EventSource('/api/v1/events');
es.addEventListener('task.completed', (e) => console.log(e.data));
```

## 5. Auth / CORS

- Dev floor may run open; production: put gateway behind `axiom-api` auth or reverse proxy (`AXIOM_API_KEY` / JWT — see `docs/COMMERCIAL_OPS.md`).
- ProductGateway sends `Access-Control-Allow-Origin: *` for demo; lock down in production.

## 6. Ops shell

- Served at `/ops` (static HTML, no bundler).
- Uses relative `fetch('/api/v1/surface')` and `POST /api/v1/tasks` — same origin as gateway.

## 7. Frontend must not

- Call DB/exchange/LLM secrets directly
- Treat UI validation as admit authority
- Bypass Surface and invent a second “source of truth” dashboard against private Cell state

## 8. Quick start

```powershell
cargo run -p axiom-demo-taskflow -- gateway --health-addr 127.0.0.1:19092
# browser: http://127.0.0.1:19092/ops
```
