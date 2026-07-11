//! Trace context propagation through `SignalEnvelope`.
//!
//! Mirrors W3C TraceContext: inject/extract `traceparent` and `tracestate`
//! headers into envelope metadata fields.

use crate::id::{SpanId, TraceId};
use crate::signal::SignalEnvelope;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub trace_flags: u8,
    pub trace_state: Option<String>,
}

impl Default for TraceContext {
    fn default() -> Self {
        Self {
            trace_id: TraceId::generate(),
            span_id: SpanId::generate(),
            trace_flags: 1,
            trace_state: None,
        }
    }
}

impl TraceContext {
    pub fn new(trace_id: TraceId, span_id: SpanId) -> Self {
        Self { trace_id, span_id, trace_flags: 1, trace_state: None }
    }

    pub fn sampled(&self) -> bool {
        self.trace_flags & 0x01 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropagationDecision {
    Propagate,
    StartNewTrace,
    Drop,
}

pub trait TraceContextInjector {
    fn inject(&self, envelope: &mut SignalEnvelope);
}

pub trait TraceContextExtractor {
    fn extract(&self, envelope: &SignalEnvelope) -> PropagationDecision;
}

/// Default injector: set `trace_id` and add synthetic `span_id` into envelope
/// metadata when features allow.
pub struct DefaultInjector;

impl TraceContextInjector for DefaultInjector {
    fn inject(&self, envelope: &mut SignalEnvelope) {
        if envelope.trace_id.is_none() {
            envelope.trace_id = Some(TraceId::generate());
        }
    }
}

/// Default extractor: if the envelope carries a trace id, propagate it;
/// otherwise start a new trace root.
pub struct DefaultExtractor;

impl TraceContextExtractor for DefaultExtractor {
    fn extract(&self, envelope: &SignalEnvelope) -> PropagationDecision {
        if envelope.trace_id.is_some() {
            PropagationDecision::Propagate
        } else {
            PropagationDecision::StartNewTrace
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::CorrelationId;
    use crate::signal::{SignalKind, VectorClock};
    use crate::version::SchemaVersion;

    fn make_envelope() -> SignalEnvelope {
        SignalEnvelope {
            msg_id: crate::id::MsgId::new("trace-test"),
            correlation_id: CorrelationId::new("trace-corr"),
            trace_id: None,
            span_id: None,
            signal_type: "TraceTest".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 0,
            kind: SignalKind::Command,
            source_layer: crate::RuntimeTier::Exec,
            target_layer: crate::RuntimeTier::Exec,
            source_cell: None,
            target_cell: None,
            payload: serde_json::Value::Null,
            schema_version: SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
            reply_to: None,
            request_id: None,
        }
    }

    #[test]
    fn injector_sets_trace_id_when_missing() {
        let mut env = make_envelope();
        DefaultInjector.inject(&mut env);
        assert!(env.trace_id.is_some());
    }

    #[test]
    fn extractor_propagates_existing_trace_id() {
        let mut env = make_envelope();
        env.trace_id = Some(TraceId::new("existing-trace"));
        let decision = DefaultExtractor.extract(&env);
        assert_eq!(decision, PropagationDecision::Propagate);
    }

    #[test]
    fn extractor_starts_new_trace_when_missing() {
        let env = make_envelope();
        let decision = DefaultExtractor.extract(&env);
        assert_eq!(decision, PropagationDecision::StartNewTrace);
    }
}
