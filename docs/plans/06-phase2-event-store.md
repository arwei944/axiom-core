# Phase 2: 事件存储 Implementation Plan

> **Goal:** 完善 axiom-store crate：实现 Event Sourcing 模式的完整事件存储，包括 Snapshot 快照、Replay 重放、auto_collect 迁移验证、持久化后端。验收标准：可以从 EventLog 重建任意 Cell 的状态，旧版本数据自动迁移，迁移链有 gap 时启动时 abort，Snapshot 可加速重放。

> **Baseline:** Event 结构体已定义；EventStore trait 已定义（append/read/read_all/read_after）；MemoryStore 已完整实现；StoreError 已有。缺少：Snapshot 机制、Replay 引擎、持久化存储、迁移验证集成、事件序列号。

---

## Global Constraints

- axiom-store 依赖 axiom-core，不依赖 axiom-runtime 或其他 workspace crate
- 事件写入后不可变（append-only）
- 所有 Event 必须可序列化（Serialize/Deserialize）
- EventStore 实现必须是线程安全的（Send + Sync）
- 读操作不阻塞写操作（最终一致即可，但对于单进程使用场景可以是强一致）
- 每个事件有全局唯一递增序列号
- cargo build/clippy/test 零警告

---

## Task 1: 完善 Event 结构体和事件序列号

**Files:**
- Modify: `crates/axiom-store/src/event.rs`

- [ ] **Step 1: 添加全局序列号 sequence_number**
  - 在 Event 结构体中添加 `pub sequence_number: u64`
  - sequence_number 在 append 时由 EventStore 分配，全局递增
  - 用于精确的重放点和快照定位

- [ ] **Step 2: 添加 event_type 字段**
  - `pub event_type: String` — 对应 Signal::signal_type()
  - 用于类型路由和反序列化

- [ ] **Step 3: 添加 schema_version 字段**
  - `pub schema_version: u32` — 写入时的 schema 版本
  - 用于重放时判断是否需要迁移

- [ ] **Step 4: 添加 metadata 字段**
  - `pub metadata: EventMetadata`
  - EventMetadata 包含：layer（写入层）、processing_time_ms（处理耗时）、was_replayed（是否重放事件）

- [ ] **Step 5: 实现 Event 的 builder 模式**
  - `EventBuilder::new(aggregate_id, event_type, payload) -> EventBuilder`
  - `.cell_id()`, `.correlation_id()`, `.vector_clock()`, `.layer()`, `.build()`
  - 自动设置 timestamp_ns 和 event_id (UUID v4)

- [ ] **Step 6: 测试**
  - 测试EventBuilder创建事件所有字段正确
  - 测试Event可序列化/反序列化往返

- [ ] **Step 7: Commit**
  - `feat(axiom-store): complete Event struct with sequence number, schema version, metadata, builder pattern`

---

## Task 2: 扩展 EventStore trait

**Files:**
- Modify: `crates/axiom-store/src/store.rs`

- [ ] **Step 1: 添加 read_after_sequence 方法**
  - `async fn read_after_sequence(&self, seq: u64) -> Result<Vec<Event>, StoreError>`
  - 读取序列号大于seq的所有事件（用于订阅）

- [ ] **Step 2: 添加 read_range 方法**
  - `async fn read_range(&self, aggregate_id: &str, from_seq: u64, to_seq: u64) -> Result<Vec<Event>, StoreError>`
  - 读取指定aggregate在序列号范围内的事件

- [ ] **Step 3: 添加 latest_sequence 方法**
  - `async fn latest_sequence(&self) -> Result<u64, StoreError>`
  - 返回当前最大序列号

- [ ] **Step 4: 添加 append_batch 方法**
  - `async fn append_batch(&self, events: Vec<Event>) -> Result<Vec<u64>, StoreError>`
  - 批量追加事件，原子性（要么全部写入，要么全部失败），返回分配的序列号列表

- [ ] **Step 5: 添加 subscribe 方法（可选，返回事件流）**
  - `fn subscribe(&self) -> impl Stream<Item = Event>`（此阶段用 broadcast channel 实现内存版）

- [ ] **Step 6: 更新 MemoryStore 实现所有新方法**
  - MemoryStore 使用 RwLock<Vec<Event>> + AtomicU64 sequence counter
  - append_batch 在写锁内分配序列号，保证原子性
  - read_after_sequence/read_range/latest_sequence 全部实现

- [ ] **Step 7: 测试**
  - 测试append和read往返
  - 测试batch append原子性
  - 测试序列号单调递增
  - 测试read_range边界情况
  - 测试latest_sequence

