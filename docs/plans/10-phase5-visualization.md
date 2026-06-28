# Phase 5: 可视化导出 Implementation Plan

> **Goal:** 完善 axiom-viz crate：实现从 Runtime 实时采集数据填充拓扑图/时间线/熵值仪表盘/追踪/指标数据结构，并提供导出功能（JSON/DOT/HTML）。验收标准：可以导出完整的系统状态快照（拓扑+时间线+熵值+进化历史），为TUI和Web UI提供数据源。

> **Baseline:** axiom-viz 已有数据结构（EntropyData/TimelineEntry/Timeline/CellNode/TopologyGraph），但只有结构体定义，没有从Runtime采集数据填充这些结构的逻辑，也没有导出功能。

---

## Global Constraints

- axiom-viz 依赖 axiom-core，不依赖 axiom-runtime（数据采集通过trait接口，降低耦合）
- 数据采集不能阻塞消息处理（异步采样，不添加到热路径）
- 导出格式必须是确定性的（相同状态产生相同输出，便于快照对比）
- 导出数据不包含敏感信息（ComplianceGuard检查）
- cargo build/clippy/test 零警告

---

## Task 1: 完善数据结构和 Trace 数据类型

**Files:**
- Modify: `crates/axiom-viz/src/lib.rs`
- Create: `crates/axiom-viz/src/trace.rs`
- Create: `crates/axiom-viz/src/metrics.rs`

- [ ] **Step 1: 添加 Trace 数据结构**
  ```rust
  pub struct TraceSpan {
      pub trace_id: String,
      pub correlation_id: String,
      pub parent_span_id: Option<String>,
      pub span_id: String,
      pub cell_id: String,
      pub signal_type: String,
      pub started_at_ns: u64,
      pub duration_ns: u64,
      pub outcome: TraceOutcome,  // Success/Failure/Timeout/Rejected
      pub error: Option<String>,
      pub children: Vec<TraceSpan>,
  }
  pub enum TraceOutcome { Success, Failure, Timeout, Rejected, CircuitBroken }
  pub struct Trace {
      pub trace_id: String,
      pub root: TraceSpan,
      pub total_duration_ns: u64,
      pub total_cells_visited: u32,
      pub total_hops: u32,
  }
  ```

- [ ] **Step 2: 添加 Metrics 数据结构**
  ```rust
  pub struct MetricsSnapshot {
      pub timestamp_ns: u64,
      pub total_messages_processed: u64,
      pub messages_per_sec: f64,
      pub avg_latency_ms: f64,
      pub p50_latency_ms: f64,
      pub p95_latency_ms: f64,
      pub p99_latency_ms: f64,
      pub total_errors: u64,
      pub total_circuit_breaks: u64,
      pub total_restarts: u64,
      pub per_cell: HashMap<String, CellMetrics>,
  }
  pub struct CellMetrics {
      pub messages_processed: u64,
      pub errors: u64,
      pub avg_latency_ms: f64,
      pub p99_latency_ms: f64,
      pub mailbox_depth: usize,
      pub circuit_state: String,
      pub restart_count: u32,
  }
  ```

- [ ] **Step 3: 添加 EvolutionView 数据结构**
  - ```rust
    pub struct EvolutionView {
        pub total_proposals: u64,
        pub adopted: u64,
        pub rejected: u64,
        pub auto_rolled_back: u64,
        pub recent_activity: Vec<EvolutionActivity>,
        pub current_canary: Option<CanaryStatus>,
    }
    pub struct EvolutionActivity { pub proposal_id: String, pub step: String, pub summary: String, pub timestamp_ns: u64 }
    pub struct CanaryStatus { pub proposal_id: String, pub duration_secs: u64, pub messages_processed: u64, pub error_rate_delta: f64 }
    ```

- [ ] **Step 4: 完善现有 TopologyGraph**
  - 添加 edge_types（消息类型）
  - 添加 layer 颜色编码（Exec=blue, Validate=green, Agent=purple, Oversight=red）
  - 添加 cell 状态颜色（Running=green, Restarting=yellow, CircuitOpen=red, Stopped=gray）

- [ ] **Step 5: 测试**
  - 所有数据结构可序列化/反序列化
  - TraceSpan 可构建树形结构

- [ ] **Step 6: Commit**
  - `feat(axiom-viz): add Trace, Metrics, and EvolutionView data structures, complete TopologyGraph with layer/status coloring`

---

## Task 2: 实现 VizCollector 数据采集trait和内存收集器

**Files:**
- Create: `crates/axiom-viz/src/collector.rs`

- [ ] **Step 1: 定义 VizDataSource trait**
  ```rust
  pub trait VizDataSource: Send + Sync {
      fn get_cell_states(&self) -> Vec<CellStateInfo>;
      fn get_message_stats(&self, cell_id: &str) -> MessageStats;
      fn get_entropy_snapshot(&self) -> EntropyData;
      fn get_recent_witnesses(&self, since_ns: u64, limit: usize) -> Vec<WitnessInfo>;
      fn get_traces(&self, trace_id: Option<&str>, limit: usize) -> Vec<Trace>;
      fn get_metrics(&self) -> MetricsSnapshot;
      fn get_evolution_view(&self) -> EvolutionView;
      fn get_routing_table(&self) -> Vec<RouteInfo>;
  }
  pub struct CellStateInfo { pub id: String, pub name: String, pub layer: String, pub state: String, pub message_types: Vec<String> }
  pub struct MessageStats { pub sent: u64, pub received: u64, pub errors: u64, pub last_message_ns: Option<u64> }
  pub struct WitnessInfo { pub witness_id: String, pub cell_id: String, pub summary: String, pub outcome: String, pub timestamp_ns: u64 }
  pub struct RouteInfo { pub from: String, pub to: String, pub signal_type: String, pub is_broadcast: bool }
  ```

