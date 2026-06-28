# Phase 5-8: 可视化导出·身份系统·技能系统·规则引擎 - 详细开发任务

> **Spec参考**：[01-agent-identity-skills-mcp-rules.md](../architecture/01-agent-identity-skills-mcp-rules.md)
>
> **前置依赖**：P4（L2运行时门禁）必须全部验收通过。
>
> **本Phase完成标志**：axiom-viz可导出完整系统快照；axiom-agent实现Identity/Skill/Rule完整闭环；可通过CLI导出可视化数据。

---

## 全局约定

- MSRV: 1.75
- 禁止使用 `async-trait`
- 所有 `unsafe` 代码必须有 `// SAFETY:` 注释
- 所有公共 API 需要 `/// rustdoc`（英文）和 `#[derive(Debug)]`
- 普通注释使用中文
- 每个任务完成后执行：`cargo build -p <crate> && cargo clippy -p <crate> -- -D warnings && cargo test -p <crate>`

---

# P5: 可视化导出 (axiom-viz)

> **目标**：实现完整的数据收集逻辑，可以导出 topology/timeline/entropy/trace/metrics 及演化历史。

## 现有数据结构回顾

已定义在 axiom-viz 中的基础结构：
- `CellNode` / `TopologyGraph` - 拓扑图
- `TimelineEntry` / `Timeline` - 时间线
- `EntropyData` - 熵数据

---

### T73: 实现 VizCollector - 订阅 Witness 流构建 Timeline

**文件**：
- 新建：`crates/axiom-viz/src/collector.rs`
- 修改：`crates/axiom-viz/src/lib.rs`

**接口定义**：
```rust
use axiom_core::witness::Witness;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::timeline::{Timeline, TimelineEntry};

/// Collects Witness events and builds Timeline incrementally
#[derive(Debug)]
pub struct VizCollector {
    timeline: Arc<RwLock<Timeline>>,
    max_entries: usize,
}

impl VizCollector {
    /// Create a new VizCollector with specified capacity
    pub fn new(max_entries: usize) -> Self {
        Self {
            timeline: Arc::new(RwLock::new(Timeline {
                entries: Vec::with_capacity(max_entries),
            })),
            max_entries,
        }
    }

    /// Process a Witness event and add to Timeline
    pub async fn on_witness(&self, witness: Witness) {
        let entry = TimelineEntry {
            cell_id: witness.cell_id,
            layer: witness.vector_clock.layer_hint(),
            timestamp_ns: witness.timestamp_ns,
            summary: witness.summary,
            outcome: match witness.outcome {
                axiom_core::witness::TransitionOutcome::Success => "success".to_string(),
                axiom_core::witness::TransitionOutcome::Failed { reason } => {
                    format!("failed: {}", reason)
                }
                axiom_core::witness::TransitionOutcome::AxiomViolated {
                    axiom_name,
                    message,
                } => {
                    format!("axiom_violated: {} - {}", axiom_name, message)
                }
            },
        };

        let mut timeline = self.timeline.write().await;
        timeline.entries.push(entry);
        if timeline.entries.len() > self.max_entries {
            let overflow = timeline.entries.len() - self.max_entries;
            timeline.entries.drain(0..overflow);
        }
    }

    /// Get current Timeline snapshot
    pub async fn timeline(&self) -> Timeline {
        self.timeline.read().await.clone()
    }

    /// Get Arc reference for registration with Runtime
    pub fn timeline_handle(&self) -> Arc<RwLock<Timeline>> {
        self.timeline.clone()
    }
}
```

**验收标准**：
- [ ] VizCollector 可接收 Witness 并转换为 TimelineEntry
- [ ] 超过 max_entries 时自动淘汰最早的条目
- [ ] timeline() 返回克隆的快照，不持有锁
- [ ] 单元测试：添加多个 Witness 后 timeline 顺序正确