- [ ] **Step 8: Commit**
  - `feat(axiom-store): extended EventStore trait with batch append, range reads, sequence queries, MemoryStore updated`

---

## Task 3: 实现 Snapshot 机制

**Files:**
- Create: `crates/axiom-store/src/snapshot.rs`
- Modify: `crates/axiom-store/src/lib.rs`

- [ ] **Step 1: 定义 Snapshot 结构体**
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct Snapshot {
      pub aggregate_id: String,
      pub sequence_number: u64,
      pub state: serde_json::Value,
      pub schema_version: u32,
      pub created_at_ns: u64,
      pub cell_id: String,
      pub vector_clock: VectorClock,
  }
  ```

- [ ] **Step 2: 定义 SnapshotStore trait**
  ```rust
  pub trait SnapshotStore: Send + Sync {
      async fn save_snapshot(&self, snapshot: Snapshot) -> Result<(), StoreError>;
      async fn load_latest_snapshot(&self, aggregate_id: &str) -> Result<Option<Snapshot>, StoreError>;
      async fn load_snapshot_at(&self, aggregate_id: &str, seq: u64) -> Result<Option<Snapshot>, StoreError>;
  }
  ```

- [ ] **Step 3: 实现 MemorySnapshotStore**
  - 使用 RwLock<HashMap<String, Vec<Snapshot>>> 存储
  - save_snapshot 追加到列表，按sequence_number排序
  - load_latest_snapshot 返回序列号最大的
  - load_snapshot_at 返回不超过seq的最新快照

- [ ] **Step 4: 添加快照策略配置**
  ```rust
  pub struct SnapshotPolicy {
      pub every_n_events: u64,  // 每N个事件创建快照
      pub max_snapshots_per_aggregate: usize,  // 每个aggregate最多保留多少快照
  }
  impl Default for SnapshotPolicy {
      fn default() -> Self { Self { every_n_events: 100, max_snapshots_per_aggregate: 5 } }
  }
  ```

- [ ] **Step 5: 测试**
  - 测试save/load快照往返
  - 测试load_latest返回最新
  - 测试load_snapshot_at返回指定位置
  - 测试快照策略

- [ ] **Step 6: Commit**
  - `feat(axiom-store): Snapshot struct, SnapshotStore trait, MemorySnapshotStore with snapshot policy`

---

## Task 4: 实现 Replay 引擎

**Files:**
- Create: `crates/axiom-store/src/replay.rs`
- Modify: `crates/axiom-store/src/lib.rs`

- [ ] **Step 1: 定义 ReplayEngine**
  ```rust
  pub struct ReplayEngine {
      event_store: Arc<dyn EventStore>,
      snapshot_store: Arc<dyn SnapshotStore>,
      migrator: Option<Arc<SchemaMigrator>>,  // 来自axiom-core
  }
  ```

- [ ] **Step 2: 实现 replay_aggregate 方法**
  - `async fn replay_aggregate<S: ReplayableState>(&self, aggregate_id: &str) -> Result<ReplayResult<S>, StoreError>`
  - ReplayResult: { state: S, last_sequence: u64, vector_clock: VectorClock, events_replayed: u64, snapshot_used: bool }
  - 流程：
    1. 尝试从 SnapshotStore 加载最新快照
    2. 如果有快照，从快照序列号之后开始读取事件
    3. 如果没有快照，从sequence=0开始读取所有事件
    4. 逐个事件调用 S::apply_event()，如果事件schema_version < 当前版本，使用migrator迁移
    5. 返回最终状态和元数据

- [ ] **Step 3: 定义 ReplayableState trait**
  ```rust
  pub trait ReplayableState: Default + Send + Sync + 'static {
      fn apply_event(&mut self, event_type: &str, payload: &serde_json::Value) -> Result<(), StoreError>;
      fn current_schema_version() -> u32;
      fn to_snapshot(&self) -> serde_json::Value;
      fn from_snapshot(value: &serde_json::Value) -> Result<Self, StoreError>;
  }
  ```

- [ ] **Step 4: 实现 replay_to 方法（时间点重放）**
  - `async fn replay_to<S: ReplayableState>(&self, aggregate_id: &str, up_to_seq: u64) -> Result<ReplayResult<S>, StoreError>`
  - 重放到指定序列号（时间旅行调试）

- [ ] **Step 5: 实现 subscribe_to_events 实时订阅**
  - 返回 Stream<Item = Event>，用于实时更新投影
  - 当新事件被append时，订阅者收到通知

- [ ] **Step 6: 添加重放统计**
  - ReplayResult 中包含：events_replayed（从快照之后重放的事件数）、total_events_for_aggregate、replay_duration_ms

- [ ] **Step 7: 测试**
  - 测试从零重放（无快照）
  - 测试从快照重放（加速）
  - 测试重放时旧版本事件自动迁移（需要axiom-core的SchemaMigrator）
  - 测试时间点重放
  - 测试事件订阅

- [ ] **Step 8: Commit**
  - `feat(axiom-store): ReplayEngine with snapshot-based replay, time-travel debugging, event subscription, auto-migration`

---

## Task 5: 实现持久化存储后端（SQLite）

**Files:**
- Create: `crates/axiom-store/src/sqlite_store.rs`
- Modify: `crates/axiom-store/Cargo.toml`

- [ ] **Step 1: 添加 rusqlite 依赖**
  - `rusqlite = { version = "0.31", features = ["bundled"] }`
  - 添加 feature flag: `sqlite = ["rusqlite"]`

- [ ] **Step 2: 定义 SQLite 表结构**
  ```sql
  CREATE TABLE IF NOT EXISTS events (
      sequence_number INTEGER PRIMARY KEY AUTOINCREMENT,
      event_id TEXT NOT NULL UNIQUE,
      aggregate_id TEXT NOT NULL,
      cell_id TEXT NOT NULL,
      correlation_id TEXT,
      event_type TEXT NOT NULL,
      schema_version INTEGER NOT NULL,
      layer TEXT NOT NULL,
      vector_clock TEXT NOT NULL,
      payload TEXT NOT NULL,
      metadata TEXT NOT NULL,
      timestamp_ns INTEGER NOT NULL
  );
  CREATE INDEX idx_events_aggregate ON events(aggregate_id, sequence_number);
  CREATE INDEX idx_events_timestamp ON events(timestamp_ns);
  CREATE TABLE IF NOT EXISTS snapshots (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      aggregate_id TEXT NOT NULL,
      sequence_number INTEGER NOT NULL,
      state TEXT NOT NULL,
      schema_version INTEGER NOT NULL,
      cell_id TEXT NOT NULL,
      vector_clock TEXT NOT NULL,
      created_at_ns INTEGER NOT NULL
  );
  CREATE INDEX idx_snapshots_aggregate_seq ON snapshots(aggregate_id, sequence_number);
  ```

- [ ] **Step 3: 实现 SqliteStore**
  - 实现 EventStore trait
  - 实现 SnapshotStore trait
  - 使用 WAL 模式提高并发
  - 使用 prepared statements 防止SQL注入
  - 使用 transactions 保证 batch append 原子性
  - 互斥锁保护连接（rusqlite 连接不是Sync），或者每个线程一个连接

- [ ] **Step 4: 实现 SqliteStore::open(path)**
  - 打开或创建数据库文件
  - 执行 CREATE TABLE IF NOT EXISTS
  - 设置 WAL mode
  - 设置 synchronous=NORMAL（平衡安全和性能）

- [ ] **Step 5: 测试**
  - 测试SqliteStore所有EventStore方法
  - 测试SqliteStore所有SnapshotStore方法
  - 测试从空数据库重放
  - 测试重启后数据持久化
  - 测试批量写入原子性（写入中途panic不损坏数据）

- [ ] **Step 6: Commit**
  - `feat(axiom-store): SQLite persistent store with WAL mode, transactions, full EventStore+SnapshotStore impl`

---

## Task 6: 启动时迁移链验证

**Files:**
- Modify: `crates/axiom-store/src/replay.rs` 或新增 `crates/axiom-store/src/migration_check.rs`

- [ ] **Step 1: 实现 MigrationChainValidator**
  - 启动时扫描 EventStore 中所有存在的 event_type
  - 检查每个 event_type 的当前 schema_version
  - 检查 axiom-core 的 MIGRATION_REGISTRY 中是否有完整的迁移链
  - 如果某个event_type存在事件数据但缺少迁移链（gap），返回启动错误

- [ ] **Step 2: 启动验证函数**
  - `pub async fn validate_migration_chains_at_startup(store: &dyn EventStore, migrator: &SchemaMigrator) -> Result<StartupValidation, StoreError>`
  - StartupValidation 包含：
    - validated_types: Vec<String> — 已验证的事件类型
    - warnings: Vec<String> — 有风险但不阻断的情况（如无事件的类型注册了migration）
    - errors: Vec<String> — 阻断启动的错误（迁移链gap）
  - 如果errors非空，返回Err(StoreError::MigrationChainGap)

- [ ] **Step 3: 添加强制升级模式**
  - 提供 `--axiom-force-migrate` 启动参数（通过CLI传入）
  - 强制模式下，遇到不可迁移的旧数据时，将该aggregate标记为需要人工处理
  - 生产环境默认关闭强制模式

- [ ] **Step 4: 测试**
  - 测试完整迁移链通过验证
  - 测试gap被检测并返回错误
  - 测试空store通过验证
  - 测试新添加的事件类型（无历史数据）通过验证

- [ ] **Step 5: Commit**
  - `feat(axiom-store): startup migration chain validation, detects gaps before boot, prevents running with incompatible schemas`

---

## Task 7: Event 幂等去重

**Files:**
- Modify: `crates/axiom-store/src/store.rs`
- Modify: `crates/axiom-store/src/memory.rs`
- Modify: `crates/axiom-store/src/sqlite_store.rs`

- [ ] **Step 1: 在 append 中检查 event_id 唯一性**
  - 如果相同 event_id 已存在，返回 StoreError::DuplicateEvent
  - 使用唯一索引保证（SQLite层已有 UNIQUE 约束）

- [ ] **Step 2: 添加幂等写入方法**
  - `async fn append_idempotent(&self, event: Event) -> Result<AppendResult, StoreError>`
  - AppendResult: { sequence_number: u64, was_duplicate: bool }
  - 如果event_id已存在，返回已有序列号+was_duplicate=true

- [ ] **Step 3: 添加 correlation_id 索引**
  - 用于按correlation_id追踪事件流
  - SQLite中添加 `CREATE INDEX idx_events_correlation ON events(correlation_id)`

- [ ] **Step 4: 添加 read_by_correlation 方法**
  - `async fn read_by_correlation(&self, correlation_id: &str) -> Result<Vec<Event>, StoreError>`

- [ ] **Step 5: 测试**
  - 测试重复event_id幂等写入
  - 测试correlation_id查询
  - 测试DuplicateError正确返回

- [ ] **Step 6: Commit**
  - `feat(axiom-store): idempotent event append with deduplication, correlation_id tracking and indexing`

---

## Task 8: 存储健康检查和指标

**Files:**
- Create: `crates/axiom-store/src/metrics.rs`
- Modify: `crates/axiom-store/src/lib.rs`

- [ ] **Step 1: 定义 StoreHealth 结构体**
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct StoreHealth {
      pub total_events: u64,
      pub total_snapshots: u64,
      pub oldest_event_ns: Option<u64>,
      pub newest_event_ns: Option<u64>,
      pub store_size_bytes: Option<u64>,
      pub write_latency_p50_ms: f64,
      pub write_latency_p99_ms: f64,
      pub read_latency_p50_ms: f64,
      pub error_count: u64,
  }
  ```