- [ ] **Step 2: 实现 InMemoryCollector**
  - 实现 VizDataSource
  - 提供 `fn record_message_sent(...)`、`fn record_message_received(...)`、`fn record_witness(...)` 等方法供Runtime调用
  - 使用环形缓冲区存储最近N条Witness（默认10000）
  - 使用RingBuffer存储最近N条Trace（默认1000）

- [ ] **Step 3: 在 axiom-runtime 中集成 InMemoryCollector**
  - RuntimeBuilder 添加 `with_viz_collector(collector: Arc<InMemoryCollector>)`
  - MessageBus 在消息投递时调用 collector.record_message_*
  - Cell handle完成后调用 collector.record_witness
  - 每10秒采样一次per-cell metrics（不每消息记录延迟，减少开销）

- [ ] **Step 4: 测试**
  - 测试InMemoryCollector记录和查询消息统计
  - 测试环形缓冲区覆盖旧数据
  - 测试Trace树形构建

- [ ] **Step 5: Commit**
  - `feat(axiom-viz): VizDataSource trait, InMemoryCollector with ring buffers, runtime integration hooks`

---

## Task 3: 实现数据导出功能

**Files:**
- Create: `crates/axiom-viz/src/export.rs`

- [ ] **Step 1: 实现 JSON 导出**
  - `fn export_json(snapshot: &SystemSnapshot) -> Result<String, VizError>`
  - SystemSnapshot 包含所有数据（topology, timeline, entropy, metrics, traces, evolution）
  - 使用 serde_json::to_string_pretty

- [ ] **Step 2: 实现 DOT (Graphviz) 导出拓扑图**
  - `fn export_dot(topology: &TopologyGraph) -> String`
  - 生成Graphviz DOT格式，按层分组（subgraph cluster），节点颜色按状态，边标注信号类型
  - 可通过 `dot -Tpng topology.dot -o topology.png` 生成图片

- [ ] **Step 3: 实现 Timeline 文本导出**
  - `fn export_timeline_text(timeline: &Timeline) -> String`
  - 按时间排序的Witness列表，格式：`[timestamp] cell_id: summary (outcome)`

- [ ] **Step 4: 实现 Trace 文本导出（类似Jaeger格式）**
  - `fn export_trace_text(trace: &Trace) -> String`
  - 树形缩进显示调用链
  - 显示每层cell、耗时、状态

- [ ] **Step 5: 实现 SystemSnapshot 构建**
  - `fn build_snapshot(source: &dyn VizDataSource) -> SystemSnapshot`
  - 从数据源一次性拉取所有数据构建完整快照

- [ ] **Step 6: 测试**
  - 测试JSON导出可反序列化回SystemSnapshot
  - 测试DOT导出包含所有节点和边
  - 测试Timeline按时间排序
  - 测试Trace树形缩进格式

- [ ] **Step 7: Commit**
  - `feat(axiom-viz): export system snapshot as JSON, topology as Graphviz DOT, timeline and trace as formatted text`

---

## Task 4: 集成到 axm CLI

**Files:**
- Modify: `crates/axiom-cli/src/commands/` (新增 viz/top/trace 命令)

- [ ] **Step 1: 实现 `axm top`**
  - 实时TUI仪表盘（使用 crossterm 或简单终端输出，初版用文本刷新）
  - 显示：全局状态、entropy进度条、各Cell状态、消息速率、P99延迟
  - 每2秒刷新
  - 按键：q退出，t切换到trace视图，e切换到entropy视图

- [ ] **Step 2: 实现 `axm trace <correlation_id>`**
  - 查询并显示指定correlation_id的Trace
  - 树形格式输出调用链
  - 显示每跳耗时和状态

- [ ] **Step 3: 实现 `axm topology [--format dot/json]`**
  - 导出当前拓扑图
  - 默认输出文本摘要，--format dot 导出Graphviz，--format json导出JSON

- [ ] **Step 4: 实现 `axm export [--output file]`**
  - 导出完整SystemSnapshot到文件
  - 默认JSON格式

- [ ] **Step 5: Commit**
  - `feat(axiom-cli): axm top (TUI dashboard), axm trace, axm topology, axm export commands`

---

## P5 阶段验收标准

| # | 验收项 | 验证方式 |
|---|--------|---------|
| 1 | cargo build -p axiom-viz 零警告 | 命令行验证 |
| 2 | cargo test -p axiom-viz 全部通过（≥15个测试） | 命令行验证 |
| 3 | Trace/Metrics/EvolutionView数据结构完整 | API审查 |
| 4 | InMemoryCollector环形缓冲区记录消息/Witness | 单元测试 |
| 5 | Runtime集成：消息处理时自动采集数据 | 集成测试 |
| 6 | JSON导出往返正确 | 单元测试 |
| 7 | DOT导出可生成有效Graphviz | 手动验证 |
| 8 | Timeline文本格式按时间排序 | 单元测试 |
| 9 | axm top/trace/topology/export命令可用 | 手动测试 |
| 10 | cargo clippy/test/fmt全部通过 | axm check |
