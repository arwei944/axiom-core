# Axiom Core 性能调优指南

> **版本:** v0.3.0
> **最后更新:** 2026-07-04

---

## 1. 性能基准

Axiom Core 内置基准测试套件位于 `crates/axiom-bench/`。运行基准测试：

```bash
cargo bench -p axiom-bench
```

### 1.1 核心基准结果

| 基准项 | 指标 | v0.2 基线 | v0.3 目标 | 实际结果 |
|--------|------|----------|----------|---------|
| bus_dispatch | 单 Cell 吞吐量 | 50K msg/s | 55K msg/s | 58K msg/s |
| mailbox_throughput | 无锁推送 | 120K msg/s | 130K msg/s | 135K msg/s |
| message_passing | 跨 Cell 延迟 | 12µs | 10µs | 9µs |
| witness_chain | 链验证耗时 | 8µs | 7µs | 6.5µs |

### 1.2 序列化对比

| 编码方式 | 平均大小 | 编码耗时 | 解码耗时 |
|----------|---------|---------|---------|
| JSON | 256 bytes | 1.2µs | 2.1µs |
| Bincode | 118 bytes | 0.4µs | 0.6µs |

> Bincode 体积减少 54%，编码速度提升 3x。

---

## 2. 调优决策树

### 2.1 高延迟排查

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
│   └── 解决：调整 WAL 模式或使用内存存储
└── 检查跨 Cell 调用链
    └── 现象：hop_count 接近 8
    └── 解决：减少层间调用或合并 Cell
```

### 2.2 高内存排查

```
内存占用高
├── 检查 snapshot 策略
│   └── 现象：MemorySnapshotStore 持续增长
│   └── 解决：启用 OnStateSize 或 EveryN 策略
├── 检查 witness 链长度
│   └── 现象：WitnessStore 未清理
│   └── 解决：配置 retention 策略
├── 检查 event 流未裁剪
│   └── 现象：EventStore 无限增长
│   └── 解决：定期归档或设置 TTL
└── 检查 DLQ 堆积
    └── 现象：DeadLetterQueue 未消费
    └── 解决：增加消费者或调整容量
```

### 2.3 高熵值排查

```
熵值持续升高
├── 检查消息失败率
│   └── 现象：MESSAGE_TOTAL status=failed 增长
│   └── 解决：修复 Cell 处理逻辑
├── 检查 Cell 重启频率
│   └── 现象：CELL_RESTARTS_TOTAL 频繁递增
│   └── 解决：调整 backoff 策略或修复 panic
├── 检查 witness 链断裂
│   └── 现象：WITNESS_CHAIN_ERRORS 增长
│   └── 解决：恢复 witness 链或快照重建
└── 检查层间调用违反
    └── 现象：LayerViolation 错误
    └── 解决：修正架构依赖方向
```

---

## 3. 关键配置参数建议

### 3.1 RuntimeConfig

| 参数 | 默认值 | 建议范围 | 说明 |
|------|--------|----------|------|
| `mailbox_capacity` | 1024 | 256-8192 | 过大导致内存浪费，过小导致消息丢失 |
| `entropy_threshold` | 100.0 | 50-500 | 触发 circuit break 的熵值上限 |
| `entropy_cooldown_ms` | 60000 | 10000-300000 | circuit break 冷却时间 |
| `dispatch_poll_interval_ms` | 10 | 5-100 | dispatch loop 轮询间隔 |

### 3.2 SnapshotPolicy

| 策略类型 | 适用场景 | 配置建议 |
|----------|----------|----------|
| `Never` | 测试环境 | 无 |
| `EveryN { n: 1000 }` | 事件驱动 | 每 1000 事件快照 |
| `EveryDuration { duration_ms: 3600000 }` | 时间驱动 | 每小时快照 |
| `OnStateSize { bytes: 10485760 }` | 大小驱动 | 状态超过 10MB 快照 |

### 3.3 序列化选择

| 场景 | 推荐 Codec | 理由 |
|------|-----------|------|
| 调试/开发 | JsonCodec | 可读性好，便于排障 |
| 生产内部总线 | BincodeCodec | 体积小、速度快 |
| 跨语言通信 | JsonCodec | 兼容性好 |

---

## 4. 性能测试命令

```bash
# 运行全部基准测试
cargo bench -p axiom-bench

# 运行特定基准
cargo bench -p axiom-bench -- bus_dispatch

# 对比两个版本的基准结果
cargo bench -p axiom-bench -- --save-baseline v0.2
# 修改代码后
cargo bench -p axiom-bench -- --baseline v0.2

# 压力测试（100 Cell，10000 消息）
cargo run -p axiom-bench -- stress --cells 100 --messages 10000
```

---

## 5. 常见性能陷阱

### 5.1 避免在 Cell::handle 中阻塞

```rust
// ❌ 错误：同步阻塞
fn handle(&mut self, msg: &MySignal) {
    std::thread::sleep(Duration::from_secs(1));
}

// ✅ 正确：异步非阻塞
async fn handle(&mut self, msg: &MySignal) {
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

### 5.2 避免频繁的 VectorClock 克隆

```rust
// ❌ 错误：每次消息都克隆
fn handle(&mut self, msg: &MySignal) {
    let vc = msg.vector_clock().clone();
}

// ✅ 正确：仅在必要时克隆
fn handle(&mut self, msg: &MySignal) {
    let vc = msg.vector_clock();
    // 只读操作
}
```

### 5.3 使用 bincode 减少总线开销

```rust
// 在内部总线使用 bincode codec
let codec = Arc::new(BincodeCodec::default());
let bus = MessageBus::with_codec(codec);
```

---

## 6. 性能监控指标

通过 `/metrics` 端点监控以下关键指标：

- `axiom_messages_total{status="success|failed|rejected"}` - 消息处理趋势
- `axiom_message_duration_seconds` - 处理耗时分布
- `axiom_cell_restarts_total` - Cell 稳定性
- `axiom_entropy_score` - 系统熵值趋势
- `axiom_dead_letters_total` - DLQ 堆积情况
- `axiom_witness_chain_errors` - Witness 链健康度

---

## 7. 扩缩容建议

### 7.1 Cell 水平扩展

- 无状态 Cell 可无限水平扩展
- 有状态 Cell 需考虑状态分片
- 建议单实例 Cell 数量 < 500

### 7.2 多实例部署

- 使用独立 EventStore 实例
- 使用负载均衡分发到不同 Runtime 实例
- 建议每实例 Cell 数量 < 200

---

## 8. 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| v0.3.0 | 2026-07-04 | 新增 bincode 基准、决策树、配置建议 |
| v0.2.0 | 2025-12-01 | 初始版本 |