- [ ] **Step 2: 在 MemoryStore 和 SqliteStore 中实现 health() 方法**
  - 维护写/读延迟的滚动统计（最近100次操作）
  - 追踪错误计数

- [ ] **Step 3: 添加 StoreMetrics 层**
  - 包装 EventStore 实现，自动记录指标
  - 使用 Decorator pattern：`struct MeteredStore<S> { inner: S, metrics: Arc<StoreMetrics> }`

- [ ] **Step 4: 测试**
  - 测试健康统计数据正确
  - 测试延迟统计p50/p99计算

- [ ] **Step 5: Commit**
  - `feat(axiom-store): store health checks, latency metrics (p50/p99), MeteredStore decorator`

---

## P2 阶段验收标准

| # | 验收项 | 验证方式 |
|---|--------|---------|
| 1 | cargo build -p axiom-store 零警告 | 命令行验证 |
| 2 | cargo test -p axiom-store 全部通过（≥25个测试） | 命令行验证 |
| 3 | EventStore trait完整（append/batch/range/subscribe） | API审查 |
| 4 | MemoryStore完整实现所有方法 | 单元测试 |
| 5 | SqliteStore持久化实现（feature-gated） | 集成测试 |
| 6 | Snapshot机制可保存/加载/加速重放 | 单元测试 |
| 7 | ReplayEngine从零重放和从快照重放 | 单元测试 |
| 8 | 旧版本事件重放时自动迁移 | 单元测试 |
| 9 | 启动时迁移链gap检测阻断启动 | 单元测试 |
| 10 | 幂等去重（DuplicateEvent错误和幂等写入） | 单元测试 |
| 11 | cargo clippy/test/fmt全部通过 | axm check |
