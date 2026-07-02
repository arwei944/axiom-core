use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args)]
pub struct TraceArgs {
    /// Correlation ID to trace
    pub correlation_id: String,

    /// Show full details including payloads
    #[arg(long)]
    pub verbose: bool,
}

pub fn run_trace(args: &TraceArgs) -> Result<ExitCode> {
    println!("=== axiom trace ===");
    println!("Correlation ID: {}", args.correlation_id);
    if args.verbose {
        println!("Verbose mode: enabled");
    }

    let trace_data = fetch_trace(&args.correlation_id).context("Failed to fetch trace")?;

    println!("\n{}", render_trace(&trace_data, args.verbose));

    Ok(ExitCode::SUCCESS)
}

struct TraceData {
    correlation_id: String,
    events: Vec<TraceEvent>,
}

struct TraceEvent {
    cell_id: String,
    layer: String,
    event_type: String,
    timestamp: String,
    duration_ms: u64,
    outcome: String,
    payload: Option<String>,
}

fn fetch_trace(correlation_id: &str) -> Result<TraceData> {
    Ok(TraceData {
        correlation_id: correlation_id.to_string(),
        events: vec![
            TraceEvent {
                cell_id: "user-input".to_string(),
                layer: "Agent".to_string(),
                event_type: "UserRequest".to_string(),
                timestamp: "2024-01-15T10:30:00.000Z".to_string(),
                duration_ms: 0,
                outcome: "Success".to_string(),
                payload: Some("{\"message\": \"review this PR\"}".to_string()),
            },
            TraceEvent {
                cell_id: "agent-planner".to_string(),
                layer: "Agent".to_string(),
                event_type: "PlanGenerated".to_string(),
                timestamp: "2024-01-15T10:30:00.120Z".to_string(),
                duration_ms: 120,
                outcome: "Success".to_string(),
                payload: Some("{\"steps\": [\"analyze code\", \"check security\", \"generate review\"]}".to_string()),
            },
            TraceEvent {
                cell_id: "validator".to_string(),
                layer: "Validate".to_string(),
                event_type: "CodeValidated".to_string(),
                timestamp: "2024-01-15T10:30:00.340Z".to_string(),
                duration_ms: 220,
                outcome: "Success".to_string(),
                payload: Some("{\"issues\": 3, \"warnings\": 5}".to_string()),
            },
            TraceEvent {
                cell_id: "exec-worker".to_string(),
                layer: "Exec".to_string(),
                event_type: "ReviewGenerated".to_string(),
                timestamp: "2024-01-15T10:30:01.150Z".to_string(),
                duration_ms: 810,
                outcome: "Success".to_string(),
                payload: Some("{\"review\": \"Code looks good, but has 3 issues...\"}".to_string()),
            },
        ],
    })
}

fn render_trace(data: &TraceData, verbose: bool) -> String {
    let mut output = String::new();

    output.push_str(&format!("Correlation ID: {}\n\n", data.correlation_id));
    output.push_str("Execution Timeline:\n");
    output.push_str("──────────────────\n\n");

    let mut total_duration = 0;

    for (i, event) in data.events.iter().enumerate() {
        let outcome_color = match event.outcome.as_str() {
            "Success" => "\x1B[32m",
            "Failed" => "\x1B[31m",
            "Pending" => "\x1B[33m",
            _ => "",
        };

        output.push_str(&format!(
            "[{}] {} → {} ({})\n",
            i + 1,
            event.cell_id,
            event.event_type,
            event.layer
        ));
        output.push_str(&format!(
            "  Time: {} | Duration: {}ms | Outcome: {} {}\x1B[0m\n",
            event.timestamp, event.duration_ms, outcome_color, event.outcome
        ));

        if verbose && event.payload.is_some() {
            output.push_str(&format!("  Payload: {}\n", event.payload.as_ref().unwrap()));
        }

        total_duration += event.duration_ms;
        output.push('\n');
    }

    output.push_str(&format!("Total Duration: {}ms\n", total_duration));

    output
}
