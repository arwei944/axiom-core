//! Signal - Typed immutable message with causal tracking (Vector Clock, correlation).
//!
//! Every Signal has required fields (no defaults that panic):
//! - msg_id: unique id for idempotency
//! - correlation_id: trace propagation chain
//! - vector_clock: causal ordering
//! - timestamp_ns: freshness
//! - kind: Command/Event/Query
//! - layer: which layer this signal originates from
//!
//! SignalEnvelope provides type-erased wrapping for the message bus.

use crate::id::{CorrelationId, MsgId, TraceId};
use crate::layer::Layer;
use crate::schema::{Schema, ValidationResult};
use crate::version::{SchemaVersion, SignalSchema, Versioned};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Vector Clock for causal ordering - tracks logical time across Cells.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock(pub HashMap<String, u64>);

impl VectorClock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the counter for a given cell after processing an event.
    pub fn increment(&mut self, cell_id: &str) {
        *self.0.entry(cell_id.to_string()).or_insert(0) += 1;
    }

    /// Merge another vector clock (takes max for each entry) on message receive.
    pub fn merge(&mut self, other: &VectorClock) {
        for (key, value) in &other.0 {
            let entry = self.0.entry(key.clone()).or_insert(0);
            *entry = (*entry).max(*value);
        }
    }

    /// Check if this clock causally precedes another (this happens-before other).
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

    /// Check for concurrent (non-orderable) events.
    pub fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.causally_precedes(other) && !other.causally_precedes(self)
    }

    /// Get the counter for a specific cell.
    pub fn get(&self, cell_id: &str) -> u64 {
        self.0.get(cell_id).copied().unwrap_or(0)
    }
}

/// Signal categories - determines routing and processing semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalKind {
    Command,
    Event,
    Query,
    Response,
}

/// Base trait for all signals - NO default implementations for required fields.
///
/// All methods are required (no unimplemented!() defaults), ensuring every
/// signal implementation provides complete metadata.
pub trait Signal: Send + Sync + 'static + Schema + serde::Serialize {
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
        SignalSchema::schema_version()
    }
}

/// Type-erased signal envelope for the message bus.
///
/// Contains all metadata needed for routing, validation, and tracing,
/// with the payload as a serialized JSON value.
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
    pub fn new<S: Signal>(signal: &S, target_layer: Layer) -> Self {
        Self {
            msg_id: signal.msg_id().clone(),
            correlation_id: signal.correlation_id().clone(),
            trace_id: signal.trace_id().cloned(),
            signal_type: signal.signal_type().to_string(),
            vector_clock: signal.vector_clock().clone(),
            timestamp_ns: now_ns(),
            kind: signal.kind(),
            source_layer: signal.layer(),
            target_layer,
            source_cell: signal.sender().map(|s| s.to_string()),
            target_cell: None,
            payload: serde_json::to_value(signal).unwrap_or(serde_json::Value::Null),
            schema_version: signal.schema_version(),
            parent_msg_id: None,
            hop_count: 0,
        }
    }

    pub fn to_cell<S: Signal>(signal: &S, target_cell: &str, target_layer: Layer) -> Self {
        let mut env = Self::new(signal, target_layer);
        env.target_cell = Some(target_cell.to_string());
        env
    }

    pub fn reply_to<S: Signal>(signal: &S, original: &SignalEnvelope, target_layer: Layer) -> Self {
        let mut env = Self::new(signal, target_layer);
        env.correlation_id = original.correlation_id.clone();
        env.trace_id = original.trace_id.clone();
        env.parent_msg_id = Some(original.msg_id.clone());
        env.hop_count = original.hop_count + 1;
        env.target_cell = original.source_cell.clone();
        env
    }

    pub fn validate_layer_transition(&self) -> crate::Result<()> {
        if !self.source_layer.can_send_to(self.target_layer) {
            return Err(crate::AxiomError::LayerViolation {
                from: self.source_layer,
                to: self.target_layer,
                signal_type: self.signal_type.clone(),
            });
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
            });
        }
        Ok(())
    }

    pub fn is_fresh(&self, max_age_ns: u64) -> bool {
        let now = now_ns();
        now.saturating_sub(self.timestamp_ns) <= max_age_ns
    }
}

/// Current time in nanoseconds since UNIX epoch.
pub fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// Clone box for dyn Signal compatibility.
pub trait SignalClone: Send + Sync {
    fn clone_box(&self) -> Box<dyn SignalDyn>;
}

/// Object-safe trait for dyn dispatch (no generics).
pub trait SignalDyn: SignalClone + Send + Sync + 'static {
    fn signal_type_dyn(&self) -> &'static str;
    fn msg_id_dyn(&self) -> &MsgId;
    fn correlation_id_dyn(&self) -> &CorrelationId;
    fn vector_clock_dyn(&self) -> &VectorClock;
    fn timestamp_ns_dyn(&self) -> u64;
    fn kind_dyn(&self) -> SignalKind;
    fn layer_dyn(&self) -> Layer;
    fn validate_dyn(&self) -> ValidationResult;
}

impl<T: Signal + Clone + serde::Serialize> SignalClone for T {
    fn clone_box(&self) -> Box<dyn SignalDyn> {
        Box::new(self.clone())
    }
}

impl<T: Signal + Clone + serde::Serialize> SignalDyn for T {
    fn signal_type_dyn(&self) -> &'static str {
        self.signal_type()
    }
    fn msg_id_dyn(&self) -> &MsgId {
        self.msg_id()
    }
    fn correlation_id_dyn(&self) -> &CorrelationId {
        self.correlation_id()
    }
    fn vector_clock_dyn(&self) -> &VectorClock {
        self.vector_clock()
    }
    fn timestamp_ns_dyn(&self) -> u64 {
        self.timestamp_ns()
    }
    fn kind_dyn(&self) -> SignalKind {
        self.kind()
    }
    fn layer_dyn(&self) -> Layer {
        self.layer()
    }
    fn validate_dyn(&self) -> ValidationResult {
        <Self as Schema>::validate(self)
    }
}

impl Clone for Box<dyn SignalDyn> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let env = SignalEnvelope::new(&cmd, Layer::Exec);
        assert!(env.validate_layer_transition().is_ok());

        let env2 = SignalEnvelope::new(&cmd, Layer::Validate);
        assert!(env2.validate_layer_transition().is_err());

        let mut bad_env = SignalEnvelope::new(&cmd, Layer::Exec);
        bad_env.source_layer = Layer::Exec;
        bad_env.target_layer = Layer::Agent;
        assert!(bad_env.validate_layer_transition().is_err());
    }

    #[test]
    fn test_hop_limit() {
        let cmd = TestCommand::new("test");
        let mut env = SignalEnvelope::new(&cmd, Layer::Exec);
        for _ in 0..8 {
            env.increment_hop().unwrap();
        }
        assert!(env.increment_hop().is_err());
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestCommand {
        msg_id: MsgId,
        correlation_id: CorrelationId,
        vector_clock: VectorClock,
        payload: String,
    }

    impl TestCommand {
        fn new(payload: &str) -> Self {
            Self {
                msg_id: MsgId::new("test-msg"),
                correlation_id: CorrelationId::new("test-corr"),
                vector_clock: VectorClock::new(),
                payload: payload.to_string(),
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
    }

    impl Schema for TestCommand {
        fn validate(&self) -> ValidationResult {
            ValidationResult::ok()
        }
    }
}
