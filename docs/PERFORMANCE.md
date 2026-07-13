# Axiom Core 性能基准测试报告

> **版本:** v0.4.0
> **最后更新:** 2026-07-14

---

## 1. 基准测试方法论

### 1.1 工具与框架

基准测试基于 [Criterion.rs](https://github.com/bheisler/criterion.rs) v0.5.1，集成在
`crates/axiom-bench` crate 中。Criterion 提供统计显著性分析、离群值检测和性能回归对比。

所有基准测试均使用 `criterion::black_box` 防止编译器优化消除关键计算。异步基准测试
使用 `criterion` 的 `async_tokio` feature，通过 `tokio::runtime::Runtime` + `block_on`
驱动异步 Future。

### 1.2 运行方式

```bash
# 运行全部基准测试（快速模式，减少采样数）
cargo bench -p axiom-bench -- --quick

# 运行特定基准套件
cargo bench -p axiom-bench --bench bus_dispatch -- --quick

# 完整模式（更多采样，结果更精确）
cargo bench -p axiom-bench
```

`--quick` 标志缩短预热时间和采样数，适合开发阶段快速验证。生产基线应使用完整模式。

### 1.3 基准测试套件

| 套件 | 文件 | 覆盖场景 |
|------|------|----------|
| `bus_dispatch` | `benches/bus_dispatch.rs` | 消息总线吞吐量、拦截器开销、守护进程决策 |
| `witness_chain` | `benches/witness_chain.rs` | Witness 哈希计算、链构建与验证、序列化 |
| `entropy_governor` | `benches/entropy_governor.rs` | 熵治理事件记录、决策、快照、衰减 |
| `sqlite_store` | `benches/sqlite_store.rs` | SQLite 事件写入（单条/批量）与查询 |
| `mailbox_throughput` | `benches/mailbox_throughput.rs` | Mailbox 推送/弹出吞吐量 |
| `message_passing` | `benches/message_passing.rs` | 信号信封创建与序列化 |

---

## 2. 测试环境说明

| 项目 | 配置 |
|------|------|
| 操作系统 | Microsoft Windows 11 专业版 |
| CPU | Intel Xeon E5-2650 v2 @ 2.60GHz, 8 核 |
| 内存 | 31.9 GB |
| Rust 工具链 | rustc 1.96.1 (31fca3adb 2026-06-26) |
| 编译模式 | bench profile (optimized, release) |
| 基准参数 | `--quick`（缩短预热与采样） |
| SQLite 模式 | WAL + synchronous=NORMAL, 内存数据库 (`sqlite::memory:`) |

---

## 3. 基准测试结果

### 3.1 消息总线吞吐量（bus_dispatch）

| 基准项 | 耗时 | 吞吐量/说明 |
|--------|------|-------------|
| guardian_intercept_allow | 6.23 ns | 架构守护进程放行路径 |
| guardian_intercept_reject | 226.62 ns | 拒绝路径（含 String 分配） |
| hop_limit_intercept | 3.04 ns | 跳数限制检查 |
| bus_register_and_publish | 7.55 µs | 注册 Cell + 发布消息（含初始化） |
| bus_publish_only | 3.10 µs | 单条消息发布（已注册 Cell） |
| bus_throughput_100 | 294.29 µs | 100 条消息批量发布，约 **340K msg/s** |

### 3.2 Witness 哈希计算（witness_chain）

| 基准项 | 优化前 | 优化后 | 提升幅度 |
|--------|--------|--------|----------|
| witness_creation | 1.16 µs | 1.16 µs | — |
| witness_serialize_json | 3.26 µs | 3.26 µs | — |
| witness_chain_verify_100 | 264.69 ns | 264.69 ns | — |
| witness_chain_verify_1000 | 3.84 µs | 3.84 µs | — |
| **witness_hash_compute** | **2.29 µs** | **2.05 µs** | **-11.2%** |
| **witness_hash_chain_build_100** | **367.83 µs** | **312.46 µs** | **-15.1%** |

> 哈希计算优化详见 [第 5 节：性能优化记录](#5-性能优化记录)。

### 3.3 熵治理决策（entropy_governor）

| 基准项 | 耗时 | 说明 |
|--------|------|------|
| entropy_record_single | 1.26 µs | 记录单条熵事件（含全局+per_cell 更新） |
| entropy_record_batch_100 | 150.44 µs | 记录 100 条混合事件（~1.5 µs/条） |
| entropy_take_action_green | 253.70 ns | Green 级别决策（快照+冷却检查，无动作） |
| entropy_snapshot | 1.81 µs | 生成全局+per_cell 熵快照 |
| entropy_decay_tick | 7.54 µs | 50 个 Cell 的衰减计算 |

### 3.4 SQLite 事件存储（sqlite_store）

| 基准项 | 耗时 | 单事件耗时 | 说明 |
|--------|------|-----------|------|
| sqlite_append_single | 236.21 µs | 236.21 µs | 单条事件写入（含事务提交） |
| sqlite_append_100_sequential | 20.89 ms | 208.85 µs | 100 条逐条写入（各含独立事务） |
| sqlite_append_batch_100 | 5.85 ms | 58.47 µs | 100 条批量写入（单事务） |
| sqlite_read_by_aggregate | 2.09 ms | — | 按 aggregate_id 读取 100 条（索引查询） |
| sqlite_read_by_cell_id | 1.94 ms | — | 按 cell_id 读取 100 条（索引查询） |
| sqlite_read_by_time_range | 1.93 ms | — | 按时间范围读取 100 条（索引查询） |

> **批量写入比逐条写入快 3.6 倍**（58.47 µs vs 208.85 µs/条），因为批量写入在单个事务
> 中完成，避免了每条事件的独立事务提交开销。

---

## 4. 性能优化建议

### 4.1 SQLite 写入优化

- **优先使用 `append_batch`** 而非循环 `append`：批量写入在单事务中完成，性能提升 3.6 倍。
- 已启用 WAL 模式和 `synchronous=NORMAL`，在持久性与性能间取得平衡。
- 如需更高写入吞吐量，可考虑批量大小调优（建议 100-500 条/批）。
- 高频写入场景可考虑异步写入队列，将多条事件合并后批量落盘。

### 4.2 Witness 哈希优化

- 已将 `compute_hash` 中的 4 次 `serde_json::to_string` 中间 String 分配替换为
  `serde_json::to_writer` 直接流式写入哈希器，消除不必要的堆分配。
- 哈希链构建（100 节点）从 367.83 µs 降至 312.46 µs（提升 15.1%）。
- `schema_version.to_string()` 仍有一次小分配，但因涉及哈希格式兼容性，保持不变。

### 4.3 消息总线优化

- 当前单条消息发布耗时 3.10 µs（~340K msg/s），满足大多数场景需求。
- `RoutingTable::resolve` 在单目标场景仍分配 `Vec<String>`，未来可考虑使用 `SmallVec`
  或内联快速路径消除该分配，但当前性能已足够。

### 4.4 熵治理优化

- `take_action` Green 路径仅 253.70 ns，适合在 dispatch loop 中每轮调用。
- `decay_tick` 随 Cell 数量线性增长（50 Cell = 7.54 µs），大规模部署时建议降低衰减频率。

---

## 5. 性能优化记录

### 5.1 Witness 哈希计算：消除中间 String 分配（P2-I3）

**文件:** `crates/axiom-kernel/src/witness.rs` — `Witness::compute_hash`

**问题:** `compute_hash` 在 SHA-256 哈希过程中通过 `serde_json::to_string(&x)?.as_bytes()`
对 `vector_clock`、`outcome`、`metrics`、`kind` 四个字段分别序列化为临时 String，再取字节
喂入哈希器。每次哈希计算产生 4 次堆分配，构建 100 节点哈希链时产生 400 次分配。

**优化:** 引入 `HashWriter` 适配器，实现 `std::io::Write`，将序列化字节直接流式写入
SHA-256 哈希器，使用 `serde_json::to_writer` 替代 `to_string`。哈希字节完全一致，现有
Witness 链保持有效。

**结果:**

| 基准项 | 优化前 | 优化后 | 提升 |
|--------|--------|--------|------|
| witness_hash_compute | 2.29 µs | 2.05 µs | -11.2% |
| witness_hash_chain_build_100 | 367.83 µs | 312.46 µs | -15.1% |

### 5.2 SQLite append_batch 修复（P2-I3）

**文件:** `crates/axiom-store/src/sqlite/queries.rs` — `EventStore::append_batch`

**问题:** `append_batch` 的 INSERT 语句缺少 `event_id` 列（该列为 `NOT NULL UNIQUE`），
导致批量写入必然失败。这是与单条 `append` 实现的偏差。

**修复:** 在列列表和绑定参数中补充 `event_id`，使其与单条 `append` 一致。批量写入基准
测试 `sqlite_append_batch_100` 现已通过。

### 5.3 消息总线锁竞争分析（P2-I3）

**文件:** `crates/axiom-runtime/src/bus.rs` — `MessageBus::publish`

**分析:** `publish` 方法依次获取 `routing.read()` 和 `cells.read()` 两个读锁。经检查：
- 两个读锁**未同时持有**：`routing` 读锁在 `resolve()` 返回后立即释放，`cells` 读锁随后获取。
- 读锁允许多读者并发，稳态下无竞争。
- `register_cell`（写锁）通常仅在启动阶段调用，不与稳态发布竞争。

**结论:** 当前无显著锁竞争问题，无需优化。

---

## 6. 已知性能瓶颈

### 6.1 SQLite 单条写入延迟

单条事件写入 236.21 µs（含事务提交）。在高频单条写入场景下（>4000 条/秒），SQLite 可能
成为瓶颈。建议使用批量写入或引入写入缓冲队列。

### 6.2 Witness 哈希链验证随链长线性增长

`witness_chain_verify_1000` 耗时 3.84 µs（~3.84 ns/条）。超长链（>10000 条）验证时间
会显著增加。建议定期快照截断链长度，或实现增量验证。

### 6.3 熵治理 per_cell 衰减

`decay_tick` 随 Cell 数量线性增长。大规模部署（>500 Cell）时，建议降低衰减频率或改用
分片衰减策略。

### 6.4 guardian_intercept_reject 的 String 分配

拒绝路径（226.62 ns）比放行路径（6.23 ns）慢 36 倍，主要因为 `Reject { reason: String }`
需要分配 String。在高拒绝率场景下可考虑使用 `&'static str` 或预分配原因池。

### 6.5 examples/quick-start 工作区成员缺失

`Cargo.toml` 的 workspace members 引用了 `examples/quick-start`，但该目录不存在，
导致 `cargo bench --workspace` 失败。需移除该成员或创建对应目录。使用 `-p axiom-bench`
可规避此问题。

---

## 7. 基准测试运行指南

```bash
# 运行全部基准测试（快速模式）
cargo bench -p axiom-bench -- --quick

# 运行特定基准套件
cargo bench -p axiom-bench --bench bus_dispatch -- --quick
cargo bench -p axiom-bench --bench witness_chain -- --quick
cargo bench -p axiom-bench --bench entropy_governor -- --quick
cargo bench -p axiom-bench --bench sqlite_store -- --quick

# 过滤特定基准项
cargo bench -p axiom-bench --bench witness_chain -- --quick witness_hash

# 保存基线用于对比
cargo bench -p axiom-bench -- --save-baseline v0.4
# 代码修改后对比
cargo bench -p axiom-bench -- --baseline v0.4

# 压力测试（100 Cell，10000 消息）
cargo run -p axiom-bench -- stress --cells 100 --messages 10000
```

> **注意:** `cargo bench --workspace` 因 `examples/quick-start` 成员缺失而失败，
> 请使用 `cargo bench -p axiom-bench` 代替。

---

## 8. 调优决策树

### 8.1 高延迟排查

```
消息处理延迟高
├── 检查 mailbox 是否满
│   └── 现象：日志出现 "mailbox full, message dropped"
│   └── 解决：增加 mailbox_capacity 或启用 DLQ
├── 检查熵值是否触发 circuit break
│   └── 现象：supervisor 拒绝处理
│   └── 解决：降低 entropy_threshold 或优化 Cell 逻辑
├── 检查 witness 持久化是否阻塞
│   └── 现象：SQLite 写入延迟
│   └── 解决：使用 append_batch 或调整 WAL 模式
└── 检查跨 Cell 调用链
    └── 现象：hop_count 接近 8
    └── 解决：减少层间调用或合并 Cell
```

### 8.2 高内存排查

```
内存占用高
├── 检查 snapshot 策略
│   └── 现象：MemorySnapshotStore 持续增长
│   └── 解决：启用 OnStateSize 或 EveryN 策略
├── 检查 witness 链长度
│   └── 现象：WitnessStore 未清理
│   └── 解决：配置 retention 策略
└── 检查 DLQ 堆积
    └── 现象：DeadLetterQueue 未消费
    └── 解决：增加消费者或调整容量
```

---

## 9. 关键配置参数建议

| 参数 | 默认值 | 建议范围 | 说明 |
|------|--------|----------|------|
| `mailbox_capacity` | 1024 | 256-8192 | 过大导致内存浪费，过小导致消息丢失 |
| `entropy_threshold` | 100.0 | 50-500 | 触发 circuit break 的熵值上限 |
| `entropy_cooldown_ms` | 60000 | 10000-300000 | circuit break 冷却时间 |
| `dispatch_poll_interval_ms` | 10 | 5-100 | dispatch loop 轮询间隔 |
| SQLite `max_connections` | 5 | 1-10 | 连接池大小（内存数据库建议 1） |

---

## 10. 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| v0.4.0 | 2026-07-14 | 新增熵治理、SQLite 存储、Witness 哈希基准；Witness 哈希计算优化 -15%；修复 append_batch 缺失 event_id 列 |
| v0.3.0 | 2026-07-04 | 新增 bincode 基准、决策树、配置建议 |
| v0.2.0 | 2025-12-01 | 初始版本 |
