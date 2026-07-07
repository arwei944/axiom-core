//! Axiom Viz - visualization data export layer.
//!
//! Exposes structured data from the runtime for visualization:
//! topology graphs, message flows, witness timelines, entropy dashboards,
//! trace data, and performance metrics.
//!
//! This crate provides JSON-serializable data structures;
//! actual rendering (TUI, Web UI, etc.) is handled by consumers.

use serde::Serialize;

pub mod cell_flow;
pub mod entropy;
pub mod kernel;
pub mod metrics;
pub mod timeline;
pub mod topology;

pub use cell_flow::{CellFlowRecord, CellFlowSnapshot};
pub use entropy::EntropyData;
#[cfg(feature = "metrics")]
pub use metrics::PrometheusRegistry;
pub use metrics::{
    active_cells, cell_restarts_total, dead_letters_total, entropy_score, init_core_metrics,
    message_duration_seconds, message_total, witness_chain_errors, CounterTrait, GaugeTrait,
    HistogramTrait, MetricDesc, MetricType, MetricsRegistry, NoopRegistry,
};
pub use timeline::{Timeline, TimelineEntry};
pub use topology::{CellNode, TopologyGraph};
pub use kernel::VizKernelAdapter;

#[derive(Debug, Clone, Serialize)]
pub struct VizSnapshot {
    pub topology: TopologyGraph,
    pub timeline: Timeline,
    pub entropy: EntropyData,
    pub flow: CellFlowSnapshot,
}
