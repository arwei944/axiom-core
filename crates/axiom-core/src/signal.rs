//! Signal - Typed immutable message with causal tracking (Vector Clock, correlation).
//!
//! Every Signal has required fields (no defaults that panic):
//! - msg_id: unique id for idempotency
//! - correlation_id: trace propagation chain
//! - vector_clock: causal ordering
//! - timestamp_ns: freshness
//! - kind: Command/Event/Query/Response
//! - layer: which layer this signal originates from
//!
//! SignalEnvelope provides type-erased wrapping for the message bus.

use crate::clock::global_clock;
use crate::id::{CorrelationId, MsgId, TraceId};
use crate::layer::Layer;
use crate::schema::ValidationResult;
use crate::version::SchemaVersion;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock(pub HashMap<String, u64>);

impl VectorClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment(&mut self, cell_id: &str) {
        *self.0.entry(cell_id.to_string()).or_insert(0) += 1;
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (key, value) in &other.0 {
            let entry = self.0.entry(key.clone()).or_insert(0);
            *entry = (*entry).max(*value);
        }
    }

    pub fn causally_precedes(&self, other: &VectorClock) -> bool {
        for (key, &self_val) in &self.0 {
            match other.0.get(key) {
                Some(&other_val) if self_val > other_val => return false,
                None if self_val > 0 => return false,
                _ => {}
            }
        }
        true
    }

    pub fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.causally_precedes(other) && !other.causally_precedes(self)
    }

    pub fn get(&self, cell_id: &str) -> u64 {
        self.0.get(cell_id).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalKind {
    Command,
    Event,
    Query,
    Response,
}

pub trait Signal: Send + Sync + 'static {
    fn signal_type(&self) -> &'static str;
    fn msg_id(&self) -> &MsgId;
    fn correlation_id(&self) -> &CorrelationId;
    fn trace_id(&self) -> Option<&TraceId> {
        None
    }
    fn vector_clock(&self) -> &VectorClock;
    fn timestamp_ns(&self) -> u64;
    fn kind(&self) -> SignalKind;
    fn layer(&self) -> Layer;
    fn sender(&self) -> Option<&str> {
        None
    }
    fn schema_version(&self) -> SchemaVersion {
        SchemaVersion::new(1)
    }

    fn as_any(&self) -> &dyn std::any::Any;
    fn clone_signal(&self) -> Box<dyn Signal>;
    fn validate(&self) -> ValidationResult;
    fn serialize_to_json(&self) -> crate::Result<serde_json::Value>;
}

