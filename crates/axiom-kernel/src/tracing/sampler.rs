//! Trace sampling strategies.
//!
//! Samplers decide whether a newly created span should be recorded and exported.

use crate::id::{SpanId, TraceId};

/// Sampler that decides recording/exporting for a span.
pub trait TraceSampler: Send + Sync {
    fn should_sample(
        &self,
        trace_id: &TraceId,
        parent_span_id: Option<&SpanId>,
        name: &str,
    ) -> SamplingDecision;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingDecision {
    RecordAndSample,
    RecordOnly,
    Drop,
}

/// Always-on sampler: records and exports every span.
pub struct AlwaysOnSampler;

impl TraceSampler for AlwaysOnSampler {
    fn should_sample(&self, _trace_id: &TraceId, _parent_span_id: Option<&SpanId>, _name: &str) -> SamplingDecision {
        SamplingDecision::RecordAndSample
    }
}

/// Probabilistic sampler: records a fixed fraction of traces.
pub struct ProbabilisticSampler {
    pub probability: f64,
}

impl ProbabilisticSampler {
    pub fn new(probability: f64) -> Self {
        Self { probability: probability.clamp(0.0, 1.0) }
    }
}

impl TraceSampler for ProbabilisticSampler {
    fn should_sample(&self, _trace_id: &TraceId, _parent_span_id: Option<&SpanId>, _name: &str) -> SamplingDecision {
        if rand::random::<f64>() < self.probability {
            SamplingDecision::RecordAndSample
        } else {
            SamplingDecision::Drop
        }
    }
}

/// Parent-based sampler: if there is a remote parent sampled span, follow it;
/// otherwise use the root sampler.
pub struct ParentBasedSampler {
    pub root: Box<dyn TraceSampler>,
}

impl TraceSampler for ParentBasedSampler {
    fn should_sample(
        &self,
        trace_id: &TraceId,
        parent_span_id: Option<&SpanId>,
        _name: &str,
    ) -> SamplingDecision {
        match parent_span_id {
            Some(_) => SamplingDecision::RecordAndSample,
            None => self.root.should_sample(trace_id, None, ""),
        }
    }
}
