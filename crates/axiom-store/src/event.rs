//! Event - immutable fact stored in the event log.

use axiom_kernel::id::{CorrelationId, MsgId};
use axiom_kernel::layer::Layer;
use axiom_kernel::signal::VectorClock;
use axiom_kernel::clock::global_clock;
use axiom_kernel::version::{EventSchema, SchemaVersion, Versioned};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EventOutcome {
    #[default]
    Success,
    Failed {
        reason: String,
    },
    AxiomViolated {
        axiom_name: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessHashData {
    pub prev_hash: Option<[u8; 32]>,
    pub state_before_hash: Option<[u8; 32]>,
    pub state_after_hash: Option<[u8; 32]>,
    pub hash: [u8; 32],
    pub signal_fingerprint: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub layer: Layer,
    pub processing_time_ms: u64,
    pub was_replayed: bool,
    pub outcome: EventOutcome,
    pub summary: String,
    pub witness_hash: Option<WitnessHashData>,
    pub payload_size_bytes: usize,
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self {
            layer: Layer::Exec,
            processing_time_ms: 0,
            was_replayed: false,
            outcome: EventOutcome::Success,
            summary: String::new(),
            witness_hash: None,
            payload_size_bytes: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub sequence_number: u64,
    pub event_id: String,
    pub aggregate_id: String,
    pub cell_id: String,
    pub correlation_id: CorrelationId,
    pub triggering_msg_id: Option<MsgId>,
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
    pub payload: serde_json::Value,
    pub event_type: String,
    pub schema_version: SchemaVersion,
    pub metadata: EventMetadata,
}

impl Event {
    pub fn new(aggregate_id: &str, event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            sequence_number: 0,
            event_id: format!("evt-{}", axiom_kernel::signal::now_ns()),
            aggregate_id: aggregate_id.to_string(),
            cell_id: String::new(),
            correlation_id: CorrelationId::new("system"),
            triggering_msg_id: None,
            vector_clock: VectorClock::new(),
            timestamp_ns: global_clock().now_ns(),
            payload,
            event_type: event_type.to_string(),
            schema_version: EventSchema::schema_version(),
            metadata: EventMetadata::default(),
        }
    }
}

pub struct EventBuilder {
    event: Event,
}

impl EventBuilder {
    pub fn new(aggregate_id: &str, event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            event: Event::new(aggregate_id, event_type, payload),
        }
    }

    pub fn cell_id(mut self, cell_id: &str) -> Self {
        self.event.cell_id = cell_id.to_string();
        self
    }

    pub fn correlation_id(mut self, cid: CorrelationId) -> Self {
        self.event.correlation_id = cid;
        self
    }

    pub fn triggering_msg_id(mut self, msg_id: MsgId) -> Self {
        self.event.triggering_msg_id = Some(msg_id);
        self
    }

    pub fn vector_clock(mut self, vc: VectorClock) -> Self {
        self.event.vector_clock = vc;
        self
    }

    pub fn layer(mut self, layer: Layer) -> Self {
        self.event.metadata.layer = layer;
        self
    }

    pub fn processing_time_ms(mut self, ms: u64) -> Self {
        self.event.metadata.processing_time_ms = ms;
        self
    }

    pub fn was_replayed(mut self, replayed: bool) -> Self {
        self.event.metadata.was_replayed = replayed;
        self
    }

    pub fn schema_version(mut self, v: SchemaVersion) -> Self {
        self.event.schema_version = v;
        self
    }

    pub fn event_id(mut self, id: &str) -> Self {
        self.event.event_id = id.to_string();
        self
    }

    pub fn timestamp_ns(mut self, ts: u64) -> Self {
        self.event.timestamp_ns = ts;
        self
    }

    pub fn outcome(mut self, outcome: EventOutcome) -> Self {
        self.event.metadata.outcome = outcome;
        self
    }

    pub fn summary(mut self, summary: &str) -> Self {
        self.event.metadata.summary = summary.to_string();
        self
    }

    pub fn witness_hash(mut self, hash: WitnessHashData) -> Self {
        self.event.metadata.witness_hash = Some(hash);
        self
    }

    pub fn payload_size_bytes(mut self, size: usize) -> Self {
        self.event.metadata.payload_size_bytes = size;
        self
    }

    pub fn build(self) -> Event {
        self.event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_builder_creates_event_with_all_fields() {
        let cid = CorrelationId::new("test-correlation");
        let mut vc = VectorClock::new();
        vc.increment("cell-1");
        let event = EventBuilder::new("agg-1", "greeting", serde_json::json!({"msg": "hello"}))
            .cell_id("cell-1")
            .correlation_id(cid.clone())
            .vector_clock(vc.clone())
            .layer(Layer::Exec)
            .processing_time_ms(42)
            .build();

        assert_eq!(event.aggregate_id, "agg-1");
        assert_eq!(event.event_type, "greeting");
        assert_eq!(event.cell_id, "cell-1");
        assert_eq!(event.correlation_id.as_str(), "test-correlation");
        assert_eq!(event.metadata.layer, Layer::Exec);
        assert_eq!(event.metadata.processing_time_ms, 42);
        assert!(!event.metadata.was_replayed);
        assert_eq!(event.payload["msg"], "hello");
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let event = EventBuilder::new("agg-1", "test-event", serde_json::json!({"x": 1}))
            .cell_id("c1")
            .layer(Layer::Validate)
            .build();

        let json = serde_json::to_string(&event).unwrap();
        let de: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(de.event_id, event.event_id);
        assert_eq!(de.aggregate_id, "agg-1");
        assert_eq!(de.event_type, "test-event");
        assert_eq!(de.schema_version, event.schema_version);
        assert_eq!(de.metadata.layer, Layer::Validate);
        assert_eq!(de.sequence_number, 0);
    }

    #[test]
    fn test_event_default_metadata() {
        let event = Event::new("a", "b", serde_json::Value::Null);
        assert!(!event.metadata.was_replayed);
        assert_eq!(event.metadata.processing_time_ms, 0);
    }
}
