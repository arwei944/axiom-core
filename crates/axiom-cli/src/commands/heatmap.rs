use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use std::process::ExitCode;

use axiom_kernel::heatmap::{
    HeatmapCollector, HeatmapExporter, JsonExporter, PrometheusExporter, VizExporter,
};

#[derive(Debug, Args)]
pub struct HeatmapArgs {
    /// Export heatmap to JSON file
    #[arg(long)]
    pub export: Option<PathBuf>,

    /// Show top N hot items
    #[arg(long, default_value = "10")]
    pub top: usize,

    /// Filter by module name
    #[arg(long)]
    pub module: Option<String>,

    /// Show entries since duration (e.g., 1h, 30m)
    #[arg(long)]
    pub since: Option<String>,

    /// Output format: json, prometheus, viz
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub fn run_heatmap(args: &HeatmapArgs) -> Result<ExitCode> {
    let mut collector = HeatmapCollector::new();
    collector.record_cell_message("cell-1");
    collector.record_cell_message("cell-1");
    collector.record_cell_message("cell-2");
    collector.record_signal_send("state_change");
    collector.record_signal_send("tool_call");
    collector.record_tool_invoke("tool-llm");
    collector.record_tool_invoke("tool-llm");
    collector.record_tool_invoke("tool-llm");
    collector.record_axiom_check("invariant-check");
    collector.record_lens_query("state-lens");

    let snapshot = collector.snapshot();

    let exporter: Box<dyn HeatmapExporter> = match args.format.as_str() {
        "prometheus" => Box::new(PrometheusExporter::new()),
        "viz" => Box::new(VizExporter::new()),
        _ => Box::new(JsonExporter::new()),
    };

    let output = exporter.export(&snapshot)?;

    if let Some(path) = &args.export {
        std::fs::write(path, output)?;
        println!("heatmap exported to {}", path.display());
        return Ok(ExitCode::SUCCESS);
    }

    println!("{}", output);
    Ok(ExitCode::SUCCESS)
}