**验证命令**：
```bash
cargo build -p axiom-viz
cargo clippy -p axiom-viz -- -D warnings
cargo test -p axiom-viz -- viz_collector
```

**Commit Message**：
```
feat(viz): T73 implement VizCollector for Witness stream

- Add VizCollector that subscribes to Witness and builds Timeline
- Implement ring buffer with max_entries capacity
- Add timeline snapshot method and handle for Runtime registration
```

---

### T74: 实现 TopologyBuilder - 从 Runtime 扫描注册 Cell 构建拓扑图

**文件**：
- 新建：`crates/axiom-viz/src/topology_builder.rs`
- 修改：`crates/axiom-viz/src/lib.rs`

**接口定义**：
```rust
use std::collections::HashMap;
use axiom_core::layer::Layer;
use crate::topology::{CellNode, CellStatus, TopologyGraph};

/// Cell registration information from Runtime
#[derive(Debug, Clone)]
pub struct CellRegistration {
    pub cell_id: String,
    pub cell_type: String,
    pub layer: Layer,
    pub status: CellStatus,
}

/// Builds TopologyGraph from registered Cells
#[derive(Debug, Default)]
pub struct TopologyBuilder {
    cells: Vec<CellRegistration>,
    edges: Vec<(String, String)>,
}

impl TopologyBuilder {
    /// Create a new empty TopologyBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a cell with the builder
    pub fn register_cell(&mut self, reg: CellRegistration) -> &mut Self {
        self.cells.push(reg);
        self
    }

    /// Add an edge between two cells (from -> to)
    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>) -> &mut Self {
        self.edges.push((from.into(), to.into()));
        self
    }

    /// Infer default edges based on layer communication rules
    pub fn infer_default_edges(&mut self) -> &mut Self {
        // Layer ordering: Core -> Store -> Runtime -> Oversight -> Agent -> Cli
        let layer_order = [
            Layer::Core,
            Layer::Store,
            Layer::Runtime,
            Layer::Oversight,
            Layer::Agent,
            Layer::Cli,
        ];

        let cells_by_layer: HashMap<Layer, Vec<String>> = self
            .cells
            .iter()
            .map(|c| (c.layer, c.cell_id.clone()))
            .fold(HashMap::new(), |mut acc, (layer, id)| {
                acc.entry(layer).or_default().push(id);
                acc
            });

        // Allowed communication directions based on architecture rules
        let allowed: &[(Layer, Layer)] = &[
            (Layer::Core, Layer::Store),
            (Layer::Core, Layer::Runtime),
            (Layer::Store, Layer::Runtime),
            (Layer::Runtime, Layer::Oversight),
            (Layer::Oversight, Layer::Agent),
            (Layer::Oversight, Layer::Runtime),
            (Layer::Agent, Layer::Cli),
            (Layer::Cli, Layer::Agent),
        ];

        for (from_layer, to_layer) in allowed {
            if let (Some(from_cells), Some(to_cells)) = (
                cells_by_layer.get(from_layer),
                cells_by_layer.get(to_layer),
            ) {
                for from in from_cells {
                    for to in to_cells {
                        self.edges.push((from.clone(), to.clone()));
                    }
                }
            }
        }

        self
    }

    /// Build the TopologyGraph
    pub fn build(&self) -> TopologyGraph {
        let nodes: Vec<CellNode> = self
            .cells
            .iter()
            .map(|reg| CellNode {
                cell_id: reg.cell_id.clone(),
                cell_type: reg.cell_type.clone(),
                layer: format!("{:?}", reg.layer),
                status: reg.status.clone(),
            })
            .collect();

        let edges: Vec<crate::topology::Edge> = self
            .edges
            .iter()
            .map(|(from, to)| crate::topology::Edge {
                from: from.clone(),
                to: to.clone(),
            })
            .collect();

        TopologyGraph { nodes, edges }
    }
}
```

