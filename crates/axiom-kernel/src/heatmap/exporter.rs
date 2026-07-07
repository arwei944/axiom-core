use crate::heatmap::collector::UsageSnapshot;
use serde_json::to_string_pretty;

pub trait HeatmapExporter {
    fn export(&self, snapshot: &UsageSnapshot) -> Result<String, crate::plugin::abi::PluginError>;
}

pub struct JsonExporter;

impl JsonExporter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl HeatmapExporter for JsonExporter {
    fn export(&self, snapshot: &UsageSnapshot) -> Result<String, crate::plugin::abi::PluginError> {
        to_string_pretty(snapshot).map_err(|e| {
            crate::plugin::abi::PluginError::HandleFailed(e.to_string())
        })
    }
}

pub struct PrometheusExporter;

impl PrometheusExporter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PrometheusExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl HeatmapExporter for PrometheusExporter {
    fn export(&self, snapshot: &UsageSnapshot) -> Result<String, crate::plugin::abi::PluginError> {
        let mut out = String::new();
        for (cell, count) in &snapshot.hot_cells {
            out.push_str(&format!("axiom_cell_messages_total{{cell=\"{}\"}} {}\n", cell, count));
        }
        for (signal, count) in &snapshot.hot_signals {
            out.push_str(&format!("axiom_signal_sends_total{{signal=\"{}\"}} {}\n", signal, count));
        }
        for (tool, count) in &snapshot.hot_tools {
            out.push_str(&format!("axiom_tool_invokes_total{{tool=\"{}\"}} {}\n", tool, count));
        }
        Ok(out)
    }
}

pub struct VizExporter;

impl VizExporter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VizExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl HeatmapExporter for VizExporter {
    fn export(&self, snapshot: &UsageSnapshot) -> Result<String, crate::plugin::abi::PluginError> {
        let json = to_string_pretty(snapshot).map_err(|e| {
            crate::plugin::abi::PluginError::HandleFailed(e.to_string())
        })?;
        Ok(format!("<!doctype html><html><body><pre>{}</pre></body></html>", json))
    }
}