impl Clone for Box<dyn Signal> {
    fn clone(&self) -> Self {
        self.clone_signal()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalEnvelope {
    pub msg_id: MsgId,
    pub correlation_id: CorrelationId,
    pub trace_id: Option<TraceId>,
    pub signal_type: String,
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
    pub kind: SignalKind,
    pub source_layer: Layer,
    pub target_layer: Layer,
    pub source_cell: Option<String>,
    pub target_cell: Option<String>,
    pub payload: serde_json::Value,
    pub schema_version: SchemaVersion,
    pub parent_msg_id: Option<MsgId>,
    pub hop_count: u32,
}

impl SignalEnvelope {
    pub fn new<S: Signal>(signal: &S, target_layer: Layer) -> crate::Result<Self> {
        Ok(Self {
            msg_id: signal.msg_id().clone(),
            correlation_id: signal.correlation_id().clone(),
            trace_id: signal.trace_id().cloned(),
            signal_type: signal.signal_type().to_string(),
            vector_clock: signal.vector_clock().clone(),
            timestamp_ns: signal.timestamp_ns(),
            kind: signal.kind(),
            source_layer: signal.layer(),
            target_layer,
            source_cell: signal.sender().map(|s| s.to_string()),
            target_cell: None,
            payload: signal.serialize_to_json()?,
            schema_version: signal.schema_version(),
            parent_msg_id: None,
            hop_count: 0,
        })
    }

    pub fn to_cell<S: Signal>(
        signal: &S,
        target_cell: &str,
        target_layer: Layer,
    ) -> crate::Result<Self> {
        let mut env = Self::new(signal, target_layer)?;
        env.target_cell = Some(target_cell.to_string());
        Ok(env)
    }

    pub fn reply_to<S: Signal>(
        signal: &S,
        original: &SignalEnvelope,
        target_layer: Layer,
    ) -> crate::Result<Self> {
        let mut env = Self::new(signal, target_layer)?;
        env.correlation_id = original.correlation_id.clone();
        env.trace_id = original.trace_id.clone();
        env.parent_msg_id = Some(original.msg_id.clone());
        env.hop_count = original.hop_count + 1;
        env.target_cell = original.source_cell.clone();
        Ok(env)
    }

    pub fn validate_layer_transition(&self) -> crate::Result<()> {
        if !self.source_layer.can_send_to(self.target_layer) {
            return Err(crate::AxiomError::LayerViolation {
                from: self.source_layer,
                to: self.target_layer,
                signal_type: self.signal_type.clone(),
                source_cell: self.source_cell.clone().unwrap_or_default(),
            });
        }
        Ok(())
    }

    pub fn validate_payload_size(&self, max_bytes: usize) -> crate::Result<()> {
        if max_bytes > 0 {
            let payload_bytes = self.payload.to_string().len();
            if payload_bytes > max_bytes {
                return Err(crate::AxiomError::SignalValidation {
                    signal_type: self.signal_type.clone(),
                    message: format!(
                        "payload size {} bytes exceeds max {} bytes",
                        payload_bytes, max_bytes
                    ),
                });
            }
        }
        Ok(())
    }

    pub fn increment_hop(&mut self) -> crate::Result<()> {
        self.hop_count += 1;
        const MAX_HOPS: u32 = 8;
        if self.hop_count > MAX_HOPS {
            return Err(crate::AxiomError::HandoffLimitExceeded {
                msg_id: self.msg_id.to_string(),
                hops: self.hop_count,
                correlation_id: self.correlation_id.to_string(),
            });
        }
        Ok(())
    }

    pub fn is_fresh(&self, max_age_ns: u64) -> bool {
        let now = global_clock().now_ns();
        now.saturating_sub(self.timestamp_ns) <= max_age_ns
    }
}

pub fn now_ns() -> u64 {
    global_clock().now_ns()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{CorrelationId, MsgId};

    #[test]
    fn test_vector_clock_increment() {
        let mut vc = VectorClock::new();
        vc.increment("cell-a");
        vc.increment("cell-a");
        assert_eq!(vc.get("cell-a"), 2);
        assert_eq!(vc.get("cell-b"), 0);
    }

    #[test]
    fn test_vector_clock_causally_precedes() {
        let mut vc1 = VectorClock::new();
        vc1.increment("a");
        let mut vc2 = vc1.clone();
        vc2.increment("b");
        assert!(vc1.causally_precedes(&vc2));
        assert!(!vc2.causally_precedes(&vc1));
    }

    #[test]
    fn test_vector_clock_merge() {
        let mut vc1 = VectorClock::new();
        vc1.increment("a");
        let mut vc2 = VectorClock::new();
        vc2.increment("b");
        vc1.merge(&vc2);
        assert_eq!(vc1.get("a"), 1);
        assert_eq!(vc1.get("b"), 1);
    }

    #[test]
    fn test_layer_validation() {
        let cmd = TestCommand::new("test");
        let env = SignalEnvelope::new(&cmd, Layer::Exec).unwrap();
        assert!(env.validate_layer_transition().is_ok());

        let mut env2 = SignalEnvelope::new(&cmd, Layer::Exec).unwrap();
        env2.target_layer = Layer::Validate;
        assert!(env2.validate_layer_transition().is_err());

        let mut bad_env = SignalEnvelope::new(&cmd, Layer::Exec).unwrap();
        bad_env.source_layer = Layer::Exec;
        bad_env.target_layer = Layer::Agent;
        assert!(bad_env.validate_layer_transition().is_err());
    }

    #[test]
    fn test_hop_limit() {
        let cmd = TestCommand::new("test");
        let mut env = SignalEnvelope::new(&cmd, Layer::Exec).unwrap();
        for _ in 0..8 {
            env.increment_hop().unwrap();
        }
        assert!(env.increment_hop().is_err());
    }

    #[test]
    fn test_signal_envelope_payload_serialization() {
        let cmd = TestCommand::new("hello");
        let env = SignalEnvelope::new(&cmd, Layer::Exec).unwrap();
        assert_eq!(env.signal_type, "TestCommand");
        assert_eq!(env.payload["payload"], serde_json::json!("hello"));
        assert_eq!(env.schema_version, SchemaVersion::new(1));
    }

    #[test]
    fn test_box_dyn_signal_clone() {
        let cmd = TestCommand::new("clone-test");
        let boxed: Box<dyn Signal> = Box::new(cmd);
        let cloned = boxed.clone();
        assert_eq!(cloned.signal_type(), "TestCommand");
        assert_eq!(cloned.msg_id().as_str(), "test-msg");
    }

    #[test]
    fn test_signal_reply_to_sets_correlation() {
        let cmd = TestCommand::new("original");
        let original_env = SignalEnvelope::new(&cmd, Layer::Exec).unwrap();
        let reply = TestCommand::new("reply");
        let reply_env = SignalEnvelope::reply_to(&reply, &original_env, Layer::Exec).unwrap();
        assert_eq!(reply_env.correlation_id.as_str(), "test-corr");
        assert_eq!(reply_env.parent_msg_id.unwrap().as_str(), "test-msg");
        assert_eq!(reply_env.hop_count, 1);
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestCommand {
        msg_id: MsgId,
        correlation_id: CorrelationId,
        vector_clock: VectorClock,
        payload: String,
    }

    impl TestCommand {
        fn new(p: &str) -> Self {
            Self {
                msg_id: MsgId::new("test-msg"),
                correlation_id: CorrelationId::new("test-corr"),
                vector_clock: VectorClock::new(),
                payload: p.to_string(),
            }
        }
    }

    impl Signal for TestCommand {
        fn signal_type(&self) -> &'static str {
            "TestCommand"
        }
        fn msg_id(&self) -> &MsgId {
            &self.msg_id
        }
        fn correlation_id(&self) -> &CorrelationId {
            &self.correlation_id
        }
        fn vector_clock(&self) -> &VectorClock {
            &self.vector_clock
        }
        fn timestamp_ns(&self) -> u64 {
            now_ns()
        }
        fn kind(&self) -> SignalKind {
            SignalKind::Command
        }
        fn layer(&self) -> Layer {
            Layer::Exec
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn clone_signal(&self) -> Box<dyn Signal> {
            Box::new(self.clone())
        }
        fn validate(&self) -> ValidationResult {
            ValidationResult::ok()
        }
        fn serialize_to_json(&self) -> crate::Result<serde_json::Value> {
            serde_json::to_value(self)
                .map_err(|e| crate::AxiomError::SignalSerialization {
                    signal_type: "TestSignal".into(),
                    message: e.to_string(),
                })
        }
    }
}
