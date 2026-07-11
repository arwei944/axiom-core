//! Distributed Tracing - OpenTelemetry-compatible observability.
//!
//! Provides trace context propagation through `SignalEnvelope`, span recording,
//! sampling, and export adapters (OTLP + console).

pub mod context;
pub mod exporter;
pub mod sampler;

use crate::id::{CorrelationId, SpanId, TraceId};
use serde::{Deserialize, Serialize};

pub use context::{PropagationDecision, TraceContext, TraceContextInjector, TraceContextExtractor};
pub use exporter::{ConsoleExporter, OTLPExporter, TraceExporter};
pub use sampler::{AlwaysOnSampler, ParentBasedSampler, ProbabilisticSampler, TraceSampler};

/// Active span recorded during signal processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub name: String,
    pub kind: SpanKind,
    pub start_time_ns: u64,
    pub end_time_ns: Option<u64>,
    pub attributes: Vec<(String, String)>,
    pub status: SpanStatus,
    pub cell_id: Option<String>,
    pub signal_type: Option<String>,
    pub correlation_id: CorrelationId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanKind {
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    Ok,
    Error { message: String },
    Unset,
}

impl Span {
    pub fn new_root(name: impl Into<String>) -> Self {
        let trace_id = TraceId::generate();
        let span_id = SpanId::generate();
        Self {
            trace_id,
            span_id,
            parent_span_id: None,
            name: name.into(),
            kind: SpanKind::Internal,
            start_time_ns: crate::clock::global_clock().now_ns(),
            end_time_ns: None,
            attributes: Vec::new(),
            status: SpanStatus::Unset,
            cell_id: None,
            signal_type: None,
            correlation_id: CorrelationId::generate(),
        }
    }

    pub fn child(&self, name: impl Into<String>) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: SpanId::generate(),
            parent_span_id: Some(self.span_id.clone()),
            name: name.into(),
            kind: SpanKind::Internal,
            start_time_ns: crate::clock::global_clock().now_ns(),
            end_time_ns: None,
            attributes: Vec::new(),
            status: SpanStatus::Unset,
            cell_id: None,
            signal_type: None,
            correlation_id: self.correlation_id.clone(),
        }
    }

    pub fn finish(mut self) -> Self {
        self.end_time_ns = Some(crate::clock::global_clock().now_ns());
        self
    }

    pub fn with_cell(mut self, cell_id: impl Into<String>) -> Self {
        self.cell_id = Some(cell_id.into());
        self
    }

    pub fn with_signal(mut self, signal_type: impl Into<String>) -> Self {
        self.signal_type = Some(signal_type.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }

    pub fn with_status(mut self, status: SpanStatus) -> Self {
        self.status = status;
        self
    }
}

/// In-memory span store for tracing CLI queries.
#[derive(Debug, Default)]
pub struct SpanStore {
    spans: parking_lot::RwLock<Vec<Span>>,
}

impl SpanStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, span: Span) {
        self.spans.write().push(span);
    }

    pub fn by_trace_id(&self, trace_id: &TraceId) -> Vec<Span> {
        self.spans
            .read()
            .iter()
            .filter(|s| &s.trace_id == trace_id)
            .cloned()
            .collect()
    }

    pub fn clear(&self) {
        self.spans.write().clear();
    }
}
