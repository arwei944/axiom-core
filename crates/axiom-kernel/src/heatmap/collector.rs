use serde::Serialize;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct UsageSnapshot {
    pub timestamp: u64,
    pub hot_cells: Vec<(String, u64)>,
    pub hot_signals: Vec<(String, u64)>,
    pub hot_tools: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct TimeRange {
    pub start_ns: u64,
    pub end_ns: u64,
}

impl TimeRange {
    pub fn new(start_ns: u64, end_ns: u64) -> Self {
        Self { start_ns, end_ns }
    }

    pub fn now() -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        Self::new(now.saturating_sub(60), now)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HeatmapCollector {
    #[serde(skip)]
    pub cell_message_count: HashMap<String, u64>,
    #[serde(skip)]
    pub signal_send_count: HashMap<String, u64>,
    #[serde(skip)]
    pub tool_invoke_count: HashMap<String, u64>,
    #[serde(skip)]
    pub axiom_check_count: HashMap<String, u64>,
    #[serde(skip)]
    pub lens_query_count: HashMap<String, u64>,
    #[serde(skip)]
    pub timeline: Vec<UsageSnapshot>,
}

impl Default for HeatmapCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl HeatmapCollector {
    pub fn new() -> Self {
        Self {
            cell_message_count: HashMap::new(),
            signal_send_count: HashMap::new(),
            tool_invoke_count: HashMap::new(),
            axiom_check_count: HashMap::new(),
            lens_query_count: HashMap::new(),
            timeline: Vec::new(),
        }
    }

    pub fn record_cell_message(&mut self, cell_id: impl Into<String>) {
        *self.cell_message_count.entry(cell_id.into()).or_default() += 1;
    }

    pub fn record_signal_send(&mut self, signal_type: impl Into<String>) {
        *self.signal_send_count.entry(signal_type.into()).or_default() += 1;
    }

    pub fn record_tool_invoke(&mut self, tool_id: impl Into<String>) {
        *self.tool_invoke_count.entry(tool_id.into()).or_default() += 1;
    }

    pub fn record_axiom_check(&mut self, axiom_id: impl Into<String>) {
        *self.axiom_check_count.entry(axiom_id.into()).or_default() += 1;
    }

    pub fn record_lens_query(&mut self, lens_id: impl Into<String>) {
        *self.lens_query_count.entry(lens_id.into()).or_default() += 1;
    }

    pub fn snapshot(&self) -> UsageSnapshot {
        UsageSnapshot {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            hot_cells: top_n(&self.cell_message_count, 10),
            hot_signals: top_n(&self.signal_send_count, 10),
            hot_tools: top_n(&self.tool_invoke_count, 10),
        }
    }
}

fn top_n(map: &HashMap<String, u64>, n: usize) -> Vec<(String, u64)> {
    let mut items: Vec<_> = map.iter().map(|(k, v)| (k.clone(), *v)).collect();
    items.sort_by_key(|b| std::cmp::Reverse(b.1));
    items.into_iter().take(n).collect()
}