**验收标准**：
- [ ] TopologyBuilder 可注册多个 Cell
- [ ] infer_default_edges 根据 11 条允许方向推断边
- [ ] build() 返回完整的 TopologyGraph
- [ ] 单元测试：注册多层 Cell 后拓扑结构正确

**验证命令**：
```bash
cargo build -p axiom-viz
cargo clippy -p axiom-viz -- -D warnings
cargo test -p axiom-viz -- topology_builder
```

**Commit Message**：
```
feat(viz): T74 implement TopologyBuilder for cell topology

- Add CellRegistration struct for Runtime cell info
- Implement TopologyBuilder with cell registration and edge addition
- Add infer_default_edges based on layer communication rules
- Build TopologyGraph with nodes and edges
```

---

### T75: 实现 EntropyTracker - 轮询 EntropyGovernor 获取实时熵数据

**文件**：
- 新建：`crates/axiom-viz/src/entropy_tracker.rs`
- 修改：`crates/axiom-viz/src/lib.rs`

**接口定义**：
```rust
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::entropy::EntropyData;

/// Trait for entropy data source (EntropyGovernor implements this)
#[async_trait::async_trait]
pub trait EntropySource: Send + Sync + 'static {
    /// Get current entropy reading
    async fn current_entropy(&self) -> EntropyData;
}

/// Tracks entropy data over time with history
#[derive(Debug)]
pub struct EntropyTracker {
    current: Arc<RwLock<EntropyData>>,
    history: Arc<RwLock<Vec<EntropyData>>>,
    max_history: usize,
}

impl EntropyTracker {
    /// Create new EntropyTracker
    pub fn new(max_history: usize) -> Self {
        Self {
            current: Arc::new(RwLock::new(EntropyData {
                overall: 0.0,
                per_cell: Default::default(),
                level: "green".to_string(),
                timestamp_ns: 0,
            })),
            history: Arc::new(RwLock::new(Vec::with_capacity(max_history))),
            max_history,
        }
    }

    /// Update with new entropy reading
    pub async fn update(&self, data: EntropyData) {
        let mut current = self.current.write().await;
        *current = data.clone();

        let mut history = self.history.write().await;
        history.push(data);
        if history.len() > self.max_history {
            let overflow = history.len() - self.max_history;
            history.drain(0..overflow);
        }
    }

    /// Get current entropy
    pub async fn current(&self) -> EntropyData {
        self.current.read().await.clone()
    }

    /// Get entropy history
    pub async fn history(&self) -> Vec<EntropyData> {
        self.history.read().await.clone()
    }

    /// Start polling an EntropySource at specified interval
    pub async fn start_polling<S: EntropySource>(
        &self,
        source: Arc<S>,
        poll_interval: Duration,
    ) {
        let current = self.current.clone();
        let history = self.history.clone();
        let max_history = self.max_history;

        tokio::spawn(async move {
            let mut ticker = interval(poll_interval);
            loop {
                ticker.tick().await;
                let data = source.current_entropy().await;

                let mut curr = current.write().await;
                *curr = data.clone();

                let mut hist = history.write().await;
                hist.push(data);
                if hist.len() > max_history {
                    let overflow = hist.len() - max_history;
                    hist.drain(0..overflow);
                }
            }
        });
    }
}
```

**验收标准**：
- [ ] EntropyTracker 可手动更新熵数据
- [ ] 维护历史记录，超过 max_history 自动淘汰
- [ ] start_polling 可从 EntropySource 定时轮询
- [ ] 单元测试：多次 update 后历史正确

**验证命令**：
```bash
cargo build -p axiom-viz
cargo clippy -p axiom-viz -- -D warnings
cargo test -p axiom-viz -- entropy_tracker
```

**Commit Message**：
```
feat(viz): T75 implement EntropyTracker with polling

- Add EntropySource trait for EntropyGovernor integration
- Implement EntropyTracker with current value and history
- Add start_polling for automatic updates from source
- Maintain bounded history with ring buffer behavior
```
