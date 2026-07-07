# 热图系统

Axiom Core v0.4.0 新增热图系统，用于实时收集和分析信号流量数据，帮助开发者理解系统运行状态。

---

## 目录

- [设计理念](#设计理念)
- [架构概览](#架构概览)
- [核心类型](#核心类型)
- [使用方法](#使用方法)
- [数据格式](#数据格式)
- [导出功能](#导出功能)
- [性能考虑](#性能考虑)

---

## 设计理念

### 1. 实时监控
- 实时收集信号流量数据
- 按时间窗口统计
- 支持秒级粒度

### 2. 多维度分析
- **时间维度**：信号数量随时间变化
- **层维度**：各层信号分布
- **类型维度**：不同信号类型的数量

### 3. 低侵入性
- 通过 feature 开关控制
- 可配置采样率
- 最小化性能影响

### 4. 可导出性
- 支持多种格式导出
- 便于集成到监控系统
- 支持实时流和快照

---

## 架构概览

```
┌─────────────────────────────────────────────────────────┐
│                   HeatmapCollector                      │
│  ┌───────────────────────────────────────────────────┐  │
│  │  时间窗口存储 (TimeWindowStore)                   │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌──────────┐  │  │
│  │  │ t-30s  │ │ t-20s  │ │ t-10s  │ │  当前    │  │  │
│  │  └────────┘ └────────┘ └────────┘ └──────────┘  │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  维度统计 (DimensionStats)                        │  │
│  │  Layer: { Exec: N, Validate: M, ... }           │  │
│  │  Kind: { Command: X, Event: Y, ... }            │  │
│  └───────────────────────────────────────────────────┘  │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────┐
│                   HeatmapExporter                      │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐    │
│  │  JSON 导出   │ │  Prometheus  │ │  自定义格式  │    │
│  └──────────────┘ └──────────────┘ └──────────────┘    │
└─────────────────────────────────────────────────────────┘
```

---

## 核心类型

### HeatmapCollector

```rust
pub struct HeatmapCollector {
    data: RwLock<HeatmapData>,
    window_size: Duration,
    retention: usize,
    sampling_rate: f64,
}

impl HeatmapCollector {
    pub fn new(window_size: Duration, retention: usize, sampling_rate: f64) -> Self;
    pub fn record_signal(&self, signal: &SignalEnvelope);
    pub fn get_data(&self) -> HeatmapData;
    pub fn reset(&self);
}
```

### HeatmapData

```rust
pub struct HeatmapData {
    pub windows: Vec<TimeWindow>,
    pub layer_stats: HashMap<Layer, u64>,
    pub kind_stats: HashMap<SignalKind, u64>,
    pub total_signals: u64,
    pub start_time: Instant,
}

pub struct TimeWindow {
    pub start: Instant,
    pub end: Instant,
    pub count: u64,
    pub layer_counts: HashMap<Layer, u64>,
}
```

### HeatmapExporter

```rust
pub trait HeatmapExporter {
    fn export(&self, data: &HeatmapData) -> Result<Vec<u8>>;
}

pub struct JsonExporter;
pub struct PrometheusExporter;
```

---

## 使用方法

### 创建收集器

```rust
use axiom_kernel::heatmap::{HeatmapCollector, HeatmapExporter, JsonExporter};
use std::time::Duration;

let collector = HeatmapCollector::new(
    Duration::from_secs(10),  // 窗口大小：10秒
    6,                        // 保留6个窗口（1分钟）
    1.0,                      // 采样率：100%
);
```

### 记录信号

```rust
collector.record_signal(&signal_envelope);
```

### 获取数据

```rust
let data = collector.get_data();
println!("Total signals: {}", data.total_signals);
println!("Exec layer: {}", data.layer_stats.get(&Layer::Exec).unwrap_or(&0));
```

### 导出数据

```rust
let exporter = JsonExporter;
let json_bytes = exporter.export(&data)?;
let json_str = String::from_utf8(json_bytes)?;
println!("Heatmap JSON:\n{}", json_str);
```

---

## 数据格式

### JSON 导出格式

```json
{
  "total_signals": 1000,
  "start_time": "2024-01-01T00:00:00Z",
  "current_time": "2024-01-01T00:01:00Z",
  "layer_stats": {
    "Exec": 600,
    "Validate": 200,
    "Agent": 150,
    "Oversight": 50
  },
  "kind_stats": {
    "Command": 400,
    "Event": 300,
    "Query": 200,
    "Response": 100
  },
  "windows": [
    {
      "start": "2024-01-01T00:00:50Z",
      "end": "2024-01-01T00:01:00Z",
      "count": 150,
      "layer_counts": {
        "Exec": 90,
        "Validate": 30,
        "Agent": 20,
        "Oversight": 10
      }
    }
  ]
}
```

### Prometheus 指标

```
# HELP axiom_heatmap_total_signals Total number of signals
# TYPE axiom_heatmap_total_signals counter
axiom_heatmap_total_signals 1000

# HELP axiom_heatmap_signals_by_layer Number of signals by layer
# TYPE axiom_heatmap_signals_by_layer gauge
axiom_heatmap_signals_by_layer{layer="Exec"} 600
axiom_heatmap_signals_by_layer{layer="Validate"} 200
axiom_heatmap_signals_by_layer{layer="Agent"} 150
axiom_heatmap_signals_by_layer{layer="Oversight"} 50

# HELP axiom_heatmap_signals_by_kind Number of signals by kind
# TYPE axiom_heatmap_signals_by_kind gauge
axiom_heatmap_signals_by_kind{kind="Command"} 400
axiom_heatmap_signals_by_kind{kind="Event"} 300
axiom_heatmap_signals_by_kind{kind="Query"} 200
axiom_heatmap_signals_by_kind{kind="Response"} 100
```

---

## 导出功能

### JSON 导出

```rust
let exporter = JsonExporter;
let bytes = exporter.export(&data)?;
```

### Prometheus 导出

```rust
let exporter = PrometheusExporter;
let bytes = exporter.export(&data)?;
```

### 自定义导出

```rust
struct CustomExporter;

impl HeatmapExporter for CustomExporter {
    fn export(&self, data: &HeatmapData) -> Result<Vec<u8>> {
        // 自定义格式
        let output = format!("Signals: {}, Layers: {:?}", data.total_signals, data.layer_stats);
        Ok(output.into_bytes())
    }
}
```

---

## 性能考虑

### 采样率

```rust
let collector = HeatmapCollector::new(
    Duration::from_secs(10),
    6,
    0.1,  // 10% 采样率，减少 90% 的数据量
);
```

### 窗口大小

- **小窗口**：更高的时间精度，但更多内存占用
- **大窗口**：更低的内存占用，但时间精度降低

### 内存估算

```rust
// 每个窗口约 1KB
// 6 个窗口 = 6KB
// 100% 采样，每秒 1000 信号，1分钟 = 60,000 信号
// 每个信号记录约 100 字节 = 6MB
```

### Feature 开关

```toml
[dependencies]
axiom-kernel = { version = "0.4", features = ["heatmap"] }
```

---

## 集成示例

### 集成到运行时

```rust
use axiom_kernel::heatmap::HeatmapCollector;

struct MyRuntime {
    heatmap: HeatmapCollector,
}

impl MyRuntime {
    pub async fn send_signal(&self, signal: SignalEnvelope) {
        self.heatmap.record_signal(&signal);
        // ... 发送逻辑
    }
}
```

### 定时导出

```rust
use tokio::time::{self, Duration};

let collector = HeatmapCollector::new(Duration::from_secs(10), 6, 1.0);

tokio::spawn(async move {
    let mut interval = time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        let data = collector.get_data();
        let exporter = JsonExporter;
        let json = exporter.export(&data).unwrap();
        // 发送到监控系统
    }
});
```

---

## 总结

热图系统为 Axiom Core 提供了强大的实时监控能力：

- **实时收集**：秒级粒度的信号流量数据
- **多维度分析**：按层、类型、时间维度统计
- **低侵入性**：可配置采样率，最小化性能影响
- **灵活导出**：支持 JSON、Prometheus 等格式
- **可扩展性**：支持自定义导出器

这种设计使开发者能够深入理解系统运行状态，及时发现性能瓶颈和异常情况。