# Heatmap System

本文档描述 Axiom Core 的热图系统设计和使用方法。

---

## 1. 概述

热图系统用于实时收集和可视化信号流量数据，帮助开发者理解系统运行状态和性能瓶颈。

### 1.1 核心功能

- **信号热图收集**：实时收集信号流量数据
- **时间维度分析**：按时间窗口统计信号数量
- **层维度分析**：按层统计信号分布
- **导出功能**：支持导出为 JSON 格式

### 1.2 核心类型

| 类型 | 职责 |
|------|------|
| `HeatmapCollector` | 热图数据收集器 |
| `HeatmapExporter` | 热图数据导出器 |
| `HeatmapData` | 热图数据结构 |

---

## 2. 快速开始

### 2.1 收集信号

```rust
use axiom_kernel::heatmap::HeatmapCollector;

let collector = HeatmapCollector::new();
collector.record_signal(&signal_envelope);
```

### 2.2 获取数据

```rust
let data = collector.get_data();
println!("Total signals: {}", data.total_signals);
println!("Exec layer: {}", data.layer_stats.get(&RuntimeTier::Exec).unwrap_or(&0));
```

### 2.3 导出数据

```rust
use axiom_kernel::heatmap::JsonExporter;

let exporter = JsonExporter;
let json_bytes = exporter.export(&data)?;
let json_str = String::from_utf8(json_bytes)?;
println!("Heatmap JSON:\n{}", json_str);
```

---

## 3. 数据格式

### 3.1 JSON 导出格式

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
  "time_windows": [
    {
      "start": "2024-01-01T00:00:00Z",
      "end": "2024-01-01T00:00:10Z",
      "count": 100
    }
  ]
}
```

---

## 4. 与 Runtime 集成

### 4.1 自动收集

Runtime 会自动将信号记录到热图：

```rust
let runtime = AxiomRuntime::new(RuntimeConfig::default()).await?;
// Runtime 内部会自动调用 HeatmapCollector::record_signal
```

### 4.2 自定义收集器

```rust
let mut runtime = AxiomRuntime::new(RuntimeConfig::default()).await?;
runtime.set_heatmap_collector(Arc::new(MyCustomCollector::new()));
```

---

## 5. 可视化

### 5.1 导出到外部工具

```rust
let data = collector.get_data();
let exporter = JsonExporter;
std::fs::write("heatmap.json", exporter.export(&data)?)?;
```

### 5.2 实时监控

```rust
loop {
    let data = collector.get_data();
    println!("QPS: {}", data.qps());
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

---

## 6. 性能考虑

- **采样率**：可通过配置调整采样率，降低开销
- **内存限制**：热图数据保留最近 N 条记录
- **异步导出**：导出操作在后台线程执行，不阻塞主循环

---

## 7. 故障排查

### 7.1 热图数据为空

检查：
1. `HeatmapCollector` 是否正确初始化
2. 信号是否通过 `record_signal` 记录
3. 采样率是否过高导致数据被丢弃

### 7.2 导出失败

检查：
1. 磁盘空间是否充足
2. 导出路径是否可写
3. JSON 序列化是否失败