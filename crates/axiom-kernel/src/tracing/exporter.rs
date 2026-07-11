//! Trace exporters for spans.
//!
//! Provides adapters to export spans to external systems:
//! - `ConsoleExporter` for local debugging
//! - `OTLPExporter` for Jaeger/Zipkin/OTel Collector

use crate::tracing::{Span, SpanStatus};

/// Exporter for completed spans.
pub trait TraceExporter: Send + Sync {
    fn export(&self, spans: &[Span]);
}

/// Logs spans to the console in a human-readable format.
pub struct ConsoleExporter {
    pub service_name: String,
}

impl ConsoleExporter {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self { service_name: service_name.into() }
    }
}

impl TraceExporter for ConsoleExporter {
    fn export(&self, spans: &[Span]) {
        for span in spans {
            let duration_ns = span
                .end_time_ns
                .map(|end| end.saturating_sub(span.start_time_ns))
                .unwrap_or_default();

            let status = match span.status {
                SpanStatus::Ok => "OK",
                SpanStatus::Unset => "UNSET",
                SpanStatus::Error { ref message } => {
                    println!(
                        "[{}] span error: trace_id={} span_id={} error={}",
                        self.service_name,
                        span.trace_id.as_str(),
                        span.span_id.as_str(),
                        message
                    );
                    "ERROR"
                }
            };

            if matches!(span.status, SpanStatus::Error { .. }) {
                // Already printed above.
                continue;
            }

            println!(
                "[{}] span: trace_id={} span_id={} name={} kind={:?} duration_ns={} status={} cell={:?} signal={:?}",
                self.service_name,
                span.trace_id.as_str(),
                span.span_id.as_str(),
                span.name,
                span.kind,
                duration_ns,
                status,
                span.cell_id,
                span.signal_type
            );
        }
    }
}

/// OTLP-compatible exporter that serializes spans as JSON.
pub struct OTLPExporter {
    pub endpoint: String,
    pub service_name: String,
}

impl OTLPExporter {
    pub fn new(endpoint: impl Into<String>, service_name: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into(), service_name: service_name.into() }
    }
}

impl TraceExporter for OTLPExporter {
    fn export(&self, spans: &[Span]) {
        let payload = serde_json::json!({
            "resource_spans": [{
                "resource": { "service.name": self.service_name },
                "scope_spans": [{
                    "spans": spans.iter().map(|s| {
                        serde_json::json!({
                            "trace_id": s.trace_id.as_str(),
                            "span_id": s.span_id.as_str(),
                            "parent_span_id": s.parent_span_id.as_ref().map(|id| id.as_str()),
                            "name": s.name,
                            "kind": format!("{:?}", s.kind),
                            "start_time_unix_nano": s.start_time_ns,
                            "end_time_unix_nano": s.end_time_ns.unwrap_or_default(),
                            "attributes": s.attributes.iter().map(|(k,v)| serde_json::json!({"key": k, "value": {"string_value": v}})).collect::<Vec<_>>(),
                            "status": { "code": match s.status { SpanStatus::Ok => 1, SpanStatus::Unset => 0, SpanStatus::Error { .. } => 2 } }
                        })
                    }).collect::<Vec<_>>()
                }]
            }]
        });

        println!("{}", serde_json::to_string_pretty(&payload).unwrap_or_default());
    }
}
